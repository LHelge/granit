use leptos::prelude::*;

#[component]
pub fn MarkdownSettings() -> impl IntoView {
    view! {
        <fieldset class="space-y-3">
            <legend class="text-xs font-semibold uppercase tracking-wider text-stone-400 mb-2">"Markdown"</legend>
            <p class="text-sm text-stone-500">"Font settings for the markdown editor will be available here."</p>
        </fieldset>
    }
}
