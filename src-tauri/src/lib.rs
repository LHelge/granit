mod agent;
mod cave;
mod config;
mod markdown;

use std::path::PathBuf;
use std::sync::Mutex;

use agent::{Agent, AgentError};
use cave::{Cave, CaveError, Note, NoteMeta};
use config::{AgentConfig, AppConfig, ConfigError};
use granit_types::{AppConfig as IpcConfig, RenderedNote};

struct AppState {
    config: Mutex<AppConfig>,
    cave: Mutex<Option<Cave>>,
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
/// Save agent settings to the global config file.
/// Cave-level config overrides are loaded at cave-open time but are not
/// currently editable through the UI.
fn save_config(
    agent: AgentConfig,
    state: tauri::State<AppState>,
) -> Result<IpcConfig, ConfigError> {
    let mut config = state.lock_config()?;
    config.agent = agent;
    config.save_global()?;
    Ok(config.to_ipc())
}

#[tauri::command]
fn open_cave(path: PathBuf, state: tauri::State<AppState>) -> Result<IpcConfig, ConfigError> {
    // Ensure cave .granit/ dir and defaults exist
    AppConfig::ensure_cave(&path)?;

    // Reload config with cave overrides
    let new_config = AppConfig::load(Some(&path))?;

    // Open the cave
    *state.lock_cave().map_err(|_| ConfigError::Poisoned)? = Some(Cave::open(path.clone()));

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
    use rig::agent::{MultiTurnStreamItem, Text};
    use rig::completion::message::Message;
    use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
    use tauri::Emitter;

    // Build agent on first use.
    {
        let mut guard = state.lock_agent()?;
        if guard.is_none() {
            let config = state.config.lock().map_err(|_| AgentError::Poisoned)?;
            *guard = Some(Agent::from_config(&config.agent)?);
        }
    }

    // Clone the inner rig agent and snapshot history — no lock held across await.
    let (inner, history) = {
        let guard = state.lock_agent()?;
        let a = guard.as_ref().ok_or(AgentError::NotInitialized)?;
        (a.inner.clone(), a.history.clone())
    };

    let mut stream = inner
        .stream_prompt(msg.as_str())
        .with_history(history)
        .multi_turn(1)
        .await;

    let mut full_response = String::new();

    loop {
        match stream.next().await {
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                Text { text },
            )))) => {
                full_response.push_str(&text);
                let _ = app.emit("agent:stream-chunk", text);
            }
            Some(Ok(MultiTurnStreamItem::FinalResponse(_))) | None => break,
            Some(Err(e)) => {
                let _ = app.emit("agent:stream-error", e.to_string());
                return Err(AgentError::Stream(e.to_string()));
            }
            _ => {}
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::ensure_global().expect("failed to initialize config");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            config: Mutex::new(config),
            cave: Mutex::new(None),
            agent: Mutex::new(None),
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
