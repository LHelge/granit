use crate::app::{components::icons::Icon, ipc, AppCtx};
use granit_types::ModelInfo;
use leptos::{prelude::*, task::spawn_local};

#[component]
pub fn ModelSelector(
    models: RwSignal<Vec<ModelInfo>>,
    models_loading: RwSignal<bool>,
    #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
    let config = expect_context::<AppCtx>().config;
    let (dropdown_open, set_dropdown_open) = signal(false);

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
        <div class="relative">
            <button
                type="button"
                class="flex min-w-0 max-w-[14rem] items-center gap-1.5 rounded px-1.5 py-1 text-xs text-neutral-content/70 transition-colors hover:bg-neutral-content/10 hover:text-neutral-content disabled:cursor-not-allowed disabled:opacity-50"
                prop:disabled=move || disabled.get() || models_loading.get()
                on:click=move |_| set_dropdown_open.update(|v| *v = !*v)
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
                // Opens upward since this sits at the bottom of the panel
                <ul class="menu menu-xs absolute bottom-full left-0 mb-1 bg-base-200 border border-base-content/15 rounded-box shadow-lg z-50 max-h-60 overflow-y-auto min-w-[12rem] py-1">
                    {move || {
                        let cfg = config.get();
                        let selected = cfg.agent.selected_model.clone().unwrap_or_default();
                        models.get().into_iter().map(|m| {
                            let is_active = m.id == selected;
                            let display = m.display_name().to_string();
                            let id = m.id.clone();
                            view! {
                                <li>
                                    <button
                                        type="button"
                                        class=if is_active { "menu-active" } else { "" }
                                        on:click=move |_| on_select(id.clone())
                                    >
                                        {display}
                                    </button>
                                </li>
                            }
                        }).collect_view()
                    }}
                </ul>
            </Show>
        </div>
    }
}
