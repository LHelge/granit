use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_cave_mut, SharedCave, ToolError};

#[derive(Deserialize)]
pub struct DeleteNoteArgs {
    /// The slug of the note to delete.
    slug: String,
}

#[derive(Serialize)]
pub struct DeleteNoteOutput {
    deleted: String,
}

pub struct DeleteNoteTool {
    pub cave: SharedCave,
}

impl Tool for DeleteNoteTool {
    const NAME: &'static str = "delete_note";
    type Error = ToolError;
    type Args = DeleteNoteArgs;
    type Output = DeleteNoteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "delete_note".to_string(),
            description: "Delete a note from the cave by its slug".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "The slug (filename without .md) of the note to delete"
                    }
                },
                "required": ["slug"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_cave_mut(&self.cave, |cave| {
            let slug = cave.resolve_slug(&args.slug)?;
            cave.delete_note(&slug)?;
            Ok(DeleteNoteOutput {
                deleted: slug,
            })
        })
    }
}
