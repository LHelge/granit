use super::AppState;
use crate::agent::{self, AgentError};
use granit_types::AttachedNote;

#[tauri::command]
pub(crate) async fn send_message(
    msg: String,
    attached_notes: Vec<AttachedNote>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), AgentError> {
    use rig::completion::message::Message;
    use tauri::Emitter;

    state.ensure_agent()?;
    let generation = state.agent_generation();

    let (agent_clone, history) = {
        let guard = state.lock_agent();
        let a = guard.as_ref().ok_or(AgentError::NotInitialized)?;
        a.snapshot()
    };

    let prompt = agent::build_agent_prompt(&msg, &attached_notes);
    let mut stream = agent_clone
        .stream_with_history(prompt.as_str(), history)
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
    let _ = app.emit("cave:notes-changed", ());
    Ok(())
}

#[tauri::command]
pub(crate) fn clear_chat(state: tauri::State<'_, AppState>) -> Result<(), AgentError> {
    let mut guard = state.lock_agent();
    if let Some(agent) = guard.as_mut() {
        agent.clear_history();
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn list_tools() -> Vec<granit_types::ToolInfo> {
    agent::tools::tool_info_list()
}
