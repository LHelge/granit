use crate::app::{
    components::icons::{Icon, ProviderIcon},
    ipc, AppCtx,
};
use leptos::{prelude::*, task::spawn_local};

#[component]
pub fn ProviderSelector(#[prop(into)] disabled: Signal<bool>) -> impl IntoView {
    let config = expect_context::<AppCtx>().config;
    let (dropdown_open, set_dropdown_open) = signal(false);

    let selected_label = move || {
        let cfg = config.get();
        cfg.agent
            .providers
            .get(cfg.agent.selected_provider)
            .map(|provider| provider.display_name())
            .unwrap_or_else(|| "No provider".to_string())
    };

    let selected_type = move || {
        let cfg = config.get();
        cfg.agent
            .providers
            .get(cfg.agent.selected_provider)
            .map(|provider| provider.provider.provider_type().to_string())
            .unwrap_or_default()
    };

    let on_select = move |idx: usize| {
        set_dropdown_open.set(false);
        spawn_local(async move {
            if let Ok(new_cfg) = ipc::select_provider(idx).await {
                config.set(new_cfg);
            }
        });
    };

    view! {
        <div class="relative">
            <button
                class="flex items-center gap-1.5 px-2 py-1 text-xs bg-base-300 border border-base-content/20 rounded hover:border-base-content/30 transition-colors text-base-content/70 disabled:opacity-50 disabled:cursor-not-allowed"
                prop:disabled=move || disabled.get()
                on:click=move |_| set_dropdown_open.update(|v| *v = !*v)
            >
                <ProviderIcon provider_type=Signal::derive(selected_type) class="w-3.5 h-3.5 shrink-0" />
                <span class="truncate">{selected_label}</span>
                <span
                    class="inline-flex w-3 h-3 shrink-0 text-base-content/50 transition-transform"
                    class:rotate-180=move || dropdown_open.get()
                >
                    <Icon icon=icondata_lu::LuChevronDown width="100%" height="100%"/>
                </span>
            </button>

            <Show when=move || dropdown_open.get()>
                <div class="absolute top-full left-0 mt-1 bg-base-300 border border-base-content/20 rounded shadow-lg z-50 max-h-60 overflow-y-auto min-w-[10rem]">
                    {move || {
                        let cfg = config.get();
                        let selected = cfg.agent.selected_provider;
                        cfg.agent.providers.iter().enumerate().map(|(idx, provider)| {
                            let is_active = idx == selected;
                            let display = provider.display_name();
                            let ptype = provider.provider.provider_type().to_string();
                            view! {
                                <button
                                    class="w-full flex items-center gap-1.5 px-3 py-1.5 text-xs text-base-content/70 hover:bg-base-content/10 transition-colors truncate"
                                    class=("bg-base-content/5", is_active)
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
