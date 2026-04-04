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

/// Build the standard set of cave tools as boxed trait objects.
pub fn cave_toolset(cave: SharedCave) -> Vec<Box<dyn ToolDyn>> {
    vec![
        Box::new(ReadNoteTool { cave: cave.clone() }),
        Box::new(ListNotesTool { cave: cave.clone() }),
        Box::new(CreateNoteTool { cave: cave.clone() }),
        Box::new(UpdateNoteTool { cave: cave.clone() }),
        Box::new(EditNoteTool { cave: cave.clone() }),
        Box::new(DeleteNoteTool { cave: cave.clone() }),
        Box::new(SearchNotesTool { cave }),
    ]
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
