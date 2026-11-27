//! Cave config changes of the UI, on top of the core.
//!
//! [`update_config`] is the write path of the settings view: it changes the
//! config through `CoreState::update_config` (which saves
//! `.granit/config.yml`, never with `active_cave`, and resets the agent when
//! its config changed), rebuilds or drops the RAG index like the Tauri
//! app's `save_config` command, and announces a [`ConfigEvent`] on the
//! app-wide [`ConfigEvents`], so the views apply it live: the workspace
//! re-themes, the note view and the chat take the new fonts, and the chat
//! starts over when the agent was reset.
//!
//! ```ignore
//! config_ops::update_config(cx, |config| config.daily_note_folder = "Journal".into())?;
//! ```
//!
//! Text and number fields use [`edit_config`] instead: the in-memory config
//! changes at once, and the save, agent reset and event follow once typing
//! pauses ([`COMMIT_DELAY`]), on Enter or blur, when the settings close,
//! before another cave opens, or before any [`update_config`]
//! ([`flush_config`]). A failed deferred save is announced as
//! [`ConfigSaveFailed`]; its edits stay pending for the next flush, and a
//! cave switch is refused while they cannot be saved.
//!
//! The agent's system prompt is a cave file, not config:
//! [`read_system_prompt`] / [`write_system_prompt`] (the latter resets the
//! agent and announces it too; a prompt it cannot write stays pending, here
//! rather than in the settings view, until [`flush_system_prompt`] writes
//! it). [`flush_all`] writes both kinds of pending change before a cave
//! switch.

use std::path::PathBuf;
use std::time::Duration;

use gpui::{App, AppContext as _, Entity, EventEmitter, Global, Task};
use granit_core::config::{ConfigChange, RagIndexAction};
use granit_core::open::build_vector_index;
use granit_types::{AppConfig, DEFAULT_SYSTEM_PROMPT_TEMPLATE};
use log::warn;

use crate::host::Host;
use crate::runtime;

/// What a config change touched, for the views to apply.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConfigEvent {
    /// The agent was reset (its config or system prompt changed): a running
    /// reply belongs to the old agent and its history is gone.
    pub agent: bool,
    /// The theme setting changed.
    pub theme: bool,
    /// The editor or chat font changed.
    pub fonts: bool,
}

/// The app-wide emitter of [`ConfigEvent`]s.
pub struct ConfigEvents;

impl EventEmitter<ConfigEvent> for ConfigEvents {}

struct GlobalConfigEvents(Entity<ConfigEvents>);

impl Global for GlobalConfigEvents {}

impl ConfigEvents {
    /// Create the emitter. Called once at startup, by [`Host::init`].
    pub(crate) fn init(cx: &mut App) {
        let entity = cx.new(|_| ConfigEvents);
        cx.set_global(GlobalConfigEvents(entity));
    }

    /// The emitter, to subscribe to.
    pub fn entity(cx: &App) -> Entity<ConfigEvents> {
        cx.global::<GlobalConfigEvents>().0.clone()
    }

    /// Announce `event` to every subscriber.
    pub fn emit(event: ConfigEvent, cx: &mut App) {
        Self::entity(cx).update(cx, |_, cx| cx.emit(event));
    }
}

/// A deferred save ([`edit_config`]) failed; the message says why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigSaveFailed(pub String);

impl EventEmitter<ConfigSaveFailed> for ConfigEvents {}

/// How long typing must pause before [`edit_config`] commits.
pub const COMMIT_DELAY: Duration = Duration::from_millis(500);

/// Edits made by [`edit_config`] and not committed yet.
struct PendingEdits {
    /// The config before the first of them.
    baseline: AppConfig,
    /// The cave they were made in.
    cave: Option<PathBuf>,
    /// Commits them after [`COMMIT_DELAY`]; replaced by every edit. `None`
    /// once a commit failed: the edits wait for the next save.
    _commit: Option<Task<()>>,
}

#[derive(Default)]
struct GlobalPendingEdits(Option<PendingEdits>);

impl Global for GlobalPendingEdits {}

/// Change the config with `f`, save it to the open cave and apply it.
/// Edits pending from [`edit_config`] are committed first. Returns whether
/// anything changed. On an error (the config file could not be written)
/// `f` is not applied; edits pending from [`edit_config`] stay in memory,
/// shown by the settings, and are saved by the next flush that succeeds.
pub fn update_config(cx: &mut App, f: impl FnOnce(&mut AppConfig)) -> Result<bool, String> {
    flush_config(cx)?;
    let core = Host::core(cx);
    let change = core.update_config(f).map_err(|err| err.to_string())?;
    Ok(apply(change, cx))
}

/// Change the config with `f` in memory now, so the settings show it, and
/// commit it like [`update_config`] once edits pause for [`COMMIT_DELAY`]
/// (or at [`flush_config`]). For text and number fields: typing a word is
/// one save and at most one agent reset, not one per key.
pub fn edit_config(cx: &mut App, f: impl FnOnce(&mut AppConfig)) {
    let core = Host::core(cx);
    let before = core.lock_config().clone();
    {
        let mut config = core.lock_config();
        f(&mut config);
        config.active_cave = None;
    }
    let baseline = cx
        .default_global::<GlobalPendingEdits>()
        .0
        .take()
        .map_or(before, |pending| pending.baseline);
    let commit = cx.spawn(async move |cx| {
        cx.background_executor().timer(COMMIT_DELAY).await;
        cx.update(commit_edits);
    });
    cx.default_global::<GlobalPendingEdits>().0 = Some(PendingEdits {
        baseline,
        cave: core.active_cave_path(),
        _commit: Some(commit),
    });
}

