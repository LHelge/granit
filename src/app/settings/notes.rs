use super::SettingsForm;
use crate::app::AppCtx;
use leptos::prelude::*;

#[component]
pub fn NotesSettings(form: RwSignal<SettingsForm>) -> impl IntoView {
    let app = expect_context::<AppCtx>();

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
            <div class="space-y-1">
                <label class="label text-xs text-base-content/50" for="daily-note-template">"Daily note template"</label>
                <p class="label text-xs text-base-content/35">"Optional template from .granit/templates used when a new daily note is created"</p>
                <select
                    id="daily-note-template"
                    class="select select-bordered select-sm w-full"
                    prop:value=move || form.get().daily_note_template_slug.unwrap_or_default()
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        form.update(|f| {
                            f.daily_note_template_slug = if value.trim().is_empty() {
                                None
                            } else {
                                Some(value)
                            };
                        });
                    }
                >
                    <option value="">"No template"</option>
                    {move || app.templates.get().into_iter().map(|template| {
                        let slug = template.slug;
                        let value = slug.clone();
                        view! {
                            <option value=value>{slug}</option>
                        }
                    }).collect_view()}
                </select>
                <Show when=move || app.templates.get().is_empty()>
                    <p class="label text-xs text-base-content/35">"Create a template from the Templates tab to make it selectable here."</p>
                </Show>
            </div>
        </fieldset>
    }
}
