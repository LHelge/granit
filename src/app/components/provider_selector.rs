use crate::app::components::icons::{Icon, ProviderIcon};
use crate::app::ipc;
use crate::app::AppCtx;
use granit_types::ProviderInfo;
use icondata_lu;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn ProviderSelector(
    providers: RwSignal<Vec<ProviderInfo>>,
    #[prop(into)] disabled: Signal<bool>,
    on_changed: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let config = expect_context::<AppCtx>().config;
    let (dropdown_open, set_dropdown_open) = signal(false);

    let selected_label = move || {
        let cfg = config.get();
        let idx = cfg.agent.selected_provider;
        providers
            .get()
            .iter()
            .find(|p| p.index == idx)
            .map(|p| p.display_name.clone())
            .unwrap_or_else(|| "No provider".to_string())
    };

    let selected_type = move || {
        let cfg = config.get();
        let idx = cfg.agent.selected_provider;
        providers
            .get()
            .iter()
            .find(|p| p.index == idx)
            .map(|p| p.provider_type.clone())
            .unwrap_or_default()
    };

    let on_select = move |idx: usize| {
        set_dropdown_open.set(false);
        spawn_local(async move {
            if let Ok(new_cfg) = ipc::select_provider(idx).await {
                config.set(new_cfg);
            }
            on_changed();
        });
    };

    view! {
        <div class="relative">
            <button
                class="flex items-center gap-1.5 px-2 py-1 text-xs bg-stone-800 border border-stone-600 rounded hover:border-stone-500 transition-colors text-stone-300 disabled:opacity-50 disabled:cursor-not-allowed"
                prop:disabled=move || disabled.get()
                on:click=move |_| set_dropdown_open.update(|v| *v = !*v)
            >
                <ProviderIcon provider_type=Signal::derive(selected_type) class="w-3.5 h-3.5 shrink-0" />
                <span class="truncate">{selected_label}</span>
                <span
                    class="inline-flex w-3 h-3 shrink-0 text-stone-400 transition-transform"
                    class:rotate-180=move || dropdown_open.get()
                >
                    <Icon icon=icondata_lu::LuChevronDown width="100%" height="100%"/>
                </span>
            </button>

            <Show when=move || dropdown_open.get()>
                <div class="absolute top-full left-0 mt-1 bg-stone-800 border border-stone-600 rounded shadow-lg z-50 max-h-60 overflow-y-auto min-w-[10rem]">
                    {move || {
                        let cfg = config.get();
                        let selected = cfg.agent.selected_provider;
                        providers.get().into_iter().map(|p| {
                            let idx = p.index;
                            let is_active = idx == selected;
                            let display = p.display_name.clone();
                            let ptype = p.provider_type.clone();
                            view! {
                                <button
                                    class="w-full flex items-center gap-1.5 px-3 py-1.5 text-xs text-stone-300 hover:bg-stone-700 transition-colors truncate"
                                    class=("bg-stone-700/50", is_active)
                                    on:click=move |_| on_select(idx)
                                >
                                    <ProviderIcon provider_type=Signal::stored(ptype.clone()) class="w-3.5 h-3.5 shrink-0" />
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
