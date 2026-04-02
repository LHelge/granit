use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_cave, SharedCave, ToolError};

#[derive(Deserialize)]
pub struct ReadActiveNoteArgs {}

#[derive(Serialize)]
pub struct ReadActiveNoteOutput {
    slug: String,
    relative_path: String,
    content: String,
}

pub struct ReadActiveNoteTool {
    pub cave: SharedCave,
}

impl Tool for ReadActiveNoteTool {
    const NAME: &'static str = "read_active_note";
    type Error = ToolError;
    type Args = ReadActiveNoteArgs;
    type Output = ReadActiveNoteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "read_active_note".to_string(),
            description:
                "Read the body of the note currently open in the editor (markdown without frontmatter). Returns an error if no note is open."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_cave(&self.cave, |cave| {
            let slug = cave.active_slug().ok_or(crate::cave::CaveError::NotFound(
                "no note is currently open in the editor".to_string(),
            ))?;
            let note = cave.read_note(slug)?;
            Ok(ReadActiveNoteOutput {
                slug: note.meta.slug,
                relative_path: note.meta.relative_path,
                content: note.content,
            })
        })
    }
}
