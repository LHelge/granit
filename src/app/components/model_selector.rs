use crate::app::components::icons::ChevronDownIcon;
use crate::app::ipc;
use crate::app::AppCtx;
use granit_types::ModelInfo;
use leptos::prelude::*;
use leptos::task::spawn_local;

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
                class="flex items-center gap-1.5 px-2 py-1 text-xs bg-stone-800 border border-stone-600 rounded hover:border-stone-500 transition-colors text-stone-300 disabled:opacity-50 disabled:cursor-not-allowed"
                prop:disabled=move || disabled.get() || models_loading.get()
                on:click=move |_| set_dropdown_open.update(|v| *v = !*v)
            >
                <span
                    class="truncate max-w-[16rem]"
                    class=("italic", move || !has_selection())
                    class=("text-stone-500", move || !has_selection())
                >{selected_label}</span>
                <ChevronDownIcon class="w-3 h-3 shrink-0 text-stone-400" open=Signal::derive(move || dropdown_open.get()) />
            </button>

            <Show when=move || dropdown_open.get()>
                // Opens upward since this sits at the bottom of the panel
                <div class="absolute bottom-full left-0 mb-1 bg-stone-800 border border-stone-600 rounded shadow-lg z-50 max-h-60 overflow-y-auto min-w-[12rem]">
                    {move || {
                        let cfg = config.get();
                        let selected = cfg.agent.selected_model.clone().unwrap_or_default();
                        models.get().into_iter().map(|m| {
                            let is_active = m.id == selected;
                            let display = m.display_name().to_string();
                            let id = m.id.clone();
                            view! {
                                <button
                                    type="button"
                                    class="w-full flex items-center px-3 py-1.5 text-xs text-stone-300 hover:bg-stone-700 transition-colors truncate"
                                    class=("bg-stone-700/50", is_active)
                                    on:click=move |_| on_select(id.clone())
                                >
                                    {display}
                                </button>
                            }
                        }).collect_view()
                    }}
                </div>
            </Show>
        </div>
    }
}
