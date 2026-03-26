use leptos::prelude::*;

/// Left sidebar — file tree for navigating the cave.
#[component]
pub fn Sidebar() -> impl IntoView {
    view! {
        <aside class="w-64 shrink-0 bg-stone-850 border-r border-stone-700 flex flex-col overflow-hidden">
            // Header
            <div class="flex items-center justify-between px-3 py-2 border-b border-stone-700">
                <span class="text-xs font-semibold uppercase tracking-wider text-stone-400">"Explorer"</span>
            </div>

            // File tree (placeholder)
            <div class="flex-1 overflow-y-auto p-2">
                <p class="text-sm text-stone-500 italic">"No cave open"</p>
            </div>
        </aside>
    }
}
