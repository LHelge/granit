//! Host-agnostic core of Granit.
//!
//! Everything that does not depend on the UI host lives here: the cave model
//! and its indexes (`cave`), markdown rendering (`markdown`), the AI agent with
//! its tools and RAG index (`agent`), encrypted backups (`backup`), and the
//! shared state the host keeps for the open cave (`state`), and the
//! `granit://` URLs rendered HTML links to (`urls`). The Tauri app in
//! `src-tauri/` wraps it with IPC commands, events and plugins; this crate must
//! never depend on Tauri.

pub mod agent;
pub mod backup;
pub mod cave;
pub mod markdown;
pub mod state;
pub mod urls;

pub use state::{with_shared_cave, CoreState, SharedCave};
