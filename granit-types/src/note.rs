use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
}

/// Result of rendering a markdown note: rendered HTML plus extracted metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedNote {
    /// Title derived from the note's filename (slug), for display as a page header.
    pub title: String,
    /// HTML rendered from the markdown body (frontmatter stripped).
    pub html: String,
    /// Parsed frontmatter, if present.
    pub frontmatter: Option<Frontmatter>,
    /// Slugs of outgoing wiki-links (`[[note-name]]`) found in the note.
    pub outgoing_links: Vec<String>,
    /// Formatted created_at in local time (e.g. "2026-03-27 14:05:00"), if present.
    pub created_display: Option<String>,
    /// Formatted modified_at in local time (e.g. "2026-03-27 14:05:00"), if present.
    pub modified_display: Option<String>,
}

/// Metadata for a note in the cave.
///
/// Intentionally kept separate from [`Note`] (full content). `list_notes`
/// returns many `NoteMeta` at once for the sidebar/tree — sending full body
/// content for every note would be expensive in both I/O and IPC payload size,
/// and grows linearly with cave size. Frontmatter is parsed (just the small
/// header block) to populate fields like `icon`; the body is discarded.
///
/// Use [`Note`] when the editor needs the body, or [`RenderedNote`] when the
/// renderer needs HTML + full frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteMeta {
    /// Filename without extension (e.g., "my-note").
    pub slug: String,
    /// Relative path from cave root (e.g., "subfolder/my-note.md").
    pub relative_path: String,
    /// Optional icon ID in PascalCase without vendor prefix (e.g., "Pencil"), from frontmatter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// Full note content returned when reading a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub meta: NoteMeta,
    pub content: String,
}
