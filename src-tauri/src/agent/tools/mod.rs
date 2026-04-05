mod create_note;
mod delete_note;
mod edit_note;
mod list_notes;
mod read_note;
mod search_notes;
mod update_note;
mod web_fetch;
mod web_search;

pub use create_note::CreateNoteTool;
pub use delete_note::DeleteNoteTool;
pub use edit_note::EditNoteTool;
pub use list_notes::ListNotesTool;
pub use read_note::ReadNoteTool;
pub use search_notes::SearchNotesTool;
pub use update_note::UpdateNoteTool;
pub use web_fetch::WebFetchTool;
pub use web_search::WebSearchTool;

use std::sync::{Arc, Mutex};

use granit_types::AgentConfig;
use rig::tool::ToolDyn;

use crate::cave::{Cave, CaveError};

/// Shared handle to the currently open cave, used by all agent tools.
pub type SharedCave = Arc<Mutex<Option<Cave>>>;

/// Static metadata about each tool, for the settings UI.
struct ToolMeta {
    name: &'static str,
    description: &'static str,
}

/// The complete catalogue of tool metadata. Order is stable.
const TOOL_CATALOGUE: &[ToolMeta] = &[
    ToolMeta {
        name: "read_note",
        description: "Read a note's content by slug (or the currently active note)",
    },
    ToolMeta {
        name: "list_notes",
        description: "List all notes in the cave with their slugs",
    },
    ToolMeta {
        name: "create_note",
        description: "Create a new markdown note in the cave",
    },
    ToolMeta {
        name: "update_note",
        description: "Replace the entire body of a note",
    },
    ToolMeta {
        name: "edit_note",
        description: "Find and replace text within a note's body",
    },
    ToolMeta {
        name: "delete_note",
        description: "Delete a note from the cave",
    },
    ToolMeta {
        name: "search_notes",
        description: "Search notes by slug (case-insensitive)",
    },
    ToolMeta {
        name: "web_fetch",
        description: "Fetch a webpage and return its content as markdown",
    },
    ToolMeta {
        name: "web_search",
        description: "Search the web using Brave Search",
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

/// Build the full toolset from config, excluding disabled tools.
pub fn build_toolset(cave: SharedCave, config: &AgentConfig) -> Vec<Box<dyn ToolDyn>> {
    let disabled = &config.disabled_tools;
    let mut tools: Vec<Box<dyn ToolDyn>> = Vec::new();

    // Cave tools
    type CaveToolBuilder = fn(SharedCave) -> Box<dyn ToolDyn>;
    let cave_entries: &[(&str, CaveToolBuilder)] = &[
        ("read_note", |c| Box::new(ReadNoteTool { cave: c })),
        ("list_notes", |c| Box::new(ListNotesTool { cave: c })),
        ("create_note", |c| Box::new(CreateNoteTool { cave: c })),
        ("update_note", |c| Box::new(UpdateNoteTool { cave: c })),
        ("edit_note", |c| Box::new(EditNoteTool { cave: c })),
        ("delete_note", |c| Box::new(DeleteNoteTool { cave: c })),
        ("search_notes", |c| Box::new(SearchNotesTool { cave: c })),
    ];

    for (name, build) in cave_entries {
        if !disabled.iter().any(|d| d == name) {
            tools.push(build(cave.clone()));
        }
    }

    // Web fetch — always available (no API key needed)
    if !disabled.iter().any(|d| d == "web_fetch") {
        tools.push(Box::new(WebFetchTool::new()));
    }

    // Web search — requires a Brave API key
    if !disabled.iter().any(|d| d == "web_search") {
        if let Some(api_key) = &config.brave_api_key {
            if !api_key.trim().is_empty() {
                tools.push(Box::new(WebSearchTool::new(api_key)));
            }
        }
    }

    tools
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
