//! Presentation templates: CSS files in `.granit/presentations/`.
//!
//! A note is presented with the template named by its `presentation`
//! frontmatter field, which is the file stem of a `.css` file in this flat
//! directory. Assets (logos, fonts) sit beside the CSS files and are shared
//! between templates by relative path; only `.css` files are indexed.
//!
//! Two defaults, `default-dark` and `default-light`, are seeded when the
//! directory does not exist yet. They are ordinary files: renaming or
//! deleting one is allowed, and a note that still references a missing
//! template simply fails to present until it is pointed elsewhere.

use super::helpers::{template_meta_from_path, validate_name, write_atomic, write_new};
use super::{Cave, CaveError};
use granit_types::{Document, DocumentMeta};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Directory under the cave root holding presentation templates.
pub const PRESENTATIONS_DIR: &str = ".granit/presentations";

/// Built-in dark template, also the seed for new templates.
pub const DEFAULT_DARK_CSS: &str = include_str!("presentations/default-dark.css");
/// Built-in light template.
pub const DEFAULT_LIGHT_CSS: &str = include_str!("presentations/default-light.css");

/// The templates seeded into a cave that has no presentations directory.
const DEFAULT_TEMPLATES: [(&str, &str); 2] = [
    ("default-dark", DEFAULT_DARK_CSS),
    ("default-light", DEFAULT_LIGHT_CSS),
];

/// Strip any trailing `.css` extension(s) from a user-supplied name, so the
/// slug is always the file stem (as for note and template names).
fn normalize_presentation_name(name: &str) -> &str {
    let mut name = name;
    while let Some(stripped) = name.strip_suffix(".css") {
        name = stripped;
    }
    name
}

fn css_filename(slug: &str) -> String {
    format!("{slug}.css")
}

impl Cave {
    /// Directory holding presentation templates and their assets.
    pub fn presentations_dir(&self) -> PathBuf {
        self.path.join(PRESENTATIONS_DIR)
    }

    /// Scan the flat presentations directory for `.css` files.
    pub(crate) fn scan_presentations(dir: &Path) -> Result<HashMap<String, PathBuf>, CaveError> {
        Self::scan_flat_dir(dir, "css")
    }

    /// Seed the default templates when the presentations directory does not
    /// exist. An existing directory is never touched, so deleted or edited
    /// defaults stay that way. Returns whether anything was written.
    pub fn seed_presentation_defaults(&mut self) -> Result<bool, CaveError> {
        let dir = self.presentations_dir();
        if dir.exists() {
            return Ok(false);
        }
        std::fs::create_dir_all(&dir)?;
        for (slug, css) in DEFAULT_TEMPLATES {
            let path = dir.join(css_filename(slug));
            write_new(&path, css)?;
            self.presentations.insert(slug.to_string(), path);
        }
        Ok(true)
    }

    /// List all presentation templates, sorted by slug.
    pub fn list_presentations(&self) -> Result<Vec<DocumentMeta>, CaveError> {
        let mut presentations: Vec<DocumentMeta> = self
            .presentations
            .values()
            .map(|abs| template_meta_from_path(abs))
            .collect();
        presentations.sort_by_key(|p| p.slug.to_lowercase());
        Ok(presentations)
    }

    /// Absolute path of the template `slug`, or `PresentationNotFound`.
    pub(crate) fn presentation_path(&self, slug: &str) -> Result<&Path, CaveError> {
        validate_name(slug)?;
        self.presentations
            .get(slug)
            .map(PathBuf::as_path)
            .ok_or_else(|| CaveError::PresentationNotFound(slug.to_string()))
    }

    /// Read a template by slug. The raw CSS is the document content.
    pub fn read_presentation(&self, slug: &str) -> Result<Document, CaveError> {
        let abs_path = self.presentation_path(slug)?;
        let content = std::fs::read_to_string(abs_path)?;
        Ok(Document {
            meta: template_meta_from_path(abs_path),
            content,
        })
    }

