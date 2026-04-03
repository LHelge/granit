use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_cave_mut, SharedCave, ToolError};

#[derive(Deserialize)]
pub struct CreateNoteArgs {
    /// The name for the new note (without .md extension).
    name: String,
    /// Optional folder path (relative to cave root) to create the note in.
    folder: Option<String>,
    /// Optional icon ID (e.g. "Star", "Book"). Omit for the default file icon.
    icon: Option<String>,
}

#[derive(Serialize)]
pub struct CreateNoteOutput {
    slug: String,
    relative_path: String,
}

pub struct CreateNoteTool {
    pub cave: SharedCave,
}

impl Tool for CreateNoteTool {
    const NAME: &'static str = "create_note";
    type Error = ToolError;
    type Args = CreateNoteArgs;
    type Output = CreateNoteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "create_note".to_string(),
            description: "Create a new markdown note in the cave".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "The name for the new note (without .md extension)"
                    },
                    "folder": {
                        "type": "string",
                        "description": "Optional subfolder path (relative to cave root) to create the note in"
                    },
                    "icon": {
                        "type": "string",
                        "description": "Optional icon ID for the note (e.g. \"Star\", \"Book\", \"Code\"). Omit for the default file icon."
                    }
                },
                "required": ["name"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_cave_mut(&self.cave, |cave| {
            let meta =
                cave.create_note(&args.name, args.folder.as_deref().map(std::path::Path::new))?;
            if let Some(icon) = args.icon {
                cave.update_note(&meta.slug, &meta.slug, "", None, Some(icon))?;
            }
            Ok(CreateNoteOutput {
                slug: meta.slug.clone(),
                relative_path: meta.relative_path,
            })
        })
    }
}
