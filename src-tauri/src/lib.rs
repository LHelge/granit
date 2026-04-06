mod agent;
mod cave;
mod markdown;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use agent::{Agent, AgentError, SharedCave};
use cave::{Cave, CaveError, ContentMatch, Note, NoteMeta};
use granit_types::{
    AgentConfig, AppConfig, AppMetadata, FontConfig, ModelInfo, RenderedNote, SidebarConfig,
    TodoList,
};
use tauri_plugin_store::StoreExt;

const APP_STATE_STORE_PATH: &str = "app-state.json";
const ACTIVE_CAVE_STORE_KEY: &str = "active_cave";

#[derive(Debug, thiserror::Error)]
enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yml::Error),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl serde::Serialize for ConfigError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

fn load_persisted_active_cave<R: tauri::Runtime, M: tauri::Manager<R>>(
    manager: &M,
) -> Result<Option<PathBuf>, String> {
    let store = manager
        .store(APP_STATE_STORE_PATH)
        .map_err(|err| err.to_string())?;

    let Some(value) = store.get(ACTIVE_CAVE_STORE_KEY) else {
        return Ok(None);
    };

    let Some(path) = value.as_str() else {
        store.delete(ACTIVE_CAVE_STORE_KEY);
        store.save().map_err(|err| err.to_string())?;
        return Ok(None);
    };

    Ok(Some(PathBuf::from(path)))
}

fn persist_active_cave<R: tauri::Runtime, M: tauri::Manager<R>>(
    manager: &M,
    path: &Path,
) -> Result<(), String> {
    let store = manager
        .store(APP_STATE_STORE_PATH)
        .map_err(|err| err.to_string())?;
    store.set(ACTIVE_CAVE_STORE_KEY, path.to_string_lossy().into_owned());
    store.save().map_err(|err| err.to_string())
}

fn clear_persisted_active_cave<R: tauri::Runtime, M: tauri::Manager<R>>(
    manager: &M,
) -> Result<(), String> {
    let store = manager
        .store(APP_STATE_STORE_PATH)
        .map_err(|err| err.to_string())?;
    store.delete(ACTIVE_CAVE_STORE_KEY);
    store.save().map_err(|err| err.to_string())
}

fn restore_active_cave<R: tauri::Runtime, M: tauri::Manager<R>>(manager: &M) -> Result<(), String> {
    let Some(path) = load_persisted_active_cave(manager)? else {
        return Ok(());
    };

    if !path.is_dir() {
        clear_persisted_active_cave(manager)?;
        return Ok(());
    }

    match Cave::open(path) {
        Ok(cave) => {
            cave.ensure_config().map_err(|err| err.to_string())?;
            let config = cave.load_config().map_err(|err| err.to_string())?;
            let state = manager.state::<AppState>();
            *state.lock_config() = config;
            state.set_cave(Some(cave));
            Ok(())
        }
        Err(_) => {
            clear_persisted_active_cave(manager)?;
            Ok(())
        }
    }
}

struct AppState {
    config: Mutex<AppConfig>,
    cave: SharedCave,
    agent: Mutex<Option<Agent>>,
    agent_generation: AtomicU64,
}

impl AppState {
    fn lock_config(&self) -> std::sync::MutexGuard<'_, AppConfig> {
        self.config.lock().expect("config mutex poisoned")
    }

    fn lock_cave(&self) -> std::sync::MutexGuard<'_, Option<Cave>> {
        self.cave.lock().expect("cave mutex poisoned")
    }

    fn lock_agent(&self) -> std::sync::MutexGuard<'_, Option<Agent>> {
        self.agent.lock().expect("agent mutex poisoned")
    }

    /// Get the path of the currently open cave, if any.
    fn active_cave_path(&self) -> Option<PathBuf> {
        self.lock_cave().as_ref().map(|c| c.path().to_path_buf())
    }

    /// Replace the currently open cave.
    fn set_cave(&self, cave: Option<Cave>) {
        *self.lock_cave() = cave;
    }

    /// Reset the agent so it rebuilds with new config on next use.
    fn reset_agent(&self) {
        self.agent_generation.fetch_add(1, Ordering::Relaxed);
        *self.lock_agent() = None;
    }

    fn agent_generation(&self) -> u64 {
        self.agent_generation.load(Ordering::Relaxed)
    }

    /// Ensure the agent is initialized from current config.
    ///
    /// Lock ordering: config is read and dropped *before* agent is locked,
    /// so this can never deadlock with code that holds config and then acquires
    /// agent (there is none), or vice-versa.
    fn ensure_agent(&self) -> Result<(), AgentError> {
        if self.lock_agent().is_some() {
            return Ok(());
        }
        let agent_config = self.lock_config().agent.clone();
        *self.lock_agent() = Some(Agent::from_config(&agent_config, self.cave.clone())?);
        Ok(())
    }

    /// Build the IPC response for commands that return the current config state.
    fn ipc_response(&self, config: &AppConfig) -> AppConfig {
        let mut ipc = config.clone();
        ipc.active_cave = self
            .active_cave_path()
            .map(|p| p.to_string_lossy().into_owned());
        ipc
    }
}

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Result<AppConfig, ConfigError> {
    let config = state.lock_config();
    Ok(state.ipc_response(&config))
}

