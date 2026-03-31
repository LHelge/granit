use leptos::prelude::*;

use super::use_editor_ctx;

/// Raw markdown editor with title input and content textarea.
#[component]
pub(super) fn Writer() -> impl IntoView {
    let ctx = use_editor_ctx();

    view! {
        <input
            type="text"
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
