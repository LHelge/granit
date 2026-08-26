mod messages;
mod mode_selector;
mod model_selector;
mod provider_selector;
mod streaming;

use crate::app::{components::icons::Icon, ipc, AppCtx};
use messages::{DisplayItem, MessageList};
use mode_selector::ModeSelector;
use model_selector::ModelSelector;
use provider_selector::ProviderSelector;

use granit_types::{resolve_note_icon, AgentDocInfo, AttachedNote, ChatMessage, ModelInfo};
use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};
use wasm_bindgen::JsCast;

fn normalized_selected_text(selected: Option<String>) -> Option<String> {
    selected
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn selection_preview(selected: &str) -> String {
    const MAX_CHARS: usize = 28;

    let collapsed = selected.split_whitespace().collect::<Vec<_>>().join(" ");
    let count = collapsed.chars().count();
    if count <= MAX_CHARS {
        return collapsed;
    }

    let truncated: String = collapsed.chars().take(MAX_CHARS).collect();
    format!("{}...", truncated)
}

fn freeze_attached_note(
    active_note: Option<granit_types::Document>,
    selected: Option<String>,
) -> Option<AttachedNote> {
    let note = active_note?;
    Some(AttachedNote {
        slug: note.meta.slug,
        content: note.content,
        selected,
    })
}

fn note_is_attached(attached_notes: &[AttachedNote], slug: &str) -> bool {
    attached_notes.iter().any(|note| note.slug == slug)
}

/// Parse a task invocation out of a typed message.
///
/// `/name rest of text` (or exactly `/name`) invokes the task `name` with
/// `rest of text` as its input. Task names may contain spaces, so the
/// longest matching name wins. Returns `None` when the message is not a
/// task invocation — it is then sent as a plain message.
fn parse_task_invocation(raw: &str, tasks: &[AgentDocInfo]) -> Option<(String, String)> {
    let rest = raw.strip_prefix('/')?;
    let name = tasks
        .iter()
        .map(|t| t.name.as_str())
        .filter(|name| {
            rest == *name
                || rest
                    .strip_prefix(name)
                    .is_some_and(|r| r.starts_with(char::is_whitespace))
        })
        .max_by_key(|name| name.len())?;
    let input = rest[name.len()..].trim().to_string();
    Some((name.to_string(), input))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn tasks(names: &[&str]) -> Vec<AgentDocInfo> {
        names
            .iter()
            .map(|n| AgentDocInfo {
                name: n.to_string(),
                description: String::new(),
            })
            .collect()
    }

    #[wasm_bindgen_test]
    fn plain_messages_are_not_task_invocations() {
        let tasks = tasks(&["summarize"]);
        assert_eq!(parse_task_invocation("hello world", &tasks), None);
        assert_eq!(parse_task_invocation("/unknown-task text", &tasks), None);
        assert_eq!(parse_task_invocation("/summarizer", &tasks), None);
    }

    #[wasm_bindgen_test]
    fn task_invocation_with_and_without_input() {
        let tasks = tasks(&["summarize"]);
        assert_eq!(
            parse_task_invocation("/summarize", &tasks),
            Some(("summarize".to_string(), String::new()))
        );
        assert_eq!(
            parse_task_invocation("/summarize chapter two", &tasks),
            Some(("summarize".to_string(), "chapter two".to_string()))
        );
    }

    #[wasm_bindgen_test]
    fn longest_task_name_wins_for_names_with_spaces() {
        let tasks = tasks(&["daily", "daily review"]);
        assert_eq!(
            parse_task_invocation("/daily review extra input", &tasks),
            Some(("daily review".to_string(), "extra input".to_string()))
        );
        assert_eq!(
            parse_task_invocation("/daily stand-up", &tasks),
            Some(("daily".to_string(), "stand-up".to_string()))
        );
    }
}

#[component]
pub fn AgentPanel(width: ReadSignal<u16>) -> impl IntoView {
    let app = expect_context::<AppCtx>();
    let config = app.config;
    let active_note = app.active_note;
    let selected_note_text = app.selected_note_text;
    let (input, set_input) = signal(String::new());
    let messages: RwSignal<Vec<DisplayItem>> = RwSignal::new(Vec::new());
    let streaming_content: RwSignal<String> = RwSignal::new(String::new());
    let is_streaming: RwSignal<bool> = RwSignal::new(false);
    let stream_error: RwSignal<Option<String>> = RwSignal::new(None);
    let messages_container: NodeRef<leptos::html::Div> = NodeRef::new();
    let attached_notes: RwSignal<Vec<AttachedNote>> = RwSignal::new(Vec::new());

    // ── Slash-command task autocomplete ───────────────────────────────
    // Typing `/` at the start of the input opens a popup of tasks from
    // `.granit/agent/tasks`. Escape dismisses it until the input changes.
    let slash_dismissed = RwSignal::new(false);
    let slash_highlight = RwSignal::new(0usize);
    let filtered_tasks = Memo::new(move |_| {
        if slash_dismissed.get() {
            return Vec::new();
        }
        let value = input.get();
        let Some(query) = value.strip_prefix('/') else {
            return Vec::new();
        };
        let query = query.to_lowercase();
        app.tasks
            .get()
            .into_iter()
            .filter(|t| t.name.to_lowercase().starts_with(&query))
            .collect::<Vec<_>>()
    });
    // Keep the highlight inside the (shrinking) candidate list.
    Effect::new(move |_| {
        let len = filtered_tasks.get().len();
        if len > 0 && slash_highlight.get_untracked() >= len {
            slash_highlight.set(len - 1);
        }
    });
    let complete_task = move |name: &str| {
        set_input.set(format!("/{name} "));
        slash_highlight.set(0);
    };

    // ── Provider / model selection state ──────────────────────────────
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
        let mode = format!("{:?}", cfg.agent.mode);
        (provider, model, mode)
    });
    let prev_identity: RwSignal<Option<(String, String, String)>> = RwSignal::new(None);
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

    let active_provider = Memo::new(move |_| {
        let cfg = config.get();
        (
            cfg.agent.selected_provider,
            cfg.agent
                .providers
                .get(cfg.agent.selected_provider)
                .cloned(),
        )
    });

    Effect::new(move |_| {
        let _ = active_provider.get();
        models.set(Vec::new());
        models_loading.set(true);
        spawn_local(async move {
            if let Ok(m) = ipc::list_models().await {
                models.set(m);
            }
            models_loading.set(false);
        });
    });

    // Register streaming event listeners.
    streaming::setup_stream_listeners(messages, streaming_content, is_streaming, stream_error);

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let raw = input.get_untracked();
        if raw.trim().is_empty() || is_streaming.get_untracked() || !has_model.get_untracked() {
            return;
        }
        // `/task-name text…` invokes a task with the remaining text as its
        // input; anything else is a plain message.
        let task_invocation = parse_task_invocation(&raw, &app.tasks.get_untracked());
        set_input.set(String::new());
        slash_dismissed.set(false);
        slash_highlight.set(0);
        stream_error.set(None);
        is_streaming.set(true);
        let attached_notes = attached_notes.get_untracked();

        spawn_local(async move {
            // The message list shows what the user typed (`/task …`); the
            // backend receives the task slug + input and renders the prompt.
            let html = ipc::render_markdown(&raw).await.ok();
            messages.update(|m| m.push(DisplayItem::message(ChatMessage::user(raw.clone()), html)));
            let (msg, task) = match &task_invocation {
                Some((name, input_text)) => (input_text.clone(), Some(name.clone())),
                None => (raw.clone(), None),
            };
            if let Err(e) = ipc::send_message(&msg, attached_notes, task.as_deref()).await {
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
            <div class="px-2 py-1.5 flex items-center">
                <ProviderSelector
                    disabled=Signal::derive(move || is_streaming.get())
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

            // Input area — chat bubble styled, model selector + send inside
            <div class="chat chat-end gap-0 pl-3 pb-3">
                <form
                    class="w-full"
                    on:submit=on_submit
                >
                    <div class="chat-bubble chat-bubble-neutral relative w-full p-0 text-left">
                        // Task autocomplete popup, floating above the input
                        <Show when=move || !filtered_tasks.get().is_empty() && !is_streaming.get()>
                            <div class="absolute bottom-full left-2 right-2 mb-1 z-50">
                                <ul class="menu menu-sm w-full bg-base-300 border border-base-content/20 rounded-box shadow-lg max-h-48 overflow-y-auto flex-nowrap">
                                    {move || {
                                        let candidates = filtered_tasks.get();
                                        let highlight = slash_highlight.get().min(candidates.len().saturating_sub(1));
                                        candidates.into_iter().enumerate().map(|(idx, task)| {
                                            let name = task.name.clone();
                                            let pick_name = name.clone();
                                            let description = task.description.clone();
                                            view! {
                                                <li>
                                                    <button
                                                        type="button"
                                                        class=move || if idx == highlight { "menu-active flex items-baseline gap-2" } else { "flex items-baseline gap-2" }
                                                        // mousedown (not click) keeps the textarea focused.
                                                        on:mousedown=move |ev| {
                                                            ev.prevent_default();
                                                            complete_task(&pick_name);
                                                        }
                                                    >
                                                        <span class="font-mono text-xs shrink-0">{format!("/{name}")}</span>
                                                        <Show when={
                                                            let description = description.clone();
                                                            move || !description.is_empty()
                                                        }>
                                                            <span class="truncate text-xs text-base-content/50">{description.clone()}</span>
                                                        </Show>
                                                    </button>
                                                </li>
                                            }
                                        }).collect_view()
                                    }}
                                </ul>
                            </div>
                        </Show>
                        <Show when=move || !attached_notes.get().is_empty() || active_note.get().is_some()>
                            <div class="px-2.5 pt-2 pb-1 flex flex-wrap gap-1.5">
                                <For
                                    each=move || attached_notes.get()
                                    key=|note| note.slug.clone()
                                    let:note
                                >
                                    {move || {
                                        let slug = note.slug.clone();
                                        let removal_slug = slug.clone();
                                        let selection_preview = note.selected.as_deref().map(selection_preview);
                                        view! {
                                            <div class="flex max-w-full items-center gap-1 rounded-md border border-info/35 bg-info/10 px-1.5 py-0.75 text-xs text-neutral-content">
                                                <span class="inline-flex w-3.5 h-3.5 shrink-0 text-accent">
                                                    <Icon icon=icondata_lu::LuFileText width="100%" height="100%"/>
                                                </span>
                                                <span class="min-w-0 flex flex-1 items-baseline gap-1 overflow-hidden">
                                                    <span class="truncate font-medium">{slug}</span>
                                                    {selection_preview.map(|text| view! {
                                                        <span class="truncate text-xs italic text-neutral-content/45">
                                                            {format!("\"{}\"", text)}
                                                        </span>
                                                    })}
                                                </span>
                                                <button
                                                    type="button"
                                                    class="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-neutral-content/65 transition-colors hover:bg-neutral-content/10 hover:text-neutral-content disabled:cursor-not-allowed disabled:opacity-35"
                                                    title="Remove attached note"
                                                    prop:disabled=move || is_streaming.get()
                                                    on:click=move |_| {
                                                        attached_notes.update(|notes| {
                                                            notes.retain(|attached| attached.slug != removal_slug);
                                                        });
                                                    }
                                                >
                                                    <Icon icon=icondata_lu::LuTrash2 width="0.75rem" height="0.75rem"/>
                                                </button>
                                            </div>
                                        }
                                    }}
                                </For>

                                {move || active_note.get().and_then(|note| {
                                    if note_is_attached(&attached_notes.get(), &note.meta.slug) {
                                        return None;
                                    }

                                    let icon = note.meta.icon.clone();
                                    let slug = note.meta.slug.clone();
                                    Some(view! {
                                        <div class="flex max-w-full items-center gap-1 rounded-md border border-neutral-content/15 bg-neutral-content/5 px-1.5 py-0.75 text-xs text-neutral-content/80">
                                            <span class="inline-flex w-3.5 h-3.5 shrink-0 text-accent">
                                                {if let Some(icon_id) = icon {
                                                    view! {
                                                        <Icon icon=resolve_note_icon(&icon_id) width="100%" height="100%"/>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <Icon icon=icondata_lu::LuFileText width="100%" height="100%"/>
                                                    }.into_any()
                                                }}
                                            </span>
                                            <span class="min-w-0 truncate font-medium">{slug.clone()}</span>
                                            <button
                                                type="button"
                                                class="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-neutral-content/65 transition-colors hover:bg-neutral-content/10 hover:text-neutral-content disabled:cursor-not-allowed disabled:opacity-35"
                                                title="Attach current note"
                                                prop:disabled=move || is_streaming.get()
                                                on:mousedown=move |ev| {
                                                    ev.prevent_default();
                                                }
                                                on:click=move |_| {
                                                    if let Some(frozen) = freeze_attached_note(
                                                        active_note.get_untracked(),
                                                        normalized_selected_text(
                                                            selected_note_text.get_untracked(),
                                                        ),
                                                    ) {
                                                        attached_notes.update(|notes| {
                                                            if !note_is_attached(notes, &frozen.slug) {
                                                                notes.push(frozen);
                                                            }
                                                        });
                                                    }
                                                }
                                            >
                                                <Icon icon=icondata_lu::LuPlus width="0.75rem" height="0.75rem"/>
                                            </button>
                                        </div>
                                    })
                                })}
                            </div>
                        </Show>

                        // Multi-line message input
                        <textarea
                            style:font-family=move || config.get().agent_font.font_family
                            style:font-size=move || format!("{}px", config.get().agent_font.font_size)
                            class="w-full resize-none bg-transparent px-3 pt-3 pb-2 text-neutral-content outline-none disabled:opacity-50 placeholder:text-neutral-content/35"
                            rows="4"
                            placeholder="Message..."
                            prop:value=move || input.get()
                            prop:disabled=move || is_streaming.get()
                            on:input=move |ev| {
                                slash_dismissed.set(false);
                                set_input.set(event_target_value(&ev));
                            }
                            on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                // While the task popup is open, the keyboard
                                // navigates it; Enter completes, not submits.
                                let candidates = filtered_tasks.get_untracked();
                                if !candidates.is_empty() {
                                    match ev.key().as_str() {
                                        "ArrowDown" => {
                                            ev.prevent_default();
                                            slash_highlight.update(|h| *h = (*h + 1) % candidates.len());
                                            return;
                                        }
                                        "ArrowUp" => {
                                            ev.prevent_default();
                                            slash_highlight.update(|h| {
                                                *h = (*h + candidates.len() - 1) % candidates.len();
                                            });
                                            return;
                                        }
                                        "Enter" | "Tab" => {
                                            ev.prevent_default();
                                            let idx = slash_highlight
                                                .get_untracked()
                                                .min(candidates.len() - 1);
                                            complete_task(&candidates[idx].name.clone());
                                            return;
                                        }
                                        "Escape" => {
                                            slash_dismissed.set(true);
                                            return;
                                        }
                                        _ => {}
                                    }
                                }
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
                            <ModeSelector
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
