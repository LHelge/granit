mod agent;
mod cave;
mod config;
mod markdown;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use agent::{Agent, AgentError, SharedCave};
use cave::{Cave, CaveError, Note, NoteMeta};
use config::{AgentConfig, AppConfig, ConfigError, SidebarConfig};
use granit_types::{AppConfig as IpcConfig, FontConfig, ModelInfo, RenderedNote};

struct AppState {
    config: Mutex<AppConfig>,
    cave: SharedCave,
    agent: Mutex<Option<Agent>>,
}

impl AppState {
    fn lock_config(&self) -> Result<std::sync::MutexGuard<'_, AppConfig>, ConfigError> {
        self.config.lock().map_err(|_| ConfigError::Poisoned)
    }

    fn lock_cave(&self) -> Result<std::sync::MutexGuard<'_, Option<Cave>>, CaveError> {
        self.cave.lock().map_err(|_| CaveError::Poisoned)
    }

    fn lock_agent(&self) -> Result<std::sync::MutexGuard<'_, Option<Agent>>, AgentError> {
        self.agent.lock().map_err(|_| AgentError::Poisoned)
    }

    /// Get the path of the currently open cave, if any.
    fn active_cave_path(&self) -> Result<Option<PathBuf>, ConfigError> {
        Ok(self
            .cave
            .lock()
            .map_err(|_| ConfigError::Poisoned)?
            .as_ref()
            .map(|c| c.path().to_path_buf()))
    }

    /// Replace the currently open cave.
    fn set_cave(&self, cave: Option<Cave>) -> Result<(), ConfigError> {
        *self.cave.lock().map_err(|_| ConfigError::Poisoned)? = cave;
        Ok(())
    }

    /// Reset the agent so it rebuilds with new config on next use.
    fn reset_agent(&self) -> Result<(), ConfigError> {
        *self.agent.lock().map_err(|_| ConfigError::Poisoned)? = None;
        Ok(())
    }

    /// Ensure the agent is initialized from current config.
    fn ensure_agent(&self) -> Result<(), AgentError> {
        let mut guard = self.lock_agent()?;
        if guard.is_none() {
            let config = self.config.lock().map_err(|_| AgentError::Poisoned)?;
            *guard = Some(Agent::from_config(&config.agent, self.cave.clone())?);
        }
        Ok(())
    }
}

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Result<IpcConfig, ConfigError> {
    let config = state.lock_config()?;
    let mut ipc = config.to_ipc();
    ipc.active_cave = state
        .active_cave_path()?
        .map(|p| p.to_string_lossy().into_owned());
    Ok(ipc)
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
/// Save settings to the global config file.
/// Cave-level config overrides are loaded at cave-open time but are not
/// currently editable through the UI.
fn save_config(
    agent: AgentConfig,
    markdown_font: FontConfig,
    reading_font: FontConfig,
    agent_font: FontConfig,
    state: tauri::State<AppState>,
) -> Result<IpcConfig, ConfigError> {
    let mut config = state.lock_config()?;
    config.agent = agent;
    config.markdown_font = markdown_font;
    config.reading_font = reading_font;
    config.agent_font = agent_font;
    config.save_global()?;
    // Reset the agent so it rebuilds with the new config on the next message.
    state.reset_agent()?;
    let mut ipc = config.to_ipc();
    ipc.active_cave = state
        .active_cave_path()?
        .map(|p| p.to_string_lossy().into_owned());
    Ok(ipc)
}

#[tauri::command]
fn save_sidebar_state(
    sidebar: SidebarConfig,
    agent_panel: SidebarConfig,
    state: tauri::State<AppState>,
) -> Result<(), ConfigError> {
    let mut config = state.lock_config()?;
    config.sidebar = sidebar;
    config.agent_panel = agent_panel;
    config.save_global()?;
    Ok(())
}

#[tauri::command]
fn open_cave(path: PathBuf, state: tauri::State<AppState>) -> Result<IpcConfig, ConfigError> {
    // Ensure cave .granit/ dir and defaults exist
    AppConfig::ensure_cave(&path)?;

    // Persist the recent-caves update to the global config BEFORE loading
    // the merged config, so cave overrides don't bleed into the global file.
    AppConfig::save_recent_cave(&path)?;

    // Reload config with cave overrides
    let new_config = AppConfig::load(Some(&path))?;

    // Open the cave
    state.set_cave(Some(Cave::open(path.clone())?))?;

    // Reset agent so it rebuilds with new config on next message
    state.reset_agent()?;

    // Update in-memory config with the merged (global + cave) values
    let mut config = state.lock_config()?;
    *config = new_config;

    // Return config with active_cave set to the just-opened path
    let mut ipc = config.to_ipc();
    ipc.active_cave = Some(path.to_string_lossy().into_owned());
    Ok(ipc)
}

fn with_cave<F, T>(state: &tauri::State<AppState>, f: F) -> Result<T, CaveError>
where
    F: FnOnce(&Cave) -> Result<T, CaveError>,
{
    let guard = state.lock_cave()?;
    let cave = guard.as_ref().ok_or(CaveError::NoCaveOpen)?;
    f(cave)
}

fn with_cave_mut<F, T>(state: &tauri::State<AppState>, f: F) -> Result<T, CaveError>
where
    F: FnOnce(&mut Cave) -> Result<T, CaveError>,
{
    let mut guard = state.lock_cave()?;
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
fn list_folders(state: tauri::State<AppState>) -> Result<Vec<String>, CaveError> {
    with_cave(&state, |cave| cave.list_folders())
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
    with_cave_mut(&state, |cave| cave.rename_note(&old_name, &new_name))
}

#[tauri::command]
fn update_note(
    old_name: String,
    new_name: String,
    content: String,
    tags: Option<Vec<String>>,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    with_cave_mut(&state, |cave| {
        cave.update_note(&old_name, &new_name, &content, tags)
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
fn render_note(name: String, state: tauri::State<AppState>) -> Result<RenderedNote, CaveError> {
    with_cave(&state, |cave| {
        let raw = cave.read_note_raw(&name)?;
        Ok(markdown::render_note(&raw, &name, |s| cave.lookup_slug(s)))
    })
}

#[tauri::command]
fn render_markdown(content: String, state: tauri::State<AppState>) -> String {
    let guard = state.lock_cave().ok();
    let cave = guard.as_ref().and_then(|g| g.as_ref());
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
    let config = state.lock_config()?;
    Ok(config
        .agent
        .providers
        .iter()
        .enumerate()
        .map(|(i, entry)| granit_types::ProviderInfo {
            index: i,
            display_name: entry.display_name(),
            provider_type: entry.provider.provider_type().to_string(),
        })
        .collect())
}

#[tauri::command]
fn select_provider(index: usize, state: tauri::State<AppState>) -> Result<IpcConfig, ConfigError> {
    let mut config = state.lock_config()?;
    if index >= config.agent.providers.len() {
        return Err(ConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Provider index {index} out of range"),
        )));
    }
    config.agent.selected_provider = index;
    config.agent.selected_model = None;
    config.save_global()?;
    state.reset_agent()?;
    let mut ipc = config.to_ipc();
    ipc.active_cave = state
        .active_cave_path()?
        .map(|p| p.to_string_lossy().into_owned());
    Ok(ipc)
}

#[tauri::command]
async fn list_models(state: tauri::State<'_, AppState>) -> Result<Vec<ModelInfo>, AgentError> {
    let provider = {
        let config = state.config.lock().map_err(|_| AgentError::Poisoned)?;
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
fn select_model(model_id: String, state: tauri::State<AppState>) -> Result<IpcConfig, ConfigError> {
    let mut config = state.lock_config()?;
    config.agent.selected_model = Some(model_id);
    config.save_global()?;
    state.reset_agent()?;
    let mut ipc = config.to_ipc();
    ipc.active_cave = state
        .active_cave_path()?
        .map(|p| p.to_string_lossy().into_owned());
    Ok(ipc)
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

    // Snapshot the agent's inner + history so no lock is held across await.
    let (agent_clone, history) = {
        let guard = state.lock_agent()?;
        let a = guard.as_ref().ok_or(AgentError::NotInitialized)?;
        a.snapshot()
    };

    let mut stream = agent_clone
        .stream_with_history(msg.as_str(), history, 10)
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
        let mut guard = state.lock_agent()?;
        if let Some(a) = guard.as_mut() {
            if !response.is_empty() {
                a.push_history(Message::user(&msg));
                a.push_history(Message::assistant(&response));
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
    let mut guard = state.lock_agent()?;
    if let Some(agent) = guard.as_mut() {
        agent.clear_history();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::ensure_global().expect("failed to initialize config");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            config: Mutex::new(config),
            cave: Arc::new(Mutex::new(None)),
            agent: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
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
            list_folders,
            read_note,
            save_note,
            delete_note,
            rename_note,
            rename_folder,
            update_note,
            render_note,
            render_markdown,
            set_active_note,
            list_providers,
            select_provider,
            list_models,
            select_model,
            send_message,
            clear_chat,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
