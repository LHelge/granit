mod messages;
mod model_selector;
mod provider_selector;
mod streaming;

use crate::app::{components::icons::Icon, ipc, AppCtx};
use messages::{DisplayItem, MessageList};
use model_selector::ModelSelector;
use provider_selector::ProviderSelector;

use granit_types::{ChatMessage, ModelInfo, ProviderInfo};
use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};
use wasm_bindgen::JsCast;

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
                is_streaming.set(false);
            }
        }
        prev_identity.set(Some(current));
    });

    // Auto-scroll to the bottom when messages or streaming content change.
    Effect::new(move |_| {
        messages.track();
        streaming_content.track();
        if let Some(el) = messages_container.get() {
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

    // Register streaming event listeners.
    streaming::setup_stream_listeners(messages, streaming_content, is_streaming, stream_error);

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
            messages.update(|m| m.push(DisplayItem::message(ChatMessage::user(msg.clone()), html)));
            if let Err(e) = ipc::send_message(&msg).await {
                stream_error.set(Some(e));
                is_streaming.set(false);
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
            <MessageList
                messages=messages
                streaming_content=streaming_content
                is_streaming=is_streaming
                stream_error=stream_error
                messages_container=messages_container
            />

            // Input area — textarea above, model selector + send below
            <div>
                <form
                    class="p-2"
                    on:submit=on_submit
                >
                    <div class="rounded-box border border-neutral/60 bg-neutral text-neutral-content overflow-hidden">
                        // Multi-line message input
                        <textarea
                            style:font-family=move || config.get().agent_font.font_family
                            style:font-size=move || format!("{}px", config.get().agent_font.font_size)
                            class="w-full resize-none bg-transparent px-3 pt-3 pb-2 text-neutral-content outline-none disabled:opacity-50 placeholder:text-neutral-content/35"
                            rows="4"
                            placeholder="Message..."
                            prop:value=move || input.get()
                            prop:disabled=move || is_streaming.get()
                            on:input=move |ev| set_input.set(event_target_value(&ev))
                            on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                if ev.key() == "Enter" && !ev.shift_key() && has_model.get() {
                                    ev.prevent_default();
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

                        // Footer row inside the same frame
                        <div class="flex items-center gap-1.5 px-2.5 py-2 text-neutral-content">
                            <ModelSelector
                                models=models
                                models_loading=models_loading
                                disabled=Signal::derive(move || is_streaming.get())
                            />
                            <div class="flex-1" />
                            <button
                                type="submit"
                                class="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded text-neutral-content/60 transition-colors hover:bg-neutral-content/10 hover:text-neutral-content disabled:cursor-not-allowed disabled:opacity-35"
                                title="Send message"
                                prop:disabled=move || is_streaming.get() || !has_model.get()
                            >
                                {move || if is_streaming.get() {
                                    view! { <span class="loading loading-spinner loading-xs" /> }.into_any()
                                } else {
                                    view! {
                                        <span class="inline-flex w-3.5 h-3.5 shrink-0">
                                            <Icon icon=icondata_lu::LuSend width="100%" height="100%"/>
                                        </span>
                                    }.into_any()
                                }}
                            </button>
                        </div>
                    </div>
                </form>
            </div>
        </aside>
    }
}
