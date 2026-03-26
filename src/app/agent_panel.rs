use leptos::prelude::*;

/// Right panel — AI agent chat interface.
#[component]
pub fn AgentPanel() -> impl IntoView {
    let (input, set_input) = signal(String::new());

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let _message = input.get_untracked();
        set_input.set(String::new());
        // TODO: send message to agent via IPC
    };

    view! {
        <aside class="w-80 shrink-0 bg-stone-850 border-l border-stone-700 flex flex-col overflow-hidden">
            // Header
            <div class="flex items-center px-3 py-2 border-b border-stone-700">
                <span class="text-xs font-semibold uppercase tracking-wider text-stone-400">"Agent"</span>
            </div>

            // Chat messages (placeholder)
            <div class="flex-1 overflow-y-auto p-3">
                <p class="text-sm text-stone-500 italic">"Ask me anything about your notes..."</p>
            </div>

            // Input
            <form
                class="p-2 border-t border-stone-700"
                on:submit=on_submit
            >
                <div class="flex gap-2">
                    <input
                        type="text"
                        class="flex-1 bg-stone-800 border border-stone-600 rounded px-3 py-1.5 text-sm text-stone-200 placeholder-stone-500 outline-none focus:border-stone-400 transition-colors"
                        placeholder="Message..."
                        prop:value=move || input.get()
                        on:input=move |ev| set_input.set(event_target_value(&ev))
                    />
                    <button
                        type="submit"
                        class="px-3 py-1.5 bg-stone-700 text-stone-300 rounded text-sm hover:bg-stone-600 transition-colors"
                    >
                        "Send"
                    </button>
                </div>
            </form>
        </aside>
    }
}
