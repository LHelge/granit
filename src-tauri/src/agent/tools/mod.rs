mod create_note;
mod delete_note;
mod edit_note;
mod list_notes;
mod read_note;
mod search_notes;
mod update_note;

pub use create_note::CreateNoteTool;
pub use delete_note::DeleteNoteTool;
pub use edit_note::EditNoteTool;
pub use list_notes::ListNotesTool;
pub use read_note::ReadNoteTool;
pub use search_notes::SearchNotesTool;
pub use update_note::UpdateNoteTool;

use std::sync::{Arc, Mutex};

use rig::tool::ToolDyn;

use crate::cave::{Cave, CaveError};

/// Shared handle to the currently open cave, used by all agent tools.
pub type SharedCave = Arc<Mutex<Option<Cave>>>;

/// Metadata about each available cave tool.
struct ToolEntry {
    name: &'static str,
    description: &'static str,
    build: fn(SharedCave) -> Box<dyn ToolDyn>,
}

/// The full catalogue of cave tools. Order is stable.
const TOOL_CATALOGUE: &[ToolEntry] = &[
    ToolEntry {
        name: "read_note",
        description: "Read a note's content by slug (or the currently active note)",
        build: |cave| Box::new(ReadNoteTool { cave }),
    },
    ToolEntry {
        name: "list_notes",
        description: "List all notes in the cave with their slugs",
        build: |cave| Box::new(ListNotesTool { cave }),
    },
    ToolEntry {
        name: "create_note",
        description: "Create a new markdown note in the cave",
        build: |cave| Box::new(CreateNoteTool { cave }),
    },
    ToolEntry {
        name: "update_note",
        description: "Replace the entire body of a note",
        build: |cave| Box::new(UpdateNoteTool { cave }),
    },
    ToolEntry {
        name: "edit_note",
        description: "Find and replace text within a note's body",
        build: |cave| Box::new(EditNoteTool { cave }),
    },
    ToolEntry {
        name: "delete_note",
        description: "Delete a note from the cave",
        build: |cave| Box::new(DeleteNoteTool { cave }),
    },
    ToolEntry {
        name: "search_notes",
        description: "Search notes by slug (case-insensitive)",
        build: |cave| Box::new(SearchNotesTool { cave }),
    },
];

/// Return metadata for all known tools (for the settings UI).
pub fn tool_info_list() -> Vec<granit_types::ToolInfo> {
    TOOL_CATALOGUE
        .iter()
        .map(|e| granit_types::ToolInfo {
            name: e.name.to_string(),
            description: e.description.to_string(),
        })
        .collect()
}

/// Build the cave toolset, excluding any disabled tool names.
pub fn cave_toolset(cave: SharedCave, disabled: &[String]) -> Vec<Box<dyn ToolDyn>> {
    TOOL_CATALOGUE
        .iter()
        .filter(|e| !disabled.iter().any(|d| d == e.name))
        .map(|e| (e.build)(cave.clone()))
        .collect()
}

/// Error type for agent tool operations.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("No cave is currently open")]
    NoCaveOpen,
    #[error("Cave error: {0}")]
    Cave(#[from] CaveError),
}

/// Helper: lock the shared cave and run a read-only closure on it.
fn with_cave<F, T>(cave: &SharedCave, f: F) -> Result<T, ToolError>
where
    F: FnOnce(&Cave) -> Result<T, CaveError>,
{
    let guard = cave.lock().expect("cave mutex poisoned");
    let cave = guard.as_ref().ok_or(ToolError::NoCaveOpen)?;
    Ok(f(cave)?)
}

/// Helper: lock the shared cave and run a mutating closure on it.
fn with_cave_mut<F, T>(cave: &SharedCave, f: F) -> Result<T, ToolError>
where
    F: FnOnce(&mut Cave) -> Result<T, CaveError>,
{
    let mut guard = cave.lock().expect("cave mutex poisoned");
    let cave = guard.as_mut().ok_or(ToolError::NoCaveOpen)?;
    Ok(f(cave)?)
}
