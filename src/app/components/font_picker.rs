use crate::app::components::icons::Icon;
use leptos::prelude::*;

#[component]
pub fn FontPicker(
    /// All available system font families.
    #[prop(into)]
    fonts: Signal<Vec<String>>,
    /// Currently selected font family.
    #[prop(into)]
    value: Signal<String>,
    /// Called when the user selects a font.
    #[prop(into)]
    set_value: Callback<String>,
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
                class="input input-bordered input-sm w-full text-left flex items-center justify-between"
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
                <span
                    class="inline-flex w-3.5 h-3.5 text-base-content/50 shrink-0 ml-2 transition-transform"
                    class:rotate-180=move || open.get()
                >
                    <Icon icon=icondata_lu::LuChevronDown width="100%" height="100%"/>
                </span>
            </button>

            // Dropdown
            <Show when=move || open.get()>
                // Invisible backdrop to close on outside click
                <div class="fixed inset-0 z-40" on:click=on_backdrop />

                <div class="absolute z-50 mt-1 w-full bg-base-100 border border-base-content/20 rounded shadow-lg max-h-60 flex flex-col">
                    // Search input
                    <div class="p-1.5 border-b border-base-content/10">
                        <input
                            type="text"
                            class="input input-bordered input-sm w-full"
                            placeholder="Search fonts…"
                            prop:value=move || search.get()
                            on:input=move |ev| set_search.set(event_target_value(&ev))
                        />
                    </div>

                    // Font list
                    <ul class="menu menu-sm overflow-y-auto flex-1 p-0 flex-nowrap w-full">
                        {move || {
                            let items = filtered();
                            if items.is_empty() {
                                view! {
                                    <li><span class="italic text-base-content/35">"No matching fonts"</span></li>
                                }.into_any()
                            } else {
                                items.into_iter().map(|font| {
                                    let font_name = font.clone();
                                    let font_style = font.clone();
                                    let font_select = font.clone();
                                    let is_selected = move || value.get() == font_name;
                                    view! {
                                        <li>
                                            <button
                                                type="button"
                                                class=move || if is_selected() { "menu-active truncate" } else { "truncate" }
                                                style:font-family=format!("'{font_style}'")
                                                on:click=move |_| {
                                                    set_value.run(font_select.clone());
                                                    set_open.set(false);
                                                }
                                            >
                                                {font.clone()}
                                            </button>
                                        </li>
                                    }
                                }).collect_view().into_any()
                            }
                        }}
                    </ul>
                </div>
            </Show>
        </div>
    }
}
