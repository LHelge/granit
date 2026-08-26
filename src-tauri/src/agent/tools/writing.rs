use rig_core::tool::PortableTool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_shared_cave, SharedCave};
use crate::cave::CaveError;

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

impl PortableTool for CreateNoteTool {
    const NAME: &'static str = "create_note";
    type Error = CaveError;
    type Args = CreateNoteArgs;
    type Output = CreateNoteOutput;

    fn description(&self) -> String {
        "Create a new markdown note in the cave".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
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
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            let meta = cave.create_note(
                &args.name,
                args.folder.as_deref().map(std::path::Path::new),
                None,
            )?;
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

impl PortableTool for UpdateNoteTool {
    const NAME: &'static str = "update_note";
    type Error = CaveError;
    type Args = UpdateNoteArgs;
    type Output = UpdateNoteOutput;

    fn description(&self) -> String {
        "Update the content of an existing note by slug. Overwrites the note body. Frontmatter is managed automatically — do not include it.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
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
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            let slug = cave.resolve_slug(&args.slug)?;
            let meta = cave.update_note(&slug, &slug, &args.content, None, args.icon, None)?;
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

impl PortableTool for EditNoteTool {
    const NAME: &'static str = "edit_note";
    type Error = CaveError;
    type Args = EditNoteArgs;
    type Output = EditNoteOutput;

    fn description(&self) -> String {
        "Replace text in a note (find and replace on the body only, excluding frontmatter). Pass a slug to target a specific note, or omit it to edit the note currently open in the editor. Fails if the text is not found."
                    .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
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
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
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
    /// Date of the daily note in YYYY-MM-DD format. Omit for today.
    date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OpenDailyNoteOutput {
    slug: String,
    relative_path: String,
    content: String,
}

pub struct OpenDailyNoteTool {
    pub cave: SharedCave,
}

impl PortableTool for OpenDailyNoteTool {
    const NAME: &'static str = "open_daily_note";
    type Error = CaveError;
    type Args = OpenDailyNoteArgs;
    type Output = OpenDailyNoteOutput;

    fn description(&self) -> String {
        "Open or create a daily note in the configured daily note folder. Pass a date (YYYY-MM-DD) to open that day's note — past, present, or future — or omit it for today. Creates the folder and note if they don't exist."
                    .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "date": {
                    "type": "string",
                    "description": "The date of the daily note in YYYY-MM-DD format. Omit for today."
                }
            },
            "required": []
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            let config = cave.load_config()?;
            let note = match args.date.as_deref() {
                Some(date) => cave.open_daily_note_for_date(
                    date,
                    &config.daily_note_folder,
                    config.daily_note_template_slug.as_deref(),
                )?,
                None => cave.open_daily_note(
                    &config.daily_note_folder,
                    config.daily_note_template_slug.as_deref(),
                )?,
            };
            Ok(OpenDailyNoteOutput {
                slug: note.meta.slug,
                relative_path: note.meta.relative_path,
                content: note.content,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;

    fn shared_cave(cave: crate::cave::Cave) -> SharedCave {
        Arc::new(Mutex::new(Some(cave)))
    }

    #[tokio::test]
    async fn open_daily_note_tool_accepts_a_date() {
        let dir = tempfile::tempdir().unwrap();
        let cave = crate::cave::Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_config().unwrap();
        let tool = OpenDailyNoteTool {
            cave: shared_cave(cave),
        };

        let output = tool
            .call(OpenDailyNoteArgs {
                date: Some("2026-08-21".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(output.slug, "2026-08-21");
        assert!(dir.path().join("Daily/2026-08-21.md").exists());

        // Omitting the date still opens today's note.
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let output = tool.call(OpenDailyNoteArgs { date: None }).await.unwrap();
        assert_eq!(output.slug, today);
    }

    #[tokio::test]
    async fn open_daily_note_tool_rejects_invalid_dates() {
        let dir = tempfile::tempdir().unwrap();
        let cave = crate::cave::Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_config().unwrap();
        let tool = OpenDailyNoteTool {
            cave: shared_cave(cave),
        };

        for bad in ["not-a-date", "2026-13-99", "../escape"] {
            let err = tool
                .call(OpenDailyNoteArgs {
                    date: Some(bad.to_string()),
                })
                .await
                .unwrap_err();
            assert!(matches!(err, CaveError::InvalidName(_)), "{bad}: {err:?}");
        }
    }
}
