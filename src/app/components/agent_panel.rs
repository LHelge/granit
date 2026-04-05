use crate::app::{
    components::{icons::Icon, model_selector::ModelSelector, provider_selector::ProviderSelector},
    ipc, AppCtx,
};

use granit_types::{ChatMessage, ChatRole, ModelInfo, ProviderInfo, ToolCallInfo};
use leptos::{
    ev::{MouseEvent, SubmitEvent},
    prelude::*,
    task::spawn_local,
};
use wasm_bindgen::JsCast;

#[derive(Clone)]
struct DisplayMessage {
    message: ChatMessage,
    rendered_html: Option<String>,
}

/// An item in the chat timeline — either a message or a tool call.
#[derive(Clone)]
enum DisplayItem {
    Message(DisplayMessage),
    ToolCall(ToolCallInfo),
}

#[component]
pub fn AgentPanel(width: ReadSignal<u16>) -> impl IntoView {
    let config = expect_context::<AppCtx>().config;
    let (input, set_input) = signal(String::new());
    let messages: RwSignal<Vec<DisplayItem>> = RwSignal::new(Vec::new());
    let streaming_content: RwSignal<String> = RwSignal::new(String::new());
    let is_streaming: RwSignal<bool> = RwSignal::new(false);
    let stream_error: RwSignal<Option<String>> = RwSignal::new(None);
    let messages_container: NodeRef<leptos::html::Div> = NodeRef::new();

    // ── Provider / model selection state ──────────────────────────────
    let providers: RwSignal<Vec<ProviderInfo>> = RwSignal::new(Vec::new());
    let models: RwSignal<Vec<ModelInfo>> = RwSignal::new(Vec::new());
    let models_loading: RwSignal<bool> = RwSignal::new(false);

    // Whether a valid model is selected (blocks sending when false).
    let has_model = Memo::new(move |_| {
        let cfg = config.get();
        let selected = cfg.agent.selected_model.clone().unwrap_or_default();
        if selected.is_empty() || models_loading.get() {
            return false;
        }
        models.get().iter().any(|m| m.id == selected)
    });

    // Track the agent identity (provider + model). When it changes (e.g.
    // after saving settings or dropdown selection), clear the chat so stale
    // history from a different provider doesn't confuse the user.
    let agent_identity = Memo::new(move |_| {
        let cfg = config.get();
        let provider = cfg
            .agent
            .providers
            .get(cfg.agent.selected_provider)
            .map(|e| e.provider.provider_type().to_string())
            .unwrap_or_default();
        let model = cfg.agent.selected_model.clone().unwrap_or_default();
        (provider, model)
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

    // Auto-scroll to the bottom when messages or streaming content change.
    Effect::new(move |_| {
        messages.track();
        streaming_content.track();
        if let Some(el) = messages_container.get() {
            // Use request_animation_frame to scroll after the DOM has updated.
            leptos::prelude::request_animation_frame(move || {
                el.set_scroll_top(el.scroll_height());
            });
        }
    });

    // Fetch providers list on mount.
    spawn_local(async move {
        if let Ok(p) = ipc::list_providers().await {
            providers.set(p);
        }
        // Also fetch model list for the currently selected provider.
        models_loading.set(true);
        if let Ok(m) = ipc::list_models().await {
            models.set(m);
        }
        models_loading.set(false);
    });

    let on_provider_changed = move || {
        models.set(Vec::new());
        models_loading.set(true);
        spawn_local(async move {
            if let Ok(m) = ipc::list_models().await {
                models.set(m);
            }
            models_loading.set(false);
        });
    };

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
                        messages.update(|m| {
                            m.push(DisplayItem::Message(DisplayMessage {
                                message: ChatMessage::assistant(content),
                                rendered_html: html,
                            }))
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
                messages.update(|m| m.push(DisplayItem::ToolCall(info)));
            })
            .await
            {
                _handles.push(h);
            }

            // Suspend forever — see comment above for why this is intentional.
            std::future::pending::<()>().await;
        });
    });

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let msg = input.get_untracked();
        if msg.trim().is_empty() || is_streaming.get_untracked() || !has_model.get_untracked() {
            return;
        }
        set_input.set(String::new());
        stream_error.set(None);
        is_streaming.set(true);

        spawn_local(async move {
            let html = ipc::render_markdown(&msg).await.ok();
            messages.update(|m| {
                m.push(DisplayItem::Message(DisplayMessage {
                    message: ChatMessage::user(msg.clone()),
                    rendered_html: html,
                }))
            });
            if let Err(e) = ipc::send_message(&msg).await {
                stream_error.set(Some(e));
                is_streaming.set(false);
            }
        });
    };

    let app = expect_context::<AppCtx>();

    // Intercept clicks on links in rendered markdown within the agent panel.
    let on_link_click = move |ev: MouseEvent| {
        let Some(target) = ev.target() else { return };
        let anchor = target
            .dyn_ref::<web_sys::Element>()
            .and_then(|el| {
                if el.tag_name().eq_ignore_ascii_case("a") {
                    Some(el.clone())
                } else {
                    el.closest("a").ok().flatten()
                }
            })
            .and_then(|el| el.dyn_into::<web_sys::HtmlAnchorElement>().ok());

        let Some(anchor) = anchor else { return };
        let href = anchor.get_attribute("href").unwrap_or_default();

        if href.is_empty() || href.starts_with('#') || href.starts_with('/') {
            return;
        }

        // External links → open in system browser
        if href.starts_with("http://") || href.starts_with("https://") {
            ev.prevent_default();
            let url = href.clone();
            spawn_local(async move {
                let _ = ipc::open_url(&url).await;
            });
            return;
        }

        // Wiki-link → navigate to note
        ev.prevent_default();
        let slug = js_sys::decode_uri_component(&href)
            .ok()
            .and_then(|s| s.as_string())
            .unwrap_or(href);
        let is_broken = anchor.class_list().contains("broken-link");
        spawn_local(async move {
            if is_broken {
                if let Ok(meta) = ipc::create_note(&slug, None).await {
                    if let Ok(all) = ipc::fetch_notes().await {
                        app.notes.set(all);
                    }
                    if let Ok(note) = ipc::read_note(&meta.slug).await {
                        app.active_note.set(Some(note));
                    }
                }
            } else if let Ok(note) = ipc::read_note(&slug).await {
                app.active_note.set(Some(note));
            }
        });
    };

    view! {
        <aside
            class="shrink-0 bg-base-200 border-l border-base-content/10 flex flex-col overflow-hidden"
            style:width=move || format!("{}px", width.get())
        >
            // Header — provider selector + clear chat
            <div class="px-2 py-1.5 border-b border-base-content/10 flex items-center">
                <ProviderSelector
                    providers=providers
                    disabled=Signal::derive(move || is_streaming.get())
                    on_changed=on_provider_changed
                />
                <div class="flex-1" />
                <div class="tooltip tooltip-left" data-tip="Clear chat">
                    <button
                        type="button"
                        class="btn btn-ghost btn-xs btn-square"
                        prop:disabled=move || is_streaming.get()
                        on:click=move |_| {
                            messages.set(Vec::new());
                            streaming_content.set(String::new());
                            stream_error.set(None);
                            spawn_local(async move {
                                let _ = ipc::clear_chat().await;
                            });
                        }
                    >
                        <Icon icon=icondata_lu::LuTrash2 width="0.875rem" height="0.875rem"/>
                    </button>
                </div>
            </div>
            // Message list
            <div
                node_ref=messages_container
                class="flex-1 min-h-0 overflow-y-auto p-3 space-y-3 flex flex-col"
                style:font-family=move || config.get().agent_font.font_family
                style:font-size=move || format!("{}px", config.get().agent_font.font_size)
                on:click=on_link_click
            >
                // Empty state
                <Show when=move || messages.get().is_empty() && !is_streaming.get() && streaming_content.get().is_empty()>
                    <p class="text-base-content/35 italic text-center mt-8">"Ask me anything about your notes..."</p>
                </Show>

                // Committed messages and tool calls
                {move || messages.get().into_iter().map(|item| {
                    match item {
                        DisplayItem::Message(dm) => {
                            let is_user = dm.message.role == ChatRole::User;
                            let chat_class = if is_user { "chat chat-end" } else { "chat chat-start" };
                            let bubble_class = if is_user {
                                "chat-bubble chat-bubble-neutral prose prose-sm max-w-none [&>*:first-child]:mt-0 [&>*:last-child]:mb-0 break-words overflow-hidden"
                            } else {
                                "chat-bubble prose prose-sm max-w-none [&>*:first-child]:mt-0 [&>*:last-child]:mb-0 break-words overflow-hidden"
                            };
                            view! {
                                <div class=chat_class>
                                    {if let Some(rendered) = dm.rendered_html {
                                        view! {
                                            <div
                                                class=bubble_class
                                                style="font-size: inherit"
                                                inner_html=rendered
                                            />
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class=bubble_class style="font-size: inherit">
                                                {dm.message.content.clone()}
                                            </div>
                                        }.into_any()
                                    }}
                                </div>
                            }.into_any()
                        }
                        DisplayItem::ToolCall(info) => {
                            view! {
                                <div class="chat chat-start">
                                    <div class="chat-bubble chat-bubble-ghost text-xs font-mono flex items-center gap-1.5 py-1.5">
                                        <span class="inline-flex w-3 h-3 shrink-0 text-base-content/35">
                                            <Icon icon=icondata_lu::LuWrench width="100%" height="100%"/>
                                        </span>
                                        <span class="text-base-content/70">{info.name}"("{info.param.as_deref().unwrap_or_default().to_string()}")"</span>
                                    </div>
                                </div>
                            }.into_any()
                        }
                    }
                }).collect_view()}

                // Streaming response in progress
                <Show when=move || is_streaming.get() || !streaming_content.get().is_empty()>
                    <div class="chat chat-start">
                        <div class="chat-bubble whitespace-pre-wrap break-words" style="font-size: inherit">
                            {move || {
                                let content = streaming_content.get();
                                if content.is_empty() {
                                    view! { <span class="loading loading-dots loading-sm" /> }.into_any()
                                } else {
                                    view! { <span>{content}<span class="loading loading-dots loading-xs ml-1 align-middle" /></span> }.into_any()
                                }
                            }}
                        </div>
                    </div>
                </Show>

                // Error
                <Show when=move || stream_error.get().is_some()>
                    <div role="alert" class="alert alert-error alert-soft alert-sm">
                        {move || stream_error.get().unwrap_or_default()}
                    </div>
                </Show>
            </div>

            // Input area — textarea above, model selector + send below
            <div class="border-t border-base-content/10">
                <form
                    class="p-2 flex flex-col gap-1.5"
                    on:submit=on_submit
                >
                    // Multi-line message input
                    <textarea
                        style:font-family=move || config.get().agent_font.font_family
                        style:font-size=move || format!("{}px", config.get().agent_font.font_size)
                        class="textarea textarea-bordered w-full resize-none disabled:opacity-50"
                        rows="4"
                        placeholder="Message... (Enter to send, Shift+Enter for newline)"
                        prop:value=move || input.get()
                        prop:disabled=move || is_streaming.get()
                        on:input=move |ev| set_input.set(event_target_value(&ev))
                        on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                            if ev.key() == "Enter" && !ev.shift_key() && has_model.get() {
                                ev.prevent_default();
                                // Programmatically submit the parent form
                                if let Some(form) = ev.target()
                                    .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
                                    .and_then(|el| el.closest("form").ok().flatten())
                                    .and_then(|f| f.dyn_into::<web_sys::HtmlFormElement>().ok())
                                {
                                    let _ = form.request_submit();
                                }
                            }
                        }
                    />
                    // Model selector + send button row
                    <div class="flex items-center gap-2">
                        <ModelSelector
                            models=models
                            models_loading=models_loading
                            disabled=Signal::derive(move || is_streaming.get())
                        />
                        <div class="flex-1" />
                        <button
                            type="submit"
                            class="btn btn-sm btn-primary"
                            prop:disabled=move || is_streaming.get() || !has_model.get()
                        >
                            {move || if is_streaming.get() { "..." } else { "Send" }}
                        </button>
                    </div>
                </form>
            </div>
        </aside>
    }
}
