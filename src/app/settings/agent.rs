use super::{ProviderFormEntry, SettingsForm};
use crate::app::components::{
    font_selector::FontSelector,
    icons::{Icon, ProviderIcon},
};
use granit_types::default_system_prompt;
use leptos::prelude::*;

#[component]
pub fn AgentSettings(form: RwSignal<SettingsForm>) -> impl IntoView {
    // Derived read signals for the font picker
    let fonts = Memo::new(move |_| form.get().system_fonts);
    let font_family = Memo::new(move |_| form.get().agent_font.font_family);
    let font_size = Memo::new(move |_| form.get().agent_font.font_size);

    let add_provider = move |_| {
        form.update(|f| {
            let id = f.next_provider_id;
            f.next_provider_id += 1;
            f.providers
                .push(ProviderFormEntry::new_default(id, "ollama"));
        });
    };

    let provider_rows = move || {
        form.get()
            .providers
            .into_iter()
            .enumerate()
            .collect::<Vec<_>>()
    };

    view! {
        <fieldset class="fieldset space-y-3">
            <legend class="fieldset-legend">"Agent"</legend>

            // Font
            <FontSelector
                fonts=fonts
                font_family=font_family
                set_font_family=Callback::new(move |v| form.update(|f| f.agent_font.font_family = v))
                font_size=font_size
                set_font_size=Callback::new(move |v| form.update(|f| f.agent_font.font_size = v))
                id="ag"
            />

            <div class="divider my-1" />

            // Max history
            <div class="space-y-1">
                <label class="label text-xs text-base-content/50" for="ag-max-history">"Max history (messages)"</label>
                <input
                    id="ag-max-history"
                    type="number"
                    min="1"
                    max="10000"
                    class="input input-bordered input-sm w-full"
                    prop:value=move || form.get().max_history.to_string()
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<usize>() {
                            form.update(|f| f.max_history = v);
                        }
                    }
                />
            </div>

            // Max turns
            <div class="space-y-1">
                <label class="label text-xs text-base-content/50" for="ag-max-turns">"Max turns (tool-call rounds)"</label>
                <input
                    id="ag-max-turns"
                    type="number"
                    min="1"
                    max="100"
                    class="input input-bordered input-sm w-full"
                    prop:value=move || form.get().max_turns.to_string()
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<usize>() {
                            form.update(|f| f.max_turns = v);
                        }
                    }
                />
            </div>

            // System prompt
            <div class="space-y-1">
                <label class="label text-xs text-base-content/50" for="ag-system-prompt">"System prompt"</label>
                <textarea
                    id="ag-system-prompt"
                    class="textarea textarea-bordered textarea-sm w-full h-24 font-mono text-xs"
                    placeholder="Enter a custom system prompt…"
                    prop:value=move || form.get().system_prompt.clone()
                    on:input=move |ev| {
                        let val = event_target_value(&ev);
                        form.update(|f| f.system_prompt = val);
                    }
                />
                <button
                    type="button"
                    class="btn btn-ghost btn-xs"
                    on:click=move |_| {
                        form.update(|f| f.system_prompt = default_system_prompt());
                    }
                >
                    "Reset to default"
                </button>
            </div>

            <div class="divider my-1" />

            // Tools
            <div class="space-y-2">
                <span class="text-xs font-semibold uppercase tracking-wider text-base-content/50">"Tools"</span>
                {move || {
                    let tools = form.get().available_tools;
                    if tools.is_empty() {
                        view! { <p class="text-xs text-base-content/35 italic">"Loading tools\u{2026}"</p> }.into_any()
                    } else {
                        tools.into_iter().map(|tool| {
                            let tool_name = tool.name.clone();
                            let tool_name_toggle = tool.name.clone();
                            let is_enabled = move || {
                                !form.get().disabled_tools.contains(&tool_name)
                            };
                            let on_toggle = move |_| {
                                let name = tool_name_toggle.clone();
                                form.update(|f| {
                                    if f.disabled_tools.contains(&name) {
                                        f.disabled_tools.retain(|t| t != &name);
                                    } else {
                                        f.disabled_tools.push(name);
                                    }
                                });
                            };
                            view! {
                                <label class="flex items-center gap-2 cursor-pointer">
                                    <input
                                        type="checkbox"
                                        class="toggle toggle-sm toggle-primary"
                                        prop:checked=is_enabled
                                        on:change=on_toggle
                                    />
                                    <div>
                                        <span class="text-xs font-mono">{tool.name.clone()}</span>
                                        <p class="text-xs text-base-content/40">{tool.description.clone()}</p>
                                    </div>
                                </label>
                            }
                        }).collect_view().into_any()
                    }
                }}
            </div>

            // Brave API key (for web search tool)
            <div class="space-y-1">
                <label class="label text-xs text-base-content/50" for="ag-brave-key">"Brave Search API key"</label>
                <input
                    id="ag-brave-key"
                    type="password"
                    class="input input-bordered input-sm w-full font-mono text-xs"
                    placeholder="BSA-…"
                    prop:value=move || form.get().brave_api_key.clone()
                    on:input=move |ev| {
                        let val = event_target_value(&ev);
                        form.update(|f| f.brave_api_key = val);
                    }
                />
                <p class="text-xs text-base-content/35">"Required for the web_search tool."</p>
            </div>

            <div class="divider my-1" />

            // Provider list header
            <div class="flex items-center justify-between">
                <span class="text-xs font-semibold uppercase tracking-wider text-base-content/50">"Providers"</span>
                <button
                    type="button"
                    class="btn btn-ghost btn-xs gap-1"
                    on:click=add_provider
                >
                    <Icon icon=icondata_lu::LuPlus width="1rem" height="1rem"/>
                    "Add"
                </button>
            </div>

            // Provider entries
            <For
                each=provider_rows
                key=|(_, provider)| provider.id
                let:provider_row
            >
                {
                    let (idx, _) = provider_row;
                    view! { <ProviderRow form=form index=idx /> }
                }
            </For>

            <Show when=move || form.get().providers.is_empty()>
                <p class="text-xs text-base-content/35 italic">"No providers configured. Click Add to create one."</p>
            </Show>
        </fieldset>
    }
}

