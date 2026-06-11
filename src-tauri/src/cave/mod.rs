mod error;
mod folders;
mod helpers;
mod notes;
mod search;
mod tags;
mod templates;
mod todos;

pub use error::CaveError;
use granit_types::AppConfig;
pub(crate) use helpers::write_atomic;
use helpers::{ensure_md_extension, validate_folder_path};
pub use helpers::{Document, DocumentMeta, RenderedDocument};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use granit_types::ContentMatch;

/// A cave — an open directory of markdown notes.
#[derive(Debug)]
pub struct Cave {
    path: PathBuf,
    /// In-memory index: slug → absolute path. Populated at open and kept in
    /// sync by create / delete / rename / update operations.
    /// Slug uniqueness is enforced globally across all subdirectories.
    notes: HashMap<String, PathBuf>,
    /// In-memory reverse wiki-link index: target slug → source slugs.
    backlinks: HashMap<String, Vec<String>>,
    /// In-memory index: heading anchor id → owning note slug. Populated from
    /// `# Heading {#id}` attributes. Shares the global slug namespace with notes.
    anchors: HashMap<String, String>,
    /// In-memory index: template slug → absolute path inside `.granit/templates`.
    templates: HashMap<String, PathBuf>,
    /// Slug of the note currently open in the editor, if any.
    active_slug: Option<String>,
    /// Whether the notes/backlinks/templates indexes have been populated.
    scanned: bool,
}

