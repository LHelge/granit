use super::{cave_selector::CaveSelector, tree_view::TreeView};
use crate::app::AppCtx;
use leptos::prelude::*;

#[component]
pub fn Sidebar(set_settings_open: WriteSignal<bool>, width: ReadSignal<u16>) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let has_cave = move || ctx.config.get().active_cave.is_some();

    view! {
        <aside
            class="shrink-0 bg-stone-850 border-r border-stone-700 flex flex-col overflow-hidden"
            style:width=move || format!("{}px", width.get())
        >
            // Note list
            <div class="flex-1 overflow-y-auto">
                <Show
                    when=has_cave
                    fallback=|| view! { <p class="p-2 text-sm text-stone-500 italic">"No cave open"</p> }
                >
                    <TreeView />
                </Show>
            </div>

            // Bottom bar: cave selector + settings
            <CaveSelector set_settings_open=set_settings_open />
        </aside>
    }
}
