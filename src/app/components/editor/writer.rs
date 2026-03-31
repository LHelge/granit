use leptos::prelude::*;
use leptos::web_sys::wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use super::use_editor_ctx;

fn request_animation_frame(f: impl FnOnce() + 'static) {
    let cb = Closure::once_into_js(f);
    let _ = leptos::web_sys::window()
        .unwrap()
        .request_animation_frame(cb.as_ref().unchecked_ref());
}

/// Raw markdown editor with title input and content textarea.
#[component]
pub(super) fn Writer() -> impl IntoView {
    let ctx = use_editor_ctx();
    let title_ref = NodeRef::<leptos::html::Input>::new();

    // Focus and select the title input when requested.
    Effect::new(move || {
        if ctx.focus_title.get() {
            ctx.focus_title.set(false);
            // Defer to next frame so the DOM value is populated before selecting.
            request_animation_frame(move || {
                if let Some(el) = title_ref.get() {
                    let input: &web_sys::HtmlInputElement = el.as_ref();
                    let _ = input.focus();
                    input.select();
                }
            });
        }
    });

    view! {
        <input
            type="text"
            node_ref=title_ref
            class="not-prose w-full bg-transparent text-white text-4xl font-extrabold leading-tight outline-none mb-2"
            placeholder="Untitled"
            prop:value=move || ctx.title_input.get()
            on:input=move |ev| ctx.title_input.set(event_target_value(&ev))
        />
        <textarea
            class="not-prose w-full flex-1 bg-transparent text-stone-300 resize-none outline-none leading-relaxed"
            placeholder="Start writing..."
            style:font-family=move || ctx.config.get().markdown_font.font_family
            style:font-size=move || format!("{}px", ctx.config.get().markdown_font.font_size)
            prop:value=move || ctx.content.get()
            on:input=move |ev| ctx.content.set(event_target_value(&ev))
        />
    }
}
