use leptos::prelude::*;

use super::use_editor_ctx;

/// Inline frontmatter editor shown between the title and content textarea.
///
/// Shows a tag editor (removable pills + add input). The icon picker is
/// rendered separately in [`Writer`], beside the title.
#[component]
pub(super) fn FrontmatterEditor() -> impl IntoView {
    let ctx = use_editor_ctx();

    // ── Tag state ────────────────────────────────────────────────────────────

    let (tag_input, set_tag_input) = signal(String::new());

    let add_tag = move || {
        let tag = tag_input.get_untracked().trim().to_lowercase();
        if tag.is_empty() {
            return;
        }
        let mut tags = ctx.tags.get_untracked();
        if !tags.contains(&tag) {
            tags.push(tag);
            ctx.tags.set(tags);
        }
        set_tag_input.set(String::new());
    };

    let remove_tag = move |tag: String| {
        let mut tags = ctx.tags.get_untracked();
        tags.retain(|t| t != &tag);
        ctx.tags.set(tags);
    };

    view! {
        // ── Tag editor ────────────────────────────────────────────────────────
        <div class="not-prose flex flex-wrap items-center gap-1.5 mb-3">
            <For
                each=move || ctx.tags.get()
                key=|tag| tag.clone()
                let:tag
            >
                {
                    let tag_for_remove = tag.clone();
                    view! {
                        <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs bg-stone-700 text-stone-300">
                            {tag.clone()}
                            <button
                                class="text-stone-500 hover:text-stone-200 leading-none"
                                on:click=move |_| remove_tag(tag_for_remove.clone())
                            >
                                "×"
                            </button>
                        </span>
                    }
                }
            </For>
            <input
                type="text"
                class="bg-transparent text-xs text-stone-400 outline-none placeholder:text-stone-600 w-24"
                placeholder="Add tag…"
                prop:value=move || tag_input.get()
                on:input=move |ev| set_tag_input.set(event_target_value(&ev))
                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                    match ev.key().as_str() {
                        "Enter" => {
                            ev.prevent_default();
                            add_tag();
                        }
                        "," => {
                            ev.prevent_default();
                            add_tag();
                        }
                        _ => {}
                    }
                }
            />
        </div>
    }
}
