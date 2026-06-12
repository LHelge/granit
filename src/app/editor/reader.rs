use super::use_editor_ctx;
use crate::app::{
    components::icons::Icon,
    ipc,
    markdown_links::{classify_markdown_link_target, MarkdownLinkTarget},
    AppCtx,
};
use granit_types::resolve_note_icon;
use leptos::prelude::*;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};

fn current_selection() -> Option<JsValue> {
    let window = web_sys::window()?;
    let get_selection = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("getSelection"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    let selection = get_selection.call0(window.as_ref()).ok()?;
    if selection.is_null() || selection.is_undefined() {
        return None;
    }

    Some(selection)
}

fn selection_text(selection: &JsValue) -> Option<String> {
    let to_string = js_sys::Reflect::get(selection, &JsValue::from_str("toString"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    let text = to_string.call0(selection).ok()?.as_string()?;

    (!text.trim().is_empty()).then_some(text)
}

fn selection_node(selection: &JsValue, key: &str) -> Option<web_sys::Node> {
    js_sys::Reflect::get(selection, &JsValue::from_str(key))
        .ok()?
        .dyn_into::<web_sys::Node>()
        .ok()
}

fn reader_contains_node(reader: &web_sys::HtmlDivElement, node: &web_sys::Node) -> bool {
    let Some(contains) = js_sys::Reflect::get(reader.as_ref(), &JsValue::from_str("contains"))
        .ok()
        .and_then(|value| value.dyn_into::<js_sys::Function>().ok())
    else {
        return false;
    };

    contains
        .call1(reader.as_ref(), node.as_ref())
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn current_reader_selection_text(reader: &web_sys::HtmlDivElement) -> Option<String> {
    let selection = current_selection()?;
    let text = selection_text(&selection)?;
    let anchor = selection_node(&selection, "anchorNode")?;
    let focus = selection_node(&selection, "focusNode")?;

    (reader_contains_node(reader, &anchor) && reader_contains_node(reader, &focus)).then_some(text)
}

fn sync_reader_selection(reader_ref: NodeRef<leptos::html::Div>, ctx: super::EditorCtx) {
    let Some(reader) = reader_ref.get() else {
        return;
    };
    let reader: &web_sys::HtmlDivElement = reader.as_ref();
    ctx.app
        .selected_note_text
        .set(current_reader_selection_text(reader));
}

/// Rendered preview of the active note with wiki-link navigation.
#[component]
pub(super) fn Reader() -> impl IntoView {
    let ctx = use_editor_ctx();
    let app_ctx = expect_context::<AppCtx>();
    let reader_ref = NodeRef::<leptos::html::Div>::new();

    // The document-level selectionchange listener is owned by this component:
    // the closure lives in `selection_listener` (`new_local` because `Closure`
    // is not Send) and is removed + dropped when the effect re-runs and on
    // unmount, so it never leaks and never lingers into writer mode, where it
    // would interfere with CM6 selection tracking.
    let selection_listener = StoredValue::new_local(None::<Closure<dyn FnMut(web_sys::Event)>>);

    let remove_selection_listener = move || {
        let Some(listener) = selection_listener
            .try_update_value(|stored| stored.take())
            .flatten()
        else {
            return;
        };
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            let _ = document.remove_event_listener_with_callback(
                "selectionchange",
                listener.as_ref().unchecked_ref(),
            );
        }
    };

    Effect::new(move |_| {
        let Some(reader) = reader_ref.get() else {
            return;
        };
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };

        // The effect re-runs when the node ref changes; replace the listener.
        remove_selection_listener();

        let reader_for_listener = reader.clone();
        let ctx_for_listener = ctx;
        let listener = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            let reader: &web_sys::HtmlDivElement = reader_for_listener.as_ref();
            ctx_for_listener
                .app
                .selected_note_text
                .set(current_reader_selection_text(reader));
        }) as Box<dyn FnMut(web_sys::Event)>);

        let _ = document
            .add_event_listener_with_callback("selectionchange", listener.as_ref().unchecked_ref());
        selection_listener.set_value(Some(listener));
    });

    on_cleanup(remove_selection_listener);

    // Intercept clicks on links and checkboxes in rendered markdown.
    // - Checkboxes toggle the underlying markdown via the backend.
    // - External links (http/https) open in the system browser.
    // - Wiki-links navigate within the app.
    let on_click = move |ev: leptos::ev::MouseEvent| {
        let Some(target) = ev.target() else { return };

        // --- Checkbox click: toggle via backend ---
        if let Some(checkbox) = target
            .dyn_ref::<web_sys::Element>()
            .and_then(|el| el.dyn_ref::<web_sys::HtmlInputElement>())
            .filter(|inp| inp.type_() == "checkbox")
        {
            ev.prevent_default();

            // The backend renderer emits `data-index="N"` on each interactive
            // checkbox. Prefer that stable index over DOM-order counting,
            // which would break if disabled or nested checkboxes are present.
            let index = checkbox
                .get_attribute("data-index")
                .and_then(|s| s.parse::<usize>().ok());
            let Some(index) = index else {
                // Missing or malformed index means we cannot safely locate
                // the todo line; ignore the click.
                return;
            };

            if let Some(slug) = ctx.active_note.get_untracked().map(|n| n.meta.slug.clone()) {
                let ctx_inner = ctx;
                let app = app_ctx;
                leptos::task::spawn_local(async move {
                    if let Err(e) = ipc::toggle_todo_by_index(&slug, index).await {
                        app.push_error("reader", format!("Failed to toggle todo: {e}"));
                        return;
                    }
                    // Re-render the note so the checkbox state reflects what's on disk
                    match ipc::render_note(&slug).await {
                        Ok(rendered) => ctx_inner.rendered_note.set(Some(rendered)),
                        Err(e) => {
                            app.push_error("reader", format!("Failed to re-render note: {e}"));
                        }
                    }
                });
            }
            return;
        }

        // --- Link click ---
        let Some(link) = classify_markdown_link_target(Some(target)) else {
            return;
        };

        ev.prevent_default();
        match link {
            MarkdownLinkTarget::External(url) => {
                let app = app_ctx;
                leptos::task::spawn_local(async move {
                    if let Err(e) = ipc::open_url(&url).await {
                        app.push_error("reader", format!("Failed to open link: {e}"));
                    }
                });
            }
            MarkdownLinkTarget::Wiki {
                slug,
                fragment,
                is_broken,
            } => ctx.navigate_wiki_link(slug, fragment, is_broken),
        }
    };

    view! {
        <div
            node_ref=reader_ref
            on:click=on_click
            on:keyup=move |_| sync_reader_selection(reader_ref, ctx)
        >
            <h1 class="!mt-0 !mb-1 flex items-center gap-2">
                {move || ctx.icon.get().map(|id| view! {
                    <span class="inline-flex w-6 h-6 shrink-0 text-accent">
                        <Icon icon=resolve_note_icon(&id) width="100%" height="100%"/>
                    </span>
                })}
                {move || ctx.favorite.get().unwrap_or(false).then(|| view! {
                    <span class="inline-flex w-5 h-5 shrink-0 text-warning" aria-label="Favorite note">
                        <Icon icon=icondata_lu::LuStar width="100%" height="100%"/>
                    </span>
                })}
                {move || ctx.rendered_note.get().map(|r| r.title).unwrap_or_default()}
            </h1>
            {move || {
                let note = ctx.rendered_note.get()?;
                let tags = note
                    .frontmatter
                    .map(|fm| fm.tags)
                    .unwrap_or_default();
                let backlinks = note.backlinks;
                let created = note.created_display;
                let modified = note.modified_display;
                let word_count = note.word_count;
                let reading_minutes = note.reading_minutes;
                if tags.is_empty()
                    && created.is_none()
                    && modified.is_none()
                    && backlinks.is_empty()
                    && word_count == 0
                {
                    return None;
                }
                Some(view! {
                    <div class="not-prose !mt-0 !mb-4 flex flex-col gap-0.5">
                        {(!tags.is_empty()).then(|| view! {
                            <div class="flex flex-wrap items-center gap-2 mb-2">
                                {tags.into_iter().map(|tag| view! {
                                    <span class="badge badge-ghost badge-sm">
                                        {tag}
                                    </span>
                                }).collect_view()}
                            </div>
                        })}
                        {created.map(|ts| view! {
                            <span class="text-xs italic text-base-content/35">{format!("Created: {ts}")}</span>
                        })}
                        {modified.map(|ts| view! {
                            <span class="text-xs italic text-base-content/35">{format!("Modified: {ts}")}</span>
                        })}
                        {(word_count > 0).then(|| {
                            let words = if word_count == 1 { "word" } else { "words" };
                            view! {
                                <span class="text-xs italic text-base-content/35">
                                    {format!("{word_count} {words} · ~{reading_minutes} min read")}
                                </span>
                            }
                        })}
                        {(!backlinks.is_empty()).then(|| {
                            let backlink_count = backlinks.len();
                            view! {
                                <details class="collapse rounded-none group -mx-2 mt-1">
                                    <summary class="collapse-title min-h-0 px-2 py-2 flex items-center justify-between gap-2 text-sm font-medium text-base-content/70">
                                        <span>{format!("Backlinks ({backlink_count})")}</span>
                                        <span class="inline-flex w-3.5 h-3.5 shrink-0 transition-transform rotate-180 group-open:rotate-0">
                                            <Icon icon=icondata_lu::LuChevronDown width="100%" height="100%"/>
                                        </span>
                                    </summary>
                                    <div class="collapse-content px-2 pt-0 pb-0">
                                        <div class="flex flex-col gap-1.5 pb-2">
                                            {backlinks.into_iter().map(|backlink| {
                                                let slug = backlink.slug.clone();
                                                let relative_path = backlink.relative_path.clone();
                                                let icon = backlink.icon.clone();
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="flex items-start gap-2 rounded-box px-2 py-1.5 text-left hover:bg-base-200/70"
                                                        on:click=move |_| ctx.navigate_wiki_link(slug.clone(), None, false)
                                                    >
                                                        <span class="inline-flex w-4 h-4 shrink-0 text-accent mt-0.5">
                                                            {icon.map(|id| view! {
                                                                <Icon icon=resolve_note_icon(&id) width="100%" height="100%"/>
                                                            }).unwrap_or_else(|| view! {
                                                                <Icon icon=icondata_lu::LuLink width="100%" height="100%"/>
                                                            })}
                                                        </span>
                                                        <span class="min-w-0 flex flex-col">
                                                            <span class="truncate text-sm text-base-content">{backlink.slug}</span>
                                                            <span class="truncate text-xs text-base-content/45">{relative_path}</span>
                                                        </span>
                                                    </button>
                                                }
                                            }).collect_view()}
                                        </div>
                                    </div>
                                </details>
                            }
                        })}
                    </div>
                })
            }}
            <div
                style:font-family=move || ctx.config.get().reading_font.font_family
                style:font-size=move || format!("{}px", ctx.config.get().reading_font.font_size)
                inner_html=move || ctx.rendered_note.get().map(|r| r.html).unwrap_or_default()
            />
        </div>
    }
}
