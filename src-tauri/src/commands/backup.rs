//! Backup commands: thin wrappers around `granit_core::backup::run`, which
//! reports progress through a callback; here it becomes the `backup:*` and
//! `restore:*` events. The core's `OperationGuard` (in `CoreState`) keeps
//! backups and restores from overlapping.

use granit_api::BackupInfo;
use granit_types::{
    AppConfig, BackupConfig, BackupProgress, BackupStage, RestoreProgress, RestoreStage,
    RestoreTarget,
};
use tauri::Emitter;
use uuid::Uuid;

use super::AppState;
use crate::backup::{self, BackupError, OperationTicket, RestoreRequest};

fn emit_progress(app: &tauri::AppHandle, stage: BackupStage) {
    let _ = app.emit("backup:progress", &BackupProgress { stage });
}

fn emit_restore_progress(app: &tauri::AppHandle, stage: RestoreStage) {
    let _ = app.emit("restore:progress", &RestoreProgress { stage });
}

/// Download, verify, decrypt, and unpack one snapshot, then open the
/// restored cave (or reopen the original after a failed in-place swap).
async fn run_restore(
    app: &tauri::AppHandle,
    state: &AppState,
    ticket: &OperationTicket,
    request: RestoreRequest,
) -> Result<AppConfig, BackupError> {
    let restored = backup::restore_backup(state, ticket, request, |stage| {
        emit_restore_progress(app, stage)
    })
    .await;
    match restored {
        Ok(restored) => super::open_cave_at(restored.cave, app, state)
            .map_err(|err| BackupError::OpenRestored(err.to_string())),
        Err(failure) => {
            // The swap failed with the original directory intact — reopen
            // it so the app is not left without a cave.
            if let Some(original) = failure.reopen {
                let _ = super::open_cave_at(original, app, state);
            }
            Err(failure.error)
        }
    }
}

/// Restore one snapshot into a new directory or over the current cave, then
/// open the restored cave.
///
/// Like `backup_now`, the invoke stays pending for the whole run and the
/// frontend follows along via `restore:progress` / `restore:done` /
/// `restore:error` events. With `credentials` set, works with no cave open
/// (disaster restore on a fresh machine).
#[tauri::command]
pub(crate) async fn restore_backup(
    id: Uuid,
    target: RestoreTarget,
    passphrase: Option<String>,
    credentials: Option<BackupConfig>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<AppConfig, BackupError> {
    let ticket = state.backup_guard().start()?;
    let request = RestoreRequest {
        id,
        target,
        passphrase,
        credentials,
    };
    let result = run_restore(&app, &state, &ticket, request).await;
    drop(ticket);
    match &result {
        Ok(config) => {
            let _ = app.emit("restore:done", &config.active_cave);
        }
        Err(err) => {
            let _ = app.emit("restore:error", err.to_string());
        }
    }
    result
}

/// Take a snapshot of the open cave and upload it to the backup backend.
///
/// The invoke stays pending for the whole run; the frontend follows along
/// via `backup:progress` / `backup:done` / `backup:error` events, mirroring
/// the agent stream commands.
#[tauri::command]
pub(crate) async fn backup_now(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<BackupInfo, BackupError> {
    let ticket = state.backup_guard().start()?;
    let result = backup::backup_now(&state, &ticket, |stage| emit_progress(&app, stage)).await;
    drop(ticket);

    match &result {
        Ok(info) => {
            let _ = app.emit("backup:done", info);
        }
        Err(err) => {
            let _ = app.emit("backup:error", err.to_string());
        }
    }
    result
}

/// Derive and cache the backup encryption key from a passphrase.
#[tauri::command]
pub(crate) async fn set_backup_passphrase(
    passphrase: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), BackupError> {
    backup::set_cave_passphrase(&state, passphrase).await
}

/// Whether the open cave has a cached backup key (i.e. a passphrase is set).
#[tauri::command]
pub(crate) fn has_backup_key(state: tauri::State<'_, AppState>) -> bool {
    backup::cave_has_key(&state)
}

/// Delete one snapshot from the backend (object store and record). Allowed
/// for pending snapshots too — that is how abandoned uploads get cleaned up.
#[tauri::command]
pub(crate) async fn delete_backup(
    id: Uuid,
    state: tauri::State<'_, AppState>,
) -> Result<(), BackupError> {
    backup::delete_backup(&state, id).await
}

/// List all snapshots stored on the backend. Uses the open cave's saved
/// config, or explicit `credentials` when restoring with no cave open.
#[tauri::command]
pub(crate) async fn list_backups(
    credentials: Option<BackupConfig>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<BackupInfo>, BackupError> {
    backup::list_backups(&state, credentials).await
}
