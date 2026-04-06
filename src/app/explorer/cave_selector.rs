use crate::app::{components::icons::Icon, ipc, AppCtx};
use leptos::prelude::*;

#[component]
pub fn CaveSelector(set_settings_open: WriteSignal<bool>) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();

    let open_and_refresh = move |path: String| {
        leptos::task::spawn_local(async move {
            match ctx.open_cave_and_refresh(&path).await {
                Ok(()) => {}
                Err(e) => {
                    ctx.push_error("cave", format!("Failed to open cave: {e}"));
                }
            }
        });
    };

    let on_pick_folder = move |_| {
        leptos::task::spawn_local(async move {
            if let Some(path) = ipc::pick_folder().await {
                open_and_refresh(path);
            }
        });
    };

    let cave_label = move || {
        let cfg = ctx.config.get();
        cfg.active_cave
            .as_deref()
            .and_then(|p| p.rsplit('/').next().or_else(|| p.rsplit('\\').next()))
            .map(|s| s.to_string())
            .unwrap_or_else(|| "No cave open".to_string())
    };

    view! {
        <div class="border-t border-base-content/10 px-2 py-2">
            <div class="flex items-center gap-1">
                <div class="flex-1">
                    <button
                        class="w-full flex items-center justify-between px-2 py-1.5 text-sm bg-base-300 border border-base-content/20 rounded hover:border-base-content/30 transition-colors text-base-content/70 text-left truncate"
                        on:click=on_pick_folder
                    >
                        <span class="truncate">{cave_label}</span>
                        <span class="inline-flex w-3.5 h-3.5 ml-1 shrink-0 text-base-content/50">
                            <Icon icon=icondata_lu::LuFolderOpen width="100%" height="100%"/>
                        </span>
                    </button>
                </div>

                // Settings gear icon
                <div class="tooltip tooltip-top z-50" data-tip="Settings">
                    <button
                        class="btn btn-ghost btn-xs btn-square"
                        on:click=move |_| set_settings_open.set(true)
                    >
                        <Icon icon=icondata_lu::LuSettings width="1rem" height="1rem"/>
                    </button>
                </div>
            </div>
        </div>
    }
}
