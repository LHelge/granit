use super::helpers::{
    ensure_md_extension, template_meta_from_path, template_meta_with_icon, validate_name,
};
use super::{Cave, CaveError};
use chrono::{Datelike, NaiveDate};
use granit_types::{Document, DocumentMeta};
use std::path::Path;

impl Cave {
    /// Create a new template in `.granit/templates`.
    pub fn create_template(&mut self, name: &str) -> Result<DocumentMeta, CaveError> {
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
        let initial_content = crate::markdown::Markdown::new_note();
        std::fs::write(&final_path, &initial_content)?;
        self.templates.insert(slug, final_path.clone());

        Ok(template_meta_from_path(&final_path))
    }

    /// List all templates in `.granit/templates`, sorted by slug.
    pub fn list_templates(&self) -> Result<Vec<DocumentMeta>, CaveError> {
        let mut templates: Vec<DocumentMeta> = self
            .templates
            .values()
            .map(|abs| template_meta_with_icon(abs))
            .collect();
        templates.sort_by_key(|t| t.slug.to_lowercase());
        Ok(templates)
    }

    /// Read a template by slug.
    pub fn read_template(&self, slug: &str) -> Result<Document, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .templates
            .get(slug)
            .ok_or_else(|| CaveError::TemplateNotFound(slug.to_string()))?;

        let raw = std::fs::read_to_string(abs_path)?;
        let md = crate::markdown::Markdown::new(&raw);
        let body = md.body().to_string();
        let mut meta = template_meta_from_path(abs_path);
        meta.icon = md.icon();
        Ok(Document {
            meta,
            content: body,
        })
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

    /// Save new content to an existing template (looked up by slug).
    pub fn save_template(&self, slug: &str, content: &str) -> Result<DocumentMeta, CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .templates
            .get(slug)
            .ok_or_else(|| CaveError::TemplateNotFound(slug.to_string()))?;

        let existing_raw = std::fs::read_to_string(abs_path)?;
        let updated = crate::markdown::Markdown::rebuild(&existing_raw, content, None, None, None);
        std::fs::write(abs_path, updated.as_str())?;
        let mut meta = template_meta_from_path(abs_path);
        meta.icon = crate::markdown::Markdown::new(&updated).icon();
        Ok(meta)
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

    /// Rename an existing template in-place within `.granit/templates`.
    pub fn rename_template(
        &mut self,
        old_slug: &str,
        new_name: &str,
    ) -> Result<DocumentMeta, CaveError> {
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

    /// Update a template's filename, content, and optionally tags and icon in one operation.
    pub fn update_template(
        &mut self,
        old_slug: &str,
        new_name: &str,
        content: &str,
        tags: Option<Vec<String>>,
        icon: Option<String>,
    ) -> Result<DocumentMeta, CaveError> {
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
        let updated = crate::markdown::Markdown::rebuild(&existing_raw, content, tags, icon, None);
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
        meta.icon = crate::markdown::Markdown::new(&updated).icon();
        Ok(meta)
    }

    // ── Template rendering helpers ─────────────────────────────────

    pub(crate) fn read_template_body(&self, slug: &str) -> Result<String, CaveError> {
        let raw = self.read_template_raw(slug)?;
        Ok(crate::markdown::Markdown::new(&raw).body().to_string())
    }

    pub(crate) fn parse_daily_note_slug(slug: &str) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(slug, "%Y-%m-%d").ok()
    }

