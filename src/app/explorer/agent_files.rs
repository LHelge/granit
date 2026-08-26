use crate::app::{
    components::icons::Icon,
    editor::{EditOpen, OpenInEdit},
    ipc, AppCtx, DocumentKind,
};
use leptos::prelude::*;

/// Explorer tab for the agent's user-authored files in `.granit/agent/`:
/// the system prompt and skills.
#[component]
pub fn AgentFiles() -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let open_in_edit = expect_context::<OpenInEdit>().0;
    let loading = RwSignal::new(false);

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

    let create_skill = move |_| {
        leptos::task::spawn_local(async move {
            loading.set(true);
            match ipc::create_skill("untitled-skill").await {
                Ok(meta) => {
                    ctx.refresh_skills().await;
                    match ipc::read_skill(&meta.slug).await {
                        Ok(skill) => {
                            open_in_edit.set(EditOpen::EditFocusTitle);
                            ctx.set_active_aux_document(DocumentKind::Skill, skill);
                        }
                        Err(e) => {
                            ctx.push_error("agent-files", format!("Failed to open skill: {e}"));
                        }
                    }
                }
                Err(e) => {
                    ctx.push_error("agent-files", format!("Failed to create skill: {e}"));
                }
            }
            loading.set(false);
        });
    };

    Effect::new(move |_| {
        if ctx.config.get().active_cave.is_some() {
            leptos::task::spawn_local(async move {
                ctx.refresh_skills().await;
            });
        } else {
            ctx.skills.set(Vec::new());
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
                // ── System prompt ─────────────────────────────────
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

                // ── Skills ────────────────────────────────────────
                <div class="flex items-center justify-between gap-2 p-2 border-b border-base-content/10">
                    <div>
                        <p class="text-sm font-medium text-base-content/80">"Skills"</p>
                        <p class="text-xs text-base-content/40">"Stored in .granit/agent/skills"</p>
                    </div>
                    <button
                        class="btn btn-ghost btn-xs btn-square text-base-content/60 hover:text-base-content"
                        disabled=move || loading.get()
                        title="New skill"
                        aria-label="New skill"
                        on:click=create_skill
                    >
                        <span class="inline-flex w-3.5 h-3.5">
                            <Icon icon=icondata_lu::LuFilePlus width="100%" height="100%"/>
                        </span>
                    </button>
                </div>

                <Show
                    when=move || !ctx.skills.get().is_empty()
                    fallback=|| view! {
                        <div class="flex-1 flex items-center justify-center p-4">
                            <p class="text-sm text-base-content/35 italic">"No skills yet"</p>
                        </div>
                    }
                >
                    <ul class="menu w-full menu-sm p-0 flex-1 overflow-y-auto flex-nowrap">
                        {move || ctx.skills.get().into_iter().map(|skill| {
                            let name = skill.name.clone();
                            let name_open = name.clone();
                            let name_delete = name.clone();
                            let name_display = name.clone();
                            let description = skill.description.clone();
                            let is_active = move || {
                                ctx.active_aux_slug(DocumentKind::Skill)
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
                                                    match ipc::read_skill(&s).await {
                                                        Ok(skill) => ctx.set_active_aux_document(DocumentKind::Skill, skill),
                                                        Err(e) => {
                                                            ctx.push_error("agent-files", format!("Failed to open skill: {e}"));
                                                        }
                                                    }
                                                });
                                            }
                                        >
                                            <span class="inline-flex w-3.5 h-3.5 shrink-0 text-accent">
                                                <Icon icon=icondata_lu::LuBrain width="100%" height="100%"/>
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
                                            title="Delete skill"
                                            on:click=move |ev| {
                                                ev.stop_propagation();
                                                let s = name_delete.clone();
                                                leptos::task::spawn_local(async move {
                                                    match ipc::delete_skill(&s).await {
                                                        Ok(()) => {
                                                            if ctx.active_aux_slug(DocumentKind::Skill).map(|active| active == s).unwrap_or(false) {
                                                                ctx.clear_active_document();
                                                            }
                                                            ctx.refresh_skills().await;
                                                        }
                                                        Err(e) => {
                                                            ctx.push_error("agent-files", format!("Failed to delete skill: {e}"));
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
                            </div>
                        </div>
                    </details>
                </div>
            </Show>
        </div>
    }
}
