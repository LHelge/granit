mod cave_selector;
mod search;
mod templates;
mod todo;
pub(crate) mod tree_view;

use crate::app::{components::icons::Icon, AppCtx};
use cave_selector::CaveSelector;
use leptos::prelude::*;
use search::Search;
use templates::Templates;
use todo::Todo;
use tree_view::TreeView;

#[component]
pub fn Explorer(set_settings_open: WriteSignal<bool>, width: ReadSignal<u16>) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let has_cave = move || ctx.config.get().active_cave.is_some();
    let (active_tab, set_active_tab) = signal(0u8);

    view! {
        <aside
            class="shrink-0 bg-base-200 border-r border-base-content/10 flex flex-col"
            style:width=move || format!("{}px", width.get())
        >
            // Tab bar
            <div role="tablist" class="tabs tabs-border shrink-0 px-1">
                <button
                    role="tab"
                    class="tab"
                    class:tab-active=move || active_tab.get() == 0
                    on:click=move |_| set_active_tab.set(0)
                >
                    <span class="inline-flex w-5 h-5">
                        <Icon icon=icondata_lu::LuFolderTree width="100%" height="100%"/>
                    </span>
                </button>
                <button
                    role="tab"
                    class="tab"
                    class:tab-active=move || active_tab.get() == 1
                    on:click=move |_| set_active_tab.set(1)
                >
                    <span class="inline-flex w-5 h-5">
                        <Icon icon=icondata_lu::LuSearch width="100%" height="100%"/>
                    </span>
                </button>
                <button
                    role="tab"
                    class="tab"
                    class:tab-active=move || active_tab.get() == 2
                    on:click=move |_| set_active_tab.set(2)
                >
                    <span class="inline-flex w-5 h-5">
                        <Icon icon=icondata_lu::LuListTodo width="100%" height="100%"/>
                    </span>
                </button>
                <button
                    role="tab"
                    class="tab"
                    class:tab-active=move || active_tab.get() == 3
                    on:click=move |_| set_active_tab.set(3)
                >
                    <span class="inline-flex w-5 h-5">
                        <Icon icon=icondata_lu::LuNotepadTextDashed width="100%" height="100%"/>
                    </span>
                </button>
            </div>

            // Tab content
            <div class="flex-1 overflow-y-auto">
                <Show when=move || active_tab.get() == 0>
                    <Show
                        when=has_cave
                        fallback=|| view! { <p class="p-2 text-sm text-base-content/35 italic">"No cave open"</p> }
                    >
                        <TreeView />
                    </Show>
                </Show>
                <Show when=move || active_tab.get() == 1>
                    <Search />
                </Show>
                <Show when=move || active_tab.get() == 2>
                    <Todo />
                </Show>
                <Show when=move || active_tab.get() == 3>
                    <Templates />
                </Show>
            </div>

            // Bottom bar: cave selector + settings (always visible)
            <CaveSelector set_settings_open=set_settings_open />
        </aside>
    }
}
