use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_cave, SharedCave, ToolError};

#[derive(Deserialize)]
pub struct EditNoteArgs {
    /// The slug of the note to edit.
    slug: String,
    /// The exact text to find in the note.
    old_text: String,
    /// The replacement text.
    new_text: String,
}

#[derive(Serialize)]
pub struct EditNoteOutput {
    slug: String,
    relative_path: String,
}

pub struct EditNoteTool {
    pub cave: SharedCave,
}

impl Tool for EditNoteTool {
    const NAME: &'static str = "edit_note";
    type Error = ToolError;
    type Args = EditNoteArgs;
    type Output = EditNoteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "edit_note".to_string(),
            description:
                "Replace text in a note by slug (find and replace). Fails if the text is not found."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "The slug (filename without .md) of the note to edit"
                    },
                    "old_text": {
                        "type": "string",
                        "description": "The exact text to find in the note"
                    },
                    "new_text": {
                        "type": "string",
                        "description": "The replacement text"
                    }
                },
                "required": ["slug", "old_text", "new_text"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_cave(&self.cave, |cave| {
            let slug = cave.resolve_slug(&args.slug)?;
            let meta = cave.edit_note(&slug, &args.old_text, &args.new_text)?;
            Ok(EditNoteOutput {
                slug: meta.slug,
                relative_path: meta.relative_path,
            })
        })
    }
}
