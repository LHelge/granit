//! Host-agnostic core of Granit.
//!
//! Everything that does not depend on the UI host lives here: the cave model
//! and its indexes (`cave`), markdown rendering (`markdown`), the month grids
//! of the daily-note calendar (`calendar`), the AI agent with
//! its tools and RAG index (`agent`), encrypted backups (`backup`), and the
//! shared state the host keeps for the open cave (`state`), config changes
//! from a settings UI (`config`), the open-cave flow and the RAG index build
//! (`open`), and the `granit://` URLs rendered HTML
//! links to (`urls`). The Tauri app in `src-tauri/` wraps it with IPC
//! commands, events and plugins, the GPUI app in `granit-app/` calls it
//! directly; this crate must never depend on a UI host.

pub mod agent;
pub mod backup;
pub mod calendar;
pub mod cave;
pub mod config;
pub mod markdown;
pub mod open;
pub mod state;
pub mod urls;

pub use state::{with_shared_cave, CoreState, SharedCave};
