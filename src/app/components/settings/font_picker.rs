use leptos::prelude::*;

use crate::app::components::icons::ChevronDownIcon;

#[component]
pub fn FontPicker(
    /// All available system font families.
    fonts: ReadSignal<Vec<String>>,
    /// Currently selected font family.
    value: ReadSignal<String>,
    /// Called when the user selects a font.
    set_value: WriteSignal<String>,
    /// HTML id for the wrapper (used for label association).
    #[prop(into)]
    id: String,
) -> impl IntoView {
    let (open, set_open) = signal(false);
    let (search, set_search) = signal(String::new());

    // Filtered font list based on search query
    let filtered = move || {
        let query = search.get().to_lowercase();
        let all = fonts.get();
        if query.is_empty() {
            all
        } else {
            all.into_iter()
                .filter(|f| f.to_lowercase().contains(&query))
                .collect()
        }
    };

    // Close dropdown when clicking outside
    let on_backdrop = move |_: leptos::ev::MouseEvent| {
        set_open.set(false);
    };

    let toggle = move |_: leptos::ev::MouseEvent| {
        let will_open = !open.get_untracked();
        set_open.set(will_open);
        if will_open {
            set_search.set(String::new());
        }
    };

    let id_clone = id.clone();

    view! {
        <div class="relative" id=id_clone>
            // Selected value button
            <button
                type="button"
                class="w-full bg-stone-900 border border-stone-600 rounded px-3 py-1.5 text-sm text-stone-200 outline-none focus:border-stone-400 transition-colors text-left flex items-center justify-between"
                on:click=toggle
            >
                <span
                    style:font-family=move || format!("'{}'", value.get())
                >
                    {move || {
                        let v = value.get();
                        if v.is_empty() { "Select font…".to_string() } else { v }
                    }}
                </span>
                <ChevronDownIcon class="w-3.5 h-3.5 text-stone-400 shrink-0 ml-2" open=open />
            </button>

            // Dropdown
            <Show when=move || open.get()>
                // Invisible backdrop to close on outside click
                <div class="fixed inset-0 z-40" on:click=on_backdrop />

                <div class="absolute z-50 mt-1 w-full bg-stone-900 border border-stone-600 rounded shadow-lg max-h-60 flex flex-col">
                    // Search input
                    <div class="p-1.5 border-b border-stone-700">
                        <input
                            type="text"
                            class="w-full bg-stone-800 border border-stone-600 rounded px-2 py-1 text-sm text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400"
                            placeholder="Search fonts…"
                            prop:value=move || search.get()
                            on:input=move |ev| set_search.set(event_target_value(&ev))
                        />
                    </div>

                    // Font list
                    <div class="overflow-y-auto flex-1">
                        {move || {
                            let items = filtered();
                            if items.is_empty() {
                                view! {
                                    <p class="px-3 py-2 text-sm text-stone-500 italic">"No matching fonts"</p>
                                }.into_any()
                            } else {
                                items.into_iter().map(|font| {
                                    let font_name = font.clone();
                                    let font_style = font.clone();
                                    let font_select = font.clone();
                                    let is_selected = move || value.get() == font_name;
                                    view! {
                                        <button
                                            type="button"
                                            class="w-full text-left px-3 py-1.5 text-sm text-stone-200 hover:bg-stone-700 transition-colors truncate"
                                            class=("bg-stone-700", is_selected)
                                            style:font-family=format!("'{font_style}'")
                                            on:click=move |_| {
                                                set_value.set(font_select.clone());
                                                set_open.set(false);
                                            }
                                        >
                                            {font.clone()}
                                        </button>
                                    }
                                }).collect_view().into_any()
                            }
                        }}
                    </div>
                </div>
            </Show>
        </div>
    }
}
