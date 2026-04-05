use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_cave, SharedCave, ToolError};

// ── list_notes ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListNotesArgs {}

#[derive(Serialize)]
pub struct ListNotesOutput {
    notes: Vec<NoteEntry>,
}

#[derive(Serialize)]
struct NoteEntry {
    slug: String,
    relative_path: String,
}

pub struct ListNotesTool {
    pub cave: SharedCave,
}

impl Tool for ListNotesTool {
    const NAME: &'static str = "list_notes";
    type Error = ToolError;
    type Args = ListNotesArgs;
    type Output = ListNotesOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "list_notes".to_string(),
            description: "List all notes in the cave with their slugs and paths".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_cave(&self.cave, |cave| {
            let notes = cave.list_notes()?;
            Ok(ListNotesOutput {
                notes: notes
                    .into_iter()
                    .map(|n| NoteEntry {
                        slug: n.slug,
                        relative_path: n.relative_path,
                    })
                    .collect(),
            })
        })
    }
}

// ── list_folders ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListFoldersArgs {}

#[derive(Serialize)]
pub struct ListFoldersOutput {
    folders: Vec<String>,
}

pub struct ListFoldersTool {
    pub cave: SharedCave,
}

impl Tool for ListFoldersTool {
    const NAME: &'static str = "list_folders";
    type Error = ToolError;
    type Args = ListFoldersArgs;
    type Output = ListFoldersOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "list_folders".to_string(),
            description: "List all folders in the cave (relative paths from cave root)".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_cave(&self.cave, |cave| {
            let folders = cave.list_folders()?;
            Ok(ListFoldersOutput { folders })
        })
    }
}

// ── search_notes ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SearchNotesArgs {
    /// Search query to match against note slugs (case-insensitive substring match).
    query: String,
}

#[derive(Serialize)]
pub struct SearchNotesOutput {
    matches: Vec<SearchMatch>,
}

#[derive(Serialize)]
struct SearchMatch {
    slug: String,
    relative_path: String,
}

pub struct SearchNotesTool {
    pub cave: SharedCave,
}

impl Tool for SearchNotesTool {
    const NAME: &'static str = "search_notes";
    type Error = ToolError;
    type Args = SearchNotesArgs;
    type Output = SearchNotesOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "search_notes".to_string(),
            description: "Search for notes by slug/filename (case-insensitive substring match)"
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query to match against note filenames"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_cave(&self.cave, |cave| {
            let all_notes = cave.list_notes()?;
            let query_lower = args.query.to_lowercase();
            let matches = all_notes
                .into_iter()
                .filter(|n| n.slug.to_lowercase().contains(&query_lower))
                .map(|n| SearchMatch {
                    slug: n.slug,
                    relative_path: n.relative_path,
                })
                .collect();
            Ok(SearchNotesOutput { matches })
        })
    }
}

// ── search_content ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SearchContentArgs {
    /// Search query to match against note body text (case-insensitive substring match).
    query: String,
}

#[derive(Serialize)]
pub struct SearchContentOutput {
    matches: Vec<ContentHit>,
}

#[derive(Serialize)]
struct ContentHit {
    slug: String,
    snippets: Vec<String>,
}

pub struct SearchContentTool {
    pub cave: SharedCave,
}

impl Tool for SearchContentTool {
    const NAME: &'static str = "search_content";
    type Error = ToolError;
    type Args = SearchContentArgs;
    type Output = SearchContentOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "search_content".to_string(),
            description:
                "Search for text inside note bodies (case-insensitive full-text search). Returns matching notes with a context snippet."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Text to search for inside note bodies"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_cave(&self.cave, |cave| {
            let hits = cave.search_content(&args.query, Some(20))?;
            let matches = hits
                .into_iter()
                .map(|h| ContentHit {
                    slug: h.slug,
                    snippets: h.snippets,
                })
                .collect();
            Ok(SearchContentOutput { matches })
        })
    }
}
