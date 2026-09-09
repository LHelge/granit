use crate::app::{
    components::icons::Icon,
    editor::{EditOpen, OpenInEdit},
    ipc, AppCtx, DocumentKind,
};
use granit_types::{resolve_note_icon, DocumentMeta};
use leptos::prelude::*;

/// The Templates tab: note templates (markdown, rendered with Tera when a
/// note is created) above presentation templates (CSS for the presentation
/// window). Each section has its own header, new button and list; the two
/// lists scroll independently and share the vertical space.
#[component]
pub fn Templates() -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let open_in_edit = expect_context::<OpenInEdit>().0;
    let loading = RwSignal::new(false);

    // ── Note templates ─────────────────────────────────────────────

    let create_template = move |_| {
        leptos::task::spawn_local(async move {
            loading.set(true);
            match ipc::create_template("untitled").await {
                Ok(meta) => {
                    ctx.refresh_templates().await;
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

    let open_template = Callback::new(move |slug: String| {
        leptos::task::spawn_local(async move {
            match ipc::read_template(&slug).await {
                Ok(template) => ctx.set_active_template_document(template),
                Err(e) => {
                    ctx.push_error("templates", format!("Failed to open template: {e}"));
                }
            }
        });
    });

    let delete_template = Callback::new(move |slug: String| {
        leptos::task::spawn_local(async move {
            match ipc::delete_template(&slug).await {
                Ok(()) => {
                    if ctx.active_aux_slug(DocumentKind::Template).as_deref() == Some(&slug) {
                        ctx.clear_active_document();
                    }
                    ctx.refresh_templates().await;
                }
                Err(e) => {
                    ctx.push_error("templates", format!("Failed to delete template: {e}"));
                }
            }
        });
    });

    // ── Presentation templates ─────────────────────────────────────

    let create_presentation = move |_| {
        leptos::task::spawn_local(async move {
            loading.set(true);
            match ipc::create_presentation("untitled").await {
                Ok(meta) => {
                    ctx.refresh_presentations().await;
                    match ipc::read_presentation(&meta.slug).await {
                        Ok(presentation) => {
                            open_in_edit.set(EditOpen::EditFocusTitle);
                            ctx.set_active_presentation_document(presentation);
                        }
                        Err(e) => {
                            ctx.push_error(
                                "presentations",
                                format!("Failed to open presentation template: {e}"),
                            );
                        }
                    }
                }
                Err(e) => {
                    ctx.push_error(
                        "presentations",
                        format!("Failed to create presentation template: {e}"),
                    );
                }
            }
            loading.set(false);
        });
    };

    let open_presentation = Callback::new(move |slug: String| {
        leptos::task::spawn_local(async move {
            match ipc::read_presentation(&slug).await {
                Ok(presentation) => ctx.set_active_presentation_document(presentation),
                Err(e) => {
                    ctx.push_error(
                        "presentations",
                        format!("Failed to open presentation template: {e}"),
                    );
                }
            }
        });
    });

    let delete_presentation = Callback::new(move |slug: String| {
        leptos::task::spawn_local(async move {
            match ipc::delete_presentation(&slug).await {
                Ok(()) => {
                    if ctx.active_aux_slug(DocumentKind::Presentation).as_deref() == Some(&slug) {
                        ctx.clear_active_document();
                    }
                    ctx.refresh_presentations().await;
                }
                Err(e) => {
                    ctx.push_error(
                        "presentations",
                        format!("Failed to delete presentation template: {e}"),
                    );
                }
            }
        });
    });

    Effect::new(move |_| {
        if ctx.config.get().active_cave.is_some() {
            leptos::task::spawn_local(async move {
                ctx.refresh_templates().await;
                ctx.refresh_presentations().await;
            });
        } else {
            ctx.templates.set(Vec::new());
            ctx.presentations.set(Vec::new());
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
                // ── Note templates ─────────────────────────────────────────
                <div class="flex-1 min-h-0 flex flex-col">
                    <SectionHeader
                        title="Note templates"
                        subtitle="Stored in .granit/templates"
                        new_label="New note template"
                        loading=loading
                        on_new=create_template
                    />
                    <TemplateList
                        kind=DocumentKind::Template
                        items=ctx.templates
                        empty_label="No note templates yet"
                        delete_label="Delete template"
                        on_open=open_template
                        on_delete=delete_template
                    />
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
                </div>

                // ── Presentation templates ─────────────────────────────────
                <div class="flex-1 min-h-0 flex flex-col border-t border-base-content/10">
                    <SectionHeader
                        title="Presentation templates"
                        subtitle="Stored in .granit/presentations"
                        new_label="New presentation template"
                        loading=loading
                        on_new=create_presentation
                    />
                    <TemplateList
                        kind=DocumentKind::Presentation
                        items=ctx.presentations
                        empty_label="No presentation templates yet"
                        delete_label="Delete presentation template"
                        on_open=open_presentation
                        on_delete=delete_presentation
                    />
                </div>
            </Show>
        </div>
    }
}

/// Section header: title, directory subtitle, and a "new" button.
#[component]
fn SectionHeader(
    title: &'static str,
    subtitle: &'static str,
    new_label: &'static str,
    loading: RwSignal<bool>,
    on_new: impl Fn(leptos::ev::MouseEvent) + 'static,
) -> impl IntoView {
    view! {
        <div class="flex items-center justify-between gap-2 p-2 border-b border-base-content/10 shrink-0">
            <div>
                <p class="text-sm font-medium text-base-content/80">{title}</p>
                <p class="text-xs text-base-content/40">{subtitle}</p>
            </div>
            <button
                class="btn btn-ghost btn-xs btn-square text-base-content/60 hover:text-base-content"
                disabled=move || loading.get()
                title=new_label
                aria-label=new_label
                on:click=on_new
            >
                <span class="inline-flex w-3.5 h-3.5">
                    <Icon icon=icondata_lu::LuFilePlus width="100%" height="100%"/>
                </span>
            </button>
        </div>
    }
}

/// Scrollable list of templates of one kind. A row opens the template in
/// the editor; its trash button deletes it. Renaming happens through the
/// editor title, as for every aux document.
#[component]
fn TemplateList(
    kind: DocumentKind,
    items: RwSignal<Vec<DocumentMeta>>,
    empty_label: &'static str,
    delete_label: &'static str,
    on_open: Callback<String>,
    on_delete: Callback<String>,
) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();

    view! {
        <Show
            when=move || !items.get().is_empty()
            fallback=move || view! {
                <div class="flex-1 flex items-center justify-center p-4">
                    <p class="text-sm text-base-content/35 italic">{empty_label}</p>
                </div>
            }
        >
            <ul class="menu w-full menu-sm p-0 flex-1 overflow-y-auto min-h-0 flex-nowrap">
                {move || items.get().into_iter().map(|item| {
                    let slug = item.slug.clone();
                    let slug_open = slug.clone();
                    let slug_delete = slug.clone();
                    let slug_display = slug.clone();
                    // Presentation templates are CSS files with no icon of
                    // their own; note templates carry a frontmatter icon.
                    let icon = match kind {
                        DocumentKind::Presentation => icondata_lu::LuPalette,
                        _ => resolve_note_icon(&item.icon.clone().unwrap_or_default()),
                    };
                    let is_active = move || {
                        ctx.active_aux_slug(kind)
                            .map(|active| active == slug)
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
                                    on:click=move |_| on_open.run(slug_open.clone())
                                >
                                    <span class="inline-flex w-3.5 h-3.5 shrink-0 text-accent">
                                        <Icon icon=icon width="100%" height="100%"/>
                                    </span>
                                    <span class="truncate">{slug_display.clone()}</span>
                                </button>
                                <button
                                    class="btn btn-ghost btn-xs btn-square text-base-content/45 hover:text-error"
                                    title=delete_label
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        on_delete.run(slug_delete.clone());
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
    }
}
