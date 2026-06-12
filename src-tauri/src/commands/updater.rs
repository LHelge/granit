use super::cave::render_markdown_for_state;
use super::store::Store;
use super::AppState;
use granit_types::{ReleaseNotes, UpdateCheckStatus};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

/// Event emitted after an update has been downloaded and installed.
/// Payload: the new version string.
pub(crate) const UPDATE_INSTALLED_EVENT: &str = "updater:installed";

#[derive(Debug, thiserror::Error)]
pub(crate) enum UpdaterError {
    #[error("Update error: {0}")]
    Updater(#[from] tauri_plugin_updater::Error),

    #[error("An update check is already running")]
    CheckInProgress,

    #[error("Store error: {0}")]
    Store(String),
}

impl serde::Serialize for UpdaterError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// An update that has been installed but whose release notes have not yet
/// been shown. Persisted in the app-state store across the restart that
/// activates the new version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PendingUpdate {
    pub(crate) version: String,
    /// Raw markdown release notes from the update manifest.
    pub(crate) notes: Option<String>,
}

/// Guards against concurrent update checks (startup check racing the
/// manual "Check for updates" button). Managed by the Tauri builder.
#[derive(Default)]
pub(crate) struct UpdateCheckGuard(AtomicBool);

/// What to do with a stored pending update at startup.
#[derive(Debug, PartialEq)]
enum PendingNotesAction {
    /// The running version is the one the pending update installed:
    /// show its release notes.
    Show(PendingUpdate),
    /// The stored version does not match the running version (e.g. the
    /// user updated manually past it): drop the stale entry silently.
    ClearStale,
    None,
}

fn pending_notes_action(
    pending: Option<PendingUpdate>,
    current_version: &str,
) -> PendingNotesAction {
    match pending {
        Some(pending) if pending.version == current_version => PendingNotesAction::Show(pending),
        Some(_) => PendingNotesAction::ClearStale,
        None => PendingNotesAction::None,
    }
}

/// Check for an update and install it if available. The new version takes
/// effect on the next launch; the release notes are persisted so that launch
/// can show them.
async fn check_and_install(app: &AppHandle) -> Result<UpdateCheckStatus, UpdaterError> {
    let guard = app.state::<UpdateCheckGuard>();
    if guard.0.swap(true, Ordering::SeqCst) {
        return Err(UpdaterError::CheckInProgress);
    }
    let result = do_check_and_install(app).await;
    guard.0.store(false, Ordering::SeqCst);
    result
}

async fn do_check_and_install(app: &AppHandle) -> Result<UpdateCheckStatus, UpdaterError> {
    let Some(update) = app.updater()?.check().await? else {
        return Ok(UpdateCheckStatus::UpToDate);
    };

    info!("downloading and installing update {}", update.version);
    update.download_and_install(|_, _| {}, || {}).await?;

    let pending = PendingUpdate {
        version: update.version.clone(),
        notes: update.body.clone(),
    };
    Store::new(app)
        .persist_pending_update(&pending)
        .map_err(UpdaterError::Store)?;

    let _ = app.emit(UPDATE_INSTALLED_EVENT, &update.version);
    info!("update {} installed, applies on restart", update.version);

    Ok(UpdateCheckStatus::Installed {
        version: update.version,
    })
}

/// Kick off the silent startup update check on a background task.
///
/// Skipped in debug builds: a dev binary has no meaningful version to
/// compare and must never be replaced by a release artifact. The manual
/// "Check for updates" command stays available for testing.
pub(crate) fn spawn_startup_update_check(app: &tauri::App) {
    if cfg!(debug_assertions) {
        return;
    }
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        match check_and_install(&handle).await {
            Ok(UpdateCheckStatus::UpToDate) => info!("startup update check: up to date"),
            Ok(UpdateCheckStatus::Installed { .. }) => {}
            Err(err) => warn!("startup update check failed: {err}"),
        }
    });
}

#[tauri::command]
pub(crate) fn get_pending_release_notes(
    app: AppHandle,
    state: tauri::State<AppState>,
) -> Result<Option<ReleaseNotes>, UpdaterError> {
    let store = Store::new(&app);
    let pending = store.load_pending_update().map_err(UpdaterError::Store)?;

    match pending_notes_action(pending, env!("CARGO_PKG_VERSION")) {
        PendingNotesAction::Show(pending) => {
            let notes = pending
                .notes
                .unwrap_or_else(|| "_No release notes._".to_string());
            Ok(Some(ReleaseNotes {
                version: pending.version,
                notes_html: render_markdown_for_state(state.inner(), &notes),
            }))
        }
        PendingNotesAction::ClearStale => {
            store.clear_pending_update().map_err(UpdaterError::Store)?;
            Ok(None)
        }
        PendingNotesAction::None => Ok(None),
    }
}

#[tauri::command]
pub(crate) fn acknowledge_release_notes(app: AppHandle) -> Result<(), UpdaterError> {
    Store::new(&app)
        .clear_pending_update()
        .map_err(UpdaterError::Store)
}

#[tauri::command]
pub(crate) async fn check_for_updates(app: AppHandle) -> Result<UpdateCheckStatus, UpdaterError> {
    check_and_install(&app).await
}

#[tauri::command]
pub(crate) fn restart_app(app: AppHandle) {
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(version: &str, notes: Option<&str>) -> PendingUpdate {
        PendingUpdate {
            version: version.to_string(),
            notes: notes.map(str::to_string),
        }
    }

    #[test]
    fn pending_notes_action_matching_version_shows_notes() {
        let action = pending_notes_action(Some(pending("1.2.3", Some("- fixes"))), "1.2.3");
        assert_eq!(
            action,
            PendingNotesAction::Show(pending("1.2.3", Some("- fixes")))
        );
    }

    #[test]
    fn pending_notes_action_stale_version_clears() {
        // The user manually installed a newer version than the one the
        // updater staged; its notes no longer describe the running app.
        let action = pending_notes_action(Some(pending("1.2.3", None)), "1.3.0");
        assert_eq!(action, PendingNotesAction::ClearStale);
    }

    #[test]
    fn pending_notes_action_nothing_stored_is_none() {
        assert_eq!(
            pending_notes_action(None, "1.2.3"),
            PendingNotesAction::None
        );
    }
}
