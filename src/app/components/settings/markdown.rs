use super::{font_picker::FontPicker, SettingsForm};
use leptos::prelude::Callback;
use leptos::prelude::*;

#[component]
pub fn MarkdownSettings(form: RwSignal<SettingsForm>) -> impl IntoView {
    let fonts = Memo::new(move |_| form.get().system_fonts);
    let font_family = Memo::new(move |_| form.get().markdown_font.font_family);
    let font_size = Memo::new(move |_| form.get().markdown_font.font_size);

    view! {
        <fieldset class="space-y-3">
            <legend class="text-xs font-semibold uppercase tracking-wider text-stone-400 mb-2">"Markdown"</legend>

            <div class="space-y-1">
                <label class="block text-xs text-stone-400">"Font family"</label>
                <FontPicker
                    fonts=fonts
                    value=font_family
                    set_value=Callback::new(move |v| form.update(|f| f.markdown_font.font_family = v))
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
                            form.update(|f| f.markdown_font.font_size = v);
                        }
                    }
                />
            </div>
        </fieldset>
    }
}
