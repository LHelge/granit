mod error;
mod note;

use chrono::{Datelike, NaiveDate};
pub use error::CaveError;
use granit_types::{AppConfig, TodoItem, TodoList};
use note::{
    ensure_md_extension, note_meta_from_relative_path, note_meta_with_frontmatter,
    template_meta_from_path, template_meta_with_icon, validate_folder_path, validate_name,
};
pub use note::{Note, NoteMeta, Template, TemplateMeta};
use std::collections::{HashMap, HashSet};
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
    /// In-memory index: template slug → absolute path inside `.granit/templates`.
    templates: HashMap<String, PathBuf>,
    /// Slug of the note currently open in the editor, if any.
    active_slug: Option<String>,
}

impl Cave {
    /// Open a cave at the given directory path. Eagerly scans recursively for
    /// `.md` files to populate the in-memory notes index.
    ///
    /// Returns an error if two files share the same slug (filename without `.md`).
    pub fn open(path: PathBuf) -> Result<Self, CaveError> {
        let notes = Self::scan_recursive(&path, &path)?;
        let backlinks = Self::build_backlinks(&notes);
        let templates = Self::scan_templates(&path.join(".granit").join("templates"))?;
        Ok(Self {
            path,
            notes,
            backlinks,
            templates,
            active_slug: None,
        })
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
        std::fs::write(self.config_path(), yaml)?;
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
    fn scan_recursive(cave_root: &Path, dir: &Path) -> Result<HashMap<String, PathBuf>, CaveError> {
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
    fn scan_templates(dir: &Path) -> Result<HashMap<String, PathBuf>, CaveError> {
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

    fn build_backlinks(notes: &HashMap<String, PathBuf>) -> HashMap<String, Vec<String>> {
        let mut backlinks: HashMap<String, HashSet<String>> = HashMap::new();

        for (source_slug, abs_path) in notes {
            let Ok(raw) = std::fs::read_to_string(abs_path) else {
                continue;
            };

            for target_slug in crate::markdown::resolved_outgoing_links(&raw, |name| {
                Self::lookup_slug_in_notes(notes, name)
            }) {
                if target_slug == *source_slug {
                    continue;
                }

                backlinks
                    .entry(target_slug)
                    .or_default()
                    .insert(source_slug.clone());
            }
        }

        backlinks
            .into_iter()
            .map(|(target_slug, source_slugs)| {
                let mut source_slugs: Vec<String> = source_slugs.into_iter().collect();
                source_slugs.sort_by_key(|slug| slug.to_lowercase());
                (target_slug, source_slugs)
            })
            .collect()
    }

    fn rebuild_backlinks(&mut self) {
        self.backlinks = Self::build_backlinks(&self.notes);
    }

    fn read_template_body(&self, slug: &str) -> Result<String, CaveError> {
        let raw = self.read_template_raw(slug)?;
        Ok(crate::markdown::strip_frontmatter(&raw).to_string())
    }

    fn parse_daily_note_slug(slug: &str) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(slug, "%Y-%m-%d").ok()
    }

    fn render_note_template(
        &self,
        template_slug: &str,
        note_slug: &str,
    ) -> Result<String, CaveError> {
        let template_body = self.read_template_body(template_slug)?;
        let mut context = tera::Context::new();
        context.insert("slug", note_slug);

        if let Some(note_date) = Self::parse_daily_note_slug(note_slug) {
            let tomorrow = note_date + chrono::Duration::days(1);
            let yesterday = note_date - chrono::Duration::days(1);
            context.insert("date", &note_date.format("%Y-%m-%d").to_string());
            context.insert("tomorrow", &tomorrow.format("%Y-%m-%d").to_string());
            context.insert("yesterday", &yesterday.format("%Y-%m-%d").to_string());
            context.insert("year", &note_date.year());
            context.insert("month", &note_date.month());
            context.insert("day", &note_date.day());
            context.insert("weekday", &note_date.format("%A").to_string());
            context.insert("weekday_short", &note_date.format("%a").to_string());
        }

        Ok(tera::Tera::one_off(&template_body, &context, false)?)
    }

    fn initial_body_for_new_note(
        &self,
        slug: &str,
        template_slug: Option<&str>,
    ) -> Result<String, CaveError> {
        if let Some(template_slug) = template_slug {
            return self.render_note_template(template_slug, slug);
        }

        Ok(String::new())
    }

    fn initial_icon_for_new_note(
        &self,
        template_slug: Option<&str>,
    ) -> Result<Option<String>, CaveError> {
        let Some(template_slug) = template_slug else {
            return Ok(None);
        };

        let raw = self.read_template_raw(template_slug)?;
        Ok(crate::markdown::read_frontmatter_icon(&raw))
    }

    fn initial_tags_for_new_note(
        &self,
        template_slug: Option<&str>,
    ) -> Result<Vec<String>, CaveError> {
        let Some(template_slug) = template_slug else {
            return Ok(Vec::new());
        };

        let raw = self.read_template_raw(template_slug)?;
        Ok(crate::markdown::read_frontmatter_tags(&raw))
    }

    /// Return the relative path from `self.path` to `abs_path` as a `PathBuf`.
    fn relative_path(&self, abs_path: &Path) -> PathBuf {
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

    /// Resolve a slug case-insensitively, returning the canonical stored slug.
    ///
    /// Returns `CaveError::NotFound` if no note matches.
    pub fn resolve_slug(&self, slug: &str) -> Result<String, CaveError> {
        self.lookup_slug(slug)
            .map(String::from)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))
    }

    pub fn backlink_slugs(&self, slug: &str) -> Result<Vec<String>, CaveError> {
        let slug = self.resolve_slug(slug)?;
        Ok(self.backlinks.get(&slug).cloned().unwrap_or_default())
    }

    pub fn backlink_note_metas(&self, slug: &str) -> Result<Vec<NoteMeta>, CaveError> {
        let backlink_slugs = self.backlink_slugs(slug)?;
        let mut backlinks: Vec<NoteMeta> = backlink_slugs
            .into_iter()
            .filter_map(|source_slug| {
                self.notes.get(&source_slug).map(|abs_path| {
                    note_meta_with_frontmatter(&self.relative_path(abs_path), abs_path)
                })
            })
            .collect();
        backlinks.sort_by_key(|meta| meta.slug.to_lowercase());
        Ok(backlinks)
    }

    /// The root directory of this cave.
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
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
                n += 1;
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

    /// Create a new note in this cave, optionally inside `folder` (relative path
    /// from cave root) and optionally seeded from a template slug. Returns the
    /// metadata of the created note.
    ///
    /// If `name` is `"untitled"` and the slug already exists anywhere in the cave,
    /// a numeric suffix is appended (`untitled-2`, `untitled-3`, …).
    /// Any other duplicate slug returns `CaveError::AlreadyExists`.
    pub fn create_note(
        &mut self,
        name: &str,
        folder: Option<&Path>,
        template_slug: Option<&str>,
    ) -> Result<NoteMeta, CaveError> {
        validate_name(name)?;

        let (filename, slug) = self.resolve_new_slug(name)?;

        let daily_config = Self::parse_daily_note_slug(&slug)
            .map(|_| self.load_config())
            .transpose()?;

        let target_dir = self.resolve_target_dir(folder, daily_config.as_ref())?;

        let effective_template_slug = template_slug.map(str::to_string).or_else(|| {
            daily_config
                .as_ref()
                .and_then(|config| config.daily_note_template_slug.clone())
        });

        let final_path = target_dir.join(&filename);
        let body = self.initial_body_for_new_note(&slug, effective_template_slug.as_deref())?;
        let tags = self.initial_tags_for_new_note(effective_template_slug.as_deref())?;
        let icon = self.initial_icon_for_new_note(effective_template_slug.as_deref())?;
        let initial_content = crate::markdown::initial_content_with_body(&body, tags, icon);
        std::fs::write(&final_path, &initial_content)?;
        self.notes.insert(slug, final_path.clone());
        self.rebuild_backlinks();

        let rel = self.relative_path(&final_path);
        Ok(note_meta_from_relative_path(&rel))
    }

    /// Create a new template in `.granit/templates`.
    pub fn create_template(&mut self, name: &str) -> Result<TemplateMeta, CaveError> {
        validate_name(name)?;

        let templates_dir = self.ensure_templates_dir()?;
        let base_filename = ensure_md_extension(name);

        let (filename, slug) = if name == "untitled" && self.templates.contains_key("untitled") {
            let mut n = 2u32;
            loop {
                let candidate_slug = format!("untitled-{n}");
                let candidate_file = format!("{candidate_slug}.md");
                if !self.templates.contains_key(&candidate_slug) {
                    break (candidate_file, candidate_slug);
                }
                n += 1;
            }
        } else if self.templates.contains_key(name) {
            return Err(CaveError::TemplateAlreadyExists(base_filename));
        } else {
            (base_filename, name.to_string())
        };

        let final_path = templates_dir.join(&filename);
        let initial_content = crate::markdown::initial_content(&slug);
        std::fs::write(&final_path, &initial_content)?;
        self.templates.insert(slug, final_path.clone());

        Ok(template_meta_from_path(&final_path))
    }

    /// Open or create today's daily note in the given folder.
    ///
    /// `folder` is a relative path from the cave root (e.g. `"Daily"` or `"Notes/Daily"`).
    /// The folder is created if it does not yet exist.
    /// If the note for today already exists it is read and returned without modification.
    /// The note slug is today's date in `YYYY-MM-DD` format.
    pub fn open_daily_note(
        &mut self,
        folder: &str,
        template_slug: Option<&str>,
    ) -> Result<Note, CaveError> {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let folder_path = Path::new(folder);

        // Ensure the daily folder exists.
        let abs_folder = self.path.join(folder_path);
        if !abs_folder.is_dir() {
            validate_folder_path(folder_path)?;
            std::fs::create_dir_all(&abs_folder)?;
        }

        // Create the note if it doesn't exist yet, rendering the configured template if possible.
        if !self.notes.contains_key(today.as_str()) {
            let body = template_slug
                .and_then(|slug| self.render_note_template(slug, &today).ok())
                .unwrap_or_default();
            let tags = template_slug
                .and_then(|slug| self.initial_tags_for_new_note(Some(slug)).ok())
                .unwrap_or_default();
            let final_path = abs_folder.join(format!("{today}.md"));
            let initial_content = crate::markdown::initial_content_with_body(
                &body,
                tags,
                Some("Calendar".to_string()),
            );
            std::fs::write(&final_path, initial_content)?;
            self.notes.insert(today.clone(), final_path);
            self.rebuild_backlinks();
        }

        self.read_note(&today)
    }

    /// Create a subdirectory within the cave.
    ///
    /// `path` is a relative path from the cave root (e.g. `Path::new("notes")` or `Path::new("projects/2026")`).
    #[allow(dead_code)]
    pub fn create_folder(&mut self, path: &Path) -> Result<(), CaveError> {
        validate_folder_path(path)?;
        let abs = self.path.join(path);
        self.check_containment(&abs)?;
        if abs.exists() {
            return Err(CaveError::AlreadyExists(
                path.to_string_lossy().into_owned(),
            ));
        }
        std::fs::create_dir_all(&abs)?;
        Ok(())
    }

    /// Delete a folder and all notes within it.
    ///
    /// `path` is a relative path from the cave root (e.g. `Path::new("notes")` or `Path::new("projects/2026")`).
    /// All notes indexed under this folder are removed from the cave index before deletion.
    pub fn delete_folder(&mut self, path: &Path) -> Result<(), CaveError> {
        validate_folder_path(path)?;
        let abs = self.path.join(path);
        if !abs.exists() {
            return Err(CaveError::NotFound(path.to_string_lossy().into_owned()));
        }
        self.check_containment(&abs)?;
        // Filesystem first — only update the index after the delete succeeds.
        std::fs::remove_dir_all(&abs)?;
        self.notes.retain(|_, note_abs| !note_abs.starts_with(&abs));
        self.rebuild_backlinks();
        Ok(())
    }

    /// List all `.md` notes in this cave (recursively), sorted by slug.
    ///
    /// Each note's frontmatter is read to populate fields like `icon` and `favorite`.
    pub fn list_notes(&self) -> Result<Vec<NoteMeta>, CaveError> {
        let mut notes: Vec<NoteMeta> = self
            .notes
            .values()
            .map(|abs| note_meta_with_frontmatter(&self.relative_path(abs), abs))
            .collect();
        notes.sort_by_key(|n| n.slug.to_lowercase());
        Ok(notes)
    }

    /// List all templates in `.granit/templates`, sorted by slug.
    pub fn list_templates(&self) -> Result<Vec<TemplateMeta>, CaveError> {
        let mut templates: Vec<TemplateMeta> = self
            .templates
            .values()
            .map(|abs| template_meta_with_icon(abs))
            .collect();
        templates.sort_by_key(|t| t.slug.to_lowercase());
        Ok(templates)
    }

    /// List all subdirectory relative paths in this cave (recursively), sorted.
    ///
    /// Hidden directories (starting with `.`) and `.granit/` are excluded.
    /// Returns paths like `"projects"`, `"projects/2026"`.
    pub fn list_folders(&self) -> Result<Vec<String>, CaveError> {
        let mut folders = Vec::new();
        Self::collect_folders(&self.path, &self.path, &mut folders)?;
        folders.sort_by_key(|f| f.to_lowercase());
        Ok(folders)
    }

    /// Recursively collect subdirectory relative paths.
    fn collect_folders(
        cave_root: &Path,
        dir: &Path,
        out: &mut Vec<String>,
    ) -> Result<(), CaveError> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str == ".granit" {
                continue;
            }
            let rel = p
                .strip_prefix(cave_root)
                .unwrap_or(&p)
                .to_string_lossy()
                .into_owned();
            out.push(rel);
            Self::collect_folders(cave_root, &p, out)?;
        }
        Ok(())
    }

    /// Search note bodies for a query string (case-insensitive substring match).
    ///
    /// Returns up to `max_results` matches, each with the note's slug and
    /// context snippets (all matching lines).
    pub fn search_content(
        &self,
        query: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<ContentMatch>, CaveError> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for (slug, abs_path) in &self.notes {
            if let Some(limit) = max_results {
                if results.len() >= limit {
                    break;
                }
            }
            let raw = std::fs::read_to_string(abs_path)?;
            let body = crate::markdown::strip_frontmatter(&raw);
            let snippets: Vec<String> = body
                .lines()
                .filter(|line| line.to_lowercase().contains(&query_lower))
                .map(|line| line.to_string())
                .collect();
            if !snippets.is_empty() {
                results.push(ContentMatch {
                    slug: slug.clone(),
                    snippets,
                });
            }
        }

        results.sort_by_key(|m| m.slug.to_lowercase());
        Ok(results)
    }

    /// Collect all todo items (`- [ ]` / `- [x]`) across every note in the cave.
    ///
    /// Todos are extracted from raw file lines (including frontmatter so that
    /// line numbers are stable and can be used with [`toggle_todo`]). The
    /// checkbox patterns `- [ ]`, `- [x]`, `- [X]`, `* [ ]`, `* [x]`,
    /// `+ [ ]`, `+ [x]` are all supported, with any leading whitespace.
    ///
    /// Results are split into two sorted lists: incomplete and completed,
    /// each sorted alphabetically by slug then by line number.
    pub fn list_todos(&self) -> Result<TodoList, CaveError> {
        let mut incomplete: Vec<TodoItem> = Vec::new();
        let mut completed: Vec<TodoItem> = Vec::new();

        for (slug, abs_path) in &self.notes {
            let raw = std::fs::read_to_string(abs_path)?;
            let rel = self.relative_path(abs_path);
            let rel_str = rel
                .components()
                .filter_map(|c| {
                    if let std::path::Component::Normal(s) = c {
                        Some(s.to_string_lossy().into_owned())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("/");

            for (idx, line) in raw.lines().enumerate() {
                let trimmed = line.trim_start();
                let is_checked = trimmed.starts_with("- [x] ")
                    || trimmed.starts_with("- [X] ")
                    || trimmed.starts_with("* [x] ")
                    || trimmed.starts_with("* [X] ")
                    || trimmed.starts_with("+ [x] ")
                    || trimmed.starts_with("+ [X] ");
                let is_unchecked = trimmed.starts_with("- [ ] ")
                    || trimmed.starts_with("* [ ] ")
                    || trimmed.starts_with("+ [ ] ");

                if is_checked || is_unchecked {
                    // Strip the marker prefix (e.g. "- [x] " = 6 chars)
                    let text = trimmed[6..].to_string();
                    let item = TodoItem {
                        slug: slug.clone(),
                        relative_path: rel_str.clone(),
                        line: idx + 1, // 1-based
                        text,
                    };
                    if is_checked {
                        completed.push(item);
                    } else {
                        incomplete.push(item);
                    }
                }
            }
        }

        incomplete.sort_by(|a, b| a.slug.cmp(&b.slug).then(a.line.cmp(&b.line)));
        completed.sort_by(|a, b| a.slug.cmp(&b.slug).then(a.line.cmp(&b.line)));
        Ok(TodoList {
            incomplete,
            completed,
        })
    }

    /// Toggle the checkbox on a specific line (1-based) in a note.
    ///
    /// `[ ]` → `[x]`, `[x]`/`[X]` → `[ ]`.  The surrounding marker prefix
    /// (`-`, `*`, `+`) and any leading whitespace are preserved.
    ///
    /// Returns an error if the line does not contain a recognised checkbox pattern.
    pub fn toggle_todo(&self, slug: &str, line: usize) -> Result<(), CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;

        let raw = std::fs::read_to_string(abs_path)?;
        let mut lines: Vec<String> = raw.lines().map(str::to_owned).collect();

        if line == 0 || line > lines.len() {
            return Err(CaveError::InvalidTodoLine(line));
        }

        let target = lines[line - 1].clone();
        let trimmed = target.trim_start();
        let leading_ws = &target[..target.len() - trimmed.len()];

        let toggled = if let Some(rest) = trimmed
            .strip_prefix("- [ ] ")
            .or_else(|| trimmed.strip_prefix("* [ ] "))
            .or_else(|| trimmed.strip_prefix("+ [ ] "))
        {
            let marker = &trimmed[..1];
            format!("{leading_ws}{marker} [x] {rest}")
        } else if let Some(rest) = trimmed
            .strip_prefix("- [x] ")
            .or_else(|| trimmed.strip_prefix("- [X] "))
            .or_else(|| trimmed.strip_prefix("* [x] "))
            .or_else(|| trimmed.strip_prefix("* [X] "))
            .or_else(|| trimmed.strip_prefix("+ [x] "))
            .or_else(|| trimmed.strip_prefix("+ [X] "))
        {
            let marker = &trimmed[..1];
            format!("{leading_ws}{marker} [ ] {rest}")
        } else {
            return Err(CaveError::InvalidTodoLine(line));
        };

        lines[line - 1] = toggled;
        // Preserve trailing newline if original had one
        let mut new_content = lines.join("\n");
        if raw.ends_with('\n') {
            new_content.push('\n');
        }

        // rebuild_with_frontmatter will re-extract the frontmatter from new_content
        // and update modified_at, then write the full file.
        let updated =
            crate::markdown::rebuild_with_frontmatter(&new_content, &new_content, None, None, None);
        std::fs::write(abs_path, updated)?;
        Ok(())
    }

    /// Toggle the checkbox identified by its 0-based index among all checkboxes
    /// in a note. This is used by the reader view, which counts checkboxes in
    /// rendered HTML and needs to map back to a source line.
    pub fn toggle_todo_by_index(&self, slug: &str, index: usize) -> Result<(), CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;

        let raw = std::fs::read_to_string(abs_path)?;
        let mut checkbox_count = 0usize;
        let mut target_line: Option<usize> = None;

        for (idx, line) in raw.lines().enumerate() {
            let trimmed = line.trim_start();
            let is_checkbox = trimmed.starts_with("- [ ] ")
                || trimmed.starts_with("- [x] ")
                || trimmed.starts_with("- [X] ")
                || trimmed.starts_with("* [ ] ")
                || trimmed.starts_with("* [x] ")
                || trimmed.starts_with("* [X] ")
                || trimmed.starts_with("+ [ ] ")
                || trimmed.starts_with("+ [x] ")
                || trimmed.starts_with("+ [X] ");

            if is_checkbox {
                if checkbox_count == index {
                    target_line = Some(idx + 1); // 1-based
                    break;
                }
                checkbox_count += 1;
            }
        }

        let line = target_line.ok_or(CaveError::InvalidTodoLine(index))?;
        self.toggle_todo(slug, line)
    }

    /// Read a note by slug.
    pub fn read_note(&self, slug: &str) -> Result<Note, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;

        let raw = std::fs::read_to_string(abs_path)?;
        let body = crate::markdown::strip_frontmatter(&raw).to_string();
        let rel = self.relative_path(abs_path);
        let mut meta = note_meta_from_relative_path(&rel);
        meta.icon = crate::markdown::read_frontmatter_icon(&raw);
        meta.favorite = crate::markdown::read_frontmatter_favorite(&raw);
        Ok(Note {
            meta,
            content: body,
        })
    }

    /// Read a template by slug.
    pub fn read_template(&self, slug: &str) -> Result<Template, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .templates
            .get(slug)
            .ok_or_else(|| CaveError::TemplateNotFound(slug.to_string()))?;

        let raw = std::fs::read_to_string(abs_path)?;
        let body = crate::markdown::strip_frontmatter(&raw).to_string();
        let mut meta = template_meta_from_path(abs_path);
        meta.icon = crate::markdown::read_frontmatter_icon(&raw);
        Ok(Template {
            meta,
            content: body,
        })
    }

    /// Read the raw file content of a note (including frontmatter).
    pub fn read_note_raw(&self, slug: &str) -> Result<String, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;
        Ok(std::fs::read_to_string(abs_path)?)
    }

    /// Read the raw file content of a template (including frontmatter).
    pub fn read_template_raw(&self, slug: &str) -> Result<String, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .templates
            .get(slug)
            .ok_or_else(|| CaveError::TemplateNotFound(slug.to_string()))?;
        Ok(std::fs::read_to_string(abs_path)?)
    }

    /// Save new content to an existing note (looked up by slug).
    pub fn save_note(&mut self, slug: &str, content: &str) -> Result<NoteMeta, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?
            .clone();

        let existing_raw = std::fs::read_to_string(&abs_path)?;
        let updated =
            crate::markdown::rebuild_with_frontmatter(&existing_raw, content, None, None, None);
        std::fs::write(&abs_path, updated.as_str())?;
        self.rebuild_backlinks();
        let rel = self.relative_path(&abs_path);
        let mut meta = note_meta_from_relative_path(&rel);
        meta.icon = crate::markdown::read_frontmatter_icon(&updated);
        meta.favorite = crate::markdown::read_frontmatter_favorite(&updated);
        Ok(meta)
    }

    /// Save new content to an existing template (looked up by slug).
    pub fn save_template(&self, slug: &str, content: &str) -> Result<TemplateMeta, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .templates
            .get(slug)
            .ok_or_else(|| CaveError::TemplateNotFound(slug.to_string()))?;

        let existing_raw = std::fs::read_to_string(abs_path)?;
        let updated =
            crate::markdown::rebuild_with_frontmatter(&existing_raw, content, None, None, None);
        std::fs::write(abs_path, updated.as_str())?;
        let mut meta = template_meta_from_path(abs_path);
        meta.icon = crate::markdown::read_frontmatter_icon(&updated);
        Ok(meta)
    }

    /// Update only a note's icon while preserving the current body and tags.
    pub fn set_note_icon(&self, slug: &str, icon: Option<String>) -> Result<NoteMeta, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;

        let existing_raw = std::fs::read_to_string(abs_path)?;
        let existing_body = crate::markdown::strip_frontmatter(&existing_raw);
        let updated = crate::markdown::rebuild_with_frontmatter(
            &existing_raw,
            existing_body,
            None,
            icon,
            None,
        );
        std::fs::write(abs_path, &updated)?;

        let rel = self.relative_path(abs_path);
        let mut meta = note_meta_from_relative_path(&rel);
        meta.icon = crate::markdown::read_frontmatter_icon(&updated);
        meta.favorite = crate::markdown::read_frontmatter_favorite(&updated);
        Ok(meta)
    }

    /// Replace `old_text` with `new_text` in an existing note (looked up by slug).
    /// Fails if `old_text` is not found in the note's content.
    #[allow(dead_code)]
    pub fn edit_note(
        &mut self,
        slug: &str,
        old_text: &str,
        new_text: &str,
    ) -> Result<NoteMeta, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?
            .clone();

        let raw = std::fs::read_to_string(&abs_path)?;
        let body = crate::markdown::strip_frontmatter(&raw);
        if !body.contains(old_text) {
            return Err(CaveError::EditNotFound);
        }
        let new_body = body.replacen(old_text, new_text, 1);
        let new_content =
            crate::markdown::rebuild_with_frontmatter(&raw, &new_body, None, None, None);
        std::fs::write(&abs_path, &new_content)?;
        self.rebuild_backlinks();

        let rel = self.relative_path(&abs_path);
        Ok(note_meta_from_relative_path(&rel))
    }

    /// Delete a note by slug.
    pub fn delete_note(&mut self, slug: &str) -> Result<(), CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?
            .clone();

        std::fs::remove_file(&abs_path)?;
        self.notes.remove(slug);
        self.rebuild_backlinks();
        Ok(())
    }

    /// Delete a template by slug.
    pub fn delete_template(&mut self, slug: &str) -> Result<(), CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .templates
            .get(slug)
            .ok_or_else(|| CaveError::TemplateNotFound(slug.to_string()))?
            .clone();

        std::fs::remove_file(&abs_path)?;
        self.templates.remove(slug);
        Ok(())
    }

    /// Move a note to a different folder within the cave.
    ///
    /// `destination` is a relative path from the cave root (e.g. `Path::new("projects")`).
    /// Passing `None` moves the note to the cave root.
    pub fn move_note(
        &mut self,
        slug: &str,
        destination: Option<&Path>,
    ) -> Result<NoteMeta, CaveError> {
        validate_name(slug)?;
        let old_abs = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?
            .clone();

        let dest_dir = if let Some(d) = destination {
            validate_folder_path(d)?;
            let abs = self.path.join(d);
            if !abs.is_dir() {
                return Err(CaveError::NotFound(d.to_string_lossy().into_owned()));
            }
            self.check_containment(&abs)?;
            abs
        } else {
            self.path.clone()
        };

        let filename = old_abs
            .file_name()
            .ok_or_else(|| CaveError::InvalidName("note has no filename".into()))?;
        let new_abs = dest_dir.join(filename);

        // No-op if already in the target dir.
        if old_abs == new_abs {
            let rel = self.relative_path(&old_abs);
            return Ok(note_meta_with_frontmatter(&rel, &old_abs));
        }

        // Prevent overwriting an existing file at the destination.
        if new_abs.exists() {
            return Err(CaveError::AlreadyExists(
                new_abs.to_string_lossy().into_owned(),
            ));
        }

        std::fs::rename(&old_abs, &new_abs)?;
        self.notes.insert(slug.to_string(), new_abs.clone());

        let rel = self.relative_path(&new_abs);
        Ok(note_meta_with_frontmatter(&rel, &new_abs))
    }

    /// Move a folder under a new parent within the cave.
    ///
    /// `source` is the relative path of the folder to move.
    /// `destination` is the relative path of the new parent directory (`None` = cave root).
    pub fn move_folder(
        &mut self,
        source: &Path,
        destination: Option<&Path>,
    ) -> Result<(), CaveError> {
        validate_folder_path(source)?;
        if let Some(d) = destination {
            validate_folder_path(d)?;
        }

        let src_abs = self.path.join(source);
        if !src_abs.is_dir() {
            return Err(CaveError::NotFound(source.to_string_lossy().into_owned()));
        }
        self.check_containment(&src_abs)?;

        let folder_name = source
            .file_name()
            .ok_or_else(|| CaveError::InvalidName("folder has no name".into()))?;

        let dest_parent = if let Some(d) = destination {
            let abs = self.path.join(d);
            if !abs.is_dir() {
                return Err(CaveError::NotFound(d.to_string_lossy().into_owned()));
            }
            self.check_containment(&abs)?;
            abs
        } else {
            self.path.clone()
        };

        let new_abs = dest_parent.join(folder_name);

        // No-op.
        if src_abs == new_abs {
            return Ok(());
        }

        // Prevent moving into itself or a subdirectory of itself.
        if new_abs.starts_with(&src_abs) {
            return Err(CaveError::InvalidName(
                "cannot move a folder into itself".into(),
            ));
        }

        if new_abs.exists() {
            return Err(CaveError::AlreadyExists(
                new_abs.to_string_lossy().into_owned(),
            ));
        }

        std::fs::rename(&src_abs, &new_abs)?;

        self.update_child_paths(&src_abs, &new_abs);

        Ok(())
    }

    /// Rename an existing note in-place (same directory, new filename).
    /// The new slug must not already exist anywhere in the cave.
    pub fn rename_note(&mut self, old_slug: &str, new_name: &str) -> Result<NoteMeta, CaveError> {
        validate_name(old_slug)?;
        validate_name(new_name)?;

        // No-op if nothing changed.
        if old_slug == new_name {
            return self.read_note(old_slug).map(|n| n.meta);
        }

        let old_abs = self
            .notes
            .get(old_slug)
            .ok_or_else(|| CaveError::NotFound(old_slug.to_string()))?
            .clone();

        let new_filename = ensure_md_extension(new_name);
        let new_abs = old_abs
            .parent()
            .unwrap_or(Path::new(""))
            .join(&new_filename);

        // Global uniqueness check.
        if self.notes.contains_key(new_name) {
            return Err(CaveError::AlreadyExists(new_filename));
        }

        std::fs::rename(&old_abs, &new_abs)?;
        self.notes.remove(old_slug);
        self.notes.insert(new_name.to_string(), new_abs.clone());
        self.rebuild_backlinks();

        let rel = self.relative_path(&new_abs);
        Ok(note_meta_with_frontmatter(&rel, &new_abs))
    }

    /// Rename an existing template in-place within `.granit/templates`.
    pub fn rename_template(
        &mut self,
        old_slug: &str,
        new_name: &str,
    ) -> Result<TemplateMeta, CaveError> {
        validate_name(old_slug)?;
        validate_name(new_name)?;

        if old_slug == new_name {
            return self.read_template(old_slug).map(|template| template.meta);
        }

        let old_abs = self
            .templates
            .get(old_slug)
            .ok_or_else(|| CaveError::TemplateNotFound(old_slug.to_string()))?
            .clone();

        let new_filename = ensure_md_extension(new_name);
        let new_abs = old_abs
            .parent()
            .unwrap_or(Path::new(""))
            .join(&new_filename);

        if self.templates.contains_key(new_name) {
            return Err(CaveError::TemplateAlreadyExists(new_filename));
        }

        std::fs::rename(&old_abs, &new_abs)?;
        self.templates.remove(old_slug);
        self.templates.insert(new_name.to_string(), new_abs.clone());

        Ok(template_meta_with_icon(&new_abs))
    }

    /// Update a note's filename, content, and optionally tags and icon in one operation.
    ///
    /// If `old_slug` and `new_name` differ the file is renamed first (same directory),
    /// then the new content is written. If `tags` is `Some`, the frontmatter tags
    /// are replaced. If `icon` is `Some`, the icon is set (`Some("")` clears it).
    pub fn update_note(
        &mut self,
        old_slug: &str,
        new_name: &str,
        content: &str,
        tags: Option<Vec<String>>,
        icon: Option<String>,
        favorite: Option<bool>,
    ) -> Result<NoteMeta, CaveError> {
        validate_name(old_slug)?;
        validate_name(new_name)?;

        let old_abs = self
            .notes
            .get(old_slug)
            .ok_or_else(|| CaveError::NotFound(old_slug.to_string()))?
            .clone();

        let (final_abs, renamed) = if old_slug != new_name {
            let new_filename = ensure_md_extension(new_name);
            let new_abs = old_abs
                .parent()
                .unwrap_or(Path::new(""))
                .join(&new_filename);

            if self.notes.contains_key(new_name) {
                return Err(CaveError::AlreadyExists(new_filename));
            }

            std::fs::rename(&old_abs, &new_abs)?;
            (new_abs, true)
        } else {
            (old_abs.clone(), false)
        };

        let existing_raw = std::fs::read_to_string(&final_abs)?;
        let updated =
            crate::markdown::rebuild_with_frontmatter(&existing_raw, content, tags, icon, favorite);
        if let Err(e) = std::fs::write(&final_abs, updated.as_str()) {
            // Rollback the rename so index stays consistent with filesystem.
            if renamed {
                if let Err(rollback_err) = std::fs::rename(&final_abs, &old_abs) {
                    return Err(CaveError::Io(format!(
                        "failed to write updated note after rename: {e}; rollback also failed: {rollback_err}"
                    )));
                }
            }
            return Err(e.into());
        }

        // Update index only after all filesystem operations succeed.
        if renamed {
            self.notes.remove(old_slug);
            self.notes.insert(new_name.to_string(), final_abs.clone());
        }
        self.rebuild_backlinks();

        let rel = self.relative_path(&final_abs);
        let mut meta = note_meta_from_relative_path(&rel);
        meta.icon = crate::markdown::read_frontmatter_icon(&updated);
        meta.favorite = crate::markdown::read_frontmatter_favorite(&updated);
        Ok(meta)
    }

    /// Update a template's filename, content, and optionally tags and icon in one operation.
    pub fn update_template(
        &mut self,
        old_slug: &str,
        new_name: &str,
        content: &str,
        tags: Option<Vec<String>>,
        icon: Option<String>,
    ) -> Result<TemplateMeta, CaveError> {
        validate_name(old_slug)?;
        validate_name(new_name)?;

        let old_abs = self
            .templates
            .get(old_slug)
            .ok_or_else(|| CaveError::TemplateNotFound(old_slug.to_string()))?
            .clone();

        let (final_abs, renamed) = if old_slug != new_name {
            let new_filename = ensure_md_extension(new_name);
            let new_abs = old_abs
                .parent()
                .unwrap_or(Path::new(""))
                .join(&new_filename);

            if self.templates.contains_key(new_name) {
                return Err(CaveError::TemplateAlreadyExists(new_filename));
            }

            std::fs::rename(&old_abs, &new_abs)?;
            (new_abs, true)
        } else {
            (old_abs.clone(), false)
        };

        let existing_raw = std::fs::read_to_string(&final_abs)?;
        let updated =
            crate::markdown::rebuild_with_frontmatter(&existing_raw, content, tags, icon, None);
        if let Err(e) = std::fs::write(&final_abs, updated.as_str()) {
            if renamed {
                if let Err(rollback_err) = std::fs::rename(&final_abs, &old_abs) {
                    return Err(CaveError::Io(format!(
                        "failed to write updated template after rename: {e}; rollback also failed: {rollback_err}"
                    )));
                }
            }
            return Err(e.into());
        }

        if renamed {
            self.templates.remove(old_slug);
            self.templates
                .insert(new_name.to_string(), final_abs.clone());
        }

        let mut meta = template_meta_from_path(&final_abs);
        meta.icon = crate::markdown::read_frontmatter_icon(&updated);
        Ok(meta)
    }

    /// Rename a folder in-place (same parent directory, new name).
    ///
    /// `source` is the relative path of the folder to rename.
    /// `new_name` is the new name for the folder (just the final component, not a path).
    /// Verify that `abs_path` is contained within the cave root by comparing
    /// canonical paths. Walks up to the nearest existing ancestor so this works
    /// for paths that are about to be created as well as paths that already exist.
    /// Returns `CaveError::InvalidName` if the path escapes the cave root.
    fn check_containment(&self, abs_path: &Path) -> Result<(), CaveError> {
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

    /// Update indexed note paths after a folder is moved or renamed.
    /// Replaces `old_prefix` with `new_prefix` for every note under the old location.
    fn update_child_paths(&mut self, old_prefix: &Path, new_prefix: &Path) {
        let updates: Vec<(String, PathBuf)> = self
            .notes
            .iter()
            .filter(|(_, abs)| abs.starts_with(old_prefix))
            .map(|(slug, abs)| {
                let suffix = abs.strip_prefix(old_prefix).unwrap();
                (slug.clone(), new_prefix.join(suffix))
            })
            .collect();
        for (slug, new_path) in updates {
            self.notes.insert(slug, new_path);
        }
    }

    pub fn rename_folder(&mut self, source: &Path, new_name: &str) -> Result<(), CaveError> {
        validate_folder_path(source)?;
        validate_name(new_name)?;

        let src_abs = self.path.join(source);
        if !src_abs.is_dir() {
            return Err(CaveError::NotFound(source.to_string_lossy().into_owned()));
        }
        self.check_containment(&src_abs)?;

        let parent = src_abs.parent().unwrap_or(&self.path);
        let new_abs = parent.join(new_name);

        // No-op if nothing changed.
        if src_abs == new_abs {
            return Ok(());
        }

        if new_abs.exists() {
            return Err(CaveError::AlreadyExists(
                new_abs.to_string_lossy().into_owned(),
            ));
        }

        std::fs::rename(&src_abs, &new_abs)?;

        self.update_child_paths(&src_abs, &new_abs);

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
    fn test_cave_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.create_note("hello", None, None).unwrap();
        assert_eq!(meta.slug, "hello");

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);

        let note = cave.read_note("hello").unwrap();
        assert!(note.content.is_empty() || !note.content.contains("---"));

        cave.save_note("hello", "# Updated\nBody").unwrap();
        let note = cave.read_note("hello").unwrap();
        assert!(note.content.contains("# Updated"));
        assert_eq!(note.meta.slug, "hello");

        cave.rename_note("hello", "world").unwrap();
        assert!(cave.read_note("hello").is_err());
        assert!(cave.read_note("world").is_ok());

        cave.delete_note("world").unwrap();
        assert!(cave.read_note("world").is_err());
    }

    #[test]
    fn test_create_note() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.create_note("my-note", None, None).unwrap();
        assert_eq!(meta.slug, "my-note");
        assert_eq!(meta.relative_path, "my-note.md");

        let content = std::fs::read_to_string(dir.path().join("my-note.md")).unwrap();
        assert!(content.contains("created_at"));
    }

    #[test]
    fn test_create_note_in_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        std::fs::create_dir(dir.path().join("notes")).unwrap();

        let meta = cave
            .create_note("nested", Some(Path::new("notes")), None)
            .unwrap();
        assert_eq!(meta.slug, "nested");
        assert_eq!(meta.relative_path, "notes/nested.md");
        assert!(dir.path().join("notes/nested.md").exists());
    }

    #[test]
    fn test_create_note_folder_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .create_note("note", Some(Path::new("nonexistent")), None)
            .unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_create_note_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_note("test", None, None).unwrap();
        let err = cave.create_note("test", None, None).unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_create_note_duplicate_slug_across_folders() {
        // Two notes with the same filename in different folders should be rejected.
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();

        cave.create_note("foo", None, None).unwrap();
        let err = cave
            .create_note("foo", Some(Path::new("sub")), None)
            .unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_create_untitled_auto_numbering() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let m1 = cave.create_note("untitled", None, None).unwrap();
        assert_eq!(m1.slug, "untitled");

        let m2 = cave.create_note("untitled", None, None).unwrap();
        assert_eq!(m2.slug, "untitled-2");

        let m3 = cave.create_note("untitled", None, None).unwrap();
        assert_eq!(m3.slug, "untitled-3");
    }

    #[test]
    fn test_create_note_uses_explicit_template_body() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
        std::fs::write(
            dir.path().join(".granit/templates/starter.md"),
            "---\ntags: [template]\nicon: Star\n---\n# {{ slug }}\nBody\n",
        )
        .unwrap();
        cave.templates = Cave::scan_templates(&dir.path().join(".granit/templates")).unwrap();

        let meta = cave
            .create_note("project-kickoff", None, Some("starter"))
            .unwrap();
        let raw = cave.read_note_raw(&meta.slug).unwrap();
        let note = cave.read_note(&meta.slug).unwrap();

        assert_eq!(note.content, "# project-kickoff\nBody\n");
        assert_eq!(note.meta.icon.as_deref(), Some("Star"));
        assert_eq!(
            crate::markdown::read_frontmatter_tags(&raw),
            vec!["template".to_string()]
        );
        assert!(raw.contains("icon: Star"));
        assert!(raw.contains("created_at"));
    }

    #[test]
    fn test_create_note_daily_slug_uses_configured_daily_template() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_config().unwrap();

        std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
        std::fs::write(
            dir.path().join(".granit/templates/daily-template.md"),
            "# {{ date }}\nNext: {{ tomorrow }}\nYesterday: {{ yesterday }}\n{{ weekday }} / {{ weekday_short }}\n",
        )
        .unwrap();
        cave.templates = Cave::scan_templates(&dir.path().join(".granit/templates")).unwrap();
        cave.save_config(&AppConfig {
            daily_note_template_slug: Some("daily-template".to_string()),
            ..AppConfig::default()
        })
        .unwrap();

        let meta = cave.create_note("2024-02-29", None, None).unwrap();
        let note = cave.read_note(&meta.slug).unwrap();

        assert_eq!(meta.relative_path, "Daily/2024-02-29.md");
        assert_eq!(
            note.content,
            "# 2024-02-29\nNext: 2024-03-01\nYesterday: 2024-02-28\nThursday / Thu\n"
        );
    }

    #[test]
    fn test_create_note_explicit_template_overrides_daily_template() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_config().unwrap();

        std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
        std::fs::write(
            dir.path().join(".granit/templates/daily-template.md"),
            "daily {{ date }}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join(".granit/templates/manual-template.md"),
            "manual {{ slug }}\n",
        )
        .unwrap();
        cave.templates = Cave::scan_templates(&dir.path().join(".granit/templates")).unwrap();
        cave.save_config(&AppConfig {
            daily_note_template_slug: Some("daily-template".to_string()),
            ..AppConfig::default()
        })
        .unwrap();

        let meta = cave
            .create_note("2024-02-29", None, Some("manual-template"))
            .unwrap();
        let note = cave.read_note(&meta.slug).unwrap();

        assert_eq!(meta.relative_path, "Daily/2024-02-29.md");
        assert_eq!(note.content, "manual 2024-02-29\n");
    }

    #[test]
    fn test_create_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_folder(Path::new("projects")).unwrap();
        assert!(dir.path().join("projects").is_dir());
    }

    #[test]
    fn test_create_folder_nested() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_folder(Path::new("a/b/c")).unwrap();
        assert!(dir.path().join("a/b/c").is_dir());
    }

    #[test]
    fn test_create_folder_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        std::fs::create_dir(dir.path().join("existing")).unwrap();
        let err = cave.create_folder(Path::new("existing")).unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_delete_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/note.md"), "# Note").unwrap();
        std::fs::write(dir.path().join("root.md"), "").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        assert!(cave.notes.contains_key("note"));
        cave.delete_folder(Path::new("sub")).unwrap();
        assert!(
            !cave.notes.contains_key("note"),
            "note should be removed from index"
        );
        assert!(cave.notes.contains_key("root"), "root note should remain");
        assert!(
            !dir.path().join("sub").exists(),
            "folder should be deleted from disk"
        );
    }

    #[test]
    fn test_delete_folder_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let err = cave.delete_folder(Path::new("ghost")).unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_delete_folder_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let err = cave.delete_folder(Path::new("../escape")).unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    #[test]
    fn test_list_notes_recursive() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("alpha.md"), "# Alpha\n").unwrap();
        std::fs::write(dir.path().join("sub/beta.md"), "# Beta\n").unwrap();
        std::fs::write(dir.path().join("not-a-note.txt"), "ignore").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 2);
        let slugs: Vec<_> = notes.iter().map(|n| n.slug.as_str()).collect();
        assert!(slugs.contains(&"alpha"));
        assert!(slugs.contains(&"beta"));

        let beta = notes.iter().find(|n| n.slug == "beta").unwrap();
        assert_eq!(beta.relative_path, "sub/beta.md");
    }

    #[test]
    fn test_list_notes_skips_hidden_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".hidden")).unwrap();
        std::fs::write(dir.path().join("visible.md"), "").unwrap();
        std::fs::write(dir.path().join(".hidden/secret.md"), "").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].slug, "visible");
    }

    #[test]
    fn test_list_notes_skips_non_md_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "").unwrap();
        std::fs::write(dir.path().join("image.png"), "").unwrap();
        std::fs::write(dir.path().join("readme.txt"), "").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].slug, "note");
    }

    #[test]
    fn test_read_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Test Note\nBody").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let note = cave.read_note("test").unwrap();
        assert_eq!(note.meta.slug, "test");
        assert!(note.content.contains("Body"));
    }

    #[test]
    fn test_read_note_nested() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/deep.md"), "# Deep\nContent").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let note = cave.read_note("deep").unwrap();
        assert_eq!(note.meta.slug, "deep");
        assert_eq!(note.meta.relative_path, "sub/deep.md");
    }

    #[test]
    fn test_read_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.read_note("nonexistent").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_read_note_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.read_note("../etc/passwd").unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    #[test]
    fn test_save_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Old\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.save_note("test", "# New Title\nNew body").unwrap();
        assert_eq!(meta.slug, "test");

        let content = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert!(content.contains("New body"));
    }

    #[test]
    fn test_save_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.save_note("missing", "content").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_save_note_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.save_note("../escape", "content").unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    #[test]
    fn test_edit_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Hello\nold text here").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.edit_note("test", "old text", "new text").unwrap();
        assert_eq!(meta.slug, "test");

        let content = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert!(content.contains("new text here"));
        assert!(!content.contains("old text"));
    }

    #[test]
    fn test_edit_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.edit_note("missing", "old", "new").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_edit_note_text_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "# Hello\nsome content").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .edit_note("test", "nonexistent text", "replacement")
            .unwrap_err();
        assert!(matches!(err, CaveError::EditNotFound));
    }

    #[test]
    fn test_edit_note_replaces_first_occurrence_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.md"), "AAA BBB AAA").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.edit_note("test", "AAA", "CCC").unwrap();
        let content = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert_eq!(content, "CCC BBB AAA");
    }

    #[test]
    fn test_open_builds_backlinks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(dir.path().join("source-a.md"), "[[target]]\n").unwrap();
        std::fs::write(dir.path().join("source-b.md"), "[[target]]\n").unwrap();

        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        assert_eq!(
            cave.backlink_slugs("target").unwrap(),
            vec!["source-a".to_string(), "source-b".to_string()]
        );
    }

    #[test]
    fn test_backlinks_deduplicate_repeated_links_from_same_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(
            dir.path().join("source.md"),
            "[[target]] and [[target|again]]\n",
        )
        .unwrap();

        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        assert_eq!(
            cave.backlink_slugs("target").unwrap(),
            vec!["source".to_string()]
        );
    }

    #[test]
    fn test_save_note_rebuilds_backlinks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(dir.path().join("source.md"), "No links yet\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        assert!(cave.backlink_slugs("target").unwrap().is_empty());

        cave.save_note("source", "[[target]]\n").unwrap();

        assert_eq!(
            cave.backlink_slugs("target").unwrap(),
            vec!["source".to_string()]
        );
    }

    #[test]
    fn test_rename_note_rebuilds_backlink_source_slug() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(dir.path().join("source.md"), "[[target]]\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.rename_note("source", "renamed-source").unwrap();

        assert_eq!(
            cave.backlink_slugs("target").unwrap(),
            vec!["renamed-source".to_string()]
        );
    }

    #[test]
    fn test_delete_note_rebuilds_backlinks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(dir.path().join("source.md"), "[[target]]\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.delete_note("source").unwrap();

        assert!(cave.backlink_slugs("target").unwrap().is_empty());
    }

    #[test]
    fn test_delete_folder_rebuilds_backlinks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(dir.path().join("sub/source.md"), "[[target]]\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.delete_folder(Path::new("sub")).unwrap();

        assert!(cave.backlink_slugs("target").unwrap().is_empty());
    }

    #[test]
    fn test_delete_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("doomed.md"), "# Bye\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        assert!(dir.path().join("doomed.md").exists());

        cave.delete_note("doomed").unwrap();
        assert!(!dir.path().join("doomed.md").exists());
    }

    #[test]
    fn test_delete_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.delete_note("ghost").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_delete_note_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.delete_note("../escape").unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    // ── move_note tests ────────────────────────────────────────────

    #[test]
    fn test_move_note_to_subfolder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "# Note").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.move_note("note", Some(Path::new("sub"))).unwrap();
        assert_eq!(meta.slug, "note");
        assert_eq!(meta.relative_path, "sub/note.md");
        assert!(!dir.path().join("note.md").exists());
        assert!(dir.path().join("sub/note.md").exists());
    }

    #[test]
    fn test_move_note_to_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/note.md"), "# Note").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.move_note("note", None).unwrap();
        assert_eq!(meta.relative_path, "note.md");
        assert!(dir.path().join("note.md").exists());
        assert!(!dir.path().join("sub/note.md").exists());
    }

    #[test]
    fn test_move_note_noop() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/note.md"), "# Note").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.move_note("note", Some(Path::new("sub"))).unwrap();
        assert_eq!(meta.relative_path, "sub/note.md");
    }

    #[test]
    fn test_move_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.move_note("ghost", Some(Path::new("sub"))).unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_move_note_dest_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .move_note("note", Some(Path::new("nonexistent")))
            .unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_move_note_file_already_exists_at_dest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.create_note("note", None, None).unwrap();
        // Place a file at the destination path that isn't in the index.
        std::fs::write(dir.path().join("sub/note.md"), "conflict").unwrap();

        let err = cave.move_note("note", Some(Path::new("sub"))).unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
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

    // ── move_folder tests ──────────────────────────────────────────

    #[test]
    fn test_move_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::create_dir(dir.path().join("dest")).unwrap();
        std::fs::write(dir.path().join("src/note.md"), "# Note").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.move_folder(Path::new("src"), Some(Path::new("dest")))
            .unwrap();
        assert!(dir.path().join("dest/src/note.md").exists());
        assert!(!dir.path().join("src").exists());
        // Index should be updated.
        let note = cave.read_note("note").unwrap();
        assert_eq!(note.meta.relative_path, "dest/src/note.md");
    }

    #[test]
    fn test_move_folder_to_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("parent/child")).unwrap();
        std::fs::write(dir.path().join("parent/child/note.md"), "").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.move_folder(Path::new("parent/child"), None).unwrap();
        assert!(dir.path().join("child/note.md").exists());
        let note = cave.read_note("note").unwrap();
        assert_eq!(note.meta.relative_path, "child/note.md");
    }

    #[test]
    fn test_move_folder_noop() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("parent/child")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.move_folder(Path::new("parent/child"), Some(Path::new("parent")))
            .unwrap();
        assert!(dir.path().join("parent/child").exists());
    }

    #[test]
    fn test_move_folder_into_itself_rejected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .move_folder(Path::new("a"), Some(Path::new("a/b")))
            .unwrap_err();
        assert!(matches!(err, CaveError::InvalidName(_)));
    }

    #[test]
    fn test_move_folder_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.move_folder(Path::new("ghost"), None).unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_move_folder_dest_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a")).unwrap();
        std::fs::create_dir(dir.path().join("b")).unwrap();
        std::fs::create_dir(dir.path().join("b/a")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .move_folder(Path::new("a"), Some(Path::new("b")))
            .unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_rename_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.md"), "# Old Note\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.rename_note("old", "new-name").unwrap();
        assert_eq!(meta.slug, "new-name");
        assert_eq!(meta.relative_path, "new-name.md");
        assert!(!dir.path().join("old.md").exists());
        assert!(dir.path().join("new-name.md").exists());
    }

    #[test]
    fn test_rename_note_noop() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("same.md"), "# Same\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.rename_note("same", "same").unwrap();
        assert_eq!(meta.slug, "same");
        assert!(dir.path().join("same.md").exists());
    }

    #[test]
    fn test_rename_note_target_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# A\n").unwrap();
        std::fs::write(dir.path().join("b.md"), "# B\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.rename_note("a", "b").unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_rename_note_target_exists_in_other_folder() {
        // Rename rejected even if the conflicting slug lives in a different folder.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("a.md"), "# A\n").unwrap();
        std::fs::write(dir.path().join("sub/b.md"), "# B\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.rename_note("a", "b").unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
    }

    #[test]
    fn test_rename_note_source_missing() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.rename_note("missing", "new").unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_update_note_content_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "old content").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave
            .update_note("note", "note", "new content", None, None, None)
            .unwrap();
        assert_eq!(meta.slug, "note");

        let saved = std::fs::read_to_string(dir.path().join("note.md")).unwrap();
        assert_eq!(saved, "new content");
    }

    #[test]
    fn test_update_note_rename_and_save() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.md"), "original content").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave
            .update_note("old", "new-name", "updated content", None, None, None)
            .unwrap();
        assert_eq!(meta.slug, "new-name");
        assert!(!dir.path().join("old.md").exists());

        let saved = std::fs::read_to_string(dir.path().join("new-name.md")).unwrap();
        assert_eq!(saved, "updated content");
    }

    #[test]
    fn test_update_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .update_note("ghost", "ghost", "content", None, None, None)
            .unwrap_err();
        assert!(matches!(err, CaveError::NotFound(_)));
    }

    #[test]
    fn test_update_note_rename_target_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "a content").unwrap();
        std::fs::write(dir.path().join("b.md"), "b content").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave
            .update_note("a", "b", "new content", None, None, None)
            .unwrap_err();
        assert!(matches!(err, CaveError::AlreadyExists(_)));
        // Original file must be untouched
        let content = std::fs::read_to_string(dir.path().join("a.md")).unwrap();
        assert_eq!(content, "a content");
    }

    #[test]
    fn test_list_notes_reads_favorite_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("favorite.md"),
            "---\nfavorite: true\nicon: Star\n---\nBody",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].favorite, Some(true));
        assert_eq!(notes[0].icon.as_deref(), Some("Star"));
    }

    #[test]
    fn test_read_note_reads_favorite_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("favorite.md"),
            "---\nfavorite: false\n---\nBody",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let note = cave.read_note("favorite").unwrap();
        assert_eq!(note.meta.favorite, Some(false));
        assert_eq!(note.content, "Body");
    }

    #[test]
    fn test_update_note_sets_favorite_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("note.md"),
            "---\ncreated_at: \"2026-01-01T00:00:00Z\"\nmodified_at: \"2026-01-01T00:00:00Z\"\n---\nBody",
        )
        .unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave
            .update_note("note", "note", "Updated", None, None, Some(true))
            .unwrap();
        let raw = std::fs::read_to_string(dir.path().join("note.md")).unwrap();

        assert_eq!(meta.favorite, Some(true));
        assert!(
            raw.contains("favorite: true"),
            "favorite should be persisted: {raw}"
        );
        assert!(raw.contains("Updated"), "body should be updated: {raw}");
    }

    #[test]
    fn test_update_note_adds_frontmatter_to_legacy_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("legacy.md"), "Legacy body").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave
            .update_note(
                "legacy",
                "legacy",
                "Updated body",
                Some(vec!["legacy".into(), "migrated".into()]),
                Some("Star".into()),
                Some(true),
            )
            .unwrap();
        let raw = std::fs::read_to_string(dir.path().join("legacy.md")).unwrap();

        assert_eq!(meta.icon.as_deref(), Some("Star"));
        assert_eq!(meta.favorite, Some(true));
        assert!(raw.starts_with("---\n"), "frontmatter should be added: {raw}");
        assert!(
            raw.contains("tags:\n- legacy\n- migrated"),
            "tags should be persisted: {raw}"
        );
        assert!(raw.contains("icon: Star"), "icon should be persisted: {raw}");
        assert!(
            raw.contains("favorite: true"),
            "favorite should be persisted: {raw}"
        );
        assert!(
            raw.contains("created_at:"),
            "created_at should be initialized: {raw}"
        );
        assert!(
            raw.contains("modified_at:"),
            "modified_at should be initialized: {raw}"
        );
        assert!(
            raw.ends_with("Updated body"),
            "body should be updated: {raw}"
        );
        assert_eq!(
            crate::markdown::read_frontmatter_tags(&raw),
            vec!["legacy", "migrated"]
        );
    }

    /// A note whose _content_ cannot be read still appears in list_notes.
    #[test]
    #[cfg(unix)]
    fn test_list_notes_includes_unreadable_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let unreadable = dir.path().join("locked.md");
        std::fs::write(&unreadable, "secret").unwrap();
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();
        // Open cave after writing so index is populated.
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        // listing still succeeds and includes the entry (slug is in the index from open-time scan)
        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].slug, "locked");

        // restore permissions so tempdir cleanup can delete the file
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[test]
    fn test_list_folders_empty_cave() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let folders = cave.list_folders().unwrap();
        assert!(folders.is_empty());
    }

    #[test]
    fn test_list_folders_includes_empty_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("empty")).unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let folders = cave.list_folders().unwrap();
        assert_eq!(folders, vec!["empty"]);
    }

    #[test]
    fn test_list_folders_nested() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
        std::fs::create_dir(dir.path().join("c")).unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let folders = cave.list_folders().unwrap();
        assert_eq!(folders, vec!["a", "a/b", "c"]);
    }

    #[test]
    fn test_list_folders_skips_hidden_and_granit() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".hidden")).unwrap();
        std::fs::create_dir_all(dir.path().join(".granit")).unwrap();
        std::fs::create_dir(dir.path().join("visible")).unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let folders = cave.list_folders().unwrap();
        assert_eq!(folders, vec!["visible"]);
    }

    #[test]
    fn test_templates_are_listed_separately_from_notes() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_template("daily-template").unwrap();
        cave.create_note("note", None, None).unwrap();

        let notes = cave.list_notes().unwrap();
        let templates = cave.list_templates().unwrap();

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].slug, "note");
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].slug, "daily-template");
        assert!(!dir.path().join("daily-template.md").exists());
        assert!(dir
            .path()
            .join(".granit/templates/daily-template.md")
            .exists());
    }

    #[test]
    fn test_update_template_renames_and_preserves_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_template("daily-template").unwrap();
        let meta = cave
            .update_template(
                "daily-template",
                "renamed-template",
                "# Body\n",
                Some(vec!["journal".to_string()]),
                Some("Star".to_string()),
            )
            .unwrap();
        let template = cave.read_template("renamed-template").unwrap();
        let raw = cave.read_template_raw("renamed-template").unwrap();

        assert_eq!(meta.slug, "renamed-template");
        assert_eq!(template.content, "# Body\n");
        assert_eq!(template.meta.icon.as_deref(), Some("Star"));
        assert!(raw.contains("journal"));
        assert!(raw.contains("icon: Star"));
        assert!(!dir
            .path()
            .join(".granit/templates/daily-template.md")
            .exists());
        assert!(dir
            .path()
            .join(".granit/templates/renamed-template.md")
            .exists());
    }

    #[test]
    fn test_open_daily_note_creates_folder_and_note() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let note = cave.open_daily_note("Daily", None).unwrap();

        assert_eq!(note.meta.slug, today);
        assert_eq!(note.meta.relative_path, format!("Daily/{today}.md"));
        assert!(dir.path().join("Daily").is_dir());
        assert!(dir.path().join(format!("Daily/{today}.md")).exists());
        assert_eq!(note.meta.icon.as_deref(), Some("Calendar"));
        assert!(note.content.is_empty(), "daily note body should stay empty");
    }

    #[test]
    fn test_open_daily_note_uses_template_body_with_fresh_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_config().unwrap();
        std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
        std::fs::write(
            dir.path().join(".granit/templates/daily-template.md"),
            "---\ntags: [template]\nicon: Star\n---\n# {{ date }}\nHello {{ weekday_short }}\n",
        )
        .unwrap();
        cave.templates = Cave::scan_templates(&dir.path().join(".granit/templates")).unwrap();

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let note = cave
            .open_daily_note("Daily", Some("daily-template"))
            .unwrap();
        let raw = cave.read_note_raw(&today).unwrap();

        assert!(note.content.contains(&format!("# {today}")));
        assert!(note.content.contains("Hello "));
        assert_eq!(
            crate::markdown::read_frontmatter_tags(&raw),
            vec!["template".to_string()]
        );
        assert!(raw.contains("icon: Calendar"));
        assert!(!raw.contains("icon: Star"));
    }

    #[test]
    fn test_set_note_icon_preserves_existing_body() {
        let dir = tempfile::tempdir().unwrap();
        let note_path = dir.path().join("note.md");
        std::fs::write(&note_path, "---\ntags: [test]\n---\nBody text\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave
            .set_note_icon("note", Some("Star".to_string()))
            .unwrap();
        let note = cave.read_note("note").unwrap();

        assert_eq!(meta.icon.as_deref(), Some("Star"));
        assert_eq!(note.meta.icon.as_deref(), Some("Star"));
        assert_eq!(note.content, "Body text\n");
    }

    #[test]
    fn test_open_daily_note_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let note1 = cave.open_daily_note("Daily", None).unwrap();
        let note2 = cave.open_daily_note("Daily", None).unwrap();

        // Second call must not create a duplicate or modify the slug
        assert_eq!(note1.meta.slug, note2.meta.slug);
        let daily_notes: Vec<_> = std::fs::read_dir(dir.path().join("Daily"))
            .unwrap()
            .collect();
        assert_eq!(daily_notes.len(), 1, "should only have one daily note file");
    }

    #[test]
    fn test_open_daily_note_existing_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Journal")).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let note = cave.open_daily_note("Journal", None).unwrap();
        assert_eq!(note.meta.slug, today);
        assert!(dir.path().join(format!("Journal/{today}.md")).exists());
    }

    #[test]
    fn test_open_daily_note_falls_back_when_template_missing_or_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let note_missing = cave
            .open_daily_note("Daily", Some("missing-template"))
            .unwrap();
        assert!(note_missing.content.is_empty());

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        cave.delete_note(&today).unwrap();

        std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
        std::fs::write(
            dir.path().join(".granit/templates/bad-template.md"),
            "{{ if broken }}",
        )
        .unwrap();
        cave.templates = Cave::scan_templates(&dir.path().join(".granit/templates")).unwrap();

        let note_invalid = cave.open_daily_note("Daily", Some("bad-template")).unwrap();
        assert!(note_invalid.content.is_empty());
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

    #[test]
    fn test_search_content_finds_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("hello.md"),
            "---\ntitle: Hello\n---\nHello world, this is a test note.",
        )
        .unwrap();
        std::fs::write(dir.path().join("other.md"), "Nothing relevant here.").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let results = cave.search_content("test note", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].slug, "hello");
        assert!(results[0].snippets.iter().any(|s| s.contains("test note")));
    }

    #[test]
    fn test_search_content_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "Rust is Great").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let results = cave.search_content("rust is great", None).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_content_no_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "Hello world").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let results = cave.search_content("nonexistent", None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_content_respects_max_results() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..5 {
            std::fs::write(
                dir.path().join(format!("note-{i}.md")),
                "common keyword here",
            )
            .unwrap();
        }
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let results = cave.search_content("common keyword", Some(2)).unwrap();
        assert_eq!(results.len(), 2);
    }

    // ── list_todos ──────────────────────────────────────────────────

    #[test]
    fn test_list_todos_finds_checkboxes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("tasks.md"),
            "---\nmodified_at: 2026-01-01T00:00:00Z\n---\n- [ ] Buy milk\n- [x] Call dentist\n- [ ] Write tests\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let todos = cave.list_todos().unwrap();
        assert_eq!(todos.incomplete.len() + todos.completed.len(), 3);

        // Incomplete todos come first, sorted by line
        assert_eq!(todos.incomplete.len(), 2);
        assert_eq!(todos.completed.len(), 1);
        assert_eq!(todos.incomplete[0].text, "Buy milk");
        assert_eq!(todos.incomplete[1].text, "Write tests");
        assert_eq!(todos.completed[0].text, "Call dentist");
    }

    #[test]
    fn test_list_todos_empty_cave() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "Just a paragraph\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let todos = cave.list_todos().unwrap();
        assert!(todos.incomplete.is_empty() && todos.completed.is_empty());
    }

    #[test]
    fn test_list_todos_multiple_markers() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("mixed.md"),
            "- [ ] Dash unchecked\n* [ ] Star unchecked\n+ [x] Plus checked\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let todos = cave.list_todos().unwrap();
        assert_eq!(todos.incomplete.len() + todos.completed.len(), 3);
        assert_eq!(todos.incomplete.len(), 2);
        assert_eq!(todos.completed.len(), 1);
    }

    #[test]
    fn test_list_todos_uppercase_x() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("caps.md"), "- [X] Uppercase X completed\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let todos = cave.list_todos().unwrap();
        assert_eq!(todos.completed.len(), 1);
        assert!(todos.incomplete.is_empty());
        assert_eq!(todos.completed[0].text, "Uppercase X completed");
    }

    #[test]
    fn test_list_todos_line_numbers() {
        let dir = tempfile::tempdir().unwrap();
        // 3 lines: header + blank + checkbox — checkbox is on line 3
        std::fs::write(
            dir.path().join("note.md"),
            "# Header\n\n- [ ] Item on line 3\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let todos = cave.list_todos().unwrap();
        assert_eq!(todos.incomplete.len(), 1);
        assert_eq!(todos.incomplete[0].line, 3);
    }

    // ── toggle_todo ─────────────────────────────────────────────────

    #[test]
    fn test_toggle_todo_unchecked_to_checked() {
        let dir = tempfile::tempdir().unwrap();
        let note_path = dir.path().join("todo.md");
        std::fs::write(
            &note_path,
            "---\nmodified_at: 2026-01-01T00:00:00Z\n---\n- [ ] Do the thing\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        // Line 4 = "- [ ] Do the thing" (1-based: fm=3 lines + 1 body line)
        cave.toggle_todo("todo", 4).unwrap();

        let content = std::fs::read_to_string(&note_path).unwrap();
        assert!(
            content.contains("- [x] Do the thing"),
            "Expected [x] but got: {content}"
        );
    }

    #[test]
    fn test_toggle_todo_checked_to_unchecked() {
        let dir = tempfile::tempdir().unwrap();
        let note_path = dir.path().join("todo.md");
        std::fs::write(
            &note_path,
            "---\nmodified_at: 2026-01-01T00:00:00Z\n---\n- [x] Already done\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.toggle_todo("todo", 4).unwrap();

        let content = std::fs::read_to_string(&note_path).unwrap();
        assert!(
            content.contains("- [ ] Already done"),
            "Expected [ ] but got: {content}"
        );
    }

    #[test]
    fn test_toggle_todo_uppercase_x() {
        let dir = tempfile::tempdir().unwrap();
        let note_path = dir.path().join("todo.md");
        std::fs::write(&note_path, "- [X] Done with capital X\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.toggle_todo("todo", 1).unwrap();

        let content = std::fs::read_to_string(&note_path).unwrap();
        assert!(content.contains("- [ ] Done with capital X"));
    }

    #[test]
    fn test_toggle_todo_invalid_line_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "Just text, no checkbox\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let result = cave.toggle_todo("note", 1);
        assert!(matches!(result, Err(CaveError::InvalidTodoLine(1))));
    }

    #[test]
    fn test_toggle_todo_out_of_bounds_line() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "One line\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let result = cave.toggle_todo("note", 999);
        assert!(matches!(result, Err(CaveError::InvalidTodoLine(999))));
    }

    // ── toggle_todo_by_index ────────────────────────────────────────

    #[test]
    fn test_toggle_todo_by_index_first() {
        let dir = tempfile::tempdir().unwrap();
        let note_path = dir.path().join("tasks.md");
        std::fs::write(&note_path, "- [ ] First\n- [ ] Second\n- [x] Third\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.toggle_todo_by_index("tasks", 0).unwrap();

        let content = std::fs::read_to_string(&note_path).unwrap();
        assert!(content.contains("- [x] First"));
        assert!(content.contains("- [ ] Second"));
        assert!(content.contains("- [x] Third"));
    }

    #[test]
    fn test_toggle_todo_by_index_middle() {
        let dir = tempfile::tempdir().unwrap();
        let note_path = dir.path().join("tasks.md");
        std::fs::write(&note_path, "- [ ] A\n- [ ] B\n- [ ] C\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.toggle_todo_by_index("tasks", 1).unwrap(); // index 1 = "B"

        let content = std::fs::read_to_string(&note_path).unwrap();
        assert!(content.contains("- [ ] A"));
        assert!(content.contains("- [x] B"));
        assert!(content.contains("- [ ] C"));
    }

    #[test]
    fn test_toggle_todo_by_index_out_of_range() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("tasks.md"), "- [ ] Only one\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let result = cave.toggle_todo_by_index("tasks", 5);
        assert!(result.is_err());
    }
}