/// Commit the edits [`edit_config`] holds, now. Edits made in a cave that
/// is no longer open are dropped (that cave's config is gone from memory).
/// Returns whether anything changed. When the config cannot be saved, the
/// edits stay pending against the config before them, so the next flush
/// (a later edit, [`update_config`], a cave switch) saves them and still
/// resets the agent when its config changed.
pub fn flush_config(cx: &mut App) -> Result<bool, String> {
    let Some(pending) = cx.default_global::<GlobalPendingEdits>().0.take() else {
        return Ok(false);
    };
    let core = Host::core(cx);
    if pending.cave != core.active_cave_path() {
        return Ok(false);
    }
    let current = core.lock_config().clone();
    match core.commit_config(&pending.baseline, current) {
        Ok(change) => Ok(apply(change, cx)),
        Err(err) => {
            cx.default_global::<GlobalPendingEdits>().0 = Some(PendingEdits {
                _commit: None,
                ..pending
            });
            Err(err.to_string())
        }
    }
}

/// [`flush_config`] where nobody waits for the outcome (a typing pause,
/// Enter or blur in a field): a failure is announced as
/// [`ConfigSaveFailed`].
pub fn commit_edits(cx: &mut App) {
    if let Err(err) = flush_config(cx) {
        warn!("Could not save the settings: {err}");
        ConfigEvents::entity(cx).update(cx, |_, cx| cx.emit(ConfigSaveFailed(err)));
    }
}

/// Apply a saved change: the RAG index, and the views through
/// [`ConfigEvents`]. Returns whether there was one.
fn apply(change: Option<ConfigChange>, cx: &mut App) -> bool {
    let Some(change) = change else {
        return false;
    };
    let core = Host::core(cx);
    match change.rag {
        RagIndexAction::Rebuild => {
            if let (Some(dirs), Some(_)) = (Host::embedding_models(cx), core.active_cave_path()) {
                build_vector_index(&core, &change.config.agent.rag, &dirs, &runtime::handle());
            }
        }
        RagIndexAction::Disable => core.set_vector_index(None),
        RagIndexAction::None => {}
    }
    ConfigEvents::emit(
        ConfigEvent {
            agent: change.agent,
            theme: change.theme,
            fonts: change.fonts,
        },
        cx,
    );
    true
}

/// The open cave's system prompt (`.granit/agent/system.md`), seeded with
/// the default template when it is missing, as the Tauri app does.
pub fn read_system_prompt(cx: &App) -> Result<String, String> {
    Host::core(cx)
        .with_cave(|cave| match cave.read_system_prompt_raw()? {
            Some(content) => Ok(content),
            None => {
                cave.write_system_prompt(DEFAULT_SYSTEM_PROMPT_TEMPLATE)?;
                Ok(DEFAULT_SYSTEM_PROMPT_TEMPLATE.to_string())
            }
        })
        .map_err(|err| err.to_string())
}

/// A system prompt [`write_system_prompt`] could not write, kept to be
/// written by [`flush_system_prompt`] (the settings may be closed by then).
struct PendingPrompt {
    text: String,
    /// The cave it was typed in.
    cave: Option<PathBuf>,
}

#[derive(Default)]
struct GlobalPendingPrompt(Option<PendingPrompt>);

impl Global for GlobalPendingPrompt {}

/// Write the open cave's system prompt and reset the agent, so the next
/// message uses it. An unchanged prompt writes nothing and keeps the agent.
/// A prompt that cannot be written stays pending for [`flush_system_prompt`]
/// (a later write replaces it).
pub fn write_system_prompt(content: &str, cx: &mut App) -> Result<(), String> {
    let core = Host::core(cx);
    let result = core
        .with_cave(|cave| {
            if cave.read_system_prompt_raw()?.as_deref() == Some(content) {
                return Ok(false);
            }
            cave.write_system_prompt(content)?;
            Ok(true)
        })
        .map_err(|err| err.to_string());
    cx.default_global::<GlobalPendingPrompt>().0 = result.is_err().then(|| PendingPrompt {
        text: content.to_string(),
        cave: core.active_cave_path(),
    });
    if !result? {
        return Ok(());
    }
    core.reset_agent();
    ConfigEvents::emit(
        ConfigEvent {
            agent: true,
            ..ConfigEvent::default()
        },
        cx,
    );
    Ok(())
}

/// Write the system prompt a failed [`write_system_prompt`] left pending,
/// if it was typed in the open cave (one typed in a cave no longer open is
/// dropped). On a failure it stays pending.
pub fn flush_system_prompt(cx: &mut App) -> Result<(), String> {
    let Some(pending) = cx.default_global::<GlobalPendingPrompt>().0.take() else {
        return Ok(());
    };
    if pending.cave != Host::cave_path(cx) {
        return Ok(());
    }
    write_system_prompt(&pending.text, cx)
}

/// Write everything the settings left pending: field edits
/// ([`flush_config`]) and a system prompt that could not be written
/// ([`flush_system_prompt`]). Both are tried; the first failure is
/// returned. Called before another cave opens.
pub fn flush_all(cx: &mut App) -> Result<(), String> {
    let config = flush_config(cx).map(|_| ());
    let prompt = flush_system_prompt(cx);
    config.and(prompt)
}

/// Whether settings changes wait to be saved: field edits or a system
/// prompt that could not be written.
pub fn has_unsaved(cx: &App) -> bool {
    cx.try_global::<GlobalPendingEdits>()
        .is_some_and(|pending| pending.0.is_some())
        || cx
            .try_global::<GlobalPendingPrompt>()
            .is_some_and(|pending| pending.0.is_some())
}
