use super::SettingsForm;
use crate::app::components::font_selector::FontSelector;
use leptos::prelude::*;

#[component]
pub fn MarkdownSettings(form: RwSignal<SettingsForm>) -> impl IntoView {
    let fonts = Memo::new(move |_| form.get().system_fonts);
    let font_family = Memo::new(move |_| form.get().markdown_font.font_family);
    let font_size = Memo::new(move |_| form.get().markdown_font.font_size);

    view! {
        <fieldset class="fieldset space-y-3">
            <legend class="fieldset-legend">"Markdown"</legend>
            <FontSelector
                fonts=fonts
                font_family=font_family
                set_font_family=Callback::new(move |v| form.update(|f| f.markdown_font.font_family = v))
                font_size=font_size
                set_font_size=Callback::new(move |v| form.update(|f| f.markdown_font.font_size = v))
                id="md"
            />
        </fieldset>
    }
}
