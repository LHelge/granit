use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_cave, SharedCave, ToolError};

#[derive(Deserialize)]
pub struct EditActiveNoteArgs {
    /// The exact text to find in the note.
    old_text: String,
    /// The replacement text.
    new_text: String,
}

#[derive(Serialize)]
pub struct EditActiveNoteOutput {
    slug: String,
    relative_path: String,
}

pub struct EditActiveNoteTool {
    pub cave: SharedCave,
}

impl Tool for EditActiveNoteTool {
    const NAME: &'static str = "edit_active_note";
    type Error = ToolError;
    type Args = EditActiveNoteArgs;
    type Output = EditActiveNoteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "edit_active_note".to_string(),
            description:
                "Replace text in the note currently open in the editor (find and replace on the body only, excluding frontmatter). Returns an error if no note is open or the text is not found."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "old_text": {
                        "type": "string",
                        "description": "The exact text to find in the note"
                    },
                    "new_text": {
                        "type": "string",
                        "description": "The replacement text"
                    }
                },
                "required": ["old_text", "new_text"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_cave(&self.cave, |cave| {
            let slug = cave.active_slug().ok_or(crate::cave::CaveError::NotFound(
                "no note is currently open in the editor".to_string(),
            ))?;
            let meta = cave.edit_note(slug, &args.old_text, &args.new_text)?;
            Ok(EditActiveNoteOutput {
                slug: meta.slug,
                relative_path: meta.relative_path,
            })
        })
    }
}
