mod navigation;
mod organization;
mod reading;
mod semantic;
mod skills;
mod todos;
mod web;
mod writing;

use crate::agent::vectordb::CaveVectorIndex;
use crate::commands::{with_shared_cave, SharedCave};
use granit_types::{AgentConfig, AgentMode};
pub use navigation::{ListFoldersTool, ListNotesTool, SearchContentTool, SearchNotesTool};
pub use organization::{
    CreateFolderTool, DeleteFolderTool, DeleteNoteTool, MoveFolderTool, MoveNoteTool,
    RenameFolderTool, RenameNoteTool,
};
pub use reading::ReadNoteTool;
use rig_agent::tool::server::ToolServer;
pub use semantic::SemanticSearchTool;
pub use skills::UseSkillTool;
pub use todos::{ListTodosTool, ToggleTodoTool};
pub use web::{WebFetchTool, WebSearchTool};
pub use writing::{CreateNoteTool, EditNoteTool, OpenDailyNoteTool, UpdateNoteTool};

/// Static metadata about each tool, for the settings UI.
struct ToolMeta {
    name: &'static str,
    description: &'static str,
}

/// The complete catalogue of tool metadata. Order is stable.
const TOOL_CATALOGUE: &[ToolMeta] = &[
    ToolMeta {
        name: "read_note",
        description: "Read a note's content and backlinks by slug (or the currently active note)",
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
        name: "semantic_search",
        description: "Find notes semantically related to a query (requires embeddings/RAG enabled)",
    },
    ToolMeta {
        name: "list_todos",
        description: "List todo checkboxes from notes, with optional filtering",
    },
    ToolMeta {
        name: "toggle_todo",
        description: "Toggle the completion status of a todo checkbox in a note",
    },
    ToolMeta {
        name: "use_skill",
        description: "Load the full instructions of a skill by name",
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

/// Tools that modify the cave. Excluded in Ask mode.
const MUTATING_TOOLS: &[&str] = &[
    "create_note",
    "update_note",
    "edit_note",
    "delete_note",
    "move_note",
    "rename_note",
    "create_folder",
    "rename_folder",
    "move_folder",
    "delete_folder",
    "open_daily_note",
    "toggle_todo",
];

/// Names of the tools that will actually be registered for `config`: the
/// catalogue minus disabled tools, minus mutating tools in Ask mode, minus
/// tools whose availability requirement is unmet (`semantic_search` needs
/// the vector index, `web_search` needs a Brave API key). Mirrors the logic
/// of [`build_toolset`]; used for the system prompt's `tools` variable.
pub fn enabled_tool_names(config: &AgentConfig, has_vector_index: bool) -> Vec<String> {
    let ask_mode = config.mode == AgentMode::Ask;
    let has_web_search_key = config
        .tool_config
        .web_search
        .api_key
        .as_deref()
        .is_some_and(|key| !key.trim().is_empty());
    TOOL_CATALOGUE
        .iter()
        .map(|meta| meta.name)
        .filter(|name| {
            let available = match *name {
                "semantic_search" => has_vector_index,
                "web_search" => has_web_search_key,
                _ => true,
            };
            available
                && !config.disabled_tools.iter().any(|d| d == name)
                && !(ask_mode && MUTATING_TOOLS.contains(name))
        })
        .map(str::to_string)
        .collect()
}

/// Build the full toolset from config, excluding disabled tools
/// and mutating tools when in Ask mode.
///
/// `vector_index` powers the `semantic_search` tool; when it is `None`
/// (RAG disabled, or the embedding model failed to load) the tool is simply
/// not registered.
pub fn build_toolset(
    cave: SharedCave,
    config: &AgentConfig,
    vector_index: Option<&CaveVectorIndex>,
) -> ToolServer {
    let disabled = &config.disabled_tools;
    let ask_mode = config.mode == AgentMode::Ask;
    let mut server = ToolServer::new();

    let is_excluded = |name: &str| -> bool {
        disabled.iter().any(|d| d == name) || (ask_mode && MUTATING_TOOLS.contains(&name))
    };

    // Cave tools
    type CaveToolBuilder = fn(ToolServer, SharedCave) -> ToolServer;
    let cave_entries: &[(&str, CaveToolBuilder)] = &[
        ("read_note", |s, c| s.tool(ReadNoteTool { cave: c })),
        ("list_notes", |s, c| s.tool(ListNotesTool { cave: c })),
        ("create_note", |s, c| s.tool(CreateNoteTool { cave: c })),
        ("update_note", |s, c| s.tool(UpdateNoteTool { cave: c })),
        ("edit_note", |s, c| s.tool(EditNoteTool { cave: c })),
        ("delete_note", |s, c| s.tool(DeleteNoteTool { cave: c })),
        ("move_note", |s, c| s.tool(MoveNoteTool { cave: c })),
        ("rename_note", |s, c| s.tool(RenameNoteTool { cave: c })),
        ("create_folder", |s, c| s.tool(CreateFolderTool { cave: c })),
        ("rename_folder", |s, c| s.tool(RenameFolderTool { cave: c })),
        ("move_folder", |s, c| s.tool(MoveFolderTool { cave: c })),
        ("delete_folder", |s, c| s.tool(DeleteFolderTool { cave: c })),
        ("open_daily_note", |s, c| {
            s.tool(OpenDailyNoteTool { cave: c })
        }),
        ("list_folders", |s, c| s.tool(ListFoldersTool { cave: c })),
        ("search_notes", |s, c| s.tool(SearchNotesTool { cave: c })),
        ("search_content", |s, c| {
            s.tool(SearchContentTool { cave: c })
        }),
        ("list_todos", |s, c| s.tool(ListTodosTool { cave: c })),
        ("toggle_todo", |s, c| s.tool(ToggleTodoTool { cave: c })),
        ("use_skill", |s, c| s.tool(UseSkillTool { cave: c })),
    ];

    for (name, add) in cave_entries {
        if !is_excluded(name) {
            server = add(server, cave.clone());
        }
    }

    // Semantic search — requires the vector index (RAG enabled and the
    // embedding model loaded).
    if !is_excluded("semantic_search") {
        if let Some(index) = vector_index {
            server = server.tool(SemanticSearchTool {
                index: index.clone(),
                default_top_n: config.rag.top_n,
            });
        }
    }

    // Web fetch — always available (no API key needed). A failed HTTP-client
    // build (TLS/proxy misconfiguration) drops the tool instead of failing
    // the whole agent.
    if !is_excluded("web_fetch") {
        match WebFetchTool::new(&config.tool_config.web_fetch) {
            Ok(tool) => server = server.tool(tool),
            Err(e) => log::warn!("web_fetch unavailable: failed to build HTTP client: {e}"),
        }
    }

    // Web search — requires a Brave API key
    if !is_excluded("web_search") {
        if let Some(api_key) = &config.tool_config.web_search.api_key {
            if !api_key.trim().is_empty() {
                match WebSearchTool::new(&config.tool_config.web_search) {
                    Ok(tool) => server = server.tool(tool),
                    Err(e) => {
                        log::warn!("web_search unavailable: failed to build HTTP client: {e}")
                    }
                }
            }
        }
    }

    server
}
