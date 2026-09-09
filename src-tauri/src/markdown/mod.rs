mod builder;
mod frontmatter;
mod render;

pub(crate) use frontmatter::split_frontmatter;

use std::cell::OnceCell;

use granit_types::Frontmatter;

/// A short-lived handle for parsing and rendering a markdown document.
///
/// Wraps a borrowed `&str` of raw markdown and lazily parses YAML frontmatter
/// on first access. Multiple accessor calls (`.body()`, `.icon()`, `.tags()`)
/// share the same cached parse result.
///
/// Builder-style associated functions that produce *new* markdown content
/// (rather than operating on an existing document) are also available:
///
/// ```ignore
/// let content = Markdown::new_note();
/// let content = Markdown::rebuild(existing, new_body, tags, icon, favorite);
/// ```
pub struct Markdown<'a> {
    raw: &'a str,
    parsed: OnceCell<(Option<Frontmatter>, &'a str)>,
    /// Cave-relative directory (forward slashes, `""` for the cave root) that
    /// relative image sources resolve against. `None` leaves image sources
    /// untouched (content that is not a file in the cave, e.g. agent chat).
    image_base: Option<String>,
}

impl<'a> Markdown<'a> {
    pub fn new(raw: &'a str) -> Self {
        Self {
            raw,
            parsed: OnceCell::new(),
            image_base: None,
        }
    }

    /// Resolve relative image sources against `dir`, the cave-relative
    /// directory of the file this markdown came from, and rewrite them to
    /// the `granit://` cave route so the webview can load them.
    pub fn with_image_base(mut self, dir: impl Into<String>) -> Self {
        self.image_base = Some(dir.into());
        self
    }

    /// Lazily parse frontmatter and return `(Option<Frontmatter>, body)`.
    fn parsed(&self) -> &(Option<Frontmatter>, &'a str) {
        self.parsed
            .get_or_init(|| frontmatter::extract_frontmatter(self.raw))
    }
}
