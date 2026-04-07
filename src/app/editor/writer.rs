use super::{
    frontmatter::FrontmatterEditor,
    text_editing::{self, EditResult, TextareaState},
    use_editor_ctx, EditorCtx,
};
use crate::app::components::{icon_picker::IconPicker, icons::Icon};
use leptos::prelude::*;
use leptos::web_sys::wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

fn request_animation_frame(f: impl FnOnce() + 'static) {
    let cb = Closure::once_into_js(f);
    let _ = leptos::web_sys::window()
        .unwrap()
        .request_animation_frame(cb.as_ref().unchecked_ref());
}

// ── DOM helpers ────────────────────────────────────────────────────

/// Create a `TextareaState` from a DOM textarea element,
/// converting UTF-16 selection offsets to char offsets.
fn textarea_state(textarea: &web_sys::HtmlTextAreaElement) -> TextareaState {
    let value = textarea.value();
    let start = textarea.selection_start().ok().flatten().unwrap_or(0) as usize;
    let end = textarea.selection_end().ok().flatten().unwrap_or(0) as usize;
    TextareaState::from_utf16(value, start, end)
}

/// Apply an `EditResult` to the DOM textarea and sync the content signal.
fn apply_result(textarea: &web_sys::HtmlTextAreaElement, result: &EditResult, ctx: EditorCtx) {
    textarea.set_value(&result.value);
    let _ = textarea.set_selection_range(result.cursor_start_utf16(), result.cursor_end_utf16());
    ctx.content.set(textarea.value());
}

// ── Paste handler ──────────────────────────────────────────────────

fn handle_paste(
    ev: web_sys::ClipboardEvent,
    content_ref: NodeRef<leptos::html::Textarea>,
    ctx: EditorCtx,
) {
    let Some(el) = content_ref.get() else { return };
    let textarea: &web_sys::HtmlTextAreaElement = el.as_ref();
    let s = textarea_state(textarea);

    let Some(data) = ev.clipboard_data() else {
        return;
    };
    let Ok(clip_text) = data.get_data("text/plain") else {
        return;
    };
    let clip_trimmed = clip_text.trim();
    if !text_editing::is_url(clip_trimmed) {
        return;
    }

    ev.prevent_default();
    let result = s.paste_url(clip_trimmed);
    apply_result(textarea, &result, ctx);
}

// ── Main keydown dispatcher ────────────────────────────────────────

fn handle_content_keydown(
    ev: leptos::ev::KeyboardEvent,
    content_ref: NodeRef<leptos::html::Textarea>,
    ctx: EditorCtx,
) {
    let Some(el) = content_ref.get() else { return };
    let textarea: &web_sys::HtmlTextAreaElement = el.as_ref();
    let s = textarea_state(textarea);

    let result = match ev.key().as_str() {
        "[" => Some(s.bracket('[', ']')),
        "(" => Some(s.bracket('(', ')')),
        "]" => s.skip_close(']'),
        ")" => s.skip_close(')'),
        "*" | "_" | "`" | "~" => Some(s.formatting_char(ev.key().chars().next().unwrap())),
        "Enter" => s.enter(),
        "Tab" => s.tab(ev.shift_key()),
        _ => None,
    };

    if let Some(r) = result {
        ev.prevent_default();
        apply_result(textarea, &r, ctx);
    }
}

/// Raw markdown editor with title input and content textarea.
#[component]
pub(super) fn Writer() -> impl IntoView {
    let ctx = use_editor_ctx();
    let title_ref = NodeRef::<leptos::html::Input>::new();
    let content_ref = NodeRef::<leptos::html::Textarea>::new();

    // Focus and select the title input when requested.
    Effect::new(move || {
        if ctx.focus_title.get() {
            ctx.focus_title.set(false);
            request_animation_frame(move || {
                if let Some(el) = title_ref.get() {
                    let input: &web_sys::HtmlInputElement = el.as_ref();
                    let _ = input.focus();
                    input.select();
                }
            });
        }
    });

    // Focus the content textarea when requested.
    Effect::new(move || {
        if ctx.focus_content.get() {
            ctx.focus_content.set(false);
            request_animation_frame(move || {
                if let Some(el) = content_ref.get() {
                    let textarea: &web_sys::HtmlTextAreaElement = el.as_ref();
                    let _ = textarea.focus();
                }
            });
        }
    });

    view! {
        <div class="not-prose flex items-center gap-2 mb-2">
            <IconPicker
                value=Signal::derive(move || ctx.icon.get())
                on_change=move |v| ctx.icon.set(v)
            />
            <Show when=move || ctx.active_note.get().is_some()>
                <button
                    type="button"
                    class=move || {
                        if ctx.favorite.get().unwrap_or(false) {
                            "inline-flex w-5 h-5 shrink-0 text-warning transition-colors hover:opacity-80"
                        } else {
                            "inline-flex w-5 h-5 shrink-0 text-base-content/35 transition-colors hover:text-base-content/55"
                        }
                    }
                    aria-label=move || {
                        if ctx.favorite.get().unwrap_or(false) {
                            "Unfavorite note"
                        } else {
                            "Favorite note"
                        }
                    }
                    on:click=move |_| {
                        let next = !ctx.favorite.get_untracked().unwrap_or(false);
                        ctx.favorite.set(Some(next));
                    }
                >
                    <Icon icon=icondata_lu::LuStar width="100%" height="100%"/>
                </button>
            </Show>
            <input
                type="text"
                node_ref=title_ref
                class="w-full bg-transparent text-white text-4xl font-extrabold leading-[1.111] outline-none p-0"
                placeholder="Untitled"
                prop:value=move || ctx.title_input.get()
                on:input=move |ev| ctx.title_input.set(event_target_value(&ev))
                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                    if ev.key() == "Enter" {
                        ev.prevent_default();
                        if let Some(el) = content_ref.get() {
                            let textarea: &web_sys::HtmlTextAreaElement = el.as_ref();
                            let _ = textarea.focus();
                        }
                    }
                }
            />
        </div>
        <FrontmatterEditor />
        <textarea
            node_ref=content_ref
            class="not-prose w-full flex-1 bg-transparent text-base-content/70 resize-none outline-none leading-relaxed"
            placeholder="Start writing..."
            style:font-family=move || ctx.config.get().markdown_font.font_family
            style:font-size=move || format!("{}px", ctx.config.get().markdown_font.font_size)
            prop:value=move || ctx.content.get()
            on:input=move |ev| ctx.content.set(event_target_value(&ev))
            on:keydown=move |ev| handle_content_keydown(ev, content_ref, ctx)
            on:paste=move |ev| handle_paste(ev, content_ref, ctx)
        />
    }
}
