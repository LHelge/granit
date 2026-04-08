use super::{font_picker::FontPicker, icons::Icon};
use leptos::prelude::*;

/// Combined font family picker + size stepper used across settings pages.
#[component]
pub fn FontSelector(
    /// All available system font families.
    #[prop(into)]
    fonts: Signal<Vec<String>>,
    /// Currently selected font family.
    #[prop(into)]
    font_family: Signal<String>,
    /// Called when the user selects a new font family.
    #[prop(into)]
    set_font_family: Callback<String>,
    /// Currently selected font size in pixels.
    #[prop(into)]
    font_size: Signal<u8>,
    /// Called when the user changes the font size.
    #[prop(into)]
    set_font_size: Callback<u8>,
    /// HTML id prefix used for label association (e.g. "md" → "md-font-family").
    #[prop(into)]
    id: String,
) -> impl IntoView {
    let id_family = format!("{id}-font-family");
    let id_size = format!("{id}-font-size");

    let step_down = move |_: leptos::ev::MouseEvent| {
        let current = font_size.get_untracked();
        if current > 8 {
            set_font_size.run(current - 1);
        }
    };
    let step_up = move |_: leptos::ev::MouseEvent| {
        let current = font_size.get_untracked();
        if current < 48 {
            set_font_size.run(current + 1);
        }
    };

    view! {
        // Font family
        <div class="space-y-1">
            <label class="label text-xs text-base-content/50" for=id_family.clone()>"Font family"</label>
            <FontPicker
                fonts=fonts
                value=font_family
                set_value=set_font_family
                id=id_family
            />
        </div>

        // Font size
        <div class="space-y-1">
            <label class="label text-xs text-base-content/50" for=id_size.clone()>"Font size (px)"</label>
            <div class="join w-full">
                <button
                    type="button"
                    class="join-item btn btn-sm btn-ghost border border-base-content/20"
                    title="Decrease font size"
                    prop:disabled=move || font_size.get() <= 8
                    on:click=step_down
                >
                    <span class="inline-flex w-4 h-4">
                        <Icon icon=icondata_lu::LuAArrowDown width="100%" height="100%"/>
                    </span>
                </button>
                <input
                    id=id_size
                    type="number"
                    min="8"
                    max="48"
                    class="join-item input input-bordered input-sm w-10 text-center"
                    prop:value=move || font_size.get().to_string()
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<u8>() {
                            if (8..=48).contains(&v) {
                                set_font_size.run(v);
                            }
                        }
                    }
                />
                <button
                    type="button"
                    class="join-item btn btn-sm btn-ghost border border-base-content/20"
                    title="Increase font size"
                    prop:disabled=move || { font_size.get() >= 48 }
                    on:click=step_up
                >
                    <span class="inline-flex w-4 h-4">
                        <Icon icon=icondata_lu::LuAArrowUp width="100%" height="100%"/>
                    </span>
                </button>
            </div>
        </div>
    }
}
