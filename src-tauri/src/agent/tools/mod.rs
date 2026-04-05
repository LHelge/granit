mod navigation;
mod organization;
mod reading;
mod web;
mod writing;

pub use navigation::{ListFoldersTool, ListNotesTool, SearchContentTool, SearchNotesTool};
pub use organization::{
    CreateFolderTool, DeleteFolderTool, DeleteNoteTool, MoveFolderTool, MoveNoteTool,
    RenameFolderTool, RenameNoteTool,
};
pub use reading::ReadNoteTool;
pub use web::{WebFetchTool, WebSearchTool};
pub use writing::{CreateNoteTool, EditNoteTool, OpenDailyNoteTool, UpdateNoteTool};

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
        name: "move_note",
        description: "Move a note to a different folder",
    },
    ToolMeta {
        name: "rename_note",
        description: "Rename a note in-place",
    },
    ToolMeta {
        name: "create_folder",
        description: "Create a new folder in the cave",
    },
    ToolMeta {
        name: "rename_folder",
        description: "Rename a folder in-place",
    },
    ToolMeta {
        name: "move_folder",
        description: "Move a folder under a new parent",
    },
    ToolMeta {
        name: "delete_folder",
        description: "Delete a folder and all its notes",
    },
    ToolMeta {
        name: "open_daily_note",
        description: "Open or create today's daily note",
    },
    ToolMeta {
        name: "list_folders",
        description: "List all folders in the cave",
    },
    ToolMeta {
        name: "search_notes",
        description: "Search notes by slug (case-insensitive)",
    },
    ToolMeta {
        name: "search_content",
        description: "Search inside note bodies (full-text)",
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
        ("move_note", |c| Box::new(MoveNoteTool { cave: c })),
        ("rename_note", |c| Box::new(RenameNoteTool { cave: c })),
        ("create_folder", |c| Box::new(CreateFolderTool { cave: c })),
        ("rename_folder", |c| Box::new(RenameFolderTool { cave: c })),
        ("move_folder", |c| Box::new(MoveFolderTool { cave: c })),
        ("delete_folder", |c| Box::new(DeleteFolderTool { cave: c })),
        ("open_daily_note", |c| {
            Box::new(OpenDailyNoteTool { cave: c })
        }),
        ("list_folders", |c| Box::new(ListFoldersTool { cave: c })),
        ("search_notes", |c| Box::new(SearchNotesTool { cave: c })),
        ("search_content", |c| {
            Box::new(SearchContentTool { cave: c })
        }),
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
