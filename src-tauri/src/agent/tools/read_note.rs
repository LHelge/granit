use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_cave, SharedCave, ToolError};

#[derive(Deserialize)]
pub struct ReadNoteArgs {
    /// The slug (filename without .md extension) of the note to read.
    slug: String,
}

#[derive(Serialize)]
pub struct ReadNoteOutput {
    slug: String,
    relative_path: String,
    content: String,
}

pub struct ReadNoteTool {
    pub cave: SharedCave,
}

impl Tool for ReadNoteTool {
    const NAME: &'static str = "read_note";
    type Error = ToolError;
    type Args = ReadNoteArgs;
    type Output = ReadNoteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "read_note".to_string(),
            description: "Read the content of a note by its slug (filename without .md extension)"
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "The slug (filename without .md) of the note to read"
                    }
                },
                "required": ["slug"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_cave(&self.cave, |cave| {
            let slug = cave.resolve_slug(&args.slug)?;
            let note = cave.read_note(&slug)?;
            Ok(ReadNoteOutput {
                slug: note.meta.slug,
                relative_path: note.meta.relative_path,
                content: note.content,
            })
        })
    }
}
