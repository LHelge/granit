use leptos::ev::KeyboardEvent;
use leptos::prelude::*;

use crate::app::components::icons::Icon;
use icondata_lu;

/// Inline rename input component. Shows a text input with the current name,
/// commits on Enter, cancels on Escape or blur.
#[component]
pub(super) fn RenameInput(
    initial: String,
    indent_style: String,
    /// `true` for a note icon, `false` for chevron + folder icon.
    note: bool,
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

    view! {
        <div class="flex items-center gap-1 py-0.5 text-sm" style=indent_style>
            {if note {
                view! {
                    <span class="w-3 shrink-0" />
                    <span class="inline-flex w-3.5 h-3.5 shrink-0 text-stone-500">
                        <Icon icon=icondata_lu::LuFileText width="100%" height="100%"/>
                    </span>
                }.into_any()
            } else {
                view! {
                    <span class="inline-flex w-3 h-3 shrink-0 transition-transform">
                        <Icon icon=icondata_lu::LuChevronRight width="100%" height="100%"/>
                    </span>
                    <Icon icon=icondata_lu::LuFolder width="0.875rem" height="0.875rem"/>
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
