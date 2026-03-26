use leptos::prelude::*;

/// Center panel — markdown editor with edit/read mode toggle.
#[component]
pub fn Editor() -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let (content, set_content) = signal(String::new());

    let toggle_mode = move |_| set_editing.update(|v| *v = !*v);

    view! {
        <main class="flex-1 flex flex-col overflow-hidden bg-stone-900">
            // Toolbar
            <div class="flex items-center gap-2 px-3 py-1.5 border-b border-stone-700 shrink-0">
                <span class="text-sm text-stone-400 flex-1">"Untitled"</span>
                <button
                    class="px-2 py-0.5 text-xs rounded border border-stone-600 text-stone-300 hover:bg-stone-700 transition-colors"
                    on:click=toggle_mode
                >
                    {move || if editing.get() { "Preview" } else { "Edit" }}
                </button>
            </div>

            // Content area
            <div class="flex-1 overflow-y-auto p-6">
                <Show
                    when=move || editing.get()
                    fallback=move || view! {
                        <div class="prose prose-invert max-w-none">
                            <p class="text-stone-500 italic">"Nothing to preview"</p>
                        </div>
                    }
                >
                    <textarea
                        class="w-full h-full bg-transparent text-stone-200 resize-none outline-none font-mono text-sm leading-relaxed"
                        placeholder="Start writing..."
                        prop:value=move || content.get()
                        on:input=move |ev| set_content.set(event_target_value(&ev))
                    />
                </Show>
            </div>
        </main>
    }
}
