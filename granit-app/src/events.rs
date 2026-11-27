//! Cave-change notifications for the UI.
//!
//! Every cave mutation made through [`crate::cave_ops`] (and, later, by the
//! agent's tools) is announced as a [`CaveEvent`] on the app-wide
//! [`CaveEvents`] entity. Views that show cave contents subscribe once and
//! refresh themselves, so a change made anywhere reaches every view:
//!
//! ```ignore
//! let events = CaveEvents::entity(cx);
//! let subscription = cx.subscribe(&events, |this, _, event: &CaveEvent, cx| {
//!     this.refresh(event, cx);
//! });
//! ```
//!
//! Code that changes the cave outside `cave_ops` (an agent tool run on the
//! tokio runtime) reports it from the foreground with [`CaveEvents::emit`],
//! typically `CaveEvent::NotesChanged` after a mutating tool call.

use gpui::{App, AppContext, Entity, EventEmitter, Global};

/// What changed in the cave.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaveEvent {
    /// A cave was opened, replacing the previous one.
    Opened,
    /// Notes or folders were created, changed, moved or renamed. Views
    /// re-read what they show.
    NotesChanged,
    /// The note `from` is now called `to`; other notes may have been
    /// rewritten to link to it. Implies `NotesChanged`.
    NoteRenamed { from: String, to: String },
    /// These notes no longer exist (a deleted note, or every note of a
    /// deleted folder). Implies `NotesChanged`.
    NotesRemoved(Vec<String>),
    /// A note template was created. (Saving a template's body announces
    /// nothing: no other view shows it.)
    TemplatesChanged,
    /// The note template `from` is now called `to`.
    TemplateRenamed { from: String, to: String },
    /// The note template no longer exists.
    TemplateRemoved(String),
}

/// The app-wide emitter of [`CaveEvent`]s.
pub struct CaveEvents;

impl EventEmitter<CaveEvent> for CaveEvents {}

struct GlobalCaveEvents(Entity<CaveEvents>);

impl Global for GlobalCaveEvents {}

impl CaveEvents {
    /// Create the emitter. Called once at startup, by [`crate::host::Host::init`].
    pub(crate) fn init(cx: &mut App) {
        let entity = cx.new(|_| CaveEvents);
        cx.set_global(GlobalCaveEvents(entity));
    }

    /// The emitter, to subscribe to.
    pub fn entity(cx: &App) -> Entity<CaveEvents> {
        cx.global::<GlobalCaveEvents>().0.clone()
    }

    /// Announce `event` to every subscriber.
    pub fn emit(event: CaveEvent, cx: &mut App) {
        Self::entity(cx).update(cx, |_, cx| cx.emit(event));
    }
}
