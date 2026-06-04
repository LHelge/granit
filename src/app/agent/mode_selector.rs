use crate::app::{components::icons::Icon, ipc, AppCtx};
use granit_types::AgentMode;
use leptos::{prelude::*, task::spawn_local};

const MODES: &[(AgentMode, &str)] = &[
    (AgentMode::Agent, "Agent"),
    (AgentMode::Ask, "Ask"),
];

#[component]
pub fn ModeSelector(#[prop(into)] disabled: Signal<bool>) -> impl IntoView {
    let config = expect_context::<AppCtx>().config;
    let (dropdown_open, set_dropdown_open) = signal(false);

    let current_mode = move || config.get().agent.mode;

    let selected_label = move || current_mode().label().to_string();

    let on_select = move |mode: AgentMode| {
        set_dropdown_open.set(false);
        spawn_local(async move {
            if let Ok(new_cfg) = ipc::select_mode(mode).await {
                config.set(new_cfg);
            }
        });
    };

    view! {
        <div class="relative">
            <button
                class="flex items-center gap-1 px-2 py-1 text-xs bg-transparent border border-neutral-content/20 rounded hover:border-neutral-content/30 transition-colors text-neutral-content/70 disabled:opacity-50 disabled:cursor-not-allowed"
                prop:disabled=move || disabled.get()
                on:click=move |_| set_dropdown_open.update(|v| *v = !*v)
            >
                <span class="truncate">{selected_label}</span>
                <span
                    class="inline-flex w-3 h-3 shrink-0 text-neutral-content/50 transition-transform"
                    class:rotate-180=move || dropdown_open.get()
                >
                    <Icon icon=icondata_lu::LuChevronDown width="100%" height="100%"/>
                </span>
            </button>

            <Show when=move || dropdown_open.get()>
                <div class="absolute bottom-full left-0 mb-1 bg-base-300 border border-base-content/20 rounded shadow-lg z-50 min-w-[6rem]">
                    {MODES.iter().map(|(mode, label)| {
                        let mode = *mode;
                        let is_active = move || current_mode() == mode;
                        view! {
                            <button
                                class="w-full px-3 py-1.5 text-xs text-left text-base-content/70 hover:bg-base-content/10 transition-colors truncate"
                                class=("bg-base-content/5", is_active)
                                on:click=move |_| on_select(mode)
                            >
                                {*label}
                            </button>
                        }
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }
}
