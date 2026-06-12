use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Parsed YAML frontmatter from a markdown note.
///
/// `title` is intentionally absent — the note title is derived from the
/// filename (slug), not from frontmatter.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorite: Option<bool>,
}

/// Result of rendering a markdown document: rendered HTML plus extracted metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedDocument {
    /// Title derived from the document's filename (slug), for display as a page header.
    pub title: String,
    /// HTML rendered from the markdown body (frontmatter stripped).
    pub html: String,
    /// Parsed frontmatter, if present.
    pub frontmatter: Option<Frontmatter>,
    /// Slugs of outgoing wiki-links (`[[note-name]]`) found in the document.
    pub outgoing_links: Vec<String>,
    /// Documents that link to this one via resolved wiki-links.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backlinks: Vec<DocumentMeta>,
    /// Formatted created_at in local time (e.g. "2026-03-27 14:05:00"), if present.
    pub created_display: Option<String>,
    /// Formatted modified_at in local time (e.g. "2026-03-27 14:05:00"), if present.
    pub modified_display: Option<String>,
    /// Number of words in the plain-text body (markdown syntax and frontmatter stripped).
    #[serde(default)]
    pub word_count: usize,
    /// Estimated reading time in whole minutes (~200 wpm, rounded up; 0 for an empty body).
    #[serde(default)]
    pub reading_minutes: usize,
}

/// Metadata for a document (note or template) in the cave.
///
/// Intentionally kept separate from [`Document`] (full content). `list_notes`
/// returns many `DocumentMeta` at once for the sidebar/tree — sending full body
/// content for every document would be expensive in both I/O and IPC payload
/// size, and grows linearly with cave size. Frontmatter is parsed (just the
/// small header block) to populate fields like `icon`; the body is discarded.
///
/// Use [`Document`] when the editor needs the body, or [`RenderedDocument`]
/// when the renderer needs HTML + full frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMeta {
    /// Filename without extension (e.g., "my-note").
    pub slug: String,
    /// Relative path from cave root (e.g., "subfolder/my-note.md").
    pub relative_path: String,
    /// Optional icon ID in PascalCase without vendor prefix (e.g., "Pencil"), from frontmatter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Optional favorite flag from frontmatter. Always `None` for templates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorite: Option<bool>,
}

/// Full document content (note or template) returned when reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub meta: DocumentMeta,
    pub content: String,
}

/// A match from a full-text content search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMatch {
    pub slug: String,
    pub snippets: Vec<String>,
}

/// A todo item extracted from a markdown checkbox (`- [ ]` / `- [x]`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// Slug of the note this todo belongs to.
    pub slug: String,
    /// Relative path from cave root (e.g. "folder/note.md").
    pub relative_path: String,
    /// 1-based line number in the raw file (including frontmatter).
    pub line: usize,
    /// The todo text, without the checkbox marker.
    pub text: String,
}

/// All todo items in a cave, pre-split into incomplete and completed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TodoList {
    /// Todos where the checkbox is unchecked, sorted by slug then line.
    pub incomplete: Vec<TodoItem>,
    /// Todos where the checkbox is checked, sorted by slug then line.
    pub completed: Vec<TodoItem>,
}

/// All unique tags in a cave, mapped to the notes that carry each tag.
///
/// Tags are sorted alphabetically (via `BTreeMap`), and notes within each
/// tag are sorted by slug.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TagMap {
    pub tags: BTreeMap<String, Vec<DocumentMeta>>,
}
