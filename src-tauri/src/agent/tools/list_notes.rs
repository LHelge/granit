use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_cave, SharedCave, ToolError};

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
