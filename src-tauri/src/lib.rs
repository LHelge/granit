mod config;

use std::path::PathBuf;
use std::sync::Mutex;

use config::{AgentConfig, AppConfig, ConfigError};

struct AppState {
    config: Mutex<AppConfig>,
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

    // Update recent caves and persist
    let mut config = state.config.lock().unwrap();
    *config = new_config;
    config.add_recent_cave(path);
    config.save_global()?;

    Ok(config.clone())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::ensure_global().expect("failed to initialize config");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            config: Mutex::new(config),
        })
        .invoke_handler(tauri::generate_handler![get_config, save_config, open_cave])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
