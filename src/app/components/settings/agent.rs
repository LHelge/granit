use leptos::prelude::*;

use super::font_picker::FontPicker;
use crate::app::components::icons::ChevronDownIcon;

#[component]
pub fn AgentSettings(
    provider: ReadSignal<String>,
    set_provider: WriteSignal<String>,
    model: ReadSignal<String>,
    set_model: WriteSignal<String>,
    base_url: ReadSignal<String>,
    set_base_url: WriteSignal<String>,
    api_key: ReadSignal<String>,
    set_api_key: WriteSignal<String>,
    api_key_is_set: ReadSignal<bool>,
    fonts: ReadSignal<Vec<String>>,
    font_family: ReadSignal<String>,
    set_font_family: WriteSignal<String>,
    font_size: ReadSignal<u8>,
    set_font_size: WriteSignal<u8>,
) -> impl IntoView {
    // When provider changes, reset model to a sensible default
    let on_provider_change = move |ev: leptos::ev::Event| {
        let new_provider = event_target_value(&ev);
        set_provider.set(new_provider.clone());
        match new_provider.as_str() {
            "ollama" => set_model.set("qwen3.5:9b".to_string()),
            "anthropic" => set_model.set("claude-sonnet-4-20250514".to_string()),
            _ => {}
        }
    };

    let is_ollama = move || provider.get() == "ollama";
    let is_anthropic = move || provider.get() == "anthropic";

    view! {
        <fieldset class="space-y-3">
            <legend class="text-xs font-semibold uppercase tracking-wider text-stone-400 mb-2">"Agent"</legend>

            // Font family
            <div class="space-y-1">
                <label class="block text-xs text-stone-400">"Font family"</label>
                <FontPicker
                    fonts=fonts
                    value=font_family
                    set_value=set_font_family
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
                            set_font_size.set(v);
                        }
                    }
                />
            </div>

            <hr class="border-stone-600" />

            // Provider selector
            <div class="space-y-1">
                <label class="block text-xs text-stone-400" for="settings-provider">"Provider"</label>
                <div class="relative">
                    <select
                        id="settings-provider"
                        class="w-full appearance-none bg-stone-900 border border-stone-600 rounded px-3 py-1.5 pr-8 text-sm text-stone-200 outline-none focus:border-stone-400 transition-colors cursor-pointer"
                        on:change=on_provider_change
                        prop:value=move || provider.get()
                    >
                        <option class="bg-stone-900 text-stone-200" value="ollama" selected=is_ollama>"Ollama"</option>
                        <option class="bg-stone-900 text-stone-200" value="anthropic" selected=is_anthropic>"Anthropic"</option>
                    </select>
                    // Custom chevron
                    <ChevronDownIcon class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-stone-400" />
                </div>
            </div>

            // Model name (always shown)
            <div class="space-y-1">
                <label class="block text-xs text-stone-400" for="settings-model">"Model"</label>
                <input
                    id="settings-model"
                    type="text"
                    class="w-full bg-stone-900 border border-stone-600 rounded px-3 py-1.5 text-sm text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors"
                    placeholder=move || if is_ollama() { "qwen3.5:9b" } else { "claude-sonnet-4-20250514" }
                    prop:value=move || model.get()
                    on:input=move |ev| set_model.set(event_target_value(&ev))
                />
            </div>

            // Ollama-specific: Base URL
            <Show when=is_ollama>
                <div class="space-y-1">
                    <label class="block text-xs text-stone-400" for="settings-base-url">"Base URL"</label>
                    <input
                        id="settings-base-url"
                        type="text"
                        class="w-full bg-stone-900 border border-stone-600 rounded px-3 py-1.5 text-sm text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors"
                        placeholder="http://localhost:11434"
                        prop:value=move || base_url.get()
                        on:input=move |ev| set_base_url.set(event_target_value(&ev))
                    />
                    <p class="text-xs text-stone-500">"Leave blank to use the default (http://localhost:11434)"</p>
                </div>
            </Show>

            // Anthropic-specific: API key
            <Show when=is_anthropic>
                <div class="space-y-1">
                    <label class="block text-xs text-stone-400" for="settings-api-key">"API Key"</label>
                    <input
                        id="settings-api-key"
                        type="password"
                        class="w-full bg-stone-900 border border-stone-600 rounded px-3 py-1.5 text-sm text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors"
                        placeholder=move || if api_key_is_set.get() { "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022} (configured)" } else { "sk-ant-..." }
                        prop:value=move || api_key.get()
                        on:input=move |ev| set_api_key.set(event_target_value(&ev))
                    />
                    <p class="text-xs text-stone-500">
                        {move || if api_key_is_set.get() {
                            "Key is configured. Enter a new value to replace it."
                        } else {
                            "Stored in secrets.env, never in config.yml."
                        }}
                    </p>
                </div>
            </Show>
        </fieldset>
    }
}