    /// Create a new template seeded from the built-in dark default.
    ///
    /// `"untitled"` auto-numbers on collision like notes and note templates;
    /// any other existing name is `PresentationAlreadyExists`.
    pub fn create_presentation(&mut self, name: &str) -> Result<DocumentMeta, CaveError> {
        let name = normalize_presentation_name(name);
        validate_name(name)?;

        let dir = self.presentations_dir();
        std::fs::create_dir_all(&dir)?;

        let slug = if name == "untitled" && self.presentations.contains_key("untitled") {
            let mut n = 2u32;
            loop {
                let candidate = format!("untitled-{n}");
                if !self.presentations.contains_key(&candidate) {
                    break candidate;
                }
                n = n
                    .checked_add(1)
                    .ok_or_else(|| CaveError::SlugExhausted("untitled".into()))?;
            }
        } else if self.presentations.contains_key(name) {
            return Err(CaveError::PresentationAlreadyExists(css_filename(name)));
        } else {
            name.to_string()
        };

        let final_path = dir.join(css_filename(&slug));
        write_new(&final_path, DEFAULT_DARK_CSS)?;
        self.presentations.insert(slug, final_path.clone());
        Ok(template_meta_from_path(&final_path))
    }

    /// Overwrite a template's CSS atomically.
    pub fn save_presentation(&self, slug: &str, content: &str) -> Result<DocumentMeta, CaveError> {
        let abs_path = self.presentation_path(slug)?;
        write_atomic(abs_path, content)?;
        Ok(template_meta_from_path(abs_path))
    }

    /// Rename a template in place. Notes referencing the old name are left
    /// untouched and show the reference as broken.
    pub fn rename_presentation(
        &mut self,
        old_slug: &str,
        new_name: &str,
    ) -> Result<DocumentMeta, CaveError> {
        let new_name = normalize_presentation_name(new_name);
        validate_name(new_name)?;
        let old_abs = self.presentation_path(old_slug)?.to_path_buf();

        if old_slug == new_name {
            return Ok(template_meta_from_path(&old_abs));
        }
        if self.presentations.contains_key(new_name) {
            return Err(CaveError::PresentationAlreadyExists(css_filename(new_name)));
        }

        let new_abs = old_abs
            .parent()
            .unwrap_or(Path::new(""))
            .join(css_filename(new_name));
        std::fs::rename(&old_abs, &new_abs)?;
        self.presentations.remove(old_slug);
        self.presentations
            .insert(new_name.to_string(), new_abs.clone());
        Ok(template_meta_from_path(&new_abs))
    }

    /// Delete a template. Notes referencing it are left untouched.
    pub fn delete_presentation(&mut self, slug: &str) -> Result<(), CaveError> {
        let abs_path = self.presentation_path(slug)?.to_path_buf();
        std::fs::remove_file(&abs_path)?;
        self.presentations.remove(slug);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cave_with_presentations(files: &[(&str, &str)]) -> (tempfile::TempDir, Cave) {
        let dir = tempfile::tempdir().unwrap();
        let presentations = dir.path().join(PRESENTATIONS_DIR);
        std::fs::create_dir_all(&presentations).unwrap();
        for (name, content) in files {
            std::fs::write(presentations.join(name), content).unwrap();
        }
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        (dir, cave)
    }

    // ── Scanning ────────────────────────────────────────────────────

    #[test]
    fn test_scan_lists_css_files_and_ignores_assets() {
        let (_dir, cave) = cave_with_presentations(&[
            ("corporate.css", "body{}"),
            ("Zebra.css", "body{}"),
            ("logo.png", "png"),
            ("font.woff2", "font"),
            ("notes.md", "# not a template"),
        ]);

        let slugs: Vec<String> = cave
            .list_presentations()
            .unwrap()
            .into_iter()
            .map(|p| p.slug)
            .collect();
        assert_eq!(slugs, vec!["corporate".to_string(), "Zebra".to_string()]);

        let meta = &cave.list_presentations().unwrap()[0];
        assert_eq!(meta.relative_path, "corporate.css");
        assert!(meta.icon.is_none());
    }

    #[test]
    fn test_scan_missing_directory_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        assert!(cave.list_presentations().unwrap().is_empty());
        assert!(!dir.path().join(PRESENTATIONS_DIR).exists());
    }

