mod agent;
mod config;
mod document;
mod icons;
mod metadata;

pub use agent::{
    default_system_prompt, AgentConfig, AttachedNote, ChatMessage, ChatRole, ModelInfo,
    ProviderConfig, ProviderEntry, ProviderInfo, ToolCallInfo, ToolInfo,
};
pub use config::{AppConfig, FontConfig, SidebarConfig};
pub use document::{
    ContentMatch, Document, DocumentMeta, Frontmatter, RenderedDocument, TagMap, TodoItem, TodoList,
};
pub use icons::{resolve_note_icon, NoteIconEntry, NOTE_ICONS};
pub use metadata::AppMetadata;
