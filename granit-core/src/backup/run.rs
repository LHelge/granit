//! The backup and restore flows, shared by both hosts.
//!
//! A host takes an [`OperationTicket`] from [`CoreState::backup_guard`]
//! (backups and restores never overlap: a backup snapshotting a
//! half-restored cave, or a restore swapping directories mid-pack, would
//! corrupt either result), then runs [`backup_now`] or [`restore_backup`]
//! on a tokio runtime with a progress callback, and reports the outcome its
//! own way (Tauri events, GPUI entity updates). Opening a restored cave is
//! the host's open-cave path, so [`restore_backup`] returns the cave to open
//! instead of opening it.
//!
//! Credentials are the open cave's `backup` config, or an explicit
//! [`BackupConfig`] for a disaster restore with no cave open (never saved).
//! The passphrase is only passed through to key derivation: it is never
//! stored and never logged.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use granit_api::{BackupInfo, CreateBackupRequest};
use granit_types::{BackupConfig, BackupStage, RestoreStage, RestoreTarget};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::client::{BackupApi, BackupApiClient};
use super::{archive, crypto, restore, BackupError};
use crate::CoreState;

/// Keeps backups and restores from overlapping. One lives in [`CoreState`].
#[derive(Default)]
pub struct OperationGuard(Arc<AtomicBool>);

impl OperationGuard {
    /// Claim the guard for one backup or restore, which runs while the
    /// ticket lives. Fails with [`BackupError::AlreadyRunning`] while
    /// another ticket is out.
    pub fn start(&self) -> Result<OperationTicket, BackupError> {
        if self.0.swap(true, Ordering::SeqCst) {
            return Err(BackupError::AlreadyRunning);
        }
        Ok(OperationTicket(self.0.clone()))
    }

    /// Whether a backup or restore is running.
    pub fn is_running(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Proof that the [`OperationGuard`] is held; releases it when dropped.
#[must_use = "the guard is released as soon as the ticket is dropped"]
pub struct OperationTicket(Arc<AtomicBool>);

impl Drop for OperationTicket {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl CoreState {
    /// The guard every backup and restore holds while it runs.
    pub fn backup_guard(&self) -> &OperationGuard {
        &self.backup_guard
    }
}

/// The backend credentials to use: an explicit override (disaster restore
/// with no cave open) or the open cave's config.
pub fn resolve_credentials(
    state: &CoreState,
    credentials: Option<BackupConfig>,
) -> Result<BackupConfig, BackupError> {
    let config = match credentials {
        Some(given) => given,
        None => state.lock_config().backup.clone(),
    };
    if !config.is_configured() {
        return Err(BackupError::NotConfigured);
    }
    Ok(config)
}

/// A client for the backend `config` names.
pub fn connect(config: &BackupConfig) -> Result<BackupApiClient, BackupError> {
    BackupApiClient::new(&config.backend_url, &config.api_key)
}

/// Run blocking work (packing, Argon2, unpacking) off the async runtime.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, BackupError> + Send + 'static,
) -> Result<T, BackupError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|err| BackupError::Io(err.to_string()))?
}

fn granit_dir(cave_path: &Path) -> PathBuf {
    cave_path.join(".granit")
}

/// The snapshot name of a cave: its directory name.
fn cave_name(cave_path: &Path) -> String {
    cave_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "cave".to_string())
}

/// Whether the open cave has a cached backup key (a passphrase is set).
pub fn cave_has_key(state: &CoreState) -> bool {
    state
        .active_cave_path()
        .is_some_and(|path| crypto::has_key_file(&granit_dir(&path)))
}

/// Derive the open cave's backup key from `passphrase` and cache it in
/// `.granit/backup.key`. Argon2id is deliberately slow; it runs on a
/// blocking thread.
pub async fn set_cave_passphrase(state: &CoreState, passphrase: String) -> Result<(), BackupError> {
    let granit_dir = granit_dir(&state.active_cave_path().ok_or(BackupError::NoCave)?);
    blocking(move || crypto::set_passphrase(&granit_dir, &passphrase)).await
}

