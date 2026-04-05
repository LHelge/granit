mod cave_selector;
pub(crate) mod tree_view;

use crate::app::AppCtx;
use cave_selector::CaveSelector;
use leptos::prelude::*;
use tree_view::TreeView;

#[component]
pub fn Explorer(set_settings_open: WriteSignal<bool>, width: ReadSignal<u16>) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let has_cave = move || ctx.config.get().active_cave.is_some();

    view! {
        <aside
            class="shrink-0 bg-base-200 border-r border-base-content/10 flex flex-col"
            style:width=move || format!("{}px", width.get())
        >
            // Note list
            <div class="flex-1 overflow-y-auto">
                <Show
                    when=has_cave
                    fallback=|| view! { <p class="p-2 text-sm text-base-content/35 italic">"No cave open"</p> }
                >
                    <TreeView />
                </Show>
            </div>

            // Bottom bar: cave selector + settings
            <CaveSelector set_settings_open=set_settings_open />
        </aside>
    }
}
