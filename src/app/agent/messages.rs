use crate::app::{components::icons::Icon, ipc, AppCtx};
use granit_types::{ChatMessage, ChatRole, ToolCallInfo};
use leptos::{ev::MouseEvent, prelude::*, task::spawn_local};
use wasm_bindgen::JsCast;

#[derive(Clone)]
pub(super) struct DisplayMessage {
    pub message: ChatMessage,
    pub rendered_html: Option<String>,
}

/// An item in the chat timeline — either a message or a tool call.
#[derive(Clone)]
pub(super) enum DisplayItem {
    Message(DisplayMessage),
    ToolCall(ToolCallInfo),
}

impl DisplayItem {
    pub fn message(msg: ChatMessage, html: Option<String>) -> Self {
        Self::Message(DisplayMessage {
            message: msg,
            rendered_html: html,
        })
    }

    pub fn tool_call(info: ToolCallInfo) -> Self {
        Self::ToolCall(info)
    }
}

/// Renders the scrollable message list: committed messages, streaming bubble, and errors.
#[component]
pub(super) fn MessageList(
    messages: RwSignal<Vec<DisplayItem>>,
    streaming_content: RwSignal<String>,
    is_streaming: RwSignal<bool>,
    stream_error: RwSignal<Option<String>>,
    messages_container: NodeRef<leptos::html::Div>,
) -> impl IntoView {
    let config = expect_context::<AppCtx>().config;
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
    }
}