/// Every snapshot on the backend, with the open cave's credentials or the
/// given ones (disaster restore).
pub async fn list_backups(
    state: &CoreState,
    credentials: Option<BackupConfig>,
) -> Result<Vec<BackupInfo>, BackupError> {
    let config = resolve_credentials(state, credentials)?;
    connect(&config)?.list_backups().await
}

/// Delete one snapshot (object and record) with the open cave's
/// credentials. Pending snapshots may be deleted too: that is how abandoned
/// uploads are cleaned up.
pub async fn delete_backup(state: &CoreState, id: Uuid) -> Result<(), BackupError> {
    let config = resolve_credentials(state, None)?;
    connect(&config)?.delete_backup(id).await
}

/// Take a snapshot of the open cave and upload it, with its saved
/// credentials and cached key.
pub async fn backup_now(
    state: &CoreState,
    _ticket: &OperationTicket,
    progress: impl Fn(BackupStage) + Send + Sync,
) -> Result<BackupInfo, BackupError> {
    let cave_path = state.active_cave_path().ok_or(BackupError::NoCave)?;
    let config = state.lock_config().backup.clone();
    if !config.is_configured() {
        return Err(BackupError::NotConfigured);
    }
    let client = connect(&config)?;
    backup_cave(&client, &cave_path, progress).await
}

/// Pack, encrypt and upload the cave at `cave_path` through `api`.
pub async fn backup_cave<A: BackupApi>(
    api: &A,
    cave_path: &Path,
    progress: impl Fn(BackupStage) + Send + Sync,
) -> Result<BackupInfo, BackupError> {
    let granit_dir = granit_dir(cave_path);
    // Check the key before the expensive packing step.
    if !crypto::has_key_file(&granit_dir) {
        return Err(BackupError::PassphraseNotSet);
    }

    progress(BackupStage::Packing);
    let root = cave_path.to_path_buf();
    let packed = blocking(move || archive::pack_cave(&root)).await?;

    progress(BackupStage::Encrypting);
    let (container, sha256_hex) = blocking(move || {
        let key = crypto::load_key(&granit_dir)?;
        let container = crypto::encrypt(&key, &packed)?;
        let sha256_hex = hex::encode(Sha256::digest(&container));
        Ok((container, sha256_hex))
    })
    .await?;

    progress(BackupStage::Uploading);
    let created = api
        .create_backup(&CreateBackupRequest {
            cave_name: cave_name(cave_path),
            size_bytes: container.len() as u64,
            sha256_hex,
        })
        .await?;
    api.upload(&created.upload_url, &created.upload_headers, container)
        .await?;
    api.complete_backup(created.backup.id).await
}

/// What to restore, where, and with what.
#[derive(Clone)]
pub struct RestoreRequest {
    /// The snapshot.
    pub id: Uuid,
    pub target: RestoreTarget,
    /// The snapshot's passphrase, when the cave's cached key does not fit
    /// it (always, with no cave open).
    pub passphrase: Option<String>,
    /// Credentials for a disaster restore; `None` uses the open cave's.
    pub credentials: Option<BackupConfig>,
}

