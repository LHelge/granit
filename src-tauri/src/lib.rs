mod cave;
mod config;

use std::path::PathBuf;
use std::sync::Mutex;

use cave::{Cave, CaveError, Note, NoteMeta};
use config::{AgentConfig, AppConfig, ConfigError};

struct AppState {
    config: Mutex<AppConfig>,
    cave: Mutex<Option<Cave>>,
}

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Result<AppConfig, ConfigError> {
    let mut config = state.config.lock().unwrap().clone();
    config.active_cave = state
        .cave
        .lock()
        .unwrap()
        .as_ref()
        .map(|c| c.path().to_path_buf());
    Ok(config)
}

#[tauri::command]
/// Save agent settings to the global config file.
/// Cave-level config overrides are loaded at cave-open time but are not
/// currently editable through the UI.
fn save_config(
    agent: AgentConfig,
    state: tauri::State<AppState>,
) -> Result<AppConfig, ConfigError> {
    let mut config = state.config.lock().unwrap();
    config.agent = agent;
    config.save_global()?;
    Ok(config.clone())
}

#[tauri::command]
fn open_cave(path: PathBuf, state: tauri::State<AppState>) -> Result<AppConfig, ConfigError> {
    // Ensure cave .granit/ dir and defaults exist
    AppConfig::ensure_cave(&path)?;

    // Reload config with cave overrides
    let new_config = AppConfig::load(Some(&path))?;

    // Open the cave
    *state.cave.lock().unwrap() = Some(Cave::open(path.clone()));

    // Update recent caves and persist
    let mut config = state.config.lock().unwrap();
    *config = new_config;
    config.add_recent_cave(path.clone());
    config.save_global()?;

    // Return config with active_cave set to the just-opened path
    let mut response = config.clone();
    response.active_cave = Some(path);
    Ok(response)
}

fn with_cave<F, T>(state: &tauri::State<AppState>, f: F) -> Result<T, CaveError>
where
    F: FnOnce(&Cave) -> Result<T, CaveError>,
{
    let guard = state.cave.lock().unwrap();
    let cave = guard.as_ref().ok_or(CaveError::NoCaveOpen)?;
    f(cave)
}

#[tauri::command]
fn create_note(name: String, state: tauri::State<AppState>) -> Result<NoteMeta, CaveError> {
    with_cave(&state, |cave| cave.create_note(&name))
}

#[tauri::command]
fn list_notes(state: tauri::State<AppState>) -> Result<Vec<NoteMeta>, CaveError> {
    with_cave(&state, |cave| cave.list_notes())
}

#[tauri::command]
fn read_note(name: String, state: tauri::State<AppState>) -> Result<Note, CaveError> {
    with_cave(&state, |cave| cave.read_note(&name))
}

#[tauri::command]
fn save_note(
    name: String,
    content: String,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    with_cave(&state, |cave| cave.save_note(&name, &content))
}

#[tauri::command]
fn rename_note(
    old_name: String,
    new_name: String,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    with_cave(&state, |cave| cave.rename_note(&old_name, &new_name))
}

#[tauri::command]
fn update_note(
    old_name: String,
    new_name: String,
    content: String,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    with_cave(&state, |cave| {
        cave.update_note(&old_name, &new_name, &content)
    })
}

#[tauri::command]
fn delete_note(name: String, state: tauri::State<AppState>) -> Result<(), CaveError> {
    with_cave(&state, |cave| cave.delete_note(&name))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::ensure_global().expect("failed to initialize config");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            config: Mutex::new(config),
            cave: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            open_cave,
            create_note,
            list_notes,
            read_note,
            save_note,
            delete_note,
            rename_note,
            update_note,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