#[tauri::command]
fn get_app_metadata() -> AppMetadata {
    let git_commit_hash = option_env!("GRANIT_GIT_HASH").unwrap_or("unknown");

    AppMetadata {
        app_name: "Granit".to_string(),
        repo_url: "https://github.com/LHelge/granit".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_commit_hash: shorten_git_hash(git_commit_hash),
        git_dirty: option_env!("GRANIT_GIT_DIRTY").unwrap_or("false") == "true",
    }
}

fn shorten_git_hash(hash: &str) -> String {
    if hash == "unknown" {
        return hash.to_string();
    }

    hash.chars().take(8).collect()
}

#[tauri::command]
fn list_system_fonts() -> Vec<String> {
    let source = font_kit::source::SystemSource::new();
    let mut families = source.all_families().unwrap_or_default();
    families.sort();
    families.dedup();
    families
}

#[tauri::command]
/// Save settings to the current config storage.
fn save_config(
    agent: AgentConfig,
    markdown_font: FontConfig,
    reading_font: FontConfig,
    agent_font: FontConfig,
    daily_note_folder: String,
    theme: String,
    state: tauri::State<AppState>,
) -> Result<AppConfig, ConfigError> {
    agent.validate().map_err(ConfigError::Validation)?;

    let mut config = state.lock_config();
    let cave = state.lock_cave();
    let cave = cave
        .as_ref()
        .ok_or_else(|| ConfigError::Validation("No cave is currently open".to_string()))?;
    config.agent = agent;
    config.markdown_font = markdown_font;
    config.reading_font = reading_font;
    config.agent_font = agent_font;
    config.daily_note_folder = daily_note_folder;
    config.theme = theme;
    cave.save_config(&config)
        .map_err(|err| ConfigError::Validation(err.to_string()))?;
    // Reset the agent so it rebuilds with the new config on the next message.
    state.reset_agent();
    Ok(state.ipc_response(&config))
}

#[tauri::command]
fn save_sidebar_state(
    sidebar: SidebarConfig,
    agent_panel: SidebarConfig,
    state: tauri::State<AppState>,
) -> Result<(), ConfigError> {
    let mut config = state.lock_config();
    config.sidebar = sidebar;
    config.agent_panel = agent_panel;
    let cave = state.lock_cave();
    let Some(cave) = cave.as_ref() else {
        return Ok(());
    };
    cave.save_config(&config)
        .map_err(|err| ConfigError::Validation(err.to_string()))?;
    Ok(())
}

