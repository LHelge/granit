mod builder;
mod frontmatter;
mod render;

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
}

impl<'a> Markdown<'a> {
    pub fn new(raw: &'a str) -> Self {
        Self {
            raw,
            parsed: OnceCell::new(),
        }
    }

    /// Lazily parse frontmatter and return `(Option<Frontmatter>, body)`.
    fn parsed(&self) -> &(Option<Frontmatter>, &'a str) {
        self.parsed
            .get_or_init(|| frontmatter::extract_frontmatter(self.raw))
    }
}
