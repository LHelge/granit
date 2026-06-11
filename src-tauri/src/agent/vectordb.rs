use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use log::{debug, info, warn};
use parking_lot::Mutex;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::commands::SharedCave;

use super::AgentError;

/// On-disk cache format for persisted embeddings.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct EmbeddingCache {
    model_name: String,
    entries: Vec<CacheEntry>,
}

/// A single cached embedding entry.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct CacheEntry {
    slug: String,
    mtime_secs: i64,
    mtime_nanos: u32,
    vector: Vec<f32>,
}

/// In-memory embedding entry for a single note.
struct EmbeddingEntry {
    mtime: SystemTime,
    vector: Vec<f32>,
}

/// Shared, cloneable vector index over cave notes.
///
/// Wraps an `Arc<Inner>` so it can be held in `AppState` and passed to the
/// agent builder. Uses fastembed for local CPU-based embeddings.
#[derive(Clone)]
pub(crate) struct CaveVectorIndex {
    inner: Arc<CaveVectorIndexInner>,
}

struct CaveVectorIndexInner {
    model: Arc<Mutex<fastembed::TextEmbedding>>,
    model_name: String,
    entries: RwLock<HashMap<String, EmbeddingEntry>>,
    cave: SharedCave,
    cache_path: PathBuf,
    /// Set when this index has been replaced (cave switch, RAG config
    /// change). An in-flight [`rebuild`] checks it between embedding chunks
    /// and aborts instead of racing the successor on the cache file.
    cancelled: std::sync::atomic::AtomicBool,
}

impl CaveVectorIndex {
    /// Create a new vector index for the given cave.
    ///
    /// When `bundled_model_dir` is `Some` and `model_name` is the default
    /// quantized model, the ONNX + tokenizer files are loaded directly from
    /// that directory (shipped with the app). Otherwise the model is
    /// downloaded via HuggingFace Hub into `models_dir`.
    ///
    /// Loads cached embeddings from `.granit/embeddings.bin` if the file
    /// exists and the model name matches. Does **not** rebuild — call
    /// [`rebuild`] afterwards (typically on a background task).
    pub(crate) fn new(
        cave: SharedCave,
        model_name: &str,
        models_dir: &Path,
        bundled_model_dir: Option<&Path>,
    ) -> Result<Self, AgentError> {
        let t0 = Instant::now();
        let model = if model_name == DEFAULT_EMBEDDING_MODEL {
            if let Some(dir) = bundled_model_dir {
                info!("loading bundled embedding model from {}", dir.display());
                load_bundled_model(dir)?
            } else {
                info!("downloading embedding model {model_name} from HuggingFace");
                load_model_hf(model_name, models_dir)?
            }
        } else {
            info!("downloading embedding model {model_name} from HuggingFace");
            load_model_hf(model_name, models_dir)?
        };
        info!("embedding model loaded in {:.1?}", t0.elapsed());

        let cache_path = {
            let guard = cave.lock();
            let cave_ref = guard.as_ref().ok_or_else(|| {
                AgentError::Embedding("cave must be open when building the vector index".into())
            })?;
            cave_ref.path().join(".granit").join("embeddings.bin")
        };

        let entries = load_cache(&cache_path, model_name);
        debug!(
            "loaded {} cached embeddings from {}",
            entries.len(),
            cache_path.display()
        );

        Ok(Self {
            inner: Arc::new(CaveVectorIndexInner {
                model: Arc::new(Mutex::new(model)),
                model_name: model_name.to_string(),
                entries: RwLock::new(entries),
                cave,
                cache_path,
                cancelled: std::sync::atomic::AtomicBool::new(false),
            }),
        })
    }

