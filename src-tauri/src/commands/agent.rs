use crate::agent::{self, AgentError};
use granit_types::AttachedNote;

use super::AppState;

fn build_agent_prompt(msg: &str, attached_notes: &[AttachedNote]) -> String {
    if attached_notes.is_empty() {
        return msg.to_string();
    }

    let attachment_size: usize = attached_notes
        .iter()
        .map(|note| {
            note.slug.len() + note.content.len() + note.selected.as_deref().map_or(0, str::len)
        })
        .sum();
    let mut prompt = String::with_capacity(msg.len() + attachment_size + 256);
    prompt.push_str(
        "Use the attached note contexts below for this turn. If the user refers to an attached note or selected text, use these attachments directly.\n\n",
    );
    prompt.push_str("<attached_notes>\n");

    for attached_note in attached_notes {
        prompt.push_str("<attached_note>\n");
        prompt.push_str("<slug>");
        prompt.push_str(&attached_note.slug);
        prompt.push_str("</slug>\n");

        if let Some(selected) = attached_note.selected.as_deref() {
            prompt.push_str("<selected_text>\n");
            prompt.push_str(selected);
            prompt.push_str("\n</selected_text>\n");
        }

        prompt.push_str("<content>\n");
        prompt.push_str(&attached_note.content);
        prompt.push_str("\n</content>\n");
        prompt.push_str("</attached_note>\n");
    }

    prompt.push_str("</attached_notes>\n\n");
    prompt.push_str("<user_request>\n");
    prompt.push_str(msg);
    prompt.push_str("\n</user_request>");

    prompt
}

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

    let prompt = build_agent_prompt(&msg, &attached_notes);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_agent_prompt_without_attachment() {
        let prompt = build_agent_prompt("Summarize this", &[]);

        assert_eq!(prompt, "Summarize this");
    }

    #[test]
    fn test_build_agent_prompt_with_attachments_and_selection() {
        let prompt = build_agent_prompt(
            "Summarize this",
            &[
                AttachedNote {
                    slug: "daily-note".into(),
                    content: "# Heading\n\nBody".into(),
                    selected: Some("Heading".into()),
                },
                AttachedNote {
                    slug: "shopping".into(),
                    content: "Milk\nEggs".into(),
                    selected: None,
                },
            ],
        );

        assert!(prompt.contains("Use the attached note contexts below for this turn"));
        assert!(prompt.contains("<attached_notes>"));
        assert!(prompt.contains("<slug>daily-note</slug>"));
        assert!(prompt.contains("<slug>shopping</slug>"));
        assert!(prompt.contains("<selected_text>\nHeading\n</selected_text>"));
        assert!(prompt.contains("<content>\n# Heading\n\nBody\n</content>"));
        assert!(prompt.contains("<content>\nMilk\nEggs\n</content>"));
        assert!(prompt.contains("<user_request>\nSummarize this\n</user_request>"));
    }
}
