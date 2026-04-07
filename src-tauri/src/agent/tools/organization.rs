use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{SharedCave, with_shared_cave};
use crate::cave::CaveError;

// ── move_note ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MoveNoteArgs {
    /// The slug of the note to move.
    slug: String,
    /// Destination folder (relative path from cave root). Empty string or omitted means cave root.
    destination: Option<String>,
}

#[derive(Serialize)]
pub struct MoveNoteOutput {
    slug: String,
    new_path: String,
}

pub struct MoveNoteTool {
    pub cave: SharedCave,
}

impl Tool for MoveNoteTool {
    const NAME: &'static str = "move_note";
    type Error = CaveError;
    type Args = MoveNoteArgs;
    type Output = MoveNoteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "move_note".to_string(),
            description: "Move a note to a different folder within the cave".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "The slug (filename without .md) of the note to move"
                    },
                    "destination": {
                        "type": "string",
                        "description": "Target folder path relative to cave root (empty or omitted for cave root)"
                    }
                },
                "required": ["slug"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            let slug = cave.resolve_slug(&args.slug)?;
            let dest = args
                .destination
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(std::path::Path::new);
            let meta = cave.move_note(&slug, dest)?;
            Ok(MoveNoteOutput {
                slug: meta.slug,
                new_path: meta.relative_path,
            })
        })
    }
}

// ── rename_note ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RenameNoteArgs {
    /// The current slug of the note to rename.
    slug: String,
    /// The new name for the note (becomes the new slug).
    new_name: String,
}

#[derive(Serialize)]
pub struct RenameNoteOutput {
    old_slug: String,
    new_slug: String,
    new_path: String,
}

pub struct RenameNoteTool {
    pub cave: SharedCave,
}

impl Tool for RenameNoteTool {
    const NAME: &'static str = "rename_note";
    type Error = CaveError;
    type Args = RenameNoteArgs;
    type Output = RenameNoteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "rename_note".to_string(),
            description: "Rename a note in-place (same folder, new filename/slug)".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "The current slug (filename without .md) of the note"
                    },
                    "new_name": {
                        "type": "string",
                        "description": "The new name for the note (becomes the new slug)"
                    }
                },
                "required": ["slug", "new_name"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            let slug = cave.resolve_slug(&args.slug)?;
            let meta = cave.rename_note(&slug, &args.new_name)?;
            Ok(RenameNoteOutput {
                old_slug: slug,
                new_slug: meta.slug,
                new_path: meta.relative_path,
            })
        })
    }
}

// ── delete_note ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DeleteNoteArgs {
    /// The slug of the note to delete.
    slug: String,
}

#[derive(Serialize)]
pub struct DeleteNoteOutput {
    deleted: String,
}

pub struct DeleteNoteTool {
    pub cave: SharedCave,
}

impl Tool for DeleteNoteTool {
    const NAME: &'static str = "delete_note";
    type Error = CaveError;
    type Args = DeleteNoteArgs;
    type Output = DeleteNoteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "delete_note".to_string(),
            description: "Delete a note from the cave by its slug".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "The slug (filename without .md) of the note to delete"
                    }
                },
                "required": ["slug"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            let slug = cave.resolve_slug(&args.slug)?;
            cave.delete_note(&slug)?;
            Ok(DeleteNoteOutput { deleted: slug })
        })
    }
}

// ── create_folder ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateFolderArgs {
    /// Relative path of the folder to create (e.g. "projects" or "notes/2026").
    path: String,
}

#[derive(Serialize)]
pub struct CreateFolderOutput {
    created: String,
}

pub struct CreateFolderTool {
    pub cave: SharedCave,
}

impl Tool for CreateFolderTool {
    const NAME: &'static str = "create_folder";
    type Error = CaveError;
    type Args = CreateFolderArgs;
    type Output = CreateFolderOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "create_folder".to_string(),
            description: "Create a new folder (subdirectory) in the cave".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path of the folder to create (e.g. \"projects\" or \"notes/2026\")"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            cave.create_folder(std::path::Path::new(&args.path))?;
            Ok(CreateFolderOutput {
                created: args.path.clone(),
            })
        })
    }
}

// ── rename_folder ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RenameFolderArgs {
    /// Relative path of the folder to rename.
    path: String,
    /// New name for the folder (just the final component, not a full path).
    new_name: String,
}

#[derive(Serialize)]
pub struct RenameFolderOutput {
    old_path: String,
    new_name: String,
}

pub struct RenameFolderTool {
    pub cave: SharedCave,
}

impl Tool for RenameFolderTool {
    const NAME: &'static str = "rename_folder";
    type Error = CaveError;
    type Args = RenameFolderArgs;
    type Output = RenameFolderOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "rename_folder".to_string(),
            description: "Rename a folder in-place (same parent, new name)".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path of the folder to rename"
                    },
                    "new_name": {
                        "type": "string",
                        "description": "New name for the folder (just the name, not a path)"
                    }
                },
                "required": ["path", "new_name"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            cave.rename_folder(std::path::Path::new(&args.path), &args.new_name)?;
            Ok(RenameFolderOutput {
                old_path: args.path.clone(),
                new_name: args.new_name.clone(),
            })
        })
    }
}

// ── move_folder ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MoveFolderArgs {
    /// Relative path of the folder to move.
    path: String,
    /// Destination parent folder (relative path). Empty or omitted means cave root.
    destination: Option<String>,
}

#[derive(Serialize)]
pub struct MoveFolderOutput {
    moved: String,
    destination: String,
}

pub struct MoveFolderTool {
    pub cave: SharedCave,
}

impl Tool for MoveFolderTool {
    const NAME: &'static str = "move_folder";
    type Error = CaveError;
    type Args = MoveFolderArgs;
    type Output = MoveFolderOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "move_folder".to_string(),
            description: "Move a folder under a new parent directory within the cave".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path of the folder to move"
                    },
                    "destination": {
                        "type": "string",
                        "description": "Destination parent folder (relative path, empty or omitted for cave root)"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            let dest = args
                .destination
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(std::path::Path::new);
            cave.move_folder(std::path::Path::new(&args.path), dest)?;
            Ok(MoveFolderOutput {
                moved: args.path.clone(),
                destination: args.destination.unwrap_or_default(),
            })
        })
    }
}

// ── delete_folder ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DeleteFolderArgs {
    /// Relative path of the folder to delete.
    path: String,
}

#[derive(Serialize)]
pub struct DeleteFolderOutput {
    deleted: String,
}

pub struct DeleteFolderTool {
    pub cave: SharedCave,
}

impl Tool for DeleteFolderTool {
    const NAME: &'static str = "delete_folder";
    type Error = CaveError;
    type Args = DeleteFolderArgs;
    type Output = DeleteFolderOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "delete_folder".to_string(),
            description: "Delete a folder and all notes within it from the cave".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path of the folder to delete"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            cave.delete_folder(std::path::Path::new(&args.path))?;
            Ok(DeleteFolderOutput {
                deleted: args.path.clone(),
            })
        })
    }
}
