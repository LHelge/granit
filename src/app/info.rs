use crate::app::{
    components::{icons::Icon, modal::Modal},
    ipc, AppCtx,
};
use granit_types::AppMetadata;
use leptos::prelude::*;

#[component]
pub fn InfoModal(set_open: WriteSignal<bool>) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let metadata = RwSignal::new(None::<AppMetadata>);
    let load_error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(true);

    let close = move || set_open.set(false);

    leptos::task::spawn_local(async move {
        match ipc::fetch_app_metadata().await {
            Ok(app_metadata) => metadata.set(Some(app_metadata)),
            Err(err) => {
                let message = format!("Failed to load app metadata: {err}");
                load_error.set(Some(message.clone()));
                ctx.push_error("app-metadata", message);
            }
        }
        loading.set(false);
    });

    view! {
        <Modal
            title="About Granit"
            subtitle="Application and build information"
            panel_class="w-[420px] max-w-[90vw]"
            on_close=Callback::new(move |()| close())
        >
            <div class="p-4 space-y-4">
                <Show
                    when=move || loading.get()
                    fallback=move || {
                        view! {
                            <Show
                                when=move || load_error.get().is_none()
                                fallback=move || view! {
                                    <div role="alert" class="alert alert-error alert-soft text-sm">
                                        {move || load_error.get().unwrap_or_default()}
                                    </div>
                                }
                            >
                                <Show
                                    when=move || metadata.get().is_some()
                                    fallback=|| view! { <></> }
                                >
                                    {move || {
                                        metadata
                                            .get()
                                            .map(|app_metadata| {
                                                view! { <MetadataDetails app_metadata /> }
                                            })
                                    }}
                                </Show>
                            </Show>
                        }
                    }
                >
                    <div class="flex items-center gap-3 text-sm text-base-content/60">
                        <span class="loading loading-spinner loading-sm"></span>
                        <span>"Loading build information..."</span>
                    </div>
                </Show>
            </div>
        </Modal>
    }
}

#[component]
fn MetadataDetails(app_metadata: AppMetadata) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let current_year = js_sys::Date::new_0().get_full_year() as i32;
    let repo_url = app_metadata.repo_url.clone();

    let open_repo = move |_| {
        let repo_url = repo_url.clone();
        let ctx = ctx;
        leptos::task::spawn_local(async move {
            if let Err(err) = ipc::open_url(&repo_url).await {
                ctx.push_error(
                    "app-metadata",
                    format!("Failed to open repository URL: {err}"),
                );
            }
        });
    };

    view! {
        <>
            <div class="space-y-1">
                <h3 class="text-lg font-semibold text-base-content">{app_metadata.app_name.clone()}</h3>
                <p class="text-sm text-base-content/60">"Minimal desktop note-taking with an integrated AI agent."</p>
            </div>

            <div class="rounded-box border border-base-content/15 overflow-hidden">
                <div class="flex items-center justify-between gap-4 px-4 py-3 border-b border-base-content/10">
                    <span class="text-sm text-base-content/55">"Repository"</span>
                    <button class="link link-primary text-sm inline-flex items-center gap-1" on:click=open_repo>
                        <span>{app_metadata.repo_url.clone()}</span>
                        <span class="inline-flex w-3.5 h-3.5">
                            <Icon icon=icondata_lu::LuExternalLink width="100%" height="100%"/>
                        </span>
                    </button>
                </div>
                <div class="flex items-center justify-between gap-4 px-4 py-3 border-b border-base-content/10">
                    <span class="text-sm text-base-content/55">"Version"</span>
                    <span class="font-mono text-sm text-base-content">{app_metadata.version.clone()}</span>
                </div>
                <div class="flex items-center justify-between gap-4 px-4 py-3 border-b border-base-content/10">
                    <span class="text-sm text-base-content/55">"Commit"</span>
                    <span class="font-mono text-sm text-base-content">{app_metadata.git_commit_hash.clone()}</span>
                </div>
                <div class="flex items-center justify-between gap-4 px-4 py-3">
                    <span class="text-sm text-base-content/55">"Dirty"</span>
                    <span class=if app_metadata.git_dirty {
                        "badge badge-warning badge-soft"
                    } else {
                        "badge badge-success badge-soft"
                    }>
                        {if app_metadata.git_dirty { "Yes" } else { "No" }}
                    </span>
                </div>
            </div>

            <p class="text-xs text-base-content/45">{format!("Copyright © {current_year} LHelge")}</p>
        </>
    }
}
