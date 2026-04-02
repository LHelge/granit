use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_cave, SharedCave, ToolError};

#[derive(Deserialize)]
pub struct UpdateNoteArgs {
    /// The slug of the note to update.
    slug: String,
    /// The new markdown content for the note.
    content: String,
}

#[derive(Serialize)]
pub struct UpdateNoteOutput {
    slug: String,
    relative_path: String,
}

pub struct UpdateNoteTool {
    pub cave: SharedCave,
}

impl Tool for UpdateNoteTool {
    const NAME: &'static str = "update_note";
    type Error = ToolError;
    type Args = UpdateNoteArgs;
    type Output = UpdateNoteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "update_note".to_string(),
            description: "Update the content of an existing note by slug. Overwrites the note body. Frontmatter is managed automatically — do not include it.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "The slug (filename without .md) of the note to update"
                    },
                    "content": {
                        "type": "string",
                        "description": "The new markdown body (no frontmatter)"
                    }
                },
                "required": ["slug", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Use save_note which preserves frontmatter and replaces the body.
        with_cave(&self.cave, |cave| {
            let slug = cave.resolve_slug(&args.slug)?;
            let meta = cave.save_note(&slug, &args.content)?;
            Ok(UpdateNoteOutput {
                slug: meta.slug,
                relative_path: meta.relative_path,
            })
        })
    }
}
