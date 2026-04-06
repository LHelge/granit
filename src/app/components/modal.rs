use crate::app::components::icons::Icon;
use leptos::prelude::*;

#[component]
pub fn Modal(
    #[prop(into)] title: String,
    #[prop(optional, into)] subtitle: Option<String>,
    #[prop(optional, into)] panel_class: String,
    on_close: Callback<()>,
    children: Children,
) -> impl IntoView {
    let panel_class = if panel_class.trim().is_empty() {
        "modal-box p-0 flex flex-col".to_string()
    } else {
        format!("modal-box p-0 flex flex-col {panel_class}")
    };
    let subtitle_text = subtitle.unwrap_or_default();
    let has_subtitle = !subtitle_text.is_empty();

    view! {
        <dialog class="modal modal-open">
            <div class=panel_class>
                <div class="flex items-center justify-between px-4 py-3 border-b border-base-content/20 shrink-0">
                    <div>
                        <h2 class="text-sm font-semibold text-base-content">{title}</h2>
                        <Show when=move || has_subtitle>
                            <p class="text-xs text-base-content/50 mt-0.5">
                                {subtitle_text.clone()}
                            </p>
                        </Show>
                    </div>
                    <button
                        class="btn btn-ghost btn-xs btn-square"
                        on:click={
                            let on_close = on_close;
                            move |_| on_close.run(())
                        }
                    >
                        <Icon icon=icondata_lu::LuX width="1rem" height="1rem"/>
                    </button>
                </div>

                {children()}
            </div>

            <form method="dialog" class="modal-backdrop">
                <button on:click={move |_| on_close.run(())}>"close"</button>
            </form>
        </dialog>
    }
}
