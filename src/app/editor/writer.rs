use super::{codemirror, frontmatter::FrontmatterEditor, use_editor_ctx};
use crate::app::components::{icon_picker::IconPicker, icons::Icon};
use leptos::prelude::*;
use leptos::web_sys::wasm_bindgen::closure::Closure;
use std::cell::Cell;
use wasm_bindgen::JsCast;

fn request_animation_frame(f: impl FnOnce() + 'static) {
    let cb = Closure::once_into_js(f);
    let _ = leptos::web_sys::window()
        .unwrap()
        .request_animation_frame(cb.as_ref().unchecked_ref());
}

/// Raw markdown editor with title input and CodeMirror 6 content editor.
#[component]
pub(super) fn Writer() -> impl IntoView {
    let ctx = use_editor_ctx();
    let title_ref = NodeRef::<leptos::html::Input>::new();
    let container_ref = NodeRef::<leptos::html::Div>::new();

    // Shared handle to the CM6 editor instance.
    let editor_handle = StoredValue::new_local(Cell::new(None::<codemirror::EditorHandle>));

    // Track the content version to detect external (note-switch) changes
    // vs internal (user-typing) changes. Incremented on each onChange from CM6.
    let internal_version = StoredValue::new_local(Cell::new(0u64));

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

    // Focus the CM6 editor when requested.
    Effect::new(move || {
        if ctx.focus_content.get() {
            ctx.focus_content.set(false);
            request_animation_frame(move || {
                editor_handle.with_value(|cell| {
                    if let Some(h) = cell.get() {
                        codemirror::focus(h);
                    }
                });
            });
        }
    });

    // Sync external content changes (note switches) into the CM6 editor.
    // The mod.rs effect sets ctx.content on note switch; we detect that here
    // by comparing against the internal version counter. When content changes
    // without the internal version bumping, it's an external change.
    let prev_content = StoredValue::new_local(Cell::new(0u64));
    Effect::new(move || {
        let content = ctx.content.get();
        let current_iv = internal_version.with_value(|c| c.get());
        let prev_iv = prev_content.with_value(|c| c.get());

        // If version hasn't changed since we last ran, this is an external update
        if current_iv == prev_iv {
            editor_handle.with_value(|cell| {
                if let Some(h) = cell.get() {
                    codemirror::set_content(h, &content);
                }
            });
        }
        prev_content.with_value(|c| c.set(current_iv));
    });

    // Sync font changes into the CM6 editor.
    Effect::new(move || {
        let config = ctx.config.get();
        editor_handle.with_value(|cell| {
            if let Some(h) = cell.get() {
                codemirror::set_font(
                    h,
                    &config.markdown_font.font_family,
                    &config.markdown_font.font_size.to_string(),
                );
            }
        });
    });

    // Mount the CM6 editor once the container div is available.
    Effect::new(move || {
        let Some(el) = container_ref.get() else {
            return;
        };

        // Only create once
        if editor_handle.with_value(|c| c.get()).is_some() {
            return;
        }

        let html_el: &web_sys::HtmlElement = el.as_ref();
        let config = ctx.config.get_untracked();
        let content = ctx.content.get_untracked();
        let slugs: Vec<String> = ctx
            .notes
            .get_untracked()
            .into_iter()
            .map(|m| m.slug)
            .collect();

        let h = codemirror::create(
            html_el,
            &content,
            &config.markdown_font.font_family,
            &config.markdown_font.font_size.to_string(),
            &slugs,
            // onChange — user edits
            move |new_content: String| {
                internal_version.with_value(|c| c.set(c.get().wrapping_add(1)));
                ctx.content.set(new_content);
            },
            // onSelectionChange — track selected text for agent panel
            move |selected: String| {
                let text = if selected.trim().is_empty() {
                    None
                } else {
                    Some(selected)
                };
                ctx.app.selected_note_text.set(text);
            },
        );

        editor_handle.with_value(|cell| cell.set(Some(h)));
    });

    // Keep the slug list up to date when notes are added/removed/renamed.
    Effect::new(move || {
        let slugs: Vec<String> = ctx.notes.get().into_iter().map(|m| m.slug).collect();
        editor_handle.with_value(|cell| {
            if let Some(h) = cell.get() {
                codemirror::set_slugs(h, &slugs);
            }
        });
    });

    // Destroy the CM6 editor on unmount.
    on_cleanup(move || {
        editor_handle.with_value(|cell| {
            if let Some(h) = cell.get() {
                codemirror::destroy(h);
                cell.set(None);
            }
        });
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
                class="w-full bg-transparent text-base-content text-4xl font-extrabold leading-[1.111] outline-none p-0"
                placeholder="Untitled"
                prop:value=move || ctx.title_input.get()
                on:input=move |ev| ctx.title_input.set(event_target_value(&ev))
                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                    if ev.key() == "Enter" {
                        ev.prevent_default();
                        editor_handle.with_value(|cell| {
                            if let Some(h) = cell.get() {
                                codemirror::focus(h);
                            }
                        });
                    }
                }
            />
        </div>
        <FrontmatterEditor />
        <div
            node_ref=container_ref
            class="not-prose w-full flex-1 min-h-0 overflow-hidden"
        />
    }
}