#[tauri::command]
fn open_cave(
    path: PathBuf,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<AppConfig, CaveError> {
    let cave = Cave::open(path.clone())?;
    cave.ensure_config()?;
    let config = cave.load_config()?;
    persist_active_cave(&app, &path).map_err(CaveError::Io)?;

    *state.lock_config() = config;
    state.set_cave(Some(cave));
    state.reset_agent();

    let config = state.lock_config();
    Ok(state.ipc_response(&config))
}

fn with_cave<F, T>(state: &tauri::State<AppState>, f: F) -> Result<T, CaveError>
where
    F: FnOnce(&Cave) -> Result<T, CaveError>,
{
    let guard = state.lock_cave();
    let cave = guard.as_ref().ok_or(CaveError::NoCaveOpen)?;
    f(cave)
}

fn with_cave_mut<F, T>(state: &tauri::State<AppState>, f: F) -> Result<T, CaveError>
where
    F: FnOnce(&mut Cave) -> Result<T, CaveError>,
{
    let mut guard = state.lock_cave();
    let cave = guard.as_mut().ok_or(CaveError::NoCaveOpen)?;
    f(cave)
}

#[tauri::command]
fn create_note(
    name: String,
    folder: Option<String>,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    with_cave_mut(&state, |cave| {
        cave.create_note(&name, folder.as_deref().map(std::path::Path::new))
    })
}

#[tauri::command]
fn create_folder(path: String, state: tauri::State<AppState>) -> Result<(), CaveError> {
    with_cave_mut(&state, |cave| {
        cave.create_folder(std::path::Path::new(&path))
    })
}

#[tauri::command]
fn delete_folder(path: String, state: tauri::State<AppState>) -> Result<(), CaveError> {
    with_cave_mut(&state, |cave| {
        cave.delete_folder(std::path::Path::new(&path))
    })
}

#[tauri::command]
fn move_note(
    name: String,
    destination: Option<String>,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    with_cave_mut(&state, |cave| {
        cave.move_note(&name, destination.as_deref().map(std::path::Path::new))
    })
}

#[tauri::command]
fn move_folder(
    source: String,
    destination: Option<String>,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    with_cave_mut(&state, |cave| {
        cave.move_folder(
            std::path::Path::new(&source),
            destination.as_deref().map(std::path::Path::new),
        )
    })
}

#[tauri::command]
fn list_notes(state: tauri::State<AppState>) -> Result<Vec<NoteMeta>, CaveError> {
    with_cave(&state, |cave| cave.list_notes())
}

#[tauri::command]
fn search_content(
    query: String,
    max_results: Option<usize>,
    state: tauri::State<AppState>,
) -> Result<Vec<ContentMatch>, CaveError> {
    with_cave(&state, |cave| cave.search_content(&query, max_results))
}

#[tauri::command]
fn list_folders(state: tauri::State<AppState>) -> Result<Vec<String>, CaveError> {
    with_cave(&state, |cave| cave.list_folders())
}

#[tauri::command]
fn read_note(name: String, state: tauri::State<AppState>) -> Result<Note, CaveError> {
    with_cave(&state, |cave| cave.read_note(&name))
}

#[tauri::command]
fn open_daily_note(state: tauri::State<AppState>) -> Result<Note, CaveError> {
    let folder = state.lock_config().daily_note_folder.clone();
    with_cave_mut(&state, |cave| cave.open_daily_note(&folder))
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
    with_cave_mut(&state, |cave| cave.rename_note(&old_name, &new_name))
}

#[tauri::command]
fn update_note(
    old_name: String,
    new_name: String,
    content: String,
    tags: Option<Vec<String>>,
    icon: Option<String>,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    with_cave_mut(&state, |cave| {
        cave.update_note(&old_name, &new_name, &content, tags, icon)
    })
}

#[tauri::command]
fn rename_folder(
    source: String,
    new_name: String,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    with_cave_mut(&state, |cave| {
        cave.rename_folder(std::path::Path::new(&source), &new_name)
    })
}

#[tauri::command]
fn delete_note(name: String, state: tauri::State<AppState>) -> Result<(), CaveError> {
    with_cave_mut(&state, |cave| cave.delete_note(&name))
}

#[tauri::command]
fn list_todos(state: tauri::State<AppState>) -> Result<TodoList, CaveError> {
    with_cave(&state, |cave| cave.list_todos())
}

#[tauri::command]
fn toggle_todo(
    slug: String,
    line: usize,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    use tauri::Emitter;
    with_cave(&state, |cave| cave.toggle_todo(&slug, line))?;
    let _ = app.emit("cave:notes-changed", ());
    Ok(())
}

#[tauri::command]
fn toggle_todo_by_index(
    slug: String,
    index: usize,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    use tauri::Emitter;
    with_cave(&state, |cave| cave.toggle_todo_by_index(&slug, index))?;
    let _ = app.emit("cave:notes-changed", ());
    Ok(())
}

#[tauri::command]
fn render_note(name: String, state: tauri::State<AppState>) -> Result<RenderedNote, CaveError> {
    with_cave(&state, |cave| {
        let raw = cave.read_note_raw(&name)?;
        Ok(markdown::render_note(&raw, &name, |s| cave.lookup_slug(s)))
    })
}

#[tauri::command]
fn render_markdown(content: String, state: tauri::State<AppState>) -> String {
    let guard = state.lock_cave();
    let cave = guard.as_ref();
    match cave {
        Some(cave) => markdown::render_markdown_with_links(&content, |s| cave.lookup_slug(s)),
        None => markdown::render_html(&content),
    }
}

#[tauri::command]
fn set_active_note(slug: Option<String>, state: tauri::State<AppState>) -> Result<(), CaveError> {
    with_cave_mut(&state, |cave| {
        cave.set_active_slug(slug);
        Ok(())
    })
}

#[tauri::command]
fn list_providers(
    state: tauri::State<AppState>,
) -> Result<Vec<granit_types::ProviderInfo>, ConfigError> {
    let config = state.lock_config();
    Ok(config
        .agent
        .providers
        .iter()
        .enumerate()
        .map(|(i, entry): (usize, _)| granit_types::ProviderInfo {
            index: i,
            display_name: entry.display_name(),
            provider_type: entry.provider.provider_type().to_string(),
        })
        .collect())
}

#[tauri::command]
fn select_provider(index: usize, state: tauri::State<AppState>) -> Result<AppConfig, ConfigError> {
    let mut config = state.lock_config();
    let cave = state.lock_cave();
    let cave = cave
        .as_ref()
        .ok_or_else(|| ConfigError::Validation("No cave is currently open".to_string()))?;
    if index >= config.agent.providers.len() {
        return Err(ConfigError::Validation(format!(
            "Provider index {index} out of range"
        )));
    }
    config.agent.selected_provider = index;
    config.agent.selected_model = None;
    cave.save_config(&config)
        .map_err(|err| ConfigError::Validation(err.to_string()))?;
    state.reset_agent();
    Ok(state.ipc_response(&config))
}

#[tauri::command]
async fn list_models(state: tauri::State<'_, AppState>) -> Result<Vec<ModelInfo>, AgentError> {
    let provider = {
        let config = state.lock_config();
        if config.agent.providers.is_empty() {
            return Err(AgentError::NoProviders);
        }
        let entry = config
            .agent
            .providers
            .get(config.agent.selected_provider)
            .ok_or(AgentError::ProviderIndexOutOfRange(
                config.agent.selected_provider,
            ))?;
        entry.provider.clone()
    };
    agent::list_models(&provider).await
}

#[tauri::command]
fn select_model(model_id: String, state: tauri::State<AppState>) -> Result<AppConfig, ConfigError> {
    let mut config = state.lock_config();
    let cave = state.lock_cave();
    let cave = cave
        .as_ref()
        .ok_or_else(|| ConfigError::Validation("No cave is currently open".to_string()))?;
    config.agent.selected_model = Some(model_id);
    cave.save_config(&config)
        .map_err(|err| ConfigError::Validation(err.to_string()))?;
    state.reset_agent();
    Ok(state.ipc_response(&config))
}

#[tauri::command]
async fn send_message(
    msg: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), AgentError> {
    use rig::completion::message::Message;
    use tauri::Emitter;

    state.ensure_agent()?;
    let generation = state.agent_generation();

    // Snapshot the agent's inner + history so no lock is held across await.
    let (agent_clone, history) = {
        let guard = state.lock_agent();
        let a = guard.as_ref().ok_or(AgentError::NotInitialized)?;
        a.snapshot()
    };

    let mut stream = agent_clone
        .stream_with_history(msg.as_str(), history)
        .await?;

    let app_handle = app.clone();
    let response = stream
        .collect_with(
            |text| {
                let _ = app.emit("agent:stream-chunk", text);
            },
            |item| match item {
                agent::AgentStreamItem::ToolCall(info) => {
                    let _ = app_handle.emit("agent:tool-call", &info);
                }
                agent::AgentStreamItem::ToolResult => {
                    let _ = app_handle.emit("cave:notes-changed", ());
                }
                _ => {}
            },
        )
        .await
        .inspect_err(|e| {
            let _ = app.emit("agent:stream-error", e.to_string());
        })?;

    // Persist history (skip empty responses to avoid API rejection).
    {
        let mut guard = state.lock_agent();
        if state.agent_generation() == generation {
            if let Some(a) = guard.as_mut() {
                if !response.is_empty() {
                    a.push_history(Message::user(&msg));
                    a.push_history(Message::assistant(&response));
                }
            }
        }
    }

    let _ = app.emit("agent:stream-done", ());
    // Tools may have mutated the cave — tell the frontend to refresh.
    let _ = app.emit("cave:notes-changed", ());
    Ok(())
}

#[tauri::command]
fn clear_chat(state: tauri::State<'_, AppState>) -> Result<(), AgentError> {
    let mut guard = state.lock_agent();
    if let Some(agent) = guard.as_mut() {
        agent.clear_history();
    }
    Ok(())
}

#[tauri::command]
fn list_tools() -> Vec<granit_types::ToolInfo> {
    agent::tools::tool_info_list()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::default();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppState {
            config: Mutex::new(config),
            cave: Arc::new(Mutex::new(None)),
            agent: Mutex::new(None),
            agent_generation: AtomicU64::new(0),
        })
        .setup(|app| {
            restore_active_cave(app).map_err(std::io::Error::other)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_app_metadata,
            save_config,
            save_sidebar_state,
            list_system_fonts,
            open_cave,
            create_note,
            create_folder,
            delete_folder,
            move_note,
            move_folder,
            list_notes,
            search_content,
            list_folders,
            read_note,
            open_daily_note,
            save_note,
            delete_note,
            rename_note,
            rename_folder,
            update_note,
            render_note,
            render_markdown,
            set_active_note,
            list_todos,
            toggle_todo,
            toggle_todo_by_index,
            list_providers,
            select_provider,
            list_models,
            select_model,
            send_message,
            clear_chat,
            list_tools,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