    pub(crate) fn render_note_template(
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

    pub(crate) fn initial_body_for_new_note(
        &self,
        slug: &str,
        template_slug: Option<&str>,
    ) -> Result<String, CaveError> {
        if let Some(template_slug) = template_slug {
            return self.render_note_template(template_slug, slug);
        }

        Ok(String::new())
    }

    pub(crate) fn initial_icon_for_new_note(
        &self,
        template_slug: Option<&str>,
    ) -> Result<Option<String>, CaveError> {
        let Some(template_slug) = template_slug else {
            return Ok(None);
        };

        let raw = self.read_template_raw(template_slug)?;
        Ok(crate::markdown::Markdown::new(&raw).icon())
    }

    pub(crate) fn initial_tags_for_new_note(
        &self,
        template_slug: Option<&str>,
    ) -> Result<Vec<String>, CaveError> {
        let Some(template_slug) = template_slug else {
            return Ok(Vec::new());
        };

        let raw = self.read_template_raw(template_slug)?;
        Ok(crate::markdown::Markdown::new(&raw).tags())
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
    ) -> Result<Document, CaveError> {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.open_daily_note_for_date(&today, folder, template_slug)
    }

    /// Open or create a daily note for a specific date.
    ///
    /// `date` must be in `YYYY-MM-DD` format.
    /// `folder` is a relative path from the cave root (e.g. `"Daily"` or `"Notes/Daily"`).
    /// The folder is created if it does not yet exist.
    /// If the note already exists it is read and returned without modification.
    pub fn open_daily_note_for_date(
        &mut self,
        date: &str,
        folder: &str,
        template_slug: Option<&str>,
    ) -> Result<Document, CaveError> {
        let folder_path = Path::new(folder);

        // Ensure the daily folder exists.
        let abs_folder = self.path.join(folder_path);
        if !abs_folder.is_dir() {
            super::helpers::validate_folder_path(folder_path)?;
            std::fs::create_dir_all(&abs_folder)?;
        }

        // Create the note if it doesn't exist yet, rendering the configured template if possible.
        if !self.notes.contains_key(date) {
            let body = template_slug
                .and_then(|slug| self.render_note_template(slug, date).ok())
                .unwrap_or_default();
            let tags = template_slug
                .and_then(|slug| self.initial_tags_for_new_note(Some(slug)).ok())
                .unwrap_or_default();
            let final_path = abs_folder.join(format!("{date}.md"));
            let initial_content = crate::markdown::Markdown::new_note_with_body(
                &body,
                tags,
                Some("Calendar".to_string()),
            );
            std::fs::write(&final_path, initial_content)?;
            self.notes.insert(date.to_string(), final_path);
            self.rebuild_backlinks();
        }

        self.read_note(date)
    }
}

#[cfg(test)]
mod tests {
    use crate::cave::Cave;

    #[test]
    fn test_templates_are_listed_separately_from_notes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("real-note.md"), "# Note").unwrap();
        std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
        std::fs::write(
            dir.path().join(".granit/templates/sample.md"),
            "template body",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let notes = cave.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].slug, "real-note");

        let templates = cave.list_templates().unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].slug, "sample");
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
    }

    #[test]
    fn test_open_daily_note_uses_template_body_with_fresh_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_config().unwrap();

        std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
        std::fs::write(
            dir.path().join(".granit/templates/daily-template.md"),
            "---\ntags: [daily]\nicon: Calendar\n---\n# {{ date }}\nNext: {{ tomorrow }}\nYesterday: {{ yesterday }}\n{{ weekday }} / {{ weekday_short }}\n",
        )
        .unwrap();
        cave.templates = Cave::scan_templates(&dir.path().join(".granit/templates")).unwrap();
        cave.save_config(&granit_types::AppConfig {
            daily_note_template_slug: Some("daily-template".to_string()),
            ..granit_types::AppConfig::default()
        })
        .unwrap();

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let note = cave
            .open_daily_note("Daily", Some("daily-template"))
            .unwrap();
        let raw = cave.read_note_raw(&today).unwrap();

        assert!(
            raw.contains("created_at:"),
            "should have fresh frontmatter, got: {raw}"
        );
        assert!(
            raw.contains("icon: Calendar"),
            "should carry template icon, got: {raw}"
        );
        assert!(
            note.content.contains("# "),
            "should have rendered heading: {}",
            note.content
        );
        assert!(
            note.content.contains("Next:"),
            "should render tomorrow: {}",
            note.content
        );
    }

    #[test]
    fn test_open_daily_note_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let note1 = cave.open_daily_note("Daily", None).unwrap();
        let note2 = cave.open_daily_note("Daily", None).unwrap();
        assert_eq!(note1.meta.slug, note2.meta.slug);
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
}