    /// Mark this index as superseded, aborting any in-flight rebuild at the
    /// next chunk boundary.
    pub(crate) fn cancel(&self) {
        self.inner
            .cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn is_cancelled(&self) -> bool {
        self.inner
            .cancelled
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Rebuild the index: compare in-memory entries against the cave's current
    /// notes, re-embed any new or modified notes, and persist the cache.
    pub(crate) async fn rebuild(&self) -> Result<(), AgentError> {
        let rebuild_start = Instant::now();

        // Snapshot cached entry mtimes so we can compare without holding the
        // async RwLock across the sync cave mutex.
        let cached_mtimes: HashMap<String, SystemTime> = {
            let entries = self.inner.entries.read().await;
            entries.iter().map(|(k, v)| (k.clone(), v.mtime)).collect()
        };

        let (slugs_with_mtimes, texts_to_embed) = {
            // Collect slug → mtime from the cave, and identify which need (re-)embedding.
            let cave_guard = self.inner.cave.lock();
            let cave = cave_guard.as_ref().ok_or(AgentError::Embedding(
                "cave not open during rebuild".to_string(),
            ))?;

            let note_paths = cave.note_paths();

            let mut slugs_with_mtimes: Vec<(String, SystemTime)> = Vec::new();
            let mut need_embedding: Vec<(String, SystemTime)> = Vec::new();

            for (slug, abs_path) in note_paths {
                let mtime = file_mtime(abs_path);
                slugs_with_mtimes.push((slug.clone(), mtime));

                let needs_update = match cached_mtimes.get(slug.as_str()) {
                    Some(existing) => *existing != mtime,
                    None => true,
                };

                if needs_update {
                    need_embedding.push((slug.clone(), mtime));
                }
            }

            // Read note bodies for notes that need embedding (still under cave lock).
            let texts_to_embed: Vec<(String, SystemTime, String)> = need_embedding
                .into_iter()
                .filter_map(|(slug, mtime)| {
                    let doc = cave.read_note(&slug).ok()?;
                    Some((slug, mtime, doc.content))
                })
                .collect();

            (slugs_with_mtimes, texts_to_embed)
        };

        let cached_count = slugs_with_mtimes.len() - texts_to_embed.len();
        debug!(
            "rebuild: {} notes total, {} cached, {} to embed",
            slugs_with_mtimes.len(),
            cached_count,
            texts_to_embed.len()
        );

        // Batch-embed on a blocking thread (fastembed is synchronous), in
        // chunks so memory stays bounded and a superseded rebuild can abort
        // between chunks instead of running to completion.
        let mut pending = texts_to_embed;
        let mut new_embeddings: Vec<(String, SystemTime, Vec<f32>)> =
            Vec::with_capacity(pending.len());
        while !pending.is_empty() {
            if self.is_cancelled() {
                info!("vector index rebuild cancelled (index superseded)");
                return Ok(());
            }

            let batch: Vec<(String, SystemTime, String)> = pending
                .drain(..pending.len().min(EMBED_CHUNK_SIZE))
                .collect();
            let docs: Vec<String> = batch.iter().map(|(_, _, text)| text.clone()).collect();

            let vectors = tokio::task::spawn_blocking({
                let model = Arc::clone(&self.inner.model);
                move || {
                    let mut model = model.lock();
                    model.embed(docs, None)
                }
            })
            .await
            .map_err(|e| AgentError::Embedding(format!("spawn_blocking panicked: {e}")))?
            .map_err(|e| AgentError::Embedding(e.to_string()))?;

            new_embeddings.extend(
                batch
                    .into_iter()
                    .zip(vectors)
                    .map(|((slug, mtime, _), vec)| (slug, mtime, vec)),
            );
        }

        // A cancelled rebuild must not touch the store or the cache file —
        // they now belong to the successor index.
        if self.is_cancelled() {
            info!("vector index rebuild cancelled (index superseded)");
            return Ok(());
        }

        // Update in-memory store.
        let current_slugs: std::collections::HashSet<&str> =
            slugs_with_mtimes.iter().map(|(s, _)| s.as_str()).collect();

        {
            let mut entries = self.inner.entries.write().await;

            // Remove entries for notes that no longer exist.
            entries.retain(|slug, _| current_slugs.contains(slug.as_str()));

            // Insert new/updated embeddings.
            for (slug, mtime, vector) in new_embeddings {
                entries.insert(slug, EmbeddingEntry { mtime, vector });
            }
        }

        // Persist to disk.
        self.save_cache().await;

        info!(
            "vector index rebuilt in {:.1?} ({} embeddings)",
            rebuild_start.elapsed(),
            slugs_with_mtimes.len()
        );

        Ok(())
    }

    /// Re-embed a single note after it has been saved or created.
    pub(crate) async fn update_note(&self, slug: &str) {
        let data = {
            let cave_guard = self.inner.cave.lock();
            let cave = match cave_guard.as_ref() {
                Some(c) => c,
                None => {
                    debug!("skipping embedding update for {slug}: no cave open");
                    return;
                }
            };
            let abs_path = match cave.note_paths().get(slug) {
                Some(p) => p.clone(),
                None => {
                    debug!("skipping embedding update for {slug}: not in note index");
                    return;
                }
            };
            let mtime = file_mtime(&abs_path);
            let content = match cave.read_note(slug) {
                Ok(doc) => doc.content,
                Err(e) => {
                    warn!("skipping embedding update for {slug}: failed to read note: {e}");
                    return;
                }
            };
            (mtime, content)
        };

        let (mtime, content) = data;

        let vector = {
            let model = Arc::clone(&self.inner.model);
            let doc = content;
            match tokio::task::spawn_blocking(move || {
                let mut model = model.lock();
                model.embed(vec![doc], None)
            })
            .await
            {
                Ok(Ok(mut vecs)) if !vecs.is_empty() => vecs.remove(0),
                Ok(Ok(_)) => {
                    warn!("embedding update for {slug} returned no vectors");
                    return;
                }
                Ok(Err(e)) => {
                    warn!("embedding update for {slug} failed: {e}");
                    return;
                }
                Err(e) => {
                    warn!("embedding update task for {slug} panicked: {e}");
                    return;
                }
            }
        };

        {
            let mut entries = self.inner.entries.write().await;
            entries.insert(slug.to_string(), EmbeddingEntry { mtime, vector });
        }

        self.save_cache().await;
    }

    /// Remove the embedding for a deleted note.
    pub(crate) async fn remove_note(&self, slug: &str) {
        {
            let mut entries = self.inner.entries.write().await;
            entries.remove(slug);
        }
        self.save_cache().await;
    }

    /// Embed a query string and find the top N most similar note slugs.
    pub(crate) async fn search(
        &self,
        query: &str,
        n: usize,
        threshold: Option<f64>,
    ) -> Result<Vec<(f64, String)>, AgentError> {
        let query_vec = {
            let model = Arc::clone(&self.inner.model);
            let q = query.to_string();
            tokio::task::spawn_blocking(move || {
                let mut model = model.lock();
                model.embed(vec![q], None)
            })
            .await
            .map_err(|e| AgentError::Embedding(format!("spawn_blocking panicked: {e}")))?
            .map_err(|e| AgentError::Embedding(e.to_string()))?
        };

        let query_vec = query_vec
            .into_iter()
            .next()
            .ok_or_else(|| AgentError::Embedding("empty embedding result".to_string()))?;

        let entries = self.inner.entries.read().await;

        let mut scored: Vec<(f64, String)> = entries
            .iter()
            .map(|(slug, entry)| {
                let score = cosine_similarity(&query_vec, &entry.vector);
                (score, slug.clone())
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        if let Some(thresh) = threshold {
            scored.retain(|(score, _)| *score >= thresh);
        }

        scored.truncate(n);
        Ok(scored)
    }

    async fn save_cache(&self) {
        let entries = self.inner.entries.read().await;

        let cache = EmbeddingCache {
            model_name: self.inner.model_name.clone(),
            entries: entries
                .iter()
                .map(|(slug, entry)| {
                    let dur = entry
                        .mtime
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default();
                    CacheEntry {
                        slug: slug.clone(),
                        mtime_secs: dur.as_secs() as i64,
                        mtime_nanos: dur.subsec_nanos(),
                        vector: entry.vector.clone(),
                    }
                })
                .collect(),
        };

        let count = cache.entries.len();
        let path = self.inner.cache_path.clone();
        let result = tokio::task::spawn_blocking(move || {
            let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&cache)
                .map_err(|e| format!("rkyv serialization failed: {e}"))?;
            crate::cave::write_atomic(&path, &bytes)
                .map_err(|e| format!("writing {}: {e}", path.display()))?;
            Ok::<usize, String>(bytes.len())
        })
        .await;

        match result {
            Ok(Ok(bytes_len)) => debug!("saved {count} embeddings ({bytes_len} bytes) to cache"),
            Ok(Err(e)) => warn!("failed to save embedding cache: {e}"),
            Err(e) => warn!("save_cache task panicked: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// VectorStoreIndex implementation
// ---------------------------------------------------------------------------

use rig_core::vector_store::request::{Filter, VectorSearchRequest};
use rig_core::vector_store::{VectorStoreError, VectorStoreIndex};

impl VectorStoreIndex for CaveVectorIndex {
    type Filter = Filter<serde_json::Value>;

    async fn top_n<T: for<'a> Deserialize<'a> + Send>(
        &self,
        req: VectorSearchRequest<Self::Filter>,
    ) -> Result<Vec<(f64, String, T)>, VectorStoreError> {
        let results = self
            .search(req.query(), req.samples() as usize, req.threshold())
            .await
            .map_err(|e| VectorStoreError::DatastoreError(Box::new(e)))?;

        let mut out = Vec::with_capacity(results.len());
        for (score, slug) in results {
            // Load note content from cave at query time.
            let content = {
                let cave_guard = self.inner.cave.lock();
                match cave_guard.as_ref() {
                    Some(cave) => cave.read_note(&slug).ok().map(|d| d.content),
                    None => None,
                }
            };

            let doc_value = serde_json::json!({
                "slug": slug,
                "content": content.unwrap_or_default(),
            });

            let doc: T = serde_json::from_value(doc_value).map_err(VectorStoreError::JsonError)?;

            out.push((score, slug, doc));
        }

        Ok(out)
    }

    async fn top_n_ids(
        &self,
        req: VectorSearchRequest<Self::Filter>,
    ) -> Result<Vec<(f64, String)>, VectorStoreError> {
        self.search(req.query(), req.samples() as usize, req.threshold())
            .await
            .map_err(|e| VectorStoreError::DatastoreError(Box::new(e)))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load the default quantized model from bundled resource files on disk.
fn load_bundled_model(dir: &Path) -> Result<fastembed::TextEmbedding, AgentError> {
    let read = |name: &str| -> Result<Vec<u8>, AgentError> {
        std::fs::read(dir.join(name))
            .map_err(|e| AgentError::Embedding(format!("failed to read bundled {name}: {e}")))
    };

    let user_model = fastembed::UserDefinedEmbeddingModel::new(
        read("model.onnx")?,
        fastembed::TokenizerFiles {
            tokenizer_file: read("tokenizer.json")?,
            config_file: read("config.json")?,
            special_tokens_map_file: read("special_tokens_map.json")?,
            tokenizer_config_file: read("tokenizer_config.json")?,
        },
    )
    .with_pooling(fastembed::Pooling::Mean)
    .with_quantization(fastembed::QuantizationMode::Dynamic);

    fastembed::TextEmbedding::try_new_from_user_defined(user_model, Default::default())
        .map_err(|e| AgentError::Embedding(e.to_string()))
}

/// Download a model from HuggingFace Hub and initialise it.
fn load_model_hf(name: &str, models_dir: &Path) -> Result<fastembed::TextEmbedding, AgentError> {
    let model_enum = parse_model_name(name)?;
    let init_options = fastembed::InitOptions::new(model_enum)
        .with_cache_dir(models_dir.to_path_buf())
        .with_show_download_progress(false);
    fastembed::TextEmbedding::try_new(init_options)
        .map_err(|e| AgentError::Embedding(e.to_string()))
}

/// Parse a model name string into a fastembed `EmbeddingModel` enum variant.
fn parse_model_name(name: &str) -> Result<fastembed::EmbeddingModel, AgentError> {
    match name {
        "AllMiniLML6V2" => Ok(fastembed::EmbeddingModel::AllMiniLML6V2),
        "AllMiniLML6V2Q" => Ok(fastembed::EmbeddingModel::AllMiniLML6V2Q),
        "BGESmallENV15" => Ok(fastembed::EmbeddingModel::BGESmallENV15),
        "BGESmallENV15Q" => Ok(fastembed::EmbeddingModel::BGESmallENV15Q),
        "BGEBaseENV15" => Ok(fastembed::EmbeddingModel::BGEBaseENV15),
        "BGEBaseENV15Q" => Ok(fastembed::EmbeddingModel::BGEBaseENV15Q),
        "BGESmallZHV15" => Ok(fastembed::EmbeddingModel::BGESmallZHV15),
        "NomicEmbedTextV15" => Ok(fastembed::EmbeddingModel::NomicEmbedTextV15),
        "NomicEmbedTextV15Q" => Ok(fastembed::EmbeddingModel::NomicEmbedTextV15Q),
        "NomicEmbedTextV1" => Ok(fastembed::EmbeddingModel::NomicEmbedTextV1),
        "MultilingualE5Small" => Ok(fastembed::EmbeddingModel::MultilingualE5Small),
        "MultilingualE5Large" => Ok(fastembed::EmbeddingModel::MultilingualE5Large),
        _ => Err(AgentError::Embedding(format!(
            "unknown embedding model: {name}"
        ))),
    }
}

const DEFAULT_EMBEDDING_MODEL: &str = "AllMiniLML6V2Q";

/// Number of documents embedded per blocking batch during a rebuild.
const EMBED_CHUNK_SIZE: usize = 32;

/// Resolve the model name from config, falling back to the default.
pub(crate) fn resolve_model_name(config_model: Option<&str>) -> &str {
    config_model.unwrap_or(DEFAULT_EMBEDDING_MODEL)
}

/// Load cached embeddings from disk. Returns an empty map on any failure
/// or if the model name doesn't match; the next rebuild re-embeds everything
/// and overwrites the cache.
fn load_cache(path: &Path, expected_model: &str) -> HashMap<String, EmbeddingEntry> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debug!("no embedding cache at {}", path.display());
            return HashMap::new();
        }
        Err(e) => {
            warn!("failed to read embedding cache {}: {e}", path.display());
            return HashMap::new();
        }
    };

    let cache: EmbeddingCache =
        match rkyv::from_bytes::<EmbeddingCache, rkyv::rancor::Error>(&bytes) {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    "embedding cache {} is corrupt, re-embedding all notes: {e}",
                    path.display()
                );
                return HashMap::new();
            }
        };

    if cache.model_name != expected_model {
        info!(
            "embedding cache model {} != configured {expected_model}, re-embedding all notes",
            cache.model_name
        );
        return HashMap::new();
    }

    cache
        .entries
        .into_iter()
        .map(|e| {
            let dur = std::time::Duration::new(e.mtime_secs as u64, e.mtime_nanos);
            let mtime = SystemTime::UNIX_EPOCH + dur;
            (
                e.slug,
                EmbeddingEntry {
                    mtime,
                    vector: e.vector,
                },
            )
        })
        .collect()
}

/// Get the filesystem modification time for a path, falling back to UNIX_EPOCH.
fn file_mtime(path: &Path) -> SystemTime {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    (dot / (norm_a * norm_b)) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let score = cosine_similarity(&v, &v);
        assert!((score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let score = cosine_similarity(&a, &b);
        assert!(score.abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_opposite_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let score = cosine_similarity(&a, &b);
        assert!((score + 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_zero_vector() {
        let a = vec![1.0, 2.0];
        let b = vec![0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn cache_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.bin");

        let mtime = SystemTime::UNIX_EPOCH + std::time::Duration::new(1700000000, 123456789);

        let cache = EmbeddingCache {
            model_name: "AllMiniLML6V2".to_string(),
            entries: vec![CacheEntry {
                slug: "test-note".to_string(),
                mtime_secs: 1700000000,
                mtime_nanos: 123456789,
                vector: vec![0.1, 0.2, 0.3],
            }],
        };

        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&cache).unwrap();
        std::fs::write(&path, &bytes).unwrap();

        let loaded = load_cache(&path, "AllMiniLML6V2");
        assert_eq!(loaded.len(), 1);

        let entry = loaded.get("test-note").unwrap();
        assert_eq!(entry.mtime, mtime);
        assert_eq!(entry.vector, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn cache_model_mismatch_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.bin");

        let cache = EmbeddingCache {
            model_name: "AllMiniLML6V2".to_string(),
            entries: vec![CacheEntry {
                slug: "note".to_string(),
                mtime_secs: 0,
                mtime_nanos: 0,
                vector: vec![0.5],
            }],
        };

        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&cache).unwrap();
        std::fs::write(&path, &bytes).unwrap();

        let loaded = load_cache(&path, "BGESmallENV15");
        assert!(loaded.is_empty());
    }

    #[test]
    fn cache_missing_file_returns_empty() {
        let loaded = load_cache(Path::new("/nonexistent/path.bin"), "AllMiniLML6V2");
        assert!(loaded.is_empty());
    }

    #[test]
    fn cache_corrupt_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.bin");
        std::fs::write(&path, b"not an rkyv archive").unwrap();

        let loaded = load_cache(&path, "AllMiniLML6V2");
        assert!(loaded.is_empty());
    }

    #[test]
    fn parse_model_name_valid() {
        assert!(parse_model_name("AllMiniLML6V2").is_ok());
        assert!(parse_model_name("BGESmallENV15").is_ok());
    }

    #[test]
    fn parse_model_name_invalid() {
        assert!(parse_model_name("NoSuchModel").is_err());
    }
}
