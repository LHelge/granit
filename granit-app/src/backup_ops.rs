//! Backups for the UI, on top of the core's flows
//! (`granit_core::backup::run`).
//!
//! The app-wide [`BackupOps`] entity knows the running backup or restore
//! and its stage ([`BackupOps::running`]), and announces [`BackupEvent`]s.
//! The flows run on the tokio runtime ([`crate::runtime`]); their progress
//! comes back over a channel and updates the entity, which the backup views
//! observe.
//!
//! Restores are started by the workspace, because a restore ends with
//! opening a cave and must finish even when the view that asked for it is
//! gone: a view calls [`request_restore`], the workspace (subscribed to
//! [`BackupEvent::RestoreRequested`]) writes pending edits and runs
//! [`restore`], then opens the cave it returns.
//!
//! Credentials are the open cave's `backup` config (typed edits are
//! committed first, [`config_ops::flush_config`]) or, for a disaster
//! restore, given with the request and never saved. The passphrase is
//! never stored or logged.

use std::path::PathBuf;

use gpui::{App, AppContext as _, Entity, EventEmitter, Global, Task};
use granit_core::backup::{self, BackupError, BackupInfo, RestoreRequest};
use granit_types::{BackupConfig, BackupStage, RestoreStage};

use crate::config_ops;
use crate::host::Host;
use crate::runtime;

/// What is running, and how far it got.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Running {
    Backup(BackupStage),
    Restore(RestoreStage),
}

impl Running {
    /// The stage, for a progress line.
    pub fn label(self) -> &'static str {
        match self {
            Running::Backup(BackupStage::Packing) => "Packing cave…",
            Running::Backup(BackupStage::Encrypting) => "Encrypting…",
            Running::Backup(BackupStage::Uploading) => "Uploading…",
            Running::Restore(RestoreStage::Downloading) => "Downloading…",
            Running::Restore(RestoreStage::Decrypting) => "Decrypting…",
            Running::Restore(RestoreStage::Unpacking) => "Unpacking…",
        }
    }
}

/// A failed restore, as the restore views show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreFailed {
    pub message: String,
    /// The cave's cached key does not fit the snapshot (or there is none)
    /// and no passphrase was given: ask for it and try again.
    pub needs_passphrase: bool,
}

/// What happened to a backup or restore.
#[derive(Debug)]
pub enum BackupEvent {
    /// A view asks for a restore; the workspace runs it.
    RestoreRequested(RestoreRequest),
    /// A backup finished.
    BackedUp(Result<BackupInfo, String>),
    /// A restore failed. A successful one opens the restored cave.
    RestoreFailed(RestoreFailed),
}

/// The app-wide backup state and emitter of [`BackupEvent`]s.
#[derive(Default)]
pub struct BackupOps {
    running: Option<Running>,
}

impl EventEmitter<BackupEvent> for BackupOps {}

struct GlobalBackupOps(Entity<BackupOps>);

impl Global for GlobalBackupOps {}

impl BackupOps {
    /// Create the entity. Called once at startup, by [`Host::init`].
    pub(crate) fn init(cx: &mut App) {
        let entity = cx.new(|_| BackupOps::default());
        cx.set_global(GlobalBackupOps(entity));
    }

    /// The entity, to observe and subscribe to.
    pub fn entity(cx: &App) -> Entity<BackupOps> {
        cx.global::<GlobalBackupOps>().0.clone()
    }

    /// The running backup or restore, if any.
    pub fn running(cx: &App) -> Option<Running> {
        Self::entity(cx).read(cx).running
    }

    pub(crate) fn emit(event: BackupEvent, cx: &mut App) {
        Self::entity(cx).update(cx, |_, cx| cx.emit(event));
    }

    pub(crate) fn set_running(running: Option<Running>, cx: &mut App) {
        Self::entity(cx).update(cx, |ops, cx| {
            ops.running = running;
            cx.notify();
        });
    }
}

