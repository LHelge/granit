use super::cave_selector::CaveSelector;
use super::tree_view::TreeView;
use crate::app::AppCtx;
use leptos::prelude::*;

#[component]
pub fn Sidebar(set_settings_open: WriteSignal<bool>) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let has_cave = move || ctx.config.get().active_cave.is_some();

    view! {
        <aside class="w-64 shrink-0 bg-stone-850 border-r border-stone-700 flex flex-col overflow-hidden">
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
