use leptos::prelude::*;

use super::font_picker::FontPicker;
use super::SettingsForm;
use crate::app::components::icons::ChevronDownIcon;
use leptos::prelude::Callback;

#[component]
pub fn AgentSettings(form: RwSignal<SettingsForm>) -> impl IntoView {
    // Derived read signals for the font picker
    let fonts = Memo::new(move |_| form.get().system_fonts);
    let font_family = Memo::new(move |_| form.get().agent_font.font_family);
    let font_size = Memo::new(move |_| form.get().agent_font.font_size);

    // When provider changes, reset model to a sensible default
    let on_provider_change = move |ev: leptos::ev::Event| {
        let new_provider = event_target_value(&ev);
        let default_model = match new_provider.as_str() {
            "ollama" => "qwen3.5:9b",
            "anthropic" => "claude-sonnet-4-20250514",
            "mistral" => "mistral-small-latest",
            _ => "",
        };
        form.update(|f| {
            f.provider = new_provider;
            f.model = default_model.to_string();
        });
    };

    let is_ollama = move || form.get().provider == "ollama";
    let is_anthropic = move || form.get().provider == "anthropic";
    let needs_api_key = move || {
        let p = form.get().provider;
        p == "anthropic" || p == "mistral"
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

            // Provider selector
            <div class="space-y-1">
                <label class="block text-xs text-stone-400" for="settings-provider">"Provider"</label>
                <div class="relative">
                    <select
                        id="settings-provider"
                        class="w-full appearance-none bg-stone-900 border border-stone-600 rounded px-3 py-1.5 pr-8 text-sm text-stone-200 outline-none focus:border-stone-400 transition-colors cursor-pointer"
                        on:change=on_provider_change
                        prop:value=move || form.get().provider
                    >
                        <option class="bg-stone-900 text-stone-200" value="ollama" selected=is_ollama>"Ollama"</option>
                        <option class="bg-stone-900 text-stone-200" value="anthropic" selected=is_anthropic>"Anthropic"</option>
                        <option class="bg-stone-900 text-stone-200" value="mistral" selected=move || form.get().provider == "mistral">"Mistral"</option>
                    </select>
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
                    placeholder=move || match form.get().provider.as_str() {
                        "ollama" => "qwen3.5:9b".to_string(),
                        "anthropic" => "claude-sonnet-4-20250514".to_string(),
                        "mistral" => "mistral-small-latest".to_string(),
                        _ => String::new(),
                    }
                    prop:value=move || form.get().model
                    on:input=move |ev| form.update(|f| f.model = event_target_value(&ev))
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
                        prop:value=move || form.get().base_url
                        on:input=move |ev| form.update(|f| f.base_url = event_target_value(&ev))
                    />
                    <p class="text-xs text-stone-500">"Leave blank to use the default (http://localhost:11434)"</p>
                </div>
            </Show>

            // API key (Anthropic and Mistral)
            <Show when=needs_api_key>
                <div class="space-y-1">
                    <label class="block text-xs text-stone-400" for="settings-api-key">"API Key"</label>
                    <input
                        id="settings-api-key"
                        type="password"
                        class="w-full bg-stone-900 border border-stone-600 rounded px-3 py-1.5 text-sm text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors"
                        placeholder=move || if form.get().api_key_is_set { "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022} (configured)" } else { "sk-ant-..." }
                        prop:value=move || form.get().api_key
                        on:input=move |ev| form.update(|f| f.api_key = event_target_value(&ev))
                    />
                    <p class="text-xs text-stone-500">
                        {move || if form.get().api_key_is_set {
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
