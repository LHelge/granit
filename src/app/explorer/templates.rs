use crate::app::{
    components::icons::Icon,
    editor::{EditOpen, OpenInEdit},
    ipc, AppCtx,
};
use granit_types::resolve_note_icon;
use leptos::prelude::*;

#[component]
pub fn Templates() -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let open_in_edit = expect_context::<OpenInEdit>().0;
    let loading = RwSignal::new(false);

    let refresh_templates = move || {
        leptos::task::spawn_local(async move {
            match ipc::fetch_templates().await {
                Ok(templates) => {
                    ctx.clear_source("templates");
                    ctx.templates.set(templates);
                }
                Err(e) => {
                    ctx.push_error("templates", format!("Failed to load templates: {e}"));
                }
            }
        });
    };

    let create_template = move |_| {
        leptos::task::spawn_local(async move {
            loading.set(true);
            match ipc::create_template("untitled").await {
                Ok(meta) => {
                    if let Ok(templates) = ipc::fetch_templates().await {
                        ctx.templates.set(templates);
                    }
                    match ipc::read_template(&meta.slug).await {
                        Ok(template) => {
                            open_in_edit.set(EditOpen::EditFocusTitle);
                            ctx.set_active_template_document(template);
                        }
                        Err(e) => {
                            ctx.push_error("templates", format!("Failed to open template: {e}"));
                        }
                    }
                }
                Err(e) => {
                    ctx.push_error("templates", format!("Failed to create template: {e}"));
                }
            }
            loading.set(false);
        });
    };

    Effect::new(move |_| {
        if ctx.config.get().active_cave.is_some() {
            refresh_templates();
        } else {
            ctx.templates.set(Vec::new());
        }
    });

    view! {
        <div class="flex flex-col h-full">
            <Show
                when=move || ctx.config.get().active_cave.is_some()
                fallback=|| view! {
                    <div class="flex-1 flex items-center justify-center p-4">
                        <p class="text-sm text-base-content/35 italic">"No cave open"</p>
                    </div>
                }
            >
                <div class="flex items-center justify-between gap-2 p-2 border-b border-base-content/10">
                    <div>
                        <p class="text-sm font-medium text-base-content/80">"Templates"</p>
                        <p class="text-xs text-base-content/40">"Stored in .granit/templates"</p>
                    </div>
                    <button
                        class="btn btn-ghost btn-xs btn-square text-base-content/60 hover:text-base-content"
                        disabled=move || loading.get()
                        title="New template"
                        aria-label="New template"
                        on:click=create_template
                    >
                        <span class="inline-flex w-3.5 h-3.5">
                            <Icon icon=icondata_lu::LuFilePlus width="100%" height="100%"/>
                        </span>
                    </button>
                </div>

                <Show
                    when=move || !ctx.templates.get().is_empty()
                    fallback=|| view! {
                        <div class="flex-1 flex items-center justify-center p-4">
                            <p class="text-sm text-base-content/35 italic">"No templates yet"</p>
                        </div>
                    }
                >
                    <ul class="menu w-full menu-sm p-0 flex-1 overflow-y-auto">
                        {move || ctx.templates.get().into_iter().map(|template| {
                            let slug = template.slug.clone();
                            let slug_open = slug.clone();
                            let slug_delete = slug.clone();
                            let slug_display = slug.clone();
                            let icon_id = template.icon.clone().unwrap_or_default();
                            let is_active = move || {
                                ctx.active_template
                                    .get()
                                    .map(|active| active.meta.slug == slug)
                                    .unwrap_or(false)
                            };
                            view! {
                                <li>
                                    <div
                                        class=move || {
                                            if is_active() {
                                                "flex w-full items-center gap-2 rounded-none bg-base-content/10 text-base-content"
                                            } else {
                                                "flex w-full items-center gap-2 rounded-none text-base-content/70 hover:bg-base-content/5 hover:text-base-content"
                                            }
                                        }
                                    >
                                        <button
                                            class="flex flex-1 items-center gap-2 text-left min-w-0 w-full"
                                            on:click=move |_| {
                                                let s = slug_open.clone();
                                                leptos::task::spawn_local(async move {
                                                    match ipc::read_template(&s).await {
                                                        Ok(template) => ctx.set_active_template_document(template),
                                                        Err(e) => {
                                                            ctx.push_error("templates", format!("Failed to open template: {e}"));
                                                        }
                                                    }
                                                });
                                            }
                                        >
                                            <span class="inline-flex w-3.5 h-3.5 shrink-0 text-accent">
                                                <Icon icon=resolve_note_icon(&icon_id) width="100%" height="100%"/>
                                            </span>
                                            <span class="truncate">{slug_display.clone()}</span>
                                        </button>
                                        <button
                                            class="btn btn-ghost btn-xs btn-square text-base-content/45 hover:text-error"
                                            title="Delete template"
                                            on:click=move |ev| {
                                                ev.stop_propagation();
                                                let s = slug_delete.clone();
                                                leptos::task::spawn_local(async move {
                                                    match ipc::delete_template(&s).await {
                                                        Ok(()) => {
                                                            if ctx.active_template.get().map(|active| active.meta.slug == s).unwrap_or(false) {
                                                                ctx.clear_active_document();
                                                            }
                                                            if let Ok(templates) = ipc::fetch_templates().await {
                                                                ctx.templates.set(templates);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            ctx.push_error("templates", format!("Failed to delete template: {e}"));
                                                        }
                                                    }
                                                });
                                            }
                                        >
                                            <Icon icon=icondata_lu::LuTrash2 width="0.875rem" height="0.875rem"/>
                                        </button>
                                    </div>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </Show>

                <div class="shrink-0 border-t border-base-content/10">
                    <details class="collapse rounded-none group">
                        <summary class="collapse-title flex items-center justify-between gap-2 py-2 text-sm font-medium text-base-content/70">
                            <span>"Template Parameters"</span>
                            <span class="inline-flex w-3.5 h-3.5 shrink-0 transition-transform rotate-180 group-open:rotate-0">
                                <Icon icon=icondata_lu::LuChevronDown width="100%" height="100%"/>
                            </span>
                        </summary>
                        <div class="collapse-content pt-0 pb-3 text-xs text-base-content/55">
                            <div class="space-y-3">
                                <div>
                                    <p class="font-medium text-base-content/75">"Available in all note templates"</p>
                                    <div class="mt-1 flex flex-wrap gap-1.5">
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ slug }}"</span>
                                    </div>
                                    <p class="mt-1 leading-relaxed">"The note slug, usually the filename without .md."</p>
                                </div>

                                <div>
                                    <p class="font-medium text-base-content/75">"Added for daily notes"</p>
                                    <div class="mt-1 flex flex-wrap gap-1.5">
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ date }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ tomorrow }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ yesterday }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ year }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ month }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ day }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ weekday }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ weekday_short }}"</span>
                                    </div>
                                    <p class="mt-1 leading-relaxed">"These are only available when the created note slug matches the daily-note date format YYYY-MM-DD."</p>
                                </div>
                            </div>
                        </div>
                    </details>
                </div>
            </Show>
        </div>
    }
}
