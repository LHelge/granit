use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use granit_api::{BackupInfo, CreateBackupRequest};
use granit_types::{
    AppConfig, BackupConfig, BackupProgress, BackupStage, RestoreProgress, RestoreStage,
    RestoreTarget,
};
use sha2::{Digest, Sha256};
use tauri::Emitter;
use uuid::Uuid;

use super::AppState;
use crate::backup::{self, BackupApiClient, BackupError};

/// Guards against concurrent backup operations: backup and restore are
/// mutually exclusive (a backup snapshotting a half-restored cave, or a
/// restore swapping directories mid-pack, would corrupt either result).
/// Managed by the Tauri builder, like `UpdateCheckGuard`.
#[derive(Default)]
pub(crate) struct OperationGuard(AtomicBool);

fn emit_progress(app: &tauri::AppHandle, stage: BackupStage) {
    let _ = app.emit("backup:progress", &BackupProgress { stage });
}

fn emit_restore_progress(app: &tauri::AppHandle, stage: RestoreStage) {
    let _ = app.emit("restore:progress", &RestoreProgress { stage });
}

/// The backend credentials to use: an explicit override (disaster restore
/// with no cave open) or the open cave's saved config.
fn resolve_credentials(
    state: &AppState,
    credentials: Option<BackupConfig>,
) -> Result<BackupConfig, BackupError> {
    let config = match credentials {
        Some(over) => over,
        None => state.lock_config().backup.clone(),
    };
    if !config.is_configured() {
        return Err(BackupError::NotConfigured);
    }
    Ok(config)
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
    client
        .upload(&created.upload_url, &created.upload_headers, container)
        .await?;
    client.complete_backup(created.backup.id).await
}

/// Download, verify, decrypt, and unpack one snapshot.
async fn run_restore(
    app: &tauri::AppHandle,
    state: &AppState,
    id: Uuid,
    target: RestoreTarget,
    passphrase: Option<String>,
    credentials: Option<BackupConfig>,
) -> Result<AppConfig, BackupError> {
    let config = resolve_credentials(state, credentials)?;
    let client = BackupApiClient::new(&config.backend_url, &config.api_key)?;
    // The in-place target needs an open cave before any work starts.
    let cave_path = state.active_cave_path();
    if matches!(target, RestoreTarget::CurrentCave) && cave_path.is_none() {
        return Err(BackupError::NoCave);
    }

    emit_restore_progress(app, RestoreStage::Downloading);
    // The download endpoint validates existence and state (404/409); the
    // list provides the recorded digest to verify the bytes against.
    let download = client.download_backup(id).await?;
    let record = client
        .list_backups()
        .await?
        .into_iter()
        .find(|backup| backup.id == id)
        .ok_or(BackupError::SnapshotNotFound)?;
    let container = client.download(&download.download_url).await?;

    emit_restore_progress(app, RestoreStage::Decrypting);
    let cached_key = cave_path
        .as_ref()
        .map(|path| path.join(".granit"))
        .filter(|granit_dir| backup::has_key_file(granit_dir));
    let plaintext = tauri::async_runtime::spawn_blocking(move || {
        let cached = cached_key
            .map(|granit_dir| backup::load_key(&granit_dir))
            .transpose()?;
        backup::verify_and_decrypt(
            &container,
            &record.sha256_hex,
            cached.as_ref(),
            passphrase.as_deref(),
        )
    })
    .await
    .map_err(join_err)??;

    emit_restore_progress(app, RestoreStage::Unpacking);
    let open_path = match target {
        RestoreTarget::NewDirectory { path } => {
            let target_dir = PathBuf::from(path);
            let unpacked_to = target_dir.clone();
            tauri::async_runtime::spawn_blocking(move || {
                backup::restore_to_new_dir(&plaintext, &unpacked_to)
            })
            .await
            .map_err(join_err)??;
            target_dir
        }
        RestoreTarget::CurrentCave => {
            let cave_path = cave_path.expect("checked above");
            // Close the cave before swapping directories underneath it, so
            // nothing writes into the renamed pre-restore copy.
            state.set_cave(None);
            state.reset_agent();
            let swap_root = cave_path.clone();
            let swapped = tauri::async_runtime::spawn_blocking(move || {
                backup::restore_in_place(&plaintext, &swap_root)
            })
            .await
            .map_err(join_err);
            match swapped {
                Ok(Ok(pre_restore)) => {
                    log::info!("pre-restore cave kept at {}", pre_restore.display());
                    cave_path
                }
                Ok(Err(err)) | Err(err) => {
                    // The swap failed with the original directory intact —
                    // reopen it so the app is not left without a cave.
                    let _ = super::open_cave_at(cave_path, app, state);
                    return Err(err);
                }
            }
        }
    };

    super::open_cave_at(open_path, app, state)
        .map_err(|err| BackupError::OpenRestored(err.to_string()))
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
    guard: tauri::State<'_, OperationGuard>,
) -> Result<AppConfig, BackupError> {
    if guard.0.swap(true, Ordering::SeqCst) {
        return Err(BackupError::AlreadyRunning);
    }
    let result = run_restore(&app, &state, id, target, passphrase, credentials).await;
    guard.0.store(false, Ordering::SeqCst);

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
    guard: tauri::State<'_, OperationGuard>,
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

/// Delete one snapshot from the backend (object store and record). Allowed
/// for pending snapshots too — that is how abandoned uploads get cleaned up.
#[tauri::command]
pub(crate) async fn delete_backup(
    id: Uuid,
    state: tauri::State<'_, AppState>,
) -> Result<(), BackupError> {
    let config = resolve_credentials(&state, None)?;
    BackupApiClient::new(&config.backend_url, &config.api_key)?
        .delete_backup(id)
        .await
}

/// List all snapshots stored on the backend. Uses the open cave's saved
/// config, or explicit `credentials` when restoring with no cave open.
#[tauri::command]
pub(crate) async fn list_backups(
    credentials: Option<BackupConfig>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<BackupInfo>, BackupError> {
    let config = resolve_credentials(&state, credentials)?;
    BackupApiClient::new(&config.backend_url, &config.api_key)?
        .list_backups()
        .await
}
