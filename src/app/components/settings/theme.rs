use crate::app::{apply_theme, ipc, AppCtx};
use granit_types::ThemeMeta;
use leptos::prelude::*;

#[component]
pub fn ThemeSettings() -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let (themes, set_themes) = signal(Vec::<ThemeMeta>::new());
    let (loading, set_loading) = signal(true);
    let (applying, set_applying) = signal(false);

    // Active theme id is persisted in config
    let active_id = move || ctx.config.get().theme;

    leptos::task::spawn_local(async move {
        if let Ok(list) = ipc::list_themes().await {
            set_themes.set(list);
        }
        set_loading.set(false);
    });

    view! {
        <fieldset>
            <legend class="text-xs font-semibold uppercase tracking-wider text-fg-muted mb-3">"Theme"</legend>
            <Show when=move || loading.get()>
                <p class="text-xs text-fg-faint italic">"Loading themes…"</p>
            </Show>
            <Show when=move || !loading.get()>
                <div class="space-y-1.5">
                    {move || themes.get().into_iter().map(|theme| {
                        let id = theme.id.clone();
                        let id_for_click = id.clone();
                        let is_active = move || active_id() == id;
                        view! {
                            <button
                                type="button"
                                class=move || {
                                    if is_active() {
                                        "w-full flex items-center justify-between px-3 py-2 rounded text-sm text-fg bg-item-hover border border-edge transition-colors"
                                    } else {
                                        "w-full flex items-center justify-between px-3 py-2 rounded text-sm text-fg-secondary hover:text-fg hover:bg-item-hover/60 border border-transparent transition-colors disabled:opacity-50"
                                    }
                                }
                                disabled=move || applying.get()
                                on:click=move |_| {
                                    let id = id_for_click.clone();
                                    set_applying.set(true);
                                    leptos::task::spawn_local(async move {
                                        match ipc::set_active_theme(&id).await {
                                            Ok(new_cfg) => {
                                                ctx.config.set(new_cfg);
                                                if let Ok(theme) = ipc::get_active_theme().await {
                                                    apply_theme(&theme);
                                                }
                                            }
                                            Err(e) => {
                                                ctx.push_error("theme", format!("Failed to set theme: {e}"));
                                            }
                                        }
                                        set_applying.set(false);
                                    });
                                }
                            >
                                <span>{theme.name}</span>
                                <span class=move || {
                                    if theme.is_dark {
                                        "text-xs px-1.5 py-0.5 rounded-full bg-item-active text-fg-muted"
                                    } else {
                                        "text-xs px-1.5 py-0.5 rounded-full bg-card text-fg-faint"
                                    }
                                }>
                                    {if theme.is_dark { "dark" } else { "light" }}
                                </span>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </Show>
        </fieldset>
    }
}
