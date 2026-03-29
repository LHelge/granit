use leptos::prelude::*;

#[component]
pub fn ReadingSettings() -> impl IntoView {
    view! {
        <fieldset class="space-y-3">
            <legend class="text-xs font-semibold uppercase tracking-wider text-stone-400 mb-2">"Reading"</legend>
            <p class="text-sm text-stone-500">"Font settings for the reading preview will be available here."</p>
        </fieldset>
    }
}