    // ── Seeding ─────────────────────────────────────────────────────

    #[test]
    fn test_seed_writes_defaults_when_directory_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        assert!(cave.seed_presentation_defaults().unwrap());

        let presentations = dir.path().join(PRESENTATIONS_DIR);
        assert_eq!(
            std::fs::read_to_string(presentations.join("default-dark.css")).unwrap(),
            DEFAULT_DARK_CSS
        );
        assert_eq!(
            std::fs::read_to_string(presentations.join("default-light.css")).unwrap(),
            DEFAULT_LIGHT_CSS
        );
        // The index is updated without a rescan.
        let slugs: Vec<String> = cave
            .list_presentations()
            .unwrap()
            .into_iter()
            .map(|p| p.slug)
            .collect();
        assert_eq!(
            slugs,
            vec!["default-dark".to_string(), "default-light".to_string()]
        );
    }

    #[test]
    fn test_seed_never_touches_an_existing_directory() {
        let (dir, mut cave) = cave_with_presentations(&[("mine.css", "body{}")]);

        assert!(!cave.seed_presentation_defaults().unwrap());

        let presentations = dir.path().join(PRESENTATIONS_DIR);
        assert!(!presentations.join("default-dark.css").exists());
        assert!(!presentations.join("default-light.css").exists());
        assert_eq!(cave.list_presentations().unwrap().len(), 1);

        // Deleting a default and reopening does not resurrect it.
        cave.delete_presentation("mine").unwrap();
        assert!(!cave.seed_presentation_defaults().unwrap());
        assert!(cave.list_presentations().unwrap().is_empty());
    }

    #[test]
    fn test_default_templates_document_the_contract() {
        for css in [DEFAULT_DARK_CSS, DEFAULT_LIGHT_CSS] {
            assert!(css.contains("1280 x 720"));
            assert!(css.contains("data-number"));
            assert!(css.contains(".slide::after"));
        }
        assert!(DEFAULT_LIGHT_CSS.contains("default-light"));
        assert!(!DEFAULT_LIGHT_CSS.contains("default-dark"));
    }

    // ── CRUD ────────────────────────────────────────────────────────

    #[test]
    fn test_read_presentation_returns_raw_css() {
        let (_dir, cave) = cave_with_presentations(&[("corporate.css", "body { color: red }")]);
        let doc = cave.read_presentation("corporate").unwrap();
        assert_eq!(doc.meta.slug, "corporate");
        assert_eq!(doc.meta.relative_path, "corporate.css");
        assert_eq!(doc.content, "body { color: red }");

        assert!(matches!(
            cave.read_presentation("missing"),
            Err(CaveError::PresentationNotFound(_))
        ));
    }

    #[test]
    fn test_create_presentation_seeds_from_dark_default() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let meta = cave.create_presentation("corporate.css").unwrap();
        assert_eq!(meta.slug, "corporate");
        assert_eq!(meta.relative_path, "corporate.css");
        assert_eq!(
            cave.read_presentation("corporate").unwrap().content,
            DEFAULT_DARK_CSS
        );

        assert!(matches!(
            cave.create_presentation("corporate"),
            Err(CaveError::PresentationAlreadyExists(_))
        ));
    }

    #[test]
    fn test_create_presentation_untitled_auto_numbers() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        assert_eq!(
            cave.create_presentation("untitled").unwrap().slug,
            "untitled"
        );
        assert_eq!(
            cave.create_presentation("untitled").unwrap().slug,
            "untitled-2"
        );
        assert_eq!(
            cave.create_presentation("untitled").unwrap().slug,
            "untitled-3"
        );
    }

    #[test]
    fn test_save_presentation_overwrites_content() {
        let (dir, cave) = cave_with_presentations(&[("corporate.css", "old")]);
        let meta = cave.save_presentation("corporate", "new { }").unwrap();
        assert_eq!(meta.slug, "corporate");
        assert_eq!(
            std::fs::read_to_string(dir.path().join(PRESENTATIONS_DIR).join("corporate.css"))
                .unwrap(),
            "new { }"
        );
        assert!(matches!(
            cave.save_presentation("missing", "x"),
            Err(CaveError::PresentationNotFound(_))
        ));
    }

    #[test]
    fn test_rename_presentation_moves_file_and_index() {
        let (dir, mut cave) =
            cave_with_presentations(&[("corporate.css", "a"), ("other.css", "b")]);

        let meta = cave.rename_presentation("corporate", "brand.css").unwrap();
        assert_eq!(meta.slug, "brand");
        let presentations = dir.path().join(PRESENTATIONS_DIR);
        assert!(presentations.join("brand.css").exists());
        assert!(!presentations.join("corporate.css").exists());
        assert_eq!(cave.read_presentation("brand").unwrap().content, "a");

        // Same name is a no-op; an existing target is refused.
        assert_eq!(
            cave.rename_presentation("brand", "brand").unwrap().slug,
            "brand"
        );
        assert!(matches!(
            cave.rename_presentation("brand", "other"),
            Err(CaveError::PresentationAlreadyExists(_))
        ));
        assert!(matches!(
            cave.rename_presentation("missing", "x"),
            Err(CaveError::PresentationNotFound(_))
        ));
    }

    #[test]
    fn test_delete_presentation_removes_file() {
        let (dir, mut cave) = cave_with_presentations(&[("corporate.css", "a")]);
        cave.delete_presentation("corporate").unwrap();
        assert!(!dir
            .path()
            .join(PRESENTATIONS_DIR)
            .join("corporate.css")
            .exists());
        assert!(cave.list_presentations().unwrap().is_empty());
        assert!(matches!(
            cave.delete_presentation("corporate"),
            Err(CaveError::PresentationNotFound(_))
        ));
    }

    #[test]
    fn test_rename_and_delete_leave_referencing_notes_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let note = "---\npresentation: corporate\n---\n# Talk\n";
        std::fs::write(dir.path().join("talk.md"), note).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.create_presentation("corporate").unwrap();

        cave.rename_presentation("corporate", "brand").unwrap();
        assert_eq!(cave.read_note_raw("talk").unwrap(), note);
        cave.delete_presentation("brand").unwrap();
        assert_eq!(cave.read_note_raw("talk").unwrap(), note);
    }

    // ── Name validation ─────────────────────────────────────────────

    #[test]
    fn test_presentation_name_validation() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        for bad in ["", "../escape", "sub/dir", ".hidden", ".css"] {
            assert!(
                matches!(
                    cave.create_presentation(bad),
                    Err(CaveError::InvalidName(_))
                ),
                "{bad:?} should be rejected"
            );
            assert!(
                matches!(cave.read_presentation(bad), Err(CaveError::InvalidName(_))),
                "{bad:?} should be rejected on read"
            );
        }
        assert!(!dir
            .path()
            .join(PRESENTATIONS_DIR)
            .join("escape.css")
            .exists());
    }

    #[test]
    fn test_normalize_presentation_name() {
        assert_eq!(normalize_presentation_name("a.css"), "a");
        assert_eq!(normalize_presentation_name("a.css.css"), "a");
        assert_eq!(normalize_presentation_name("a"), "a");
        assert_eq!(normalize_presentation_name("a.md"), "a.md");
    }
}
