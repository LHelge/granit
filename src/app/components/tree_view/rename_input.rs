use leptos::ev::KeyboardEvent;
use leptos::prelude::*;

/// Inline rename input component. Shows a text input with the current name,
/// commits on Enter, cancels on Escape or blur.
#[component]
pub(super) fn RenameInput(
    initial: String,
    indent_style: String,
    /// "note" or "folder" — determines which icon to show.
    icon: &'static str,
    on_confirm: Callback<String>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let (text, set_text) = signal(initial);
    let input_ref = NodeRef::<leptos::html::Input>::new();

    // Auto-focus after mount.
    Effect::new(move || {
        if let Some(el) = input_ref.get() {
            let el: &web_sys::HtmlInputElement = &el;
            let _ = el.focus();
            el.select();
        }
    });

    let on_keydown = move |e: KeyboardEvent| match e.key().as_str() {
        "Enter" => {
            e.prevent_default();
            on_confirm.run(text.get().trim().to_string());
        }
        "Escape" => {
            e.prevent_default();
            on_cancel.run(());
        }
        _ => {}
    };

    let on_blur = move |_| {
        on_confirm.run(text.get().trim().to_string());
    };

    let note_icon = icon == "note";

    view! {
        <div class="flex items-center gap-1 py-0.5 text-sm" style=indent_style>
            {if note_icon {
                view! {
                    <span class="w-3 shrink-0" />
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 shrink-0 text-stone-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                    </svg>
                }.into_any()
            } else {
                view! {
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3 shrink-0 text-stone-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" />
                    </svg>
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V7z" />
                    </svg>
                }.into_any()
            }}
            <input
                type="text"
                prop:value=move || text.get()
                node_ref=input_ref
                class="flex-1 bg-stone-700 text-stone-100 text-sm px-1 py-0 rounded border border-stone-500 focus:outline-none focus:border-stone-400 min-w-0"
                on:input=move |ev| set_text.set(event_target_value(&ev))
                on:keydown=on_keydown
                on:blur=on_blur
            />
        </div>
    }
}
