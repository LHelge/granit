mod agent;
mod config;
mod document;
mod icons;
mod metadata;
mod update;

pub use agent::{
    AgentConfig, AgentDocInfo, AgentMode, AttachedNote, ChatMessage, ChatRole, ModelInfo,
    ProviderConfig, ProviderEntry, ProviderInfo, RagConfig, ToolCallInfo, ToolInfo, ToolsConfig,
    WebFetchConfig, WebSearchConfig, DEFAULT_SYSTEM_PROMPT_TEMPLATE,
};
pub use config::{AppConfig, FontConfig, SidebarConfig};
pub use document::{
    ContentMatch, Document, DocumentMeta, Frontmatter, RenderedDocument, TagMap, TodoItem, TodoList,
};
pub use icons::{resolve_note_icon, NoteIconEntry, NOTE_ICONS};
pub use metadata::AppMetadata;
pub use update::{ReleaseNotes, UpdateCheckStatus};