impl Cave {
    /// Create a cave handle for the given directory without scanning its
    /// contents. Only the path is stored; the notes, backlinks, and templates
    /// indexes remain empty until [`ensure_scanned`] (or [`open`]) is called.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            notes: HashMap::new(),
            backlinks: HashMap::new(),
            anchors: HashMap::new(),
            templates: HashMap::new(),
            active_slug: None,
            scanned: false,
        }
    }

    /// Populate the in-memory indexes (notes, backlinks, templates) by
    /// recursively scanning the cave directory. This is a no-op if the cave
    /// has already been scanned.
    pub fn ensure_scanned(&mut self) -> Result<(), CaveError> {
        if self.scanned {
            return Ok(());
        }
        self.notes = Self::scan_recursive(&self.path, &self.path)?;
        // Strict: a duplicate anchor (vs a note or another anchor) refuses to open.
        let (anchors, collision) = Self::collect_anchors(&self.notes);
        if let Some(err) = collision {
            return Err(err);
        }
        self.anchors = anchors;
        self.backlinks = Self::build_backlinks(&self.notes, &self.anchors);
        self.templates = Self::scan_templates(&self.templates_dir())?;
        self.scanned = true;
        Ok(())
    }

    /// Open a cave at the given directory path. Eagerly scans recursively for
    /// `.md` files to populate the in-memory notes index.
    ///
    /// Returns an error if two files share the same slug (filename without `.md`).
    pub fn open(path: PathBuf) -> Result<Self, CaveError> {
        let mut cave = Self::new(path);
        cave.ensure_scanned()?;
        Ok(cave)
    }

    pub fn config_path(&self) -> PathBuf {
        self.path.join(".granit").join("config.yml")
    }

    pub fn templates_dir(&self) -> PathBuf {
        self.path.join(".granit").join("templates")
    }

    pub fn ensure_config(&self) -> Result<(), CaveError> {
        std::fs::create_dir_all(self.path.join(".granit"))?;

        let path = self.config_path();
        if !path.exists() {
            self.save_config(&AppConfig::default())?;
        }

        Ok(())
    }

    pub fn load_config(&self) -> Result<AppConfig, CaveError> {
        let path = self.config_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => {
                // serde_yml 0.0.13 rejects fully empty documents; an empty
                // config.yml must keep meaning "all defaults".
                if contents.trim().is_empty() {
                    return Ok(AppConfig::default());
                }
                let mut config: AppConfig = serde_yml::from_str(&contents)?;
                config.active_cave = None;
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AppConfig::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), CaveError> {
        let mut stored = config.clone();
        stored.active_cave = None;
        let yaml = serde_yml::to_string(&stored)?;
        helpers::write_atomic(&self.config_path(), yaml)?;
        Ok(())
    }

    fn ensure_templates_dir(&self) -> Result<PathBuf, CaveError> {
        let path = self.templates_dir();
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    /// Recursively scan `dir` for `.md` files and return a slug → absolute-path map.
    ///
    /// Subdirectories starting with `.` (hidden) are skipped, as is `.granit/`.
    /// Returns an error if two files share the same slug.
    pub(crate) fn scan_recursive(
        cave_root: &Path,
        dir: &Path,
    ) -> Result<HashMap<String, PathBuf>, CaveError> {
        let mut notes: HashMap<String, PathBuf> = HashMap::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let p = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if p.is_dir() {
                // Skip hidden dirs and the cave config dir.
                if name_str.starts_with('.') || name_str == ".granit" {
                    continue;
                }
                let sub = Self::scan_recursive(cave_root, &p)?;
                for (slug, abs_path) in sub {
                    match notes.entry(slug) {
                        std::collections::hash_map::Entry::Occupied(e) => {
                            let existing_rel = e
                                .get()
                                .strip_prefix(cave_root)
                                .unwrap_or(e.get())
                                .to_string_lossy()
                                .into_owned();
                            let new_rel = abs_path
                                .strip_prefix(cave_root)
                                .unwrap_or(&abs_path)
                                .to_string_lossy()
                                .into_owned();
                            return Err(CaveError::DuplicateSlug {
                                slug: e.key().clone(),
                                paths: vec![existing_rel, new_rel],
                            });
                        }
                        std::collections::hash_map::Entry::Vacant(v) => {
                            v.insert(abs_path);
                        }
                    }
                }
            } else if p.is_file() {
                if let Some(ext) = p.extension() {
                    if ext == "md" {
                        let slug = p
                            .file_stem()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        match notes.entry(slug) {
                            std::collections::hash_map::Entry::Occupied(e) => {
                                let existing_rel = e
                                    .get()
                                    .strip_prefix(cave_root)
                                    .unwrap_or(e.get())
                                    .to_string_lossy()
                                    .into_owned();
                                let new_rel = p
                                    .strip_prefix(cave_root)
                                    .unwrap_or(&p)
                                    .to_string_lossy()
                                    .into_owned();
                                return Err(CaveError::DuplicateSlug {
                                    slug: e.key().clone(),
                                    paths: vec![existing_rel, new_rel],
                                });
                            }
                            std::collections::hash_map::Entry::Vacant(v) => {
                                v.insert(p);
                            }
                        }
                    }
                }
            }
        }
        Ok(notes)
    }

    /// Scan the flat `.granit/templates` directory for markdown template files.
    pub(crate) fn scan_templates(dir: &Path) -> Result<HashMap<String, PathBuf>, CaveError> {
        if !dir.is_dir() {
            return Ok(HashMap::new());
        }

        let mut templates: HashMap<String, PathBuf> = HashMap::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() || path.extension().is_none_or(|ext| ext != "md") {
                continue;
            }

            let slug = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            match templates.entry(slug) {
                std::collections::hash_map::Entry::Occupied(e) => {
                    let existing_rel = e.get().to_string_lossy().into_owned();
                    let new_rel = path.to_string_lossy().into_owned();
                    return Err(CaveError::DuplicateTemplateSlug {
                        slug: e.key().clone(),
                        paths: vec![existing_rel, new_rel],
                    });
                }
                std::collections::hash_map::Entry::Vacant(v) => {
                    v.insert(path);
                }
            }
        }

        Ok(templates)
    }

    /// Return the relative path from `self.path` to `abs_path` as a `PathBuf`.
    pub(crate) fn relative_path(&self, abs_path: &Path) -> PathBuf {
        abs_path
            .strip_prefix(&self.path)
            .unwrap_or(abs_path)
            .to_path_buf()
    }

    /// Look up a note slug by name (case-insensitive).
    ///
    /// Returns the stored slug if found, `None` otherwise. Designed to be passed
    /// as a closure to `markdown::resolve_wiki_links`.
    pub fn lookup_slug(&self, name: &str) -> Option<&str> {
        Self::lookup_slug_in_notes(&self.notes, name)
    }

    fn lookup_slug_in_notes<'a>(
        notes: &'a HashMap<String, PathBuf>,
        name: &str,
    ) -> Option<&'a str> {
        let lower = name.to_lowercase();
        notes
            .keys()
            .find(|slug| slug.to_lowercase() == lower)
            .map(String::as_str)
    }

    /// Resolve a wiki-link target name to its href (case-insensitive).
    ///
    /// A note name resolves to its canonical slug; a heading anchor resolves to
    /// `note-slug#anchor-id`. Notes take precedence over anchors. Returns `None`
    /// for unresolved (broken) links. Used as the resolver for markdown rendering.
    pub fn resolve_link(&self, name: &str) -> Option<String> {
        Self::resolve_link_in(&self.notes, &self.anchors, name)
    }

    fn resolve_link_in(
        notes: &HashMap<String, PathBuf>,
        anchors: &HashMap<String, String>,
        name: &str,
    ) -> Option<String> {
        if let Some(slug) = Self::lookup_slug_in_notes(notes, name) {
            return Some(slug.to_string());
        }
        let lower = name.to_lowercase();
        anchors
            .iter()
            .find(|(anchor_id, _)| anchor_id.to_lowercase() == lower)
            .map(|(anchor_id, note_slug)| format!("{note_slug}#{anchor_id}"))
    }

    /// All heading anchor ids declared across the cave, for link completion.
    pub fn list_anchors(&self) -> Vec<String> {
        self.anchors.keys().cloned().collect()
    }

    /// Resolve a slug case-insensitively, returning the canonical stored slug.
    ///
    /// Returns `CaveError::NotFound` if no note matches.
    pub fn resolve_slug(&self, slug: &str) -> Result<String, CaveError> {
        self.lookup_slug(slug)
            .map(String::from)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))
    }

    /// The root directory of this cave.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Slug → absolute path map for all notes in the cave.
    pub(crate) fn note_paths(&self) -> &HashMap<String, PathBuf> {
        &self.notes
    }

    /// Set the slug of the note currently open in the editor.
    pub fn set_active_slug(&mut self, slug: Option<String>) {
        self.active_slug = slug;
    }

    /// Get the slug of the note currently open in the editor.
    pub fn active_slug(&self) -> Option<&str> {
        self.active_slug.as_deref()
    }

    /// Resolve the filename and slug for a new note, handling `"untitled"` auto-numbering.
    fn resolve_new_slug(&self, name: &str) -> Result<(String, String), CaveError> {
        let base_filename = ensure_md_extension(name);
        if name == "untitled" && self.notes.contains_key("untitled") {
            let mut n = 2u32;
            loop {
                let candidate_slug = format!("untitled-{n}");
                if !self.notes.contains_key(&candidate_slug) {
                    return Ok((format!("{candidate_slug}.md"), candidate_slug));
                }
                n = n
                    .checked_add(1)
                    .ok_or_else(|| CaveError::SlugExhausted("untitled".into()))?;
            }
        } else if self.notes.contains_key(name) {
            Err(CaveError::AlreadyExists(base_filename))
        } else {
            Ok((base_filename, name.to_string()))
        }
    }

    /// Resolve the target directory for a new note.
    ///
    /// Priority: explicit `folder` > daily note default folder > cave root.
    fn resolve_target_dir(
        &self,
        folder: Option<&Path>,
        daily_config: Option<&AppConfig>,
    ) -> Result<PathBuf, CaveError> {
        if let Some(f) = folder {
            validate_folder_path(f)?;
            let d = self.path.join(f);
            if !d.is_dir() {
                return Err(CaveError::NotFound(f.to_string_lossy().into_owned()));
            }
            self.check_containment(&d)?;
            Ok(d)
        } else if let Some(config) = daily_config {
            let daily_folder = Path::new(&config.daily_note_folder);
            validate_folder_path(daily_folder)?;
            let d = self.path.join(daily_folder);
            std::fs::create_dir_all(&d)?;
            self.check_containment(&d)?;
            Ok(d)
        } else {
            Ok(self.path.clone())
        }
    }

    /// Verify that `abs_path` is contained within the cave root by comparing
    /// canonical paths. Walks up to the nearest existing ancestor so this works
    /// for paths that are about to be created as well as paths that already exist.
    /// Returns `CaveError::InvalidName` if the path escapes the cave root.
    pub(crate) fn check_containment(&self, abs_path: &Path) -> Result<(), CaveError> {
        let canonical_root =
            std::fs::canonicalize(&self.path).map_err(|e| CaveError::Io(e.to_string()))?;
        let mut candidate = abs_path;
        let canonical_candidate = loop {
            if candidate.exists() {
                break std::fs::canonicalize(candidate)
                    .map_err(|e| CaveError::Io(e.to_string()))?;
            }
            candidate = candidate
                .parent()
                .ok_or_else(|| CaveError::InvalidName("path escapes the cave root".to_string()))?;
        };
        if !canonical_candidate.starts_with(&canonical_root) {
            return Err(CaveError::InvalidName(
                "path escapes the cave root".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_config_bootstraps_default_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.ensure_config().unwrap();

        let config_path = dir.path().join(".granit/config.yml");
        assert!(config_path.exists());

        let config = cave.load_config().unwrap();
        assert_eq!(config.theme, "dark");
        assert_eq!(config.daily_note_folder, "Daily");
    }

    #[test]
    fn test_load_config_empty_file_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(dir.path().join(".granit")).unwrap();
        std::fs::write(cave.config_path(), "").unwrap();

        let config = cave.load_config().unwrap();
        assert_eq!(config.theme, "dark");
        assert_eq!(config.daily_note_folder, "Daily");
    }

    #[test]
    fn test_save_config_does_not_persist_active_cave() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_config().unwrap();

        let config = AppConfig {
            theme: "latte".to_string(),
            active_cave: Some("/tmp/cave".to_string()),
            ..AppConfig::default()
        };

        cave.save_config(&config).unwrap();

        let stored = std::fs::read_to_string(cave.config_path()).unwrap();
        assert!(!stored.contains("active_cave"));
        assert!(!stored.contains("/tmp/cave"));

        let loaded = cave.load_config().unwrap();
        assert_eq!(loaded.theme, "latte");
        assert!(loaded.active_cave.is_none());
    }

    #[test]
    fn test_cave_configs_are_isolated_per_cave() {
        let dir_a = tempfile::tempdir().unwrap();
        let dir_b = tempfile::tempdir().unwrap();

        let cave_a = Cave::open(dir_a.path().to_path_buf()).unwrap();
        let cave_b = Cave::open(dir_b.path().to_path_buf()).unwrap();
        cave_a.ensure_config().unwrap();
        cave_b.ensure_config().unwrap();

        cave_a
            .save_config(&AppConfig {
                theme: "latte".to_string(),
                daily_note_folder: "Journal".to_string(),
                ..AppConfig::default()
            })
            .unwrap();
        cave_b
            .save_config(&AppConfig {
                theme: "forest".to_string(),
                daily_note_folder: "Daily Notes".to_string(),
                ..AppConfig::default()
            })
            .unwrap();

        let loaded_a = cave_a.load_config().unwrap();
        let loaded_b = cave_b.load_config().unwrap();

        assert_eq!(loaded_a.theme, "latte");
        assert_eq!(loaded_a.daily_note_folder, "Journal");
        assert_eq!(loaded_b.theme, "forest");
        assert_eq!(loaded_b.daily_note_folder, "Daily Notes");
    }

    #[test]
    fn test_open_cave_rejects_duplicate_slugs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("dup.md"), "root").unwrap();
        std::fs::write(dir.path().join("sub/dup.md"), "sub").unwrap();

        let err = Cave::open(dir.path().to_path_buf()).unwrap_err();
        assert!(
            matches!(err, CaveError::DuplicateSlug { ref slug, .. } if slug == "dup"),
            "expected DuplicateSlug error, got: {err:?}"
        );
    }

    // ── resolve_slug / lookup_slug ─────────────────────────────────────────

    #[test]
    fn test_resolve_slug_exact_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("my-note.md"), "").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        assert_eq!(cave.resolve_slug("my-note").unwrap(), "my-note");
    }

    #[test]
    fn test_resolve_slug_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("my-note.md"), "").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        // Regardless of casing, the canonical stored slug is returned.
        assert_eq!(cave.resolve_slug("MY-NOTE").unwrap(), "my-note");
        assert_eq!(cave.resolve_slug("My-Note").unwrap(), "my-note");
        assert_eq!(cave.resolve_slug("my-NOTE").unwrap(), "my-note");
    }

    #[test]
    fn test_resolve_slug_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let err = cave.resolve_slug("missing").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_lookup_slug_case_insensitive_returns_canonical() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Project-Alpha.md"), "").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        // lookup_slug used during wiki-link resolution
        assert_eq!(cave.lookup_slug("project-alpha"), Some("Project-Alpha"));
        assert_eq!(cave.lookup_slug("PROJECT-ALPHA"), Some("Project-Alpha"));
        assert!(cave.lookup_slug("project-beta").is_none());
    }

    // ── Heading anchors ────────────────────────────────────────────────────

    #[test]
    fn test_anchor_index_built_on_scan_and_resolves_to_note_fragment() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Car brands.md"),
            "# Volvo {#volvo}\n\nbla\n\n# SAAB {#saab}\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        // Notes still take precedence and resolve to a bare slug.
        assert_eq!(
            cave.resolve_link("Car brands"),
            Some("Car brands".to_string())
        );
        // Anchors resolve to `note#anchor`, case-insensitively.
        assert_eq!(
            cave.resolve_link("Volvo"),
            Some("Car brands#volvo".to_string())
        );
        assert_eq!(
            cave.resolve_link("saab"),
            Some("Car brands#saab".to_string())
        );
        assert!(cave.resolve_link("Ford").is_none());

        let mut anchors = cave.list_anchors();
        anchors.sort();
        assert_eq!(anchors, vec!["saab".to_string(), "volvo".to_string()]);
    }

    #[test]
    fn test_open_cave_rejects_anchor_colliding_with_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Volvo.md"), "the car").unwrap();
        std::fs::write(dir.path().join("Brands.md"), "# Volvo {#volvo}\n").unwrap();

        let err = Cave::open(dir.path().to_path_buf()).unwrap_err();
        assert!(
            matches!(err, CaveError::DuplicateAnchor { ref slug, .. } if slug == "volvo"),
            "expected DuplicateAnchor, got: {err:?}"
        );
    }

    #[test]
    fn test_open_cave_rejects_duplicate_anchors_across_notes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# X {#dup}\n").unwrap();
        std::fs::write(dir.path().join("b.md"), "# Y {#dup}\n").unwrap();

        let err = Cave::open(dir.path().to_path_buf()).unwrap_err();
        assert!(
            matches!(err, CaveError::DuplicateAnchor { ref slug, .. } if slug == "dup"),
            "expected DuplicateAnchor, got: {err:?}"
        );
    }
}
