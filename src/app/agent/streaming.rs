use super::messages::DisplayItem;
use crate::app::ipc;
use granit_types::ChatMessage;
use leptos::{prelude::*, task::spawn_local};

/// Register Tauri event listeners for agent streaming.
///
/// The EventHandle values returned by listen_* contain JS closures that
/// unsubscribe on drop. We need them alive for the component's lifetime:
///
///  1. `spawn_local` (leptos::task) ties the future to the current
///     reactive owner, which is the Effect's owner (this component).
///  2. `_handles` lives inside that future's state.
///  3. `std::future::pending().await` suspends forever, preventing the
///     async block from completing and dropping `_handles`.
///  4. On component unmount the reactive owner is cleaned up, which
///     cancels the future, drops `_handles`, and calls the JS unlisten
///     functions.
///
/// This is a deliberate pattern — not a forgotten await.
pub(super) fn setup_stream_listeners(
    messages: RwSignal<Vec<DisplayItem>>,
    streaming_content: RwSignal<String>,
    is_streaming: RwSignal<bool>,
    stream_error: RwSignal<Option<String>>,
) {
    Effect::new(move |_| {
        spawn_local(async move {
            let mut _handles = Vec::new();

            // chunk → append to streaming_content
            if let Some(h) = ipc::listen_stream_chunk(move |text| {
                streaming_content.update(|s| s.push_str(&text));
            })
            .await
            {
                _handles.push(h);
            }

            // done → render markdown, then move into messages
            if let Some(h) = ipc::listen_stream_done(move || {
                let content = streaming_content.get_untracked();
                if !content.is_empty() {
                    spawn_local(async move {
                        let html = ipc::render_markdown(&content).await.ok();
                        messages.update(|m| {
                            m.push(DisplayItem::message(ChatMessage::assistant(content), html))
                        });
                        streaming_content.set(String::new());
                        is_streaming.set(false);
                    });
                } else {
                    is_streaming.set(false);
                }
            })
            .await
            {
                _handles.push(h);
            }

            // error
            if let Some(h) = ipc::listen_stream_error(move |err| {
                stream_error.set(Some(err));
                streaming_content.set(String::new());
                is_streaming.set(false);
            })
            .await
            {
                _handles.push(h);
            }

            // tool-call → show inline in timeline
            if let Some(h) = ipc::listen_tool_call(move |info| {
                messages.update(|m| m.push(DisplayItem::tool_call(info)));
            })
            .await
            {
                _handles.push(h);
            }

            // Suspend forever — see doc comment above for why this is intentional.
            std::future::pending::<()>().await;
        });
    });
}
