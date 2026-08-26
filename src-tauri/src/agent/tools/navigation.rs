use rig_core::tool::PortableTool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_shared_cave, SharedCave};
use crate::cave::CaveError;

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

impl PortableTool for ListNotesTool {
    const NAME: &'static str = "list_notes";
    type Error = CaveError;
    type Args = ListNotesArgs;
    type Output = ListNotesOutput;

    fn description(&self) -> String {
        "List all notes in the cave with their slugs and paths".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
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

impl PortableTool for ListFoldersTool {
    const NAME: &'static str = "list_folders";
    type Error = CaveError;
    type Args = ListFoldersArgs;
    type Output = ListFoldersOutput;

    fn description(&self) -> String {
        "List all folders in the cave (relative paths from cave root)".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
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

impl PortableTool for SearchNotesTool {
    const NAME: &'static str = "search_notes";
    type Error = CaveError;
    type Args = SearchNotesArgs;
    type Output = SearchNotesOutput;

    fn description(&self) -> String {
        "Search for notes by slug/filename (case-insensitive substring match)".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query to match against note filenames"
                }
            },
            "required": ["query"]
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
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

impl PortableTool for SearchContentTool {
    const NAME: &'static str = "search_content";
    type Error = CaveError;
    type Args = SearchContentArgs;
    type Output = SearchContentOutput;

    fn description(&self) -> String {
        "Search for text inside note bodies (case-insensitive full-text search). Returns matching notes with a context snippet."
                    .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Text to search for inside note bodies"
                }
            },
            "required": ["query"]
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
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

// ── list_tags ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListTagsArgs {}

#[derive(Debug, Serialize)]
pub struct ListTagsOutput {
    /// Tag name → slugs of the notes carrying it.
    tags: std::collections::BTreeMap<String, Vec<String>>,
}

pub struct ListTagsTool {
    pub cave: SharedCave,
}

impl PortableTool for ListTagsTool {
    const NAME: &'static str = "list_tags";
    type Error = CaveError;
    type Args = ListTagsArgs;
    type Output = ListTagsOutput;

    fn description(&self) -> String {
        "List all frontmatter tags used in the cave, with the slugs of the notes carrying each tag. Use this to find notes by tag or to reuse existing tags instead of inventing new ones."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            let map = cave.list_tags()?;
            Ok(ListTagsOutput {
                tags: map
                    .tags
                    .into_iter()
                    .map(|(tag, notes)| (tag, notes.into_iter().map(|n| n.slug).collect()))
                    .collect(),
            })
        })
    }
}

// ── list_templates ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListTemplatesArgs {}

#[derive(Debug, Serialize)]
pub struct ListTemplatesOutput {
    templates: Vec<String>,
}

pub struct ListTemplatesTool {
    pub cave: SharedCave,
}

impl PortableTool for ListTemplatesTool {
    const NAME: &'static str = "list_templates";
    type Error = CaveError;
    type Args = ListTemplatesArgs;
    type Output = ListTemplatesOutput;

    fn description(&self) -> String {
        "List the note templates available in the cave. Pass one of these slugs as the template argument of create_note to seed a new note from it."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            Ok(ListTemplatesOutput {
                templates: cave.list_templates()?.into_iter().map(|t| t.slug).collect(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;

    #[tokio::test]
    async fn list_tags_tool_maps_tags_to_note_slugs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.md"),
            "---\ntags: [projekt, viktigt]\n---\nA",
        )
        .unwrap();
        std::fs::write(dir.path().join("b.md"), "---\ntags: [projekt]\n---\nB").unwrap();
        let cave = crate::cave::Cave::open(dir.path().to_path_buf()).unwrap();
        let tool = ListTagsTool {
            cave: Arc::new(Mutex::new(Some(cave))),
        };

        let output = tool.call(ListTagsArgs {}).await.unwrap();
        let mut projekt = output.tags.get("projekt").cloned().unwrap();
        projekt.sort();
        assert_eq!(projekt, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(output.tags.get("viktigt").unwrap(), &vec!["a".to_string()]);
    }
}
