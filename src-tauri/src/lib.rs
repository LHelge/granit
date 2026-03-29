mod agent;
mod cave;
mod config;
mod markdown;

use std::path::PathBuf;
use std::sync::Mutex;

use agent::{Agent, AgentError};
use cave::{Cave, CaveError, Note, NoteMeta};
use config::{AgentConfig, AppConfig, ConfigError, Secrets};
use granit_types::{AppConfig as IpcConfig, FontConfig, RenderedNote};

struct AppState {
    config: Mutex<AppConfig>,
    cave: Mutex<Option<Cave>>,
    agent: Mutex<Option<Agent>>,
    secrets: Mutex<Secrets>,
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

    fn lock_secrets(&self) -> Result<std::sync::MutexGuard<'_, Secrets>, ConfigError> {
        self.secrets.lock().map_err(|_| ConfigError::Poisoned)
    }
}

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Result<IpcConfig, ConfigError> {
    let config = state.lock_config()?;
    let active_cave = state
        .lock_cave()
        .map_err(|_| ConfigError::Poisoned)?
        .as_ref()
        .map(|c| c.path().to_string_lossy().into_owned());
    let mut ipc = config.to_ipc();
    ipc.active_cave = active_cave;
    Ok(ipc)
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
    *state.agent.lock().map_err(|_| ConfigError::Poisoned)? = None;
    let mut ipc = config.to_ipc();
    ipc.active_cave = state
        .lock_cave()
        .map_err(|_| ConfigError::Poisoned)?
        .as_ref()
        .map(|c| c.path().to_string_lossy().into_owned());
    Ok(ipc)
}

#[tauri::command]
fn open_cave(path: PathBuf, state: tauri::State<AppState>) -> Result<IpcConfig, ConfigError> {
    // Ensure cave .granit/ dir and defaults exist
    AppConfig::ensure_cave(&path)?;

    // Reload config with cave overrides
    let new_config = AppConfig::load(Some(&path))?;

    // Reload secrets with cave layer
    let new_secrets = config::load_secrets(Some(&path))?;
    *state.lock_secrets()? = new_secrets;

    // Open the cave
    *state.lock_cave().map_err(|_| ConfigError::Poisoned)? = Some(Cave::open(path.clone()));

    // Reset agent so it rebuilds with new config/secrets on next message
    *state.agent.lock().map_err(|_| ConfigError::Poisoned)? = None;

    // Update recent caves and persist
    let mut config = state.lock_config()?;
    *config = new_config;
    config.add_recent_cave(path.clone());
    config.save_global()?;

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
fn create_note(name: String, state: tauri::State<AppState>) -> Result<NoteMeta, CaveError> {
    with_cave_mut(&state, |cave| cave.create_note(&name))
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
    with_cave_mut(&state, |cave| cave.rename_note(&old_name, &new_name))
}

#[tauri::command]
fn update_note(
    old_name: String,
    new_name: String,
    content: String,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    with_cave_mut(&state, |cave| {
        cave.update_note(&old_name, &new_name, &content)
    })
}

#[tauri::command]
fn delete_note(name: String, state: tauri::State<AppState>) -> Result<(), CaveError> {
    with_cave_mut(&state, |cave| cave.delete_note(&name))
}

#[tauri::command]
fn render_note(name: String, state: tauri::State<AppState>) -> Result<RenderedNote, CaveError> {
    with_cave(&state, |cave| {
        let note = cave.read_note(&name)?;
        Ok(markdown::render_note(&note.content, &note.meta.slug, |s| {
            cave.lookup_slug(s)
        }))
    })
}

#[tauri::command]
fn render_markdown(content: String) -> String {
    markdown::render_html(&content)
}

#[tauri::command]
async fn send_message(
    msg: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), AgentError> {
    use futures::StreamExt;
    use rig::completion::message::Message;
    use tauri::Emitter;

    use agent::AgentStreamItem;

    // Build agent on first use.
    {
        let mut guard = state.lock_agent()?;
        if guard.is_none() {
            let config = state.config.lock().map_err(|_| AgentError::Poisoned)?;
            let secrets = state.secrets.lock().map_err(|_| AgentError::Poisoned)?;
            *guard = Some(Agent::from_config(&config.agent, &secrets)?);
        }
    }

    // Snapshot the agent's inner + history so no lock is held across await.
    let (agent_clone, history) = {
        let guard = state.lock_agent()?;
        let a = guard.as_ref().ok_or(AgentError::NotInitialized)?;
        (a.clone(), a.history.clone())
    };

    let mut stream = agent_clone
        .stream_with_history(msg.as_str(), history, 1)
        .await?;

    let mut full_response = String::new();

    loop {
        match stream.next().await {
            Some(Ok(AgentStreamItem::Text(text))) => {
                full_response.push_str(&text);
                let _ = app.emit("agent:stream-chunk", text);
            }
            Some(Ok(AgentStreamItem::Done)) | None => break,
            Some(Err(e)) => {
                let _ = app.emit("agent:stream-error", e.to_string());
                return Err(e);
            }
            Some(Ok(AgentStreamItem::Other)) => {}
        }
    }

    // Persist history.
    {
        let mut guard = state.lock_agent()?;
        if let Some(a) = guard.as_mut() {
            a.history.push(Message::user(&msg));
            a.history.push(Message::assistant(&full_response));
        }
    }

    let _ = app.emit("agent:stream-done", ());
    Ok(())
}

#[tauri::command]
fn get_secret(key: String, state: tauri::State<AppState>) -> Result<Option<bool>, ConfigError> {
    let secrets = state.lock_secrets()?;
    // Return whether the key is set, never the actual value
    Ok(secrets.get(&key).map(|_| true))
}

#[tauri::command]
fn set_secret(
    key: String,
    value: String,
    state: tauri::State<AppState>,
) -> Result<(), ConfigError> {
    config::validate_secret_value(&value)?;
    config::write_global_secret(&key, &value)?;
    // Reload secrets so the in-memory state is up to date
    let cave_path = state
        .lock_cave()
        .map_err(|_| ConfigError::Poisoned)?
        .as_ref()
        .map(|c| c.path().to_path_buf());
    let new_secrets = config::load_secrets(cave_path.as_deref())?;
    *state.lock_secrets()? = new_secrets;
    // Reset agent so it picks up the new secret
    *state.agent.lock().map_err(|_| ConfigError::Poisoned)? = None;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::ensure_global().expect("failed to initialize config");
    let secrets = config::load_secrets(None).expect("failed to load secrets");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            config: Mutex::new(config),
            cave: Mutex::new(None),
            agent: Mutex::new(None),
            secrets: Mutex::new(secrets),
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
            render_note,
            render_markdown,
            send_message,
            get_secret,
            set_secret,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
