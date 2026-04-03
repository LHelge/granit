use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_cave_mut, SharedCave, ToolError};

#[derive(Deserialize)]
pub struct UpdateNoteArgs {
    /// The slug of the note to update.
    slug: String,
    /// The new markdown content for the note.
    content: String,
    /// Optional icon ID to set (e.g. "Star"). Omit to preserve the existing icon.
    icon: Option<String>,
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
                    },
                    "icon": {
                        "type": "string",
                        "description": "Optional icon ID to set (e.g. \"Star\", \"Book\", \"Code\"). Omit to preserve the existing icon."
                    }
                },
                "required": ["slug", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_cave_mut(&self.cave, |cave| {
            let slug = cave.resolve_slug(&args.slug)?;
            // Pass slug as new_name (no rename), None for tags (preserve), icon as provided.
            let meta = cave.update_note(&slug, &slug, &args.content, None, args.icon)?;
            Ok(UpdateNoteOutput {
                slug: meta.slug,
                relative_path: meta.relative_path,
            })
        })
    }
}
