use super::SettingsForm;
use crate::app::{components::icons::Icon, AppCtx};
use granit_types::resolve_note_icon;
use leptos::prelude::*;

#[component]
pub fn NotesSettings(form: RwSignal<SettingsForm>) -> impl IntoView {
    let app = expect_context::<AppCtx>();
    let folder_open = RwSignal::new(false);
    let template_open = RwSignal::new(false);

    view! {
        <fieldset class="fieldset space-y-3">
            <legend class="fieldset-legend">"Notes"</legend>

            // Daily note folder
            <div class="space-y-1">
                <label class="label text-xs text-base-content/50" for="daily-note-folder">"Daily note folder"</label>
                <p class="label text-xs text-base-content/35">"Folder (relative to cave root) where daily notes are stored. Default: Daily"</p>
                <div class="relative w-full">
                    <button
                        id="daily-note-folder"
                        type="button"
                        class="flex items-center gap-2 input input-bordered input-sm w-full text-left cursor-pointer"
                        on:click=move |_| {
                            folder_open.update(|v| *v = !*v);
                            template_open.set(false);
                        }
                    >
                        <span class="inline-flex w-3.5 h-3.5 shrink-0 text-base-content/50">
                            <Icon icon=icondata_lu::LuFolder width="100%" height="100%"/>
                        </span>
                        <span class="truncate text-sm">{move || form.get().daily_note_folder}</span>
                        <span
                            class="inline-flex w-3 h-3 ml-auto shrink-0 text-base-content/40 transition-transform"
                            class:rotate-180=move || folder_open.get()
                        >
                            <Icon icon=icondata_lu::LuChevronDown width="100%" height="100%"/>
                        </span>
                    </button>
                    <Show when=move || folder_open.get()>
                        <>
                            <div class="fixed inset-0 z-40" on:click=move |ev| { ev.stop_propagation(); folder_open.set(false); }/>
                            <ul class="absolute top-full left-0 z-50 w-full mt-1 menu menu-sm bg-base-200 border border-base-content/15 rounded-box shadow-md p-1 max-h-48 overflow-y-auto">
                                {move || {
                                    let folders = app.folders.get();
                                    if folders.is_empty() {
                                        view! {
                                            <li><span class="text-base-content/40 italic">"No folders in cave"</span></li>
                                        }.into_any()
                                    } else {
                                        folders.into_iter().map(|path| {
                                            let path_set = path.clone();
                                            let is_selected = move || form.get().daily_note_folder == path;
                                            view! {
                                                <li>
                                                    <button
                                                        type="button"
                                                        class=move || if is_selected() {
                                                            "flex items-center gap-2 bg-base-content/10"
                                                        } else {
                                                            "flex items-center gap-2"
                                                        }
                                                        on:click=move |_| {
                                                            let p = path_set.clone();
                                                            form.update(|f| f.daily_note_folder = p);
                                                            folder_open.set(false);
                                                        }
                                                    >
                                                        <span class="inline-flex w-3.5 h-3.5 shrink-0 text-base-content/50">
                                                            <Icon icon=icondata_lu::LuFolder width="100%" height="100%"/>
                                                        </span>
                                                        <span class="truncate">{path_set.clone()}</span>
                                                    </button>
                                                </li>
                                            }
                                        }).collect_view().into_any()
                                    }
                                }}
                            </ul>
                        </>
                    </Show>
                </div>
            </div>

            // Daily note template
            <div class="space-y-1">
                <label class="label text-xs text-base-content/50" for="daily-note-template">"Daily note template"</label>
                <p class="label text-xs text-base-content/35">"Optional template from .granit/templates used when a new daily note is created"</p>
                <div class="relative w-full">
                    <button
                        id="daily-note-template"
                        type="button"
                        class="flex items-center gap-2 input input-bordered input-sm w-full text-left cursor-pointer"
                        on:click=move |_| {
                            template_open.update(|v| *v = !*v);
                            folder_open.set(false);
                        }
                    >
                        {move || {
                            let slug = form.get().daily_note_template_slug;
                            match slug {
                                None => view! {
                                    <span class="text-base-content/40 text-sm grow">"No template"</span>
                                }.into_any(),
                                Some(s) => {
                                    let icon_id = app.templates.get()
                                        .into_iter()
                                        .find(|t| t.slug == s)
                                        .and_then(|t| t.icon)
                                        .unwrap_or_default();
                                    view! {
                                        <span class="inline-flex w-3.5 h-3.5 shrink-0 text-accent">
                                            <Icon icon=resolve_note_icon(&icon_id) width="100%" height="100%"/>
                                        </span>
                                        <span class="truncate text-sm grow">{s}</span>
                                    }.into_any()
                                }
                            }
                        }}
                        <span
                            class="inline-flex w-3 h-3 ml-auto shrink-0 text-base-content/40 transition-transform"
                            class:rotate-180=move || template_open.get()
                        >
                            <Icon icon=icondata_lu::LuChevronDown width="100%" height="100%"/>
                        </span>
                    </button>
                    <Show when=move || template_open.get()>
                        <>
                            <div class="fixed inset-0 z-40" on:click=move |ev| { ev.stop_propagation(); template_open.set(false); }/>
                            <ul class="absolute top-full left-0 z-50 w-full mt-1 menu menu-sm bg-base-200 border border-base-content/15 rounded-box shadow-md p-1 max-h-48 overflow-y-auto">
                                <li>
                                    <button
                                        type="button"
                                        class=move || if form.get().daily_note_template_slug.is_none() {
                                            "flex items-center gap-2 text-base-content/50 bg-base-content/10"
                                        } else {
                                            "flex items-center gap-2 text-base-content/50"
                                        }
                                        on:click=move |_| {
                                            form.update(|f| f.daily_note_template_slug = None);
                                            template_open.set(false);
                                        }
                                    >
                                        "No template"
                                    </button>
                                </li>
                                {move || app.templates.get().into_iter().map(|template| {
                                    let slug = template.slug.clone();
                                    let slug_set = slug.clone();
                                    let icon_id = template.icon.clone().unwrap_or_default();
                                    let is_selected = move || {
                                        form.get().daily_note_template_slug.as_deref() == Some(&slug)
                                    };
                                    view! {
                                        <li>
                                            <button
                                                type="button"
                                                class=move || if is_selected() {
                                                    "flex items-center gap-2 bg-base-content/10"
                                                } else {
                                                    "flex items-center gap-2"
                                                }
                                                on:click=move |_| {
                                                    let s = slug_set.clone();
                                                    form.update(|f| f.daily_note_template_slug = Some(s));
                                                    template_open.set(false);
                                                }
                                            >
                                                <span class="inline-flex w-3.5 h-3.5 shrink-0 text-accent">
                                                    <Icon icon=resolve_note_icon(&icon_id) width="100%" height="100%"/>
                                                </span>
                                                <span class="truncate">{slug_set.clone()}</span>
                                            </button>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        </>
                    </Show>
                </div>
                <Show when=move || app.templates.get().is_empty()>
                    <p class="label text-xs text-base-content/35">"Create a template from the Templates tab to make it selectable here."</p>
                </Show>
            </div>
        </fieldset>
    }
}
