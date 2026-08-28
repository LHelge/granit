use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use granit_api::{BackupInfo, CreateBackupRequest};
use granit_types::{BackupConfig, BackupProgress, BackupStage};
use sha2::{Digest, Sha256};
use tauri::Emitter;

use super::AppState;
use crate::backup::{self, BackupApiClient, BackupError};

/// Guards against concurrent backups (double-clicking "Back up now").
/// Managed by the Tauri builder, like `UpdateCheckGuard`.
#[derive(Default)]
pub(crate) struct BackupGuard(AtomicBool);

fn emit_progress(app: &tauri::AppHandle, stage: BackupStage) {
    let _ = app.emit("backup:progress", &BackupProgress { stage });
}

fn cave_context(state: &AppState) -> Result<(PathBuf, String, BackupConfig), BackupError> {
    let cave_path = state.active_cave_path().ok_or(BackupError::NoCave)?;
    let cave_name = cave_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "cave".to_string());
    let config = state.lock_config().backup.clone();
    Ok((cave_path, cave_name, config))
}

fn join_err(err: tauri::Error) -> BackupError {
    BackupError::Io(err.to_string())
}

async fn run_backup(app: &tauri::AppHandle, state: &AppState) -> Result<BackupInfo, BackupError> {
    let (cave_path, cave_name, config) = cave_context(state)?;
    if !config.is_configured() {
        return Err(BackupError::NotConfigured);
    }
    let client = BackupApiClient::new(&config.backend_url, &config.api_key)?;
    let granit_dir = cave_path.join(".granit");
    // Check the key before the expensive packing step.
    if !backup::has_key_file(&granit_dir) {
        return Err(BackupError::PassphraseNotSet);
    }

    emit_progress(app, BackupStage::Packing);
    let packed = tauri::async_runtime::spawn_blocking(move || backup::pack_cave(&cave_path))
        .await
        .map_err(join_err)??;

    emit_progress(app, BackupStage::Encrypting);
    let (container, sha256_hex) = tauri::async_runtime::spawn_blocking(move || {
        let key = backup::load_key(&granit_dir)?;
        let container = backup::encrypt(&key, &packed)?;
        let sha256_hex = hex::encode(Sha256::digest(&container));
        Ok::<_, BackupError>((container, sha256_hex))
    })
    .await
    .map_err(join_err)??;

    emit_progress(app, BackupStage::Uploading);
    let created = client
        .create_backup(&CreateBackupRequest {
            cave_name,
            size_bytes: container.len() as u64,
            sha256_hex,
        })
        .await?;
    client.upload(&created.upload_url, container).await?;
    client.complete_backup(created.backup.id).await
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
    guard: tauri::State<'_, BackupGuard>,
) -> Result<BackupInfo, BackupError> {
    if guard.0.swap(true, Ordering::SeqCst) {
        return Err(BackupError::AlreadyRunning);
    }
    let result = run_backup(&app, &state).await;
    guard.0.store(false, Ordering::SeqCst);

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
    let cave_path = state.active_cave_path().ok_or(BackupError::NoCave)?;
    let granit_dir = cave_path.join(".granit");
    // Argon2id is deliberately slow — keep it off the async runtime.
    tauri::async_runtime::spawn_blocking(move || backup::set_passphrase(&granit_dir, &passphrase))
        .await
        .map_err(join_err)?
}

/// Whether the open cave has a cached backup key (i.e. a passphrase is set).
#[tauri::command]
pub(crate) fn has_backup_key(state: tauri::State<'_, AppState>) -> bool {
    state
        .active_cave_path()
        .map(|path| backup::has_key_file(&path.join(".granit")))
        .unwrap_or(false)
}

/// List all snapshots stored on the backend for the configured API key.
#[tauri::command]
pub(crate) async fn list_backups(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<BackupInfo>, BackupError> {
    let config = state.lock_config().backup.clone();
    if !config.is_configured() {
        return Err(BackupError::NotConfigured);
    }
    BackupApiClient::new(&config.backend_url, &config.api_key)?
        .list_backups()
        .await
}