/// Commit typed settings edits, so the flows read the credentials as shown.
fn flush_settings(cx: &mut App) -> Result<(), String> {
    config_ops::flush_config(cx)
        .map(|_| ())
        .map_err(|err| format!("Could not save the settings: {err}"))
}

/// Run `work` on tokio with a progress callback, showing each stage as
/// `stage(..)` on [`BackupOps`] until it finishes. Returns its output on the
/// foreground; [`BackupOps::running`] is cleared before the task resolves.
fn run_with_progress<S, T, F, Fut>(
    first: Running,
    stage: fn(S) -> Running,
    work: F,
    cx: &mut App,
) -> Task<Result<T, String>>
where
    S: Send + 'static,
    T: Send + 'static,
    F: FnOnce(Box<dyn Fn(S) + Send + Sync>) -> Fut,
    Fut: std::future::Future<Output = T> + Send + 'static,
{
    BackupOps::set_running(Some(first), cx);
    let (sender, mut stages) = tokio::sync::mpsc::unbounded_channel();
    let progress: Box<dyn Fn(S) + Send + Sync> = Box::new(move |stage| {
        let _ = sender.send(stage);
    });
    // The callback (and with it the sender) is dropped when `work` is done,
    // which ends the stage loop below.
    let handle = runtime::spawn(work(progress));
    cx.spawn(async move |cx| {
        while let Some(next) = stages.recv().await {
            cx.update(|cx| BackupOps::set_running(Some(stage(next)), cx));
        }
        let output = handle.await.map_err(|err| err.to_string());
        cx.update(|cx| BackupOps::set_running(None, cx));
        output
    })
}

/// Back up the open cave with its saved credentials and cached key. The
/// outcome is announced as [`BackupEvent::BackedUp`].
pub fn backup_now(cx: &mut App) {
    if let Err(err) = flush_settings(cx) {
        BackupOps::emit(BackupEvent::BackedUp(Err(err)), cx);
        return;
    }
    let core = Host::core(cx);
    let ticket = match core.backup_guard().start() {
        Ok(ticket) => ticket,
        Err(err) => {
            BackupOps::emit(BackupEvent::BackedUp(Err(err.to_string())), cx);
            return;
        }
    };
    let task = run_with_progress(
        Running::Backup(BackupStage::Packing),
        Running::Backup,
        move |progress| async move {
            let result = backup::backup_now(&core, &ticket, progress).await;
            drop(ticket);
            result
        },
        cx,
    );
    cx.spawn(async move |cx| {
        let result = match task.await {
            Ok(result) => result.map_err(|err| err.to_string()),
            Err(err) => Err(err),
        };
        // Not logged: a request error can quote a presigned URL, whose
        // signature is a credential. The Backup page shows it.
        cx.update(|cx| BackupOps::emit(BackupEvent::BackedUp(result), cx));
    })
    .detach();
}

/// Ask the workspace to run a restore ([`BackupEvent::RestoreRequested`]).
pub fn request_restore(request: RestoreRequest, cx: &mut App) {
    BackupOps::emit(BackupEvent::RestoreRequested(request), cx);
}

/// Announce a restore that could not start or failed.
pub fn report_restore_failure(message: String, cx: &mut App) {
    BackupOps::emit(
        BackupEvent::RestoreFailed(RestoreFailed {
            message,
            needs_passphrase: false,
        }),
        cx,
    );
}

/// What the workspace does after a restore.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RestoreOutcome {
    /// The cave to open: the restored one, or the original again after an
    /// in-place restore failed past closing it.
    pub open: Option<PathBuf>,
    /// Why the restore failed (also announced as
    /// [`BackupEvent::RestoreFailed`]).
    pub error: Option<String>,
}

