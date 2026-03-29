use leptos::prelude::*;

use super::font_picker::FontPicker;

#[component]
pub fn MarkdownSettings(
    fonts: ReadSignal<Vec<String>>,
    font_family: ReadSignal<String>,
    set_font_family: WriteSignal<String>,
    font_size: ReadSignal<u8>,
    set_font_size: WriteSignal<u8>,
) -> impl IntoView {
    view! {
        <fieldset class="space-y-3">
            <legend class="text-xs font-semibold uppercase tracking-wider text-stone-400 mb-2">"Markdown"</legend>

            <div class="space-y-1">
                <label class="block text-xs text-stone-400">"Font family"</label>
                <FontPicker
                    fonts=fonts
                    value=font_family
                    set_value=set_font_family
                    id="md-font-family"
                />
            </div>

            <div class="space-y-1">
                <label class="block text-xs text-stone-400" for="md-font-size">"Font size (px)"</label>
                <input
                    id="md-font-size"
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
        </fieldset>
    }
}
