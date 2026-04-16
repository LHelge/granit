mod agent;
mod config;
mod document;
mod error;
mod events;
mod icons;
mod metadata;

pub use agent::{
    default_system_prompt, AgentConfig, AttachedNote, ChatMessage, ChatRole, ModelInfo,
    ProviderConfig, ProviderEntry, ToolCallInfo, ToolInfo, ToolsConfig, WebFetchConfig,
    WebSearchConfig,
};
pub use config::{AppConfig, FontConfig, SidebarConfig};
pub use document::{
    ContentMatch, Document, DocumentMeta, Frontmatter, RenderedDocument, TagMap, TodoItem, TodoList,
};
pub use error::IpcError;
pub use events::{
    AGENT_STREAM_CHUNK, AGENT_STREAM_DONE, AGENT_STREAM_ERROR, AGENT_TOOL_CALL, CAVE_NOTES_CHANGED,
};
pub use icons::{resolve_note_icon, NoteIconEntry, NOTE_ICONS};
pub use metadata::AppMetadata;
