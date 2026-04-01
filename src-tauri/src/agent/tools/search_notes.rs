use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_cave, SharedCave, ToolError};

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
