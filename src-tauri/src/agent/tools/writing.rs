use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_cave, with_cave_mut, SharedCave, ToolError};

// ── create_note ────────────────────────────────────────────────────

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
                cave.set_note_icon(&meta.slug, Some(icon))?;
            }
            Ok(CreateNoteOutput {
                slug: meta.slug.clone(),
                relative_path: meta.relative_path,
            })
        })
    }
}

// ── update_note ────────────────────────────────────────────────────

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
            let meta = cave.update_note(&slug, &slug, &args.content, None, args.icon)?;
            Ok(UpdateNoteOutput {
                slug: meta.slug,
                relative_path: meta.relative_path,
            })
        })
    }
}

// ── edit_note ──────────────────────────────────────────────────────

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

// ── open_daily_note ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct OpenDailyNoteArgs {
    /// Folder for daily notes (relative path from cave root, e.g. "Daily").
    folder: Option<String>,
}

#[derive(Serialize)]
pub struct OpenDailyNoteOutput {
    slug: String,
    relative_path: String,
    content: String,
}

pub struct OpenDailyNoteTool {
    pub cave: SharedCave,
}

impl Tool for OpenDailyNoteTool {
    const NAME: &'static str = "open_daily_note";
    type Error = ToolError;
    type Args = OpenDailyNoteArgs;
    type Output = OpenDailyNoteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "open_daily_note".to_string(),
            description:
                "Open or create today's daily note. Creates the folder and note if they don't exist."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "folder": {
                        "type": "string",
                        "description": "Folder for daily notes (default: \"Daily\")"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_cave_mut(&self.cave, |cave| {
            let folder = args.folder.as_deref().unwrap_or("Daily");
            let note = cave.open_daily_note(folder)?;
            Ok(OpenDailyNoteOutput {
                slug: note.meta.slug,
                relative_path: note.meta.relative_path,
                content: note.content,
            })
        })
    }
}
