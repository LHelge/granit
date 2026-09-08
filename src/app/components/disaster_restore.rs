//! Disaster restore: recover a cave from the backup backend when no cave
//! is open (fresh machine, lost disk). Collects the backend connection,
//! lists that key's snapshots, and restores one into a new folder.
//!
//! With no cave open there is never a cached key, so the passphrase is
//! always required here — unlike the settings restore flow, which only
//! prompts when the engine reports the cached key unusable.

use crate::app::components::modal::Modal;
use crate::app::settings::backup::{format_size, restore_stage_label};
use crate::app::{ipc, AppCtx};
use granit_api::{BackupInfo, BackupState};
use granit_types::{BackupConfig, RestoreStage, RestoreTarget};
use leptos::prelude::*;

#[component]
pub fn DisasterRestoreModal(set_open: WriteSignal<bool>) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();

    // Step 1: connection. Nothing is saved anywhere — the credentials ride
    // along with each command as an override.
    let backend_url = RwSignal::new(String::new());
    let api_key = RwSignal::new(String::new());
    let listing = RwSignal::new(false);

    // Step 2: that key's snapshots, present once listed successfully.
    let snapshots = RwSignal::new(None::<Vec<BackupInfo>>);

    // Step 3: chosen snapshot, target folder, and passphrase.
    let selected = RwSignal::new(None::<BackupInfo>);
    let target_path = RwSignal::new(None::<String>);
    let restore_passphrase = RwSignal::new(String::new());

    let restore_stage = RwSignal::new(None::<RestoreStage>);
    let error = RwSignal::new(None::<String>);

    // Stage updates arrive over the same restore:* events the settings
    // flow uses; the outcome is the invoke result.
    leptos::task::spawn_local_scoped_with_cancellation(async move {
        let _progress = ipc::listen_restore_progress(move |progress| {
            restore_stage.set(Some(progress.stage));
        })
        .await;
        std::future::pending::<()>().await;
    });

    let credentials = move || BackupConfig {
        backend_url: backend_url.get_untracked(),
        api_key: api_key.get_untracked(),
    };

    let on_list = move |_| {
        listing.set(true);
        error.set(None);
        leptos::task::spawn_local_scoped_with_cancellation(async move {
            match ipc::list_backups(Some(&credentials())).await {
                Ok(list) => snapshots.set(Some(list)),
                Err(e) => error.set(Some(format!("Could not list snapshots: {e}"))),
            }
            listing.set(false);
        });
    };

    let on_pick_target = move |_| {
        leptos::task::spawn_local_scoped_with_cancellation(async move {
            if let Some(path) = ipc::pick_folder().await {
                target_path.set(Some(path));
            }
        });
    };

    // Runs unscoped, like the settings restore: the app must switch to the
    // restored cave even if this dialog is closed mid-run.
    let on_restore = move |_| {
        let Some(info) = selected.get_untracked() else {
            return;
        };
        let Some(path) = target_path.get_untracked() else {
            return;
        };
        let passphrase = restore_passphrase.get_untracked();
        error.set(None);
        restore_stage.set(Some(RestoreStage::Downloading));
        leptos::task::spawn_local(async move {
            let result = ipc::restore_backup(
                &info.id.to_string(),
                &RestoreTarget::NewDirectory { path },
                Some(&passphrase),
                Some(&credentials()),
            )
            .await;
            restore_stage.try_set(None);
            match result {
                Ok(new_config) => {
                    ctx.apply_opened_cave(new_config).await;
                    set_open.set(false);
                }
                Err(e) => {
                    error.try_set(Some(e));
                }
            }
        });
    };

    let busy = move || listing.get() || restore_stage.get().is_some();
    let can_list =
        move || !backend_url.get().trim().is_empty() && !api_key.get().trim().is_empty() && !busy();
    let can_restore =
        move || target_path.get().is_some() && !restore_passphrase.get().is_empty() && !busy();

    view! {
        <Modal
            title="Restore from backup"
            subtitle="Recover a cave from your backup backend onto this machine"
            panel_class="w-[480px] max-w-[90vw]"
            on_close=Callback::new(move |()| set_open.set(false))
        >
            <div class="p-4 space-y-3 overflow-y-auto">
                {move || if selected.get().is_none() {
                    view! {
                        // ── Connection + snapshot list ─────────────
                        <div class="space-y-3">
                            <div class="space-y-1">
                                <label class="label text-xs text-base-content/50" for="restore-url">"Backend URL"</label>
                                <input
                                    id="restore-url"
                                    type="text"
                                    class="input input-bordered input-sm w-full"
                                    placeholder="https://backup.example.com"
                                    prop:value=move || backend_url.get()
                                    on:input=move |ev| backend_url.set(event_target_value(&ev))
                                />
                            </div>
                            <div class="space-y-1">
                                <label class="label text-xs text-base-content/50" for="restore-api-key">"API key"</label>
                                <input
                                    id="restore-api-key"
                                    type="password"
                                    class="input input-bordered input-sm w-full"
                                    placeholder="grnt_…"
                                    prop:value=move || api_key.get()
                                    on:input=move |ev| api_key.set(event_target_value(&ev))
                                />
                            </div>
                            <button
                                type="button"
                                class="btn btn-sm btn-primary"
                                disabled=move || !can_list()
                                on:click=on_list
                            >
                                {move || if listing.get() {
                                    view! { <span class="loading loading-spinner loading-xs"></span> }.into_any()
                                } else {
                                    view! { "List snapshots" }.into_any()
                                }}
                            </button>

                            {move || snapshots.get().map(|list| {
                                if list.is_empty() {
                                    return view! {
                                        <p class="text-xs text-base-content/35">"This key has no snapshots."</p>
                                    }.into_any();
                                }
                                view! {
                                    <div class="overflow-x-auto">
                                        <table class="table table-xs">
                                            <thead>
                                                <tr>
                                                    <th>"Taken (UTC)"</th>
                                                    <th>"Cave"</th>
                                                    <th>"Size"</th>
                                                    <th></th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {list.into_iter().map(|info| {
                                                    let is_complete = info.state == BackupState::Complete;
                                                    let pick_info = info.clone();
                                                    view! {
                                                        <tr>
                                                            <td>{info.created_at.format("%Y-%m-%d %H:%M").to_string()}</td>
                                                            <td>{info.cave_name}</td>
                                                            <td>{format_size(info.size_bytes)}</td>
                                                            <td>
                                                                // Pending uploads were never verified
                                                                // and cannot be restored.
                                                                {is_complete.then(move || view! {
                                                                    <button
                                                                        type="button"
                                                                        class="btn btn-ghost btn-xs"
                                                                        on:click=move |_| {
                                                                            selected.set(Some(pick_info.clone()));
                                                                            target_path.set(None);
                                                                            restore_passphrase.set(String::new());
                                                                            error.set(None);
                                                                        }
                                                                    >
                                                                        "Restore"
                                                                    </button>
                                                                })}
                                                            </td>
                                                        </tr>
                                                    }
                                                }).collect_view()}
                                            </tbody>
                                        </table>
                                    </div>
                                }.into_any()
                            })}
                        </div>
                    }.into_any()
                } else {
                    // ── Target folder + passphrase ─────────────────
                    let info = selected.get().expect("checked above");
                    let summary = format!(
                        "Snapshot from {} ({}, {})",
                        info.created_at.format("%Y-%m-%d %H:%M"),
                        info.cave_name,
                        format_size(info.size_bytes),
                    );
                    view! {
                        <div class="space-y-3">
                            <p class="text-xs font-medium">{summary}</p>
                            <div class="space-y-1">
                                <label class="label text-xs text-base-content/50">"Restore into"</label>
                                <p class="text-xs text-base-content/35">"Pick an empty folder; the restored cave opens from there."</p>
                                <div class="flex items-center gap-2">
                                    <button
                                        type="button"
                                        class="btn btn-sm shrink-0"
                                        disabled=busy
                                        on:click=on_pick_target
                                    >
                                        "Choose folder…"
                                    </button>
                                    <span class="text-xs text-base-content/60 truncate">
                                        {move || target_path.get().unwrap_or_else(|| "No folder chosen".to_string())}
                                    </span>
                                </div>
                            </div>
                            <div class="space-y-1">
                                <label class="label text-xs text-base-content/50" for="restore-passphrase">"Backup passphrase"</label>
                                <p class="text-xs text-base-content/35">"The passphrase this snapshot was encrypted with."</p>
                                <input
                                    id="restore-passphrase"
                                    type="password"
                                    class="input input-bordered input-sm w-full"
                                    prop:value=move || restore_passphrase.get()
                                    on:input=move |ev| restore_passphrase.set(event_target_value(&ev))
                                />
                            </div>
                            <div class="flex items-center gap-3">
                                <button
                                    type="button"
                                    class="btn btn-sm btn-primary"
                                    disabled=move || !can_restore()
                                    on:click=on_restore
                                >
                                    "Restore"
                                </button>
                                <button
                                    type="button"
                                    class="btn btn-sm btn-ghost"
                                    disabled=busy
                                    on:click=move |_| selected.set(None)
                                >
                                    "Back"
                                </button>
                                <Show when=move || restore_stage.get().is_some()>
                                    <span class="flex items-center gap-2 text-xs text-base-content/60">
                                        <span class="loading loading-spinner loading-xs"></span>
                                        {move || restore_stage.get().map(restore_stage_label).unwrap_or_default()}
                                    </span>
                                </Show>
                            </div>
                        </div>
                    }.into_any()
                }}

                {move || error.get().map(|e| view! {
                    <div class="alert alert-error py-2 text-xs">{e}</div>
                })}
            </div>
        </Modal>
    }
}
