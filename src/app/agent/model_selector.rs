use crate::app::{components::icons::Icon, ipc, AppCtx};
use granit_types::ModelInfo;
use leptos::{prelude::*, task::spawn_local};
use wasm_bindgen::JsCast;

#[component]
pub fn ModelSelector(
    models: RwSignal<Vec<ModelInfo>>,
    models_loading: RwSignal<bool>,
    #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
    let config = expect_context::<AppCtx>().config;
    let (dropdown_open, set_dropdown_open) = signal(false);
    let trigger_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let dropdown_style: RwSignal<String> = RwSignal::new(String::new());

    let update_dropdown_position = move || {
        let Some(trigger) = trigger_ref.get_untracked() else {
            return;
        };
        let Some(window) = web_sys::window() else {
            return;
        };
        let Ok(height) = window.inner_height() else {
            return;
        };
        let Some(viewport_height) = height.as_f64() else {
            return;
        };

        let rect = trigger
            .unchecked_into::<web_sys::Element>()
            .get_bounding_client_rect();
        let gutter = 8.0;
        let space_above = (rect.top() - gutter).max(0.0);
        let space_below = (viewport_height - rect.bottom() - gutter).max(0.0);
        let min_width = rect.width().max(192.0);

        let style = if space_above >= space_below {
            let max_height = space_above.max(160.0);
            let bottom = (viewport_height - rect.top()) + 4.0;
            format!(
                "left: {:.0}px; bottom: {:.0}px; min-width: {:.0}px; max-height: {:.0}px;",
                rect.left(),
                bottom,
                min_width,
                max_height
            )
        } else {
            let max_height = space_below.max(160.0);
            let top = rect.bottom() + 4.0;
            format!(
                "left: {:.0}px; top: {:.0}px; min-width: {:.0}px; max-height: {:.0}px;",
                rect.left(),
                top,
                min_width,
                max_height
            )
        };

        dropdown_style.set(style);
    };

    let has_selection = move || {
        let cfg = config.get();
        let selected = cfg.agent.selected_model.clone().unwrap_or_default();
        if selected.is_empty() || models_loading.get() {
            return false;
        }
        let list = models.get();
        list.iter().any(|m| m.id == selected)
    };

    let selected_label = move || {
        let cfg = config.get();
        let selected = cfg.agent.selected_model.clone().unwrap_or_default();
        if models_loading.get() {
            return "Loading…".to_string();
        }
        let list = models.get();
        if list.is_empty() {
            return "No models".to_string();
        }
        list.iter()
            .find(|m| m.id == selected)
            .map(|m| m.display_name().to_string())
            .unwrap_or_else(|| "Select a model…".to_string())
    };

    let on_select = move |model_id: String| {
        set_dropdown_open.set(false);
        spawn_local(async move {
            if let Ok(new_cfg) = ipc::select_model(&model_id).await {
                config.set(new_cfg);
            }
        });
    };

    view! {
        <div class="relative" node_ref=trigger_ref>
            <button
                type="button"
                class="flex min-w-0 max-w-[14rem] items-center gap-1.5 rounded px-1.5 py-1 text-xs text-neutral-content/70 transition-colors hover:bg-neutral-content/10 hover:text-neutral-content disabled:cursor-not-allowed disabled:opacity-50"
                prop:disabled=move || disabled.get() || models_loading.get()
                on:click=move |_| {
                    let will_open = !dropdown_open.get_untracked();
                    set_dropdown_open.set(will_open);
                    if will_open {
                        update_dropdown_position();
                    }
                }
            >
                <span
                    class="truncate"
                    class=("italic", move || !has_selection())
                    class=("text-neutral-content/40", move || !has_selection())
                >
                    {move || if models_loading.get() {
                        view! { <span class="loading loading-spinner loading-xs" /> }.into_any()
                    } else {
                        view! { {selected_label()} }.into_any()
                    }}
                </span>
                <span
                    class="inline-flex w-3 h-3 shrink-0 text-neutral-content/55 transition-transform"
                    class:rotate-180=move || dropdown_open.get()
                >
                    <Icon icon=icondata_lu::LuChevronDown width="100%" height="100%"/>
                </span>
            </button>

            <Show when=move || dropdown_open.get()>
                <>
                    <div class="fixed inset-0 z-40" on:click=move |_| set_dropdown_open.set(false)/>
                    <div
                        class="fixed z-50 flex flex-col overflow-y-auto overflow-x-hidden rounded border border-neutral-content/20 bg-neutral py-1 text-neutral-content shadow-lg"
                        style=move || dropdown_style.get()
                    >
                        {move || {
                            let cfg = config.get();
                            let selected = cfg.agent.selected_model.clone().unwrap_or_default();
                            models.get().into_iter().map(|m| {
                                let is_active = m.id == selected;
                                let display = m.display_name().to_string();
                                let title = display.clone();
                                let id = m.id.clone();
                                view! {
                                    <button
                                        type="button"
                                        class="w-full truncate px-3 py-1.5 text-left text-xs text-neutral-content/70 transition-colors hover:bg-neutral-content/10 hover:text-neutral-content"
                                        class=("bg-neutral-content/12 text-neutral-content", is_active)
                                        title=title
                                        on:click=move |_| on_select(id.clone())
                                    >
                                        {display}
                                    </button>
                                }
                            }).collect_view()
                        }}
                    </div>
                </>
            </Show>
        </div>
    }
}
