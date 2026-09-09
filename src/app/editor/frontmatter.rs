use super::use_editor_ctx;
use crate::app::components::icons::Icon;
use leptos::prelude::*;

fn normalize_tag(raw: &str) -> Option<String> {
    let tag = raw.trim().to_lowercase();
    if tag.is_empty() {
        return None;
    }
    Some(tag)
}

fn add_tag(tags: &mut Vec<String>, raw: &str) {
    let Some(tag) = normalize_tag(raw) else {
        return;
    };

    if !tags.contains(&tag) {
        tags.push(tag);
    }
}

fn remove_tag(tags: &mut Vec<String>, tag: &str) {
    tags.retain(|existing| existing != tag);
}

/// Value of the `<option>` that unbinds a note from any presentation
/// template.
const NO_PRESENTATION: &str = "";

/// Inline frontmatter editor shown between the title and content textarea.
///
/// Shows a tag editor (removable pills + add input) and, for notes, the
/// presentation template dropdown. The icon picker is rendered separately
/// in [`Writer`], beside the title.
#[component]
pub(super) fn FrontmatterEditor() -> impl IntoView {
    let ctx = use_editor_ctx();

    // ── Presentation template ────────────────────────────────────────────────

    let is_note = move || ctx.active_note.get().is_some();

    // A note may name a template that no longer exists (renamed or deleted):
    // keep showing it as a broken entry instead of silently clearing it.
    let missing_presentation = move || {
        let current = ctx.presentation.get()?;
        let known = ctx
            .app
            .presentations
            .get()
            .iter()
            .any(|p| p.slug == current);
        (!known).then_some(current)
    };

    let select_presentation = move |ev: leptos::ev::Event| {
        let value = event_target_value(&ev);
        ctx.presentation
            .set((value != NO_PRESENTATION).then_some(value));
        ctx.schedule_autosave();
    };

    // ── Tag state ────────────────────────────────────────────────────────────

    let (tag_input, set_tag_input) = signal(String::new());

    let add_tag = move || {
        let input = tag_input.get_untracked();
        if normalize_tag(&input).is_none() {
            return;
        }

        let mut tags = ctx.tags.get_untracked();
        add_tag(&mut tags, &input);
        ctx.tags.set(tags);
        ctx.schedule_autosave();
        set_tag_input.set(String::new());
    };

    let remove_tag = move |tag: String| {
        let mut tags = ctx.tags.get_untracked();
        remove_tag(&mut tags, &tag);
        ctx.tags.set(tags);
        ctx.schedule_autosave();
    };

    view! {
        <div class="not-prose flex flex-wrap items-center gap-1.5 mb-3">
            // ── Tag editor ────────────────────────────────────────────────────
            <For
                each=move || ctx.tags.get()
                key=|tag| tag.clone()
                let:tag
            >
                {
                    let tag_for_remove = tag.clone();
                    view! {
                        <span class="badge badge-ghost badge-sm gap-1">
                            {tag.clone()}
                            <button
                                class="text-base-content/35 hover:text-base-content leading-none"
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
                class="bg-transparent text-xs text-base-content/50 outline-none placeholder:text-base-content/20 w-24"
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
            // ── Presentation template ─────────────────────────────────────────
            <Show when=is_note>
                <span class="ml-auto inline-flex items-center gap-1.5 text-xs text-base-content/50">
                    <span class="inline-flex w-3.5 h-3.5">
                        <Icon icon=icondata_lu::LuPresentation width="100%" height="100%"/>
                    </span>
                    <select
                        class="select select-ghost select-xs w-auto min-w-0 text-xs"
                        aria-label="Presentation template"
                        on:change=select_presentation
                        prop:value=move || ctx.presentation.get().unwrap_or_default()
                    >
                        <option value=NO_PRESENTATION>"No presentation"</option>
                        {move || missing_presentation().map(|slug| view! {
                            <option value=slug.clone()>{format!("{slug} (missing)")}</option>
                        })}
                        {move || ctx.app.presentations.get().into_iter().map(|p| {
                            let label = p.slug.clone();
                            view! { <option value=p.slug>{label}</option> }
                        }).collect_view()}
                    </select>
                </span>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn normalize_tag_trims_and_lowercases() {
        assert_eq!(normalize_tag("  Work Log  "), Some("work log".into()));
    }

    #[wasm_bindgen_test]
    fn normalize_tag_rejects_blank_input() {
        assert_eq!(normalize_tag("   \n\t  "), None);
    }

    #[wasm_bindgen_test]
    fn add_tag_appends_normalized_value_once() {
        let mut tags = vec!["daily".into()];

        add_tag(&mut tags, "  NewTag  ");
        add_tag(&mut tags, "newtag");

        assert_eq!(tags, vec!["daily", "newtag"]);
    }

    #[wasm_bindgen_test]
    fn add_tag_ignores_empty_values() {
        let mut tags = vec!["daily".into()];

        add_tag(&mut tags, "   ");

        assert_eq!(tags, vec!["daily"]);
    }

    #[wasm_bindgen_test]
    fn remove_tag_deletes_matching_entries() {
        let mut tags = vec!["daily".into(), "project".into(), "daily".into()];

        remove_tag(&mut tags, "daily");

        assert_eq!(tags, vec!["project"]);
    }
}
