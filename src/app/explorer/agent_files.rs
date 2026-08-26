use crate::app::{
    components::icons::Icon,
    editor::{EditOpen, OpenInEdit},
    ipc, AppCtx, DocumentKind,
};
use granit_types::{AgentDocInfo, Document, DocumentMeta};
use leptos::prelude::*;

/// Which agent-document list a section shows. Only `Skill` and `Task` are
/// valid here; the dispatch helpers below map to the matching IPC calls.
async fn read_doc(kind: DocumentKind, name: &str) -> Result<Document, String> {
    match kind {
        DocumentKind::Skill => ipc::read_skill(name).await,
        _ => ipc::read_task(name).await,
    }
}

async fn create_doc(kind: DocumentKind, name: &str) -> Result<DocumentMeta, String> {
    match kind {
        DocumentKind::Skill => ipc::create_skill(name).await,
        _ => ipc::create_task(name).await,
    }
}

async fn delete_doc(kind: DocumentKind, name: &str) -> Result<(), String> {
    match kind {
        DocumentKind::Skill => ipc::delete_skill(name).await,
        _ => ipc::delete_task(name).await,
    }
}

async fn refresh_docs(ctx: AppCtx, kind: DocumentKind) {
    match kind {
        DocumentKind::Skill => ctx.refresh_skills().await,
        _ => ctx.refresh_tasks().await,
    }
}

struct SectionMeta {
    label: &'static str,
    subtitle: &'static str,
    noun: &'static str,
    untitled: &'static str,
    icon: icondata_core::Icon,
}

fn section_meta(kind: DocumentKind) -> SectionMeta {
    match kind {
        DocumentKind::Skill => SectionMeta {
            label: "Skills",
            subtitle: "Stored in .granit/agent/skills",
            noun: "skill",
            untitled: "untitled-skill",
            icon: icondata_lu::LuBrain,
        },
        _ => SectionMeta {
            label: "Tasks",
            subtitle: "Stored in .granit/agent/tasks",
            noun: "task",
            untitled: "untitled-task",
            icon: icondata_lu::LuZap,
        },
    }
}