/// A single provider editor row.
#[component]
fn ProviderRow(form: RwSignal<SettingsForm>, index: usize) -> impl IntoView {
    let (show_key, set_show_key) = signal(false);
    let (type_open, set_type_open) = signal(false);

    /// All available provider types with labels.
    const PROVIDER_TYPES: &[(&str, &str)] = &[
        ("ollama", "Ollama"),
        ("anthropic", "Anthropic"),
        ("mistral", "Mistral"),
        ("prisma", "Prisma"),
    ];

    let provider_type = move || {
        form.get()
            .providers
            .get(index)
            .map(|p| p.provider_type.clone())
            .unwrap_or_default()
    };
    let needs_api_key = move || {
        form.get()
            .providers
            .get(index)
            .map(|p| p.needs_api_key())
            .unwrap_or(false)
    };
    let needs_base_url = move || {
        form.get()
            .providers
            .get(index)
            .map(|p| p.needs_base_url())
            .unwrap_or(false)
    };
    let type_label = move || {
        form.get()
            .providers
            .get(index)
            .map(|p| p.type_label().to_string())
            .unwrap_or_default()
    };

    let on_type_select = move |new_type: &str| {
        let new_type = new_type.to_string();
        set_type_open.set(false);
        form.update(|f| {
            if let Some(p) = f.providers.get_mut(index) {
                p.provider_type = new_type;
                p.api_key.clear();
                p.base_url.clear();
            }
        });
    };

    let on_name_input = move |ev: leptos::ev::Event| {
        let val = event_target_value(&ev);
        form.update(|f| {
            if let Some(p) = f.providers.get_mut(index) {
                p.name = val;
            }
        });
    };

    let on_base_url_input = move |ev: leptos::ev::Event| {
        let val = event_target_value(&ev);
        form.update(|f| {
            if let Some(p) = f.providers.get_mut(index) {
                p.base_url = val;
            }
        });
    };

    let on_api_key_input = move |ev: leptos::ev::Event| {
        let val = event_target_value(&ev);
        form.update(|f| {
            if let Some(p) = f.providers.get_mut(index) {
                p.api_key = val;
            }
        });
    };

    let on_remove = move |_| {
        form.update(|f| {
            if index < f.providers.len() {
                f.providers.remove(index);
            }
        });
    };

    view! {
        <div class="border border-base-content/10 rounded p-2.5 space-y-2 bg-base-300/40">
            // Top row: type selector + name + remove button
            <div class="flex items-center gap-2">
                <div class="relative shrink-0">
                    <button
                        type="button"
                        class="flex items-center gap-1.5 px-2 py-1 text-xs bg-base-100 border border-base-content/20 rounded hover:border-base-content/30 transition-colors text-base-content cursor-pointer"
                        on:click=move |_| set_type_open.update(|v| *v = !*v)
                    >
                        <ProviderIcon provider_type=Signal::derive(provider_type) class="w-3.5 h-3.5 shrink-0" />
                        <span>{move || PROVIDER_TYPES.iter().find(|(k, _)| *k == provider_type()).map(|(_, l)| *l).unwrap_or("Select")}</span>
                        <span
                            class="inline-flex w-3 h-3 shrink-0 text-base-content/50 transition-transform"
                            class:rotate-180=move || type_open.get()
                        >
                            <Icon icon=icondata_lu::LuChevronDown width="100%" height="100%"/>
                        </span>
                    </button>
                    <Show when=move || type_open.get()>
                        <ul class="menu menu-xs absolute top-full left-0 mt-1 bg-base-300 border border-base-content/20 rounded shadow-lg z-50 min-w-[10rem] p-1">
                            {PROVIDER_TYPES.iter().map(|(ptype, label)| {
                                let ptype_str = *ptype;
                                let label_str = *label;
                                let ptype_owned = ptype.to_string();
                                view! {
                                    <li>
                                        <button
                                            type="button"
                                            class=move || if provider_type() == ptype_str { "menu-active" } else { "" }
                                            on:click=move |_| on_type_select(ptype_str)
                                        >
                                            <ProviderIcon provider_type=Signal::stored(ptype_owned.clone()) class="w-3.5 h-3.5 shrink-0" />
                                            {label_str}
                                        </button>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    </Show>
                </div>
                <input
                    type="text"
                    class="input input-bordered input-xs flex-1 min-w-0"
                    placeholder=move || format!("Name (default: {})", type_label())
                    prop:value=move || form.get().providers.get(index).map(|p| p.name.clone()).unwrap_or_default()
                    on:input=on_name_input
                />
                <div class="tooltip tooltip-left" data-tip="Remove provider">
                    <button
                        type="button"
                        class="btn btn-ghost btn-xs btn-square shrink-0 text-base-content/35 hover:text-error!"
                        on:click=on_remove
                    >
                        <Icon icon=icondata_lu::LuTrash2 width="1rem" height="1rem"/>
                    </button>
                </div>
            </div>

            // Base URL (Ollama only)
            <Show when=needs_base_url>
                <input
                    type="text"
                    class="input input-bordered input-xs w-full"
                    placeholder="Base URL (default: http://localhost:11434)"
                    prop:value=move || form.get().providers.get(index).map(|p| p.base_url.clone()).unwrap_or_default()
                    on:input=on_base_url_input
                />
            </Show>

            // API key (Anthropic, Mistral, Prisma)
            <Show when=needs_api_key>
                <div class="flex items-center gap-1">
                    <input
                        type=move || if show_key.get() { "text" } else { "password" }
                        class="input input-bordered input-xs flex-1 min-w-0 font-mono"
                        placeholder="API key"
                        prop:value=move || form.get().providers.get(index).map(|p| p.api_key.clone()).unwrap_or_default()
                        on:input=on_api_key_input
                    />
                    <div class="tooltip tooltip-left" data-tip=move || if show_key.get() { "Hide API key" } else { "Show API key" }>
                        <button
                            type="button"
                            class="btn btn-ghost btn-xs btn-square shrink-0"
                            on:click=move |_| set_show_key.update(|v| *v = !*v)
                        >
                            {move || if show_key.get() {
                                view! { <Icon icon=icondata_lu::LuEyeOff width="1rem" height="1rem"/> }.into_any()
                            } else {
                                view! { <Icon icon=icondata_lu::LuEye width="1rem" height="1rem"/> }.into_any()
                            }}
                        </button>
                    </div>
                </div>
            </Show>
        </div>
    }
}
