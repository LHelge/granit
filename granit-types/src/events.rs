//! Event names used across the Tauri IPC boundary.
//!
//! Centralizing these as `pub const &str` here keeps frontend and
//! backend in lock-step — a typo fails at compile time instead of
//! silently dropping events at runtime.

/// Emitted whenever cave contents change (notes or folders). Payload
/// is unit — the frontend reacts by refetching what it cares about.
pub const CAVE_NOTES_CHANGED: &str = "cave:notes-changed";

/// Emitted for every streaming text chunk from the agent. Payload is
/// a `String` (the chunk).
pub const AGENT_STREAM_CHUNK: &str = "agent:stream-chunk";

/// Emitted once when agent streaming completes successfully. Unit payload.
pub const AGENT_STREAM_DONE: &str = "agent:stream-done";

/// Emitted once when agent streaming errors. Payload is a `String`
/// (the error message).
pub const AGENT_STREAM_ERROR: &str = "agent:stream-error";

/// Emitted when the agent invokes a tool. Payload is [`ToolCallInfo`].
pub const AGENT_TOOL_CALL: &str = "agent:tool-call";