/// Never shows the passphrase or the credentials.
impl std::fmt::Debug for RestoreRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestoreRequest")
            .field("id", &self.id)
            .field("target", &self.target)
            .field(
                "passphrase",
                &self.passphrase.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "credentials",
                &self.credentials.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

/// A finished restore.
#[derive(Debug, PartialEq, Eq)]
pub struct Restored {
    /// The restored cave, for the host to open.
    pub cave: PathBuf,
    /// Where an in-place restore kept the previous contents.
    pub pre_restore: Option<PathBuf>,
}

/// A failed restore.
#[derive(Debug)]
pub struct RestoreFailure {
    pub error: BackupError,
    /// The cave the host should open again: an in-place restore closes the
    /// cave before swapping directories, and a failed swap leaves the
    /// original directory intact.
    pub reopen: Option<PathBuf>,
}

impl From<BackupError> for RestoreFailure {
    fn from(error: BackupError) -> Self {
        Self {
            error,
            reopen: None,
        }
    }
}

/// Download, verify, decrypt and unpack one snapshot, into a new directory
/// or over the open cave. The host then opens [`Restored::cave`].
pub async fn restore_backup(
    state: &CoreState,
    _ticket: &OperationTicket,
    request: RestoreRequest,
    progress: impl Fn(RestoreStage) + Send + Sync,
) -> Result<Restored, RestoreFailure> {
    let config = resolve_credentials(state, request.credentials)?;
    let client = connect(&config)?;
    restore_with(
        state,
        &client,
        request.id,
        request.target,
        request.passphrase,
        progress,
    )
    .await
}

/// [`restore_backup`] through `api`.
///
/// The cave's cached key decrypts silently when it fits the archive;
/// otherwise the passphrase is needed ([`BackupError::PassphraseRequired`]
/// without one, so the host can ask for it). An in-place restore closes the
/// cave (and resets the agent) before swapping directories, so nothing
/// writes into the pre-restore copy.
pub async fn restore_with<A: BackupApi>(
    state: &CoreState,
    api: &A,
    id: Uuid,
    target: RestoreTarget,
    passphrase: Option<String>,
    progress: impl Fn(RestoreStage) + Send + Sync,
) -> Result<Restored, RestoreFailure> {
    // The in-place target needs an open cave before any work starts.
    let cave_path = state.active_cave_path();
    if matches!(target, RestoreTarget::CurrentCave) && cave_path.is_none() {
        return Err(BackupError::NoCave.into());
    }

    progress(RestoreStage::Downloading);
    // The download endpoint validates existence and state (404/409); the
    // list provides the recorded digest to verify the bytes against.
    let download = api.download_backup(id).await?;
    let record = api
        .list_backups()
        .await?
        .into_iter()
        .find(|backup| backup.id == id)
        .ok_or(BackupError::SnapshotNotFound)?;
    let container = api.download(&download.download_url).await?;

    progress(RestoreStage::Decrypting);
    let cached_key = cave_path
        .as_deref()
        .map(granit_dir)
        .filter(|dir| crypto::has_key_file(dir));
    let plaintext = blocking(move || {
        let cached = cached_key.map(|dir| crypto::load_key(&dir)).transpose()?;
        restore::verify_and_decrypt(
            &container,
            &record.sha256_hex,
            cached.as_ref(),
            passphrase.as_deref(),
        )
    })
    .await?;

    progress(RestoreStage::Unpacking);
    match target {
        RestoreTarget::NewDirectory { path } => {
            let target_dir = PathBuf::from(path);
            let unpack_to = target_dir.clone();
            blocking(move || restore::restore_to_new_dir(&plaintext, &unpack_to)).await?;
            Ok(Restored {
                cave: target_dir,
                pre_restore: None,
            })
        }
        RestoreTarget::CurrentCave => {
            let cave_path = cave_path.expect("checked above");
            {
                // The host may have opened another cave while the snapshot
                // downloaded: never close (or replace) a cave other than the
                // one the restore was asked for.
                let mut open = state.lock_cave();
                if open.as_ref().map(|cave| cave.path()) != Some(cave_path.as_path()) {
                    return Err(BackupError::CaveChanged.into());
                }
                *open = None;
            }
            state.reset_agent();
            let swap_root = cave_path.clone();
            match blocking(move || restore::restore_in_place(&plaintext, &swap_root)).await {
                Ok(pre_restore) => {
                    log::info!("pre-restore cave kept at {}", pre_restore.display());
                    Ok(Restored {
                        cave: cave_path,
                        pre_restore: Some(pre_restore),
                    })
                }
                Err(error) => Err(RestoreFailure {
                    error,
                    reopen: Some(cave_path),
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::Utc;
    use granit_api::{BackupState, CreateBackupResponse, DownloadBackupResponse};
    use granit_types::AppConfig;
    use parking_lot::Mutex;

    use super::*;
    use crate::cave::Cave;

    const PASSPHRASE: &str = "correct horse battery";

    /// An in-memory backend: records and the uploaded objects by URL.
    #[derive(Default)]
    struct FakeApi {
        records: Mutex<Vec<BackupInfo>>,
        objects: Mutex<BTreeMap<String, Vec<u8>>>,
        /// Runs once when an archive is downloaded (the host acting
        /// meanwhile).
        on_download: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    }

    fn object_url(id: Uuid) -> String {
        format!("fake://objects/{id}")
    }

    impl BackupApi for FakeApi {
        async fn create_backup(
            &self,
            request: &CreateBackupRequest,
        ) -> Result<CreateBackupResponse, BackupError> {
            let mut records = self.records.lock();
            let backup = BackupInfo {
                id: Uuid::from_u128(records.len() as u128 + 1),
                cave_name: request.cave_name.clone(),
                size_bytes: request.size_bytes,
                sha256_hex: request.sha256_hex.clone(),
                state: BackupState::Pending,
                created_at: Utc::now(),
                completed_at: None,
            };
            records.push(backup.clone());
            Ok(CreateBackupResponse {
                upload_url: object_url(backup.id),
                upload_headers: BTreeMap::new(),
                upload_expires_at: Utc::now(),
                backup,
            })
        }

        async fn upload(
            &self,
            presigned_url: &str,
            _headers: &BTreeMap<String, String>,
            body: Vec<u8>,
        ) -> Result<(), BackupError> {
            self.objects.lock().insert(presigned_url.to_string(), body);
            Ok(())
        }

        async fn complete_backup(&self, id: Uuid) -> Result<BackupInfo, BackupError> {
            let mut records = self.records.lock();
            let record = records
                .iter_mut()
                .find(|record| record.id == id)
                .ok_or(BackupError::SnapshotNotFound)?;
            let uploaded = self.objects.lock().get(&object_url(id)).cloned();
            match uploaded {
                Some(bytes) if hex::encode(Sha256::digest(&bytes)) == record.sha256_hex => {
                    record.state = BackupState::Complete;
                    record.completed_at = Some(Utc::now());
                    Ok(record.clone())
                }
                _ => Err(BackupError::Api {
                    status: 409,
                    message: "upload does not match".into(),
                }),
            }
        }

        async fn list_backups(&self) -> Result<Vec<BackupInfo>, BackupError> {
            Ok(self.records.lock().iter().rev().cloned().collect())
        }

        async fn delete_backup(&self, id: Uuid) -> Result<(), BackupError> {
            self.records.lock().retain(|record| record.id != id);
            self.objects.lock().remove(&object_url(id));
            Ok(())
        }

        async fn download_backup(&self, id: Uuid) -> Result<DownloadBackupResponse, BackupError> {
            let records = self.records.lock();
            match records.iter().find(|record| record.id == id) {
                Some(record) if record.state == BackupState::Complete => {
                    Ok(DownloadBackupResponse {
                        download_url: object_url(id),
                        download_expires_at: Utc::now(),
                    })
                }
                Some(_) => Err(BackupError::Api {
                    status: 409,
                    message: "pending".into(),
                }),
                None => Err(BackupError::Api {
                    status: 404,
                    message: "not found".into(),
                }),
            }
        }

        async fn download(&self, presigned_url: &str) -> Result<Vec<u8>, BackupError> {
            let hook = self.on_download.lock().take();
            if let Some(hook) = hook {
                hook();
            }
            self.objects
                .lock()
                .get(presigned_url)
                .cloned()
                .ok_or(BackupError::Api {
                    status: 404,
                    message: "no object".into(),
                })
        }
    }

    /// A cave with one note and a cached key, open in a fresh core.
    fn open_cave(parent: &Path, name: &str, body: &str) -> (PathBuf, CoreState) {
        let root = parent.join(name);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("note.md"), body).unwrap();
        crypto::write_fast_key_file(&granit_dir(&root), PASSPHRASE);
        let state = CoreState::new(AppConfig::default());
        state.set_cave(Some(Cave::open(root.clone()).unwrap()));
        (root, state)
    }

    fn stages<T: Copy + Send>() -> (Arc<Mutex<Vec<T>>>, impl Fn(T) + Send + Sync) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let record = {
            let seen = seen.clone();
            move |stage| seen.lock().push(stage)
        };
        (seen, record)
    }

    #[test]
    fn the_guard_admits_one_operation_at_a_time() {
        let state = CoreState::new(AppConfig::default());
        let ticket = state.backup_guard().start().unwrap();
        assert!(state.backup_guard().is_running());
        assert!(matches!(
            state.backup_guard().start(),
            Err(BackupError::AlreadyRunning)
        ));
        drop(ticket);
        assert!(!state.backup_guard().is_running());
        let _again = state.backup_guard().start().unwrap();
    }

    #[tokio::test]
    async fn a_backup_restores_into_a_new_folder() {
        let parent = tempfile::tempdir().unwrap();
        let (root, state) = open_cave(parent.path(), "my-cave", "# Kept\n");
        let api = FakeApi::default();

        let (seen, progress) = stages();
        let info = backup_cave(&api, &root, progress).await.unwrap();
        assert_eq!(info.cave_name, "my-cave");
        assert_eq!(info.state, BackupState::Complete);
        assert_eq!(
            *seen.lock(),
            vec![
                BackupStage::Packing,
                BackupStage::Encrypting,
                BackupStage::Uploading
            ]
        );

        // The cached key fits, so no passphrase is needed.
        let target = parent.path().join("restored");
        let (seen, progress) = stages();
        let restored = restore_with(
            &state,
            &api,
            info.id,
            RestoreTarget::NewDirectory {
                path: target.display().to_string(),
            },
            None,
            progress,
        )
        .await
        .unwrap();
        assert_eq!(
            restored,
            Restored {
                cave: target.clone(),
                pre_restore: None
            }
        );
        assert_eq!(
            *seen.lock(),
            vec![
                RestoreStage::Downloading,
                RestoreStage::Decrypting,
                RestoreStage::Unpacking
            ]
        );
        assert_eq!(
            std::fs::read_to_string(target.join("note.md")).unwrap(),
            "# Kept\n"
        );
        assert!(
            !target.join(".granit/backup.key").exists(),
            "the key never travels in an archive"
        );
        assert_eq!(state.active_cave_path(), Some(root), "the cave stays open");
    }

    #[tokio::test]
    async fn a_restore_with_no_cave_open_needs_the_passphrase() {
        let parent = tempfile::tempdir().unwrap();
        let (root, _) = open_cave(parent.path(), "lost", "# Lost\n");
        let api = FakeApi::default();
        let info = backup_cave(&api, &root, |_| {}).await.unwrap();

        // A fresh machine: no cave open, so no cached key.
        let state = CoreState::new(AppConfig::default());
        let target = parent.path().join("found");
        let to_new_dir = || RestoreTarget::NewDirectory {
            path: target.display().to_string(),
        };
        let failure = restore_with(&state, &api, info.id, to_new_dir(), None, |_| {})
            .await
            .unwrap_err();
        assert!(matches!(failure.error, BackupError::PassphraseRequired));
        assert_eq!(failure.reopen, None);

        let failure = restore_with(
            &state,
            &api,
            info.id,
            to_new_dir(),
            Some("not the passphrase".into()),
            |_| {},
        )
        .await
        .unwrap_err();
        assert!(matches!(failure.error, BackupError::Decrypt));

        let restored = restore_with(
            &state,
            &api,
            info.id,
            to_new_dir(),
            Some(PASSPHRASE.into()),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(restored.cave, target);
        assert_eq!(
            std::fs::read_to_string(target.join("note.md")).unwrap(),
            "# Lost\n"
        );
    }

    #[tokio::test]
    async fn an_in_place_restore_closes_the_cave_and_keeps_the_old_contents() {
        let parent = tempfile::tempdir().unwrap();
        let (root, state) = open_cave(parent.path(), "cave", "# Snapshot\n");
        let api = FakeApi::default();
        let info = backup_cave(&api, &root, |_| {}).await.unwrap();
        std::fs::write(root.join("note.md"), "# Edited later\n").unwrap();
        let generation = state.agent_generation();

        let restored = restore_with(
            &state,
            &api,
            info.id,
            RestoreTarget::CurrentCave,
            None,
            |_| {},
        )
        .await
        .unwrap();

        assert_eq!(restored.cave, root);
        assert_eq!(state.active_cave_path(), None, "the host reopens it");
        assert_eq!(state.agent_generation(), generation + 1);
        assert_eq!(
            std::fs::read_to_string(root.join("note.md")).unwrap(),
            "# Snapshot\n"
        );
        assert!(
            root.join(".granit/backup.key").is_file(),
            "key carried over"
        );
        let pre_restore = restored.pre_restore.expect("an in-place restore");
        assert_eq!(
            std::fs::read_to_string(pre_restore.join("note.md")).unwrap(),
            "# Edited later\n"
        );
    }

    #[tokio::test]
    async fn a_failed_swap_asks_the_host_to_reopen_the_cave() {
        let parent = tempfile::tempdir().unwrap();
        let (root, state) = open_cave(parent.path(), "cave", "# Original\n");
        // A snapshot that decrypts but is not an archive.
        let api = FakeApi::default();
        let key = crypto::fast_key(PASSPHRASE);
        let container = crypto::encrypt(&key, b"not a zstd stream").unwrap();
        let created = api
            .create_backup(&CreateBackupRequest {
                cave_name: "cave".into(),
                size_bytes: container.len() as u64,
                sha256_hex: hex::encode(Sha256::digest(&container)),
            })
            .await
            .unwrap();
        api.upload(&created.upload_url, &BTreeMap::new(), container)
            .await
            .unwrap();
        api.complete_backup(created.backup.id).await.unwrap();

        let failure = restore_with(
            &state,
            &api,
            created.backup.id,
            RestoreTarget::CurrentCave,
            None,
            |_| {},
        )
        .await
        .unwrap_err();

        assert_eq!(failure.reopen, Some(root.clone()));
        assert_eq!(
            std::fs::read_to_string(root.join("note.md")).unwrap(),
            "# Original\n"
        );
    }

    #[tokio::test]
    async fn an_in_place_restore_leaves_a_cave_opened_meanwhile_alone() {
        let parent = tempfile::tempdir().unwrap();
        let (root, state) = open_cave(parent.path(), "first", "# First\n");
        let api = FakeApi::default();
        let info = backup_cave(&api, &root, |_| {}).await.unwrap();
        std::fs::write(root.join("note.md"), "# First, edited\n").unwrap();

        // While the snapshot downloads, the user opens another cave.
        let state = Arc::new(state);
        let other = parent.path().join("second");
        std::fs::create_dir_all(&other).unwrap();
        *api.on_download.lock() = Some(Box::new({
            let state = state.clone();
            let other = other.clone();
            move || state.set_cave(Some(Cave::open(other).unwrap()))
        }));

        let failure = restore_with(
            &state,
            &api,
            info.id,
            RestoreTarget::CurrentCave,
            None,
            |_| {},
        )
        .await
        .unwrap_err();

        assert!(matches!(failure.error, BackupError::CaveChanged));
        assert_eq!(failure.reopen, None, "nothing to reopen");
        assert_eq!(state.active_cave_path(), Some(other), "still open");
        assert_eq!(
            std::fs::read_to_string(root.join("note.md")).unwrap(),
            "# First, edited\n",
            "the first cave was not replaced"
        );
    }

    #[tokio::test]
    async fn an_in_place_restore_needs_an_open_cave() {
        let state = CoreState::new(AppConfig::default());
        let failure = restore_with(
            &state,
            &FakeApi::default(),
            Uuid::nil(),
            RestoreTarget::CurrentCave,
            None,
            |_| {},
        )
        .await
        .unwrap_err();
        assert!(matches!(failure.error, BackupError::NoCave));
    }

    #[tokio::test]
    async fn a_restore_into_a_non_empty_folder_is_refused() {
        let parent = tempfile::tempdir().unwrap();
        let (root, state) = open_cave(parent.path(), "cave", "# A\n");
        let api = FakeApi::default();
        let info = backup_cave(&api, &root, |_| {}).await.unwrap();
        let busy = parent.path().join("busy");
        std::fs::create_dir(&busy).unwrap();
        std::fs::write(busy.join("keep.txt"), "keep").unwrap();

        let failure = restore_with(
            &state,
            &api,
            info.id,
            RestoreTarget::NewDirectory {
                path: busy.display().to_string(),
            },
            None,
            |_| {},
        )
        .await
        .unwrap_err();
        assert!(matches!(failure.error, BackupError::TargetNotEmpty));
        assert_eq!(state.active_cave_path(), Some(root));
    }

    #[tokio::test]
    async fn a_backup_needs_a_passphrase_and_credentials() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("bare");
        std::fs::create_dir_all(&root).unwrap();
        let api = FakeApi::default();
        assert!(matches!(
            backup_cave(&api, &root, |_| {}).await,
            Err(BackupError::PassphraseNotSet)
        ));
        assert!(api.records.lock().is_empty(), "nothing was announced");

        let (_, state) = open_cave(parent.path(), "cave", "# A\n");
        let ticket = state.backup_guard().start().unwrap();
        assert!(matches!(
            backup_now(&state, &ticket, |_| {}).await,
            Err(BackupError::NotConfigured)
        ));
        let empty = CoreState::new(AppConfig::default());
        assert!(matches!(
            backup_now(&empty, &ticket, |_| {}).await,
            Err(BackupError::NoCave)
        ));
    }

    #[test]
    fn a_request_never_shows_its_secrets() {
        let request = RestoreRequest {
            id: Uuid::nil(),
            target: RestoreTarget::CurrentCave,
            passphrase: Some(PASSPHRASE.into()),
            credentials: Some(BackupConfig {
                backend_url: "http://localhost:8080".into(),
                api_key: "grnt_secret".into(),
            }),
        };
        let shown = format!("{request:?}");
        assert!(!shown.contains(PASSPHRASE), "{shown}");
        assert!(!shown.contains("grnt_secret"), "{shown}");
        assert!(shown.contains("CurrentCave"), "{shown}");
    }

    #[test]
    fn credentials_come_from_the_override_or_the_cave_config() {
        let state = CoreState::new(AppConfig::default());
        assert!(matches!(
            resolve_credentials(&state, None),
            Err(BackupError::NotConfigured)
        ));
        let given = BackupConfig {
            backend_url: "http://localhost:8080".into(),
            api_key: "grnt_given".into(),
        };
        assert_eq!(
            resolve_credentials(&state, Some(given.clone())).unwrap(),
            given
        );
        state.lock_config().backup = BackupConfig {
            backend_url: "http://localhost:8080".into(),
            api_key: "grnt_saved".into(),
        };
        assert_eq!(
            resolve_credentials(&state, None).unwrap().api_key,
            "grnt_saved"
        );
    }

    #[tokio::test]
    async fn setting_a_passphrase_caches_a_key_for_the_open_cave() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("cave");
        std::fs::create_dir_all(root.join(".granit")).unwrap();
        let state = CoreState::new(AppConfig::default());
        assert!(matches!(
            set_cave_passphrase(&state, PASSPHRASE.into()).await,
            Err(BackupError::NoCave)
        ));
        state.set_cave(Some(Cave::open(root).unwrap()));
        assert!(!cave_has_key(&state));
        assert!(matches!(
            set_cave_passphrase(&state, "short".into()).await,
            Err(BackupError::WeakPassphrase)
        ));
        set_cave_passphrase(&state, PASSPHRASE.into())
            .await
            .unwrap();
        assert!(cave_has_key(&state));
    }
}
