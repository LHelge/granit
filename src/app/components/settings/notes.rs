use super::SettingsForm;
use leptos::prelude::*;

#[component]
pub fn NotesSettings(form: RwSignal<SettingsForm>) -> impl IntoView {
    view! {
        <fieldset class="space-y-3">
            <legend class="text-xs font-semibold uppercase tracking-wider text-stone-400 mb-2">"Notes"</legend>
            <div class="space-y-1">
                <label class="block text-xs text-stone-400" for="daily-note-folder">"Daily note folder"</label>
                <p class="text-xs text-stone-500">"Folder (relative to cave root) where daily notes are stored. Default: Daily"</p>
                <input
                    id="daily-note-folder"
                    type="text"
                    placeholder="Daily"
                    class="w-full bg-stone-900 border border-stone-600 rounded px-3 py-1.5 text-sm text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors"
                    prop:value=move || form.get().daily_note_folder
                    on:input=move |ev| {
                        let v = event_target_value(&ev);
                        let folder = if v.trim().is_empty() {
                            "Daily".to_string()
                        } else {
                            v
                        };
                        form.update(|f| f.daily_note_folder = folder);
                    }
                />
            </div>
        </fieldset>
    }
}
