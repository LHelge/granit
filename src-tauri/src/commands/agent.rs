use super::AppState;
use crate::agent::{self, AgentError};
use granit_types::{
    AttachedNote, AGENT_STREAM_CHUNK, AGENT_STREAM_DONE, AGENT_STREAM_ERROR, AGENT_TOOL_CALL,
    CAVE_NOTES_CHANGED,
};
use tauri::Emitter;
use tracing::{debug, info, instrument, warn};

/// Emit a Tauri event and log any send failure at WARN level.
fn emit<S: serde::Serialize + Clone>(app: &tauri::AppHandle, event: &'static str, payload: S) {
    if let Err(e) = app.emit(event, payload) {
        warn!(event, error = %e, "failed to emit event");
    }
}

#[tauri::command]
#[instrument(skip_all, fields(msg_len = msg.len(), attached = attached_notes.len()))]
pub(crate) async fn send_message(
    msg: String,
    attached_notes: Vec<AttachedNote>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), AgentError> {
    use rig::completion::message::Message;

    state.ensure_agent()?;
    let generation = state.agent_generation();

    let (agent_clone, history) = {
        let guard = state.lock_agent();
        let a = guard.as_ref().ok_or(AgentError::NotInitialized)?;
        a.snapshot()
    };

    debug!(history_len = history.len(), "starting agent stream");

    let prompt = agent::build_agent_prompt(&msg, &attached_notes);
    let mut stream = agent_clone
        .stream_with_history(prompt.as_str(), history)
        .await?;

    let app_chunks = app.clone();
    let app_tools = app.clone();
    let response = stream
        .collect_with(
            move |text| emit(&app_chunks, AGENT_STREAM_CHUNK, text),
            move |item| match item {
                agent::AgentStreamItem::ToolCall(info) => {
                    info!(tool = %info.name, "agent tool call");
                    emit(&app_tools, AGENT_TOOL_CALL, &info);
                }
                agent::AgentStreamItem::ToolResult => {
                    emit(&app_tools, CAVE_NOTES_CHANGED, ());
                }
                _ => {}
            },
        )
        .await
        .inspect_err(|e| {
            warn!(error = %e, "agent stream failed");
            emit(&app, AGENT_STREAM_ERROR, e.to_string());
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

    emit(&app, AGENT_STREAM_DONE, ());
    emit(&app, CAVE_NOTES_CHANGED, ());
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
