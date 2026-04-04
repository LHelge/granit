use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_cave, SharedCave, ToolError};

#[derive(Deserialize)]
pub struct EditNoteArgs {
    /// The slug of the note to edit. If omitted, edits the note currently open in the editor.
    slug: Option<String>,
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
                "Replace text in a note (find and replace on the body only, excluding frontmatter). Pass a slug to target a specific note, or omit it to edit the note currently open in the editor. Fails if the text is not found."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "The slug (filename without .md) of the note to edit. Omit to edit the active note."
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
                "required": ["old_text", "new_text"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_cave(&self.cave, |cave| {
            let slug = match &args.slug {
                Some(s) => cave.resolve_slug(s)?.to_string(),
                None => cave
                    .active_slug()
                    .ok_or_else(|| {
                        crate::cave::CaveError::NotFound(
                            "no note is currently open in the editor".to_string(),
                        )
                    })?
                    .to_string(),
            };
            let meta = cave.edit_note(&slug, &args.old_text, &args.new_text)?;
            Ok(EditNoteOutput {
                slug: meta.slug,
                relative_path: meta.relative_path,
            })
        })
    }
}