/// One collapsible-free list section for skills or tasks: header with a
/// create button, rows with name + description, delete buttons.
#[component]
fn AgentDocSection(kind: DocumentKind) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let open_in_edit = expect_context::<OpenInEdit>().0;
    let loading = RwSignal::new(false);
    let meta = section_meta(kind);
    let (noun, untitled, row_icon) = (meta.noun, meta.untitled, meta.icon);

    let items = move || match kind {
        DocumentKind::Skill => ctx.skills.get(),
        _ => ctx.tasks.get(),
    };

    let create = move |_| {
        leptos::task::spawn_local(async move {
            loading.set(true);
            match create_doc(kind, untitled).await {
                Ok(meta) => {
                    refresh_docs(ctx, kind).await;
                    match read_doc(kind, &meta.slug).await {
                        Ok(doc) => {
                            open_in_edit.set(EditOpen::EditFocusTitle);
                            ctx.set_active_aux_document(kind, doc);
                        }
                        Err(e) => {
                            ctx.push_error("agent-files", format!("Failed to open {noun}: {e}"));
                        }
                    }
                }
                Err(e) => {
                    ctx.push_error("agent-files", format!("Failed to create {noun}: {e}"));
                }
            }
            loading.set(false);
        });
    };

    view! {
        <div class="flex items-center justify-between gap-2 p-2 border-b border-base-content/10">
            <div>
                <p class="text-sm font-medium text-base-content/80">{meta.label}</p>
                <p class="text-xs text-base-content/40">{meta.subtitle}</p>
            </div>
            <button
                class="btn btn-ghost btn-xs btn-square text-base-content/60 hover:text-base-content"
                disabled=move || loading.get()
                title=format!("New {noun}")
                aria-label=format!("New {noun}")
                on:click=create
            >
                <span class="inline-flex w-3.5 h-3.5">
                    <Icon icon=icondata_lu::LuFilePlus width="100%" height="100%"/>
                </span>
            </button>
        </div>

        <Show
            when=move || !items().is_empty()
            fallback=move || view! {
                <p class="p-3 text-sm text-base-content/35 italic">{format!("No {noun}s yet")}</p>
            }
        >
            <ul class="menu w-full menu-sm p-0 overflow-y-auto flex-nowrap">
                {move || items().into_iter().map(|doc: AgentDocInfo| {
                    let name = doc.name.clone();
                    let name_open = name.clone();
                    let name_delete = name.clone();
                    let name_display = name.clone();
                    let description = doc.description.clone();
                    let is_active = move || {
                        ctx.active_aux_slug(kind)
                            .map(|active| active == name)
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
                                        let s = name_open.clone();
                                        leptos::task::spawn_local(async move {
                                            match read_doc(kind, &s).await {
                                                Ok(doc) => ctx.set_active_aux_document(kind, doc),
                                                Err(e) => {
                                                    ctx.push_error("agent-files", format!("Failed to open {noun}: {e}"));
                                                }
                                            }
                                        });
                                    }
                                >
                                    <span class="inline-flex w-3.5 h-3.5 shrink-0 text-accent">
                                        <Icon icon=row_icon width="100%" height="100%"/>
                                    </span>
                                    <span class="min-w-0 flex flex-col">
                                        <span class="truncate">{name_display.clone()}</span>
                                        <Show when={
                                            let description = description.clone();
                                            move || !description.is_empty()
                                        }>
                                            <span class="truncate text-xs text-base-content/45">{description.clone()}</span>
                                        </Show>
                                    </span>
                                </button>
                                <button
                                    class="btn btn-ghost btn-xs btn-square text-base-content/45 hover:text-error"
                                    title=format!("Delete {noun}")
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        let s = name_delete.clone();
                                        leptos::task::spawn_local(async move {
                                            match delete_doc(kind, &s).await {
                                                Ok(()) => {
                                                    if ctx.active_aux_slug(kind).map(|active| active == s).unwrap_or(false) {
                                                        ctx.clear_active_document();
                                                    }
                                                    refresh_docs(ctx, kind).await;
                                                }
                                                Err(e) => {
                                                    ctx.push_error("agent-files", format!("Failed to delete {noun}: {e}"));
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
    }
}

/// Explorer tab for the agent's user-authored files in `.granit/agent/`:
/// the system prompt, skills, and tasks.
#[component]
pub fn AgentFiles() -> impl IntoView {
    let ctx = expect_context::<AppCtx>();

    let open_system_prompt = move |_| {
        leptos::task::spawn_local(async move {
            match ipc::read_system_prompt().await {
                Ok(doc) => ctx.set_active_aux_document(DocumentKind::SystemPrompt, doc),
                Err(e) => {
                    ctx.push_error("agent-files", format!("Failed to open system prompt: {e}"));
                }
            }
        });
    };

    Effect::new(move |_| {
        if ctx.config.get().active_cave.is_some() {
            leptos::task::spawn_local(async move {
                ctx.refresh_skills().await;
                ctx.refresh_tasks().await;
            });
        } else {
            ctx.skills.set(Vec::new());
            ctx.tasks.set(Vec::new());
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
                <div class="flex-1 overflow-y-auto">
                    // ── System prompt ─────────────────────────────
                    <div class="p-2 border-b border-base-content/10">
                        <p class="text-sm font-medium text-base-content/80">"System prompt"</p>
                        <p class="text-xs text-base-content/40">"Stored in .granit/agent/system.md"</p>
                        <ul class="menu w-full menu-sm p-0 mt-1">
                            <li>
                                <button
                                    class=move || {
                                        if ctx.active_aux_slug(DocumentKind::SystemPrompt).is_some() {
                                            "flex w-full items-center gap-2 rounded-none bg-base-content/10 text-base-content"
                                        } else {
                                            "flex w-full items-center gap-2 rounded-none text-base-content/70 hover:bg-base-content/5 hover:text-base-content"
                                        }
                                    }
                                    on:click=open_system_prompt
                                >
                                    <span class="inline-flex w-3.5 h-3.5 shrink-0 text-accent">
                                        <Icon icon=icondata_lu::LuBot width="100%" height="100%"/>
                                    </span>
                                    <span class="truncate">"system"</span>
                                </button>
                            </li>
                        </ul>
                    </div>

                    <AgentDocSection kind=DocumentKind::Skill />
                    <AgentDocSection kind=DocumentKind::Task />
                </div>

                // ── Help ──────────────────────────────────────────
                <div class="shrink-0 border-t border-base-content/10">
                    <details class="collapse rounded-none group">
                        <summary class="collapse-title flex items-center justify-between gap-2 py-2 text-sm font-medium text-base-content/70">
                            <span>"Agent Files Help"</span>
                            <span class="inline-flex w-3.5 h-3.5 shrink-0 transition-transform rotate-180 group-open:rotate-0">
                                <Icon icon=icondata_lu::LuChevronDown width="100%" height="100%"/>
                            </span>
                        </summary>
                        <div class="collapse-content pt-0 pb-3 text-xs text-base-content/55">
                            <div class="space-y-3">
                                <div>
                                    <p class="font-medium text-base-content/75">"System prompt"</p>
                                    <p class="mt-1 leading-relaxed">
                                        "A Tera template rendered when the agent starts. Available variables:"
                                    </p>
                                    <div class="mt-1 flex flex-wrap gap-1.5">
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ mode }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ tools }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ icons }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ skills }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ rag }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ today }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ weekday }}"</span>
                                    </div>
                                </div>
                                <div>
                                    <p class="font-medium text-base-content/75">"Skills"</p>
                                    <p class="mt-1 leading-relaxed">
                                        "Each skill is a folder with a SKILL.md following the Agent Skills format: YAML frontmatter with name (matching the folder, kebab-case) and description, then the instructions. The agent sees every name and description and loads the full instructions on demand with the use_skill tool. Only SKILL.md is used; scripts and reference files are ignored."
                                    </p>
                                </div>
                                <div>
                                    <p class="font-medium text-base-content/75">"Tasks"</p>
                                    <p class="mt-1 leading-relaxed">
                                        "Markdown files with a description in the frontmatter, invoked from the chat by typing / followed by the task name. The body is a Tera template that becomes the prompt. Available variables:"
                                    </p>
                                    <div class="mt-1 flex flex-wrap gap-1.5">
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ input }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ active_note }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ today }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ tomorrow }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ yesterday }}"</span>
                                        <span class="badge badge-ghost badge-sm font-mono">"{{ weekday }}"</span>
                                    </div>
                                    <p class="mt-1 leading-relaxed">"{{ active_note }} is only set while a note is open."</p>
                                </div>
                            </div>
                        </div>
                    </details>
                </div>
            </Show>
        </div>
    }
}
