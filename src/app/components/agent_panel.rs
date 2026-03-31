use crate::app::ipc;
use crate::app::AppCtx;
use granit_types::{ChatMessage, ChatRole};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Clone)]
struct DisplayMessage {
    message: ChatMessage,
    rendered_html: Option<String>,
}

#[component]
pub fn AgentPanel() -> impl IntoView {
    let config = expect_context::<AppCtx>().config;
    let (input, set_input) = signal(String::new());
    let messages: RwSignal<Vec<DisplayMessage>> = RwSignal::new(Vec::new());
    let streaming_content: RwSignal<String> = RwSignal::new(String::new());
    let is_streaming: RwSignal<bool> = RwSignal::new(false);
    let stream_error: RwSignal<Option<String>> = RwSignal::new(None);

    // Track the agent identity (provider + model). When it changes (e.g.
    // after saving settings), clear the chat so stale history from a
    // different provider doesn't confuse the user.
    let agent_identity = Memo::new(move |_| {
        let cfg = config.get();
        (cfg.agent.provider.clone(), cfg.agent.model.clone())
    });
    let prev_identity: RwSignal<Option<(String, String)>> = RwSignal::new(None);
    Effect::new(move |_| {
        let current = agent_identity.get();
        let prev = prev_identity.get_untracked();
        if let Some(prev) = prev {
            if prev != current {
                messages.set(Vec::new());
                streaming_content.set(String::new());
                stream_error.set(None);
            }
        }
        prev_identity.set(Some(current));
    });

    // Register Tauri event listeners once on mount.
    //
    // The EventHandle values returned by listen_* contain JS closures that
    // unsubscribe on drop. We need them alive for the component's lifetime:
    //
    //  1. `spawn_local` (leptos::task) ties the future to the current
    //     reactive owner, which is the Effect's owner (this component).
    //  2. `_handles` lives inside that future's state.
    //  3. `std::future::pending().await` suspends forever, preventing the
    //     async block from completing and dropping `_handles`.
    //  4. On component unmount the reactive owner is cleaned up, which
    //     cancels the future, drops `_handles`, and calls the JS unlisten
    //     functions.
    //
    // This is a deliberate pattern — not a forgotten await.
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
                        messages.update(|m| m.push(DisplayMessage {
                            message: ChatMessage::assistant(content),
                            rendered_html: html,
                        }));
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

            // Suspend forever — see comment above for why this is intentional.
            std::future::pending::<()>().await;
        });
    });

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let msg = input.get_untracked();
        if msg.trim().is_empty() || is_streaming.get_untracked() {
            return;
        }
        set_input.set(String::new());
        stream_error.set(None);
        messages.update(|m| m.push(DisplayMessage {
            message: ChatMessage::user(msg.clone()),
            rendered_html: None,
        }));
        is_streaming.set(true);

        spawn_local(async move {
            if let Err(e) = ipc::send_message(&msg).await {
                stream_error.set(Some(e));
                is_streaming.set(false);
            }
        });
    };

    view! {
        <aside class="w-80 shrink-0 bg-stone-850 border-l border-stone-700 flex flex-col overflow-hidden">
            // Header — provider and model
            <div class="px-3 py-1.5 border-b border-stone-700 text-xs text-stone-500 truncate">
                {move || {
                    let (provider, model) = agent_identity.get();
                    format!("{provider} / {model}")
                }}
            </div>
            // Message list
            <div
                class="flex-1 overflow-y-auto p-3 space-y-3 flex flex-col"
                style:font-family=move || config.get().agent_font.font_family
                style:font-size=move || format!("{}px", config.get().agent_font.font_size)
            >
                // Empty state
                <Show when=move || messages.get().is_empty() && !is_streaming.get() && streaming_content.get().is_empty()>
                    <p class="text-stone-500 italic text-center mt-8">"Ask me anything about your notes..."</p>
                </Show>

                // Committed messages
                <For
                    each=move || messages.get()
                    key=|dm| format!("{:?}{}", dm.message.role, dm.message.content.len())
                    children=|dm| {
                        let is_user = dm.message.role == ChatRole::User;
                        let bubble_class = if is_user {
                            "max-w-[85%] px-3 py-2 rounded-lg bg-stone-600 text-stone-100 whitespace-pre-wrap break-words"
                        } else {
                            "max-w-[85%] px-3 py-2 rounded-lg bg-stone-800 text-stone-200 prose prose-sm prose-invert max-w-none"
                        };
                        let bubble_style = if is_user { "" } else { "font-size: inherit" };
                        let wrapper_class = if is_user { "flex justify-end" } else { "flex justify-start" };
                        view! {
                            <div class=wrapper_class>
                                {if let Some(rendered) = dm.rendered_html {
                                    view! {
                                        <div
                                            class=bubble_class
                                            style=bubble_style
                                            inner_html=rendered
                                        />
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class=bubble_class>
                                            {dm.message.content.clone()}
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                        }
                    }
                />

                // Streaming response in progress
                <Show when=move || is_streaming.get() || !streaming_content.get().is_empty()>
                    <div class="flex justify-start">
                        <div class="max-w-[85%] px-3 py-2 rounded-lg bg-stone-750 text-stone-200 whitespace-pre-wrap break-words">
                            {move || {
                                let content = streaming_content.get();
                                if content.is_empty() {
                                    view! { <span class="inline-block w-2 h-4 bg-stone-400 animate-pulse rounded-sm" /> }.into_any()
                                } else {
                                    view! { <span>{content}<span class="inline-block ml-0.5 w-1.5 h-3.5 bg-stone-400 animate-pulse rounded-sm align-middle" /></span> }.into_any()
                                }
                            }}
                        </div>
                    </div>
                </Show>

                // Error
                <Show when=move || stream_error.get().is_some()>
                    <div class="px-3 py-2 rounded-lg bg-red-900/40 border border-red-700 text-red-300">
                        {move || stream_error.get().unwrap_or_default()}
                    </div>
                </Show>
            </div>

            // Input
            <form
                class="p-2 border-t border-stone-700"
                on:submit=on_submit
            >
                <div class="flex gap-2">
                    <input
                        type="text"
                        style:font-family=move || config.get().agent_font.font_family
                        style:font-size=move || format!("{}px", config.get().agent_font.font_size)
                        class="flex-1 bg-stone-800 border border-stone-600 rounded px-3 py-1.5 text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors disabled:opacity-50"
                        placeholder="Message..."
                        prop:value=move || input.get()
                        prop:disabled=move || is_streaming.get()
                        on:input=move |ev| set_input.set(event_target_value(&ev))
                    />
                    <button
                        type="submit"
                        class="px-3 py-1.5 bg-stone-700 text-stone-300 rounded text-sm hover:bg-stone-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                        prop:disabled=move || is_streaming.get()
                    >
                        {move || if is_streaming.get() { "..." } else { "Send" }}
                    </button>
                </div>
            </form>
        </aside>
    }
}
