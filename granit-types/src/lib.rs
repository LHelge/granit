mod agent;
mod config;
mod icons;
mod note;

pub use agent::{
    default_system_prompt, AgentConfig, ChatMessage, ChatRole, ModelInfo, ProviderConfig,
    ProviderEntry, ProviderInfo, ToolCallInfo, ToolInfo,
};
pub use config::{AppConfig, FontConfig, SidebarConfig};
pub use icons::{resolve_note_icon, NoteIconEntry, NOTE_ICONS};
pub use note::{ContentMatch, Frontmatter, Note, NoteMeta, RenderedNote};
