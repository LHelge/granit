use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Parsed YAML frontmatter from a markdown note.
///
/// `title` is intentionally absent — the note title is derived from the
/// filename (slug), not from frontmatter.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub modified_at: Option<DateTime<Utc>>,
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
}

/// Metadata for a note in the cave.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteMeta {
    /// Filename without extension (e.g., "my-note").
    pub slug: String,
    /// Relative path from cave root (e.g., "subfolder/my-note.md").
    pub relative_path: String,
}

/// Full note content returned when reading a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub meta: NoteMeta,
    pub content: String,
}

/// Agent provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub provider: String,
    pub model: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
        }
    }
}

/// Application configuration as returned over IPC.
///
/// Paths are represented as strings for cross-platform serialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub recent_caves: Vec<String>,
    pub agent: AgentConfig,
    /// The currently open cave path, if any. Runtime-only — not persisted.
    pub active_cave: Option<String>,
}
