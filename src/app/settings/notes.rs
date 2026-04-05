use super::SettingsForm;
use leptos::prelude::*;

#[component]
pub fn NotesSettings(form: RwSignal<SettingsForm>) -> impl IntoView {
    view! {
        <fieldset class="fieldset space-y-3">
            <legend class="fieldset-legend">"Notes"</legend>
            <div class="space-y-1">
                <label class="label text-xs text-base-content/50" for="daily-note-folder">"Daily note folder"</label>
                <p class="label text-xs text-base-content/35">"Folder (relative to cave root) where daily notes are stored. Default: Daily"</p>
                <input
                    id="daily-note-folder"
                    type="text"
                    placeholder="Daily"
                    class="input input-bordered input-sm w-full"
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
