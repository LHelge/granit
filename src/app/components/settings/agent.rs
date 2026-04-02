use leptos::prelude::*;

use super::font_picker::FontPicker;
use super::{ProviderFormEntry, SettingsForm};
use crate::app::components::icons::{
    ChevronDownIcon, EyeIcon, EyeSlashIcon, PlusIcon, ProviderIcon, TrashIcon,
};
use leptos::prelude::Callback;

#[component]
pub fn AgentSettings(form: RwSignal<SettingsForm>) -> impl IntoView {
    // Derived read signals for the font picker
    let fonts = Memo::new(move |_| form.get().system_fonts);
    let font_family = Memo::new(move |_| form.get().agent_font.font_family);
    let font_size = Memo::new(move |_| form.get().agent_font.font_size);

    let add_provider = move |_| {
        form.update(|f| {
            f.providers.push(ProviderFormEntry::new_default("ollama"));
        });
    };

    view! {
        <fieldset class="space-y-3">
            <legend class="text-xs font-semibold uppercase tracking-wider text-stone-400 mb-2">"Agent"</legend>

            // Font family
            <div class="space-y-1">
                <label class="block text-xs text-stone-400">"Font family"</label>
                <FontPicker
                    fonts=fonts
                    value=font_family
                    set_value=Callback::new(move |v| form.update(|f| f.agent_font.font_family = v))
                    id="ag-font-family"
                />
            </div>

            // Font size
            <div class="space-y-1">
                <label class="block text-xs text-stone-400" for="ag-font-size">"Font size (px)"</label>
                <input
                    id="ag-font-size"
                    type="number"
                    min="8"
                    max="48"
                    class="w-full bg-stone-900 border border-stone-600 rounded px-3 py-1.5 text-sm text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors"
                    prop:value=move || font_size.get().to_string()
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<u8>() {
                            form.update(|f| f.agent_font.font_size = v);
                        }
                    }
                />
            </div>

            <hr class="border-stone-600" />

            // Provider list header
            <div class="flex items-center justify-between">
                <span class="text-xs font-semibold uppercase tracking-wider text-stone-400">"Providers"</span>
                <button
                    type="button"
                    class="flex items-center gap-1 text-xs text-stone-400 hover:text-stone-200 transition-colors"
                    on:click=add_provider
                >
                    <PlusIcon />
                    "Add"
                </button>
            </div>

            // Provider entries
            {move || {
                let count = form.get().providers.len();
                (0..count).map(|idx| {
                    view! { <ProviderRow form=form index=idx /> }
                }).collect_view()
            }}

            <Show when=move || form.get().providers.is_empty()>
                <p class="text-xs text-stone-500 italic">"No providers configured. Click Add to create one."</p>
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
        <div class="border border-stone-700 rounded p-2.5 space-y-2 bg-stone-800/40">
            // Top row: type selector + name + remove button
            <div class="flex items-center gap-2">
                <div class="relative shrink-0">
                    <button
                        type="button"
                        class="flex items-center gap-1.5 px-2 py-1 text-xs bg-stone-900 border border-stone-600 rounded hover:border-stone-500 transition-colors text-stone-200 cursor-pointer"
                        on:click=move |_| set_type_open.update(|v| *v = !*v)
                    >
                        <ProviderIcon provider_type=Signal::derive(provider_type) class="w-3.5 h-3.5 shrink-0" />
                        <span>{move || PROVIDER_TYPES.iter().find(|(k, _)| *k == provider_type()).map(|(_, l)| *l).unwrap_or("Select")}</span>
                        <ChevronDownIcon class="w-3 h-3 shrink-0 text-stone-400" open=Signal::derive(move || type_open.get()) />
                    </button>
                    <Show when=move || type_open.get()>
                        <div class="absolute top-full left-0 mt-1 bg-stone-800 border border-stone-600 rounded shadow-lg z-50 min-w-[10rem]">
                            {PROVIDER_TYPES.iter().map(|(ptype, label)| {
                                let ptype_str = *ptype;
                                let label_str = *label;
                                let ptype_owned = ptype.to_string();
                                view! {
                                    <button
                                        type="button"
                                        class="w-full flex items-center gap-1.5 px-3 py-1.5 text-xs text-stone-300 hover:bg-stone-700 transition-colors"
                                        class=("bg-stone-700/50", move || provider_type() == ptype_str)
                                        on:click=move |_| on_type_select(ptype_str)
                                    >
                                        <ProviderIcon provider_type=Signal::stored(ptype_owned.clone()) class="w-3.5 h-3.5 shrink-0" />
                                        {label_str}
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    </Show>
                </div>
                <input
                    type="text"
                    class="flex-1 min-w-0 bg-stone-900 border border-stone-600 rounded px-2 py-1 text-xs text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors"
                    placeholder=move || format!("Name (default: {})", type_label())
                    prop:value=move || form.get().providers.get(index).map(|p| p.name.clone()).unwrap_or_default()
                    on:input=on_name_input
                />
                <button
                    type="button"
                    class="shrink-0 p-1 rounded text-stone-500 hover:text-red-400 hover:bg-stone-700 transition-colors"
                    title="Remove provider"
                    on:click=on_remove
                >
                    <TrashIcon />
                </button>
            </div>

            // Base URL (Ollama only)
            <Show when=needs_base_url>
                <input
                    type="text"
                    class="w-full bg-stone-900 border border-stone-600 rounded px-2 py-1 text-xs text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors"
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
                        class="flex-1 min-w-0 bg-stone-900 border border-stone-600 rounded px-2 py-1 text-xs text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors font-mono"
                        placeholder="API key"
                        prop:value=move || form.get().providers.get(index).map(|p| p.api_key.clone()).unwrap_or_default()
                        on:input=on_api_key_input
                    />
                    <button
                        type="button"
                        class="shrink-0 p-1 rounded text-stone-500 hover:text-stone-200 transition-colors"
                        title=move || if show_key.get() { "Hide API key" } else { "Show API key" }
                        on:click=move |_| set_show_key.update(|v| *v = !*v)
                    >
                        {move || if show_key.get() {
                            view! { <EyeSlashIcon /> }.into_any()
                        } else {
                            view! { <EyeIcon /> }.into_any()
                        }}
                    </button>
                </div>
            </Show>
        </div>
    }
}