/// Run a restore. Failures are announced as [`BackupEvent::RestoreFailed`];
/// the caller opens [`RestoreOutcome::open`]. The caller writes pending
/// edits first: an in-place restore closes the cave before it swaps the
/// directories.
pub fn restore(request: RestoreRequest, cx: &mut App) -> Task<RestoreOutcome> {
    let fail = |message: String, cx: &mut App| {
        report_restore_failure(message.clone(), cx);
        Task::ready(RestoreOutcome {
            open: None,
            error: Some(message),
        })
    };
    if let Err(err) = flush_settings(cx) {
        return fail(err, cx);
    }
    let core = Host::core(cx);
    let ticket = match core.backup_guard().start() {
        Ok(ticket) => ticket,
        Err(err) => return fail(err.to_string(), cx),
    };
    let task = run_with_progress(
        Running::Restore(RestoreStage::Downloading),
        Running::Restore,
        move |progress| async move {
            let result = backup::restore_backup(&core, &ticket, request, progress).await;
            drop(ticket);
            result
        },
        cx,
    );
    cx.spawn(async move |cx| {
        let (outcome, failed) = match task.await {
            Ok(Ok(restored)) => (
                RestoreOutcome {
                    open: Some(restored.cave),
                    error: None,
                },
                None,
            ),
            Ok(Err(failure)) => {
                let failed = RestoreFailed {
                    needs_passphrase: matches!(failure.error, BackupError::PassphraseRequired),
                    message: failure.error.to_string(),
                };
                (
                    RestoreOutcome {
                        open: failure.reopen,
                        error: Some(failed.message.clone()),
                    },
                    Some(failed),
                )
            }
            Err(err) => (
                RestoreOutcome {
                    open: None,
                    error: Some(err.clone()),
                },
                Some(RestoreFailed {
                    message: err,
                    needs_passphrase: false,
                }),
            ),
        };
        // Not logged, like a failed backup.
        if let Some(failed) = failed {
            cx.update(|cx| BackupOps::emit(BackupEvent::RestoreFailed(failed), cx));
        }
        outcome
    })
}

/// The snapshots on the backend, with the open cave's credentials or the
/// given ones (disaster restore).
pub fn list(
    credentials: Option<BackupConfig>,
    cx: &mut App,
) -> Task<Result<Vec<BackupInfo>, String>> {
    if credentials.is_none() {
        if let Err(err) = flush_settings(cx) {
            return Task::ready(Err(err));
        }
    }
    let core = Host::core(cx);
    let handle = runtime::spawn(async move { backup::list_backups(&core, credentials).await });
    cx.spawn(async move |_| match handle.await {
        Ok(result) => result.map_err(|err| err.to_string()),
        Err(err) => Err(err.to_string()),
    })
}

/// Delete a snapshot with the open cave's credentials.
pub fn delete(info: &BackupInfo, cx: &mut App) -> Task<Result<(), String>> {
    if let Err(err) = flush_settings(cx) {
        return Task::ready(Err(err));
    }
    let core = Host::core(cx);
    let id = info.id;
    let handle = runtime::spawn(async move { backup::delete_backup(&core, id).await });
    cx.spawn(async move |_| match handle.await {
        Ok(result) => result.map_err(|err| err.to_string()),
        Err(err) => Err(err.to_string()),
    })
}

/// Derive and cache the open cave's backup key from `passphrase`.
pub fn set_passphrase(passphrase: String, cx: &mut App) -> Task<Result<(), String>> {
    let core = Host::core(cx);
    let handle =
        runtime::spawn(async move { backup::set_cave_passphrase(&core, passphrase).await });
    cx.spawn(async move |_| match handle.await {
        Ok(result) => result.map_err(|err| err.to_string()),
        Err(err) => Err(err.to_string()),
    })
}

/// Whether the open cave has a cached backup key.
pub fn has_key(cx: &App) -> bool {
    backup::cave_has_key(&Host::core(cx))
}

/// Whether the open cave's backend URL and API key are filled in.
pub fn is_configured(cx: &App) -> bool {
    Host::core(cx).lock_config().backup.is_configured()
}

/// A byte count for people.
pub fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

/// "2026-11-27 10:00 (my-cave, 1.2 MiB)": a snapshot in one line.
pub fn summary(info: &BackupInfo) -> String {
    format!(
        "{} ({}, {})",
        info.created_at.format("%Y-%m-%d %H:%M"),
        info.cave_name,
        format_size(info.size_bytes)
    )
}
