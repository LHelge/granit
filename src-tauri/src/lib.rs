mod cave;
mod config;

use std::path::PathBuf;
use std::sync::Mutex;

use cave::{CaveError, Note, NoteMeta};
use config::{AgentConfig, AppConfig, ConfigError};

struct AppState {
    config: Mutex<AppConfig>,
    cave_path: Mutex<Option<PathBuf>>,
}

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Result<AppConfig, ConfigError> {
    let config = state.config.lock().unwrap();
    Ok(config.clone())
}

#[tauri::command]
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

    // Update cave path
    *state.cave_path.lock().unwrap() = Some(path.clone());

    // Update recent caves and persist
    let mut config = state.config.lock().unwrap();
    *config = new_config;
    config.add_recent_cave(path);
    config.save_global()?;

    Ok(config.clone())
}

fn get_cave_path(state: &tauri::State<AppState>) -> Result<PathBuf, CaveError> {
    state
        .cave_path
        .lock()
        .unwrap()
        .clone()
        .ok_or(CaveError::NoCaveOpen)
}

#[tauri::command]
fn create_note(name: String, state: tauri::State<AppState>) -> Result<NoteMeta, CaveError> {
    let cave_path = get_cave_path(&state)?;
    cave::create_note(&cave_path, &name)
}

#[tauri::command]
fn list_notes(state: tauri::State<AppState>) -> Result<Vec<NoteMeta>, CaveError> {
    let cave_path = get_cave_path(&state)?;
    cave::list_notes(&cave_path)
}

#[tauri::command]
fn read_note(name: String, state: tauri::State<AppState>) -> Result<Note, CaveError> {
    let cave_path = get_cave_path(&state)?;
    cave::read_note(&cave_path, &name)
}

#[tauri::command]
fn save_note(
    name: String,
    content: String,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    let cave_path = get_cave_path(&state)?;
    cave::save_note(&cave_path, &name, &content)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::ensure_global().expect("failed to initialize config");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            config: Mutex::new(config),
            cave_path: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            open_cave,
            create_note,
            list_notes,
            read_note,
            save_note,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
