use super::SettingsForm;
use crate::app::{ipc, AppCtx};
use granit_api::{BackupInfo, BackupState};
use granit_types::{BackupConfig, BackupStage};
use leptos::prelude::*;

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn stage_label(stage: BackupStage) -> &'static str {
    match stage {
        BackupStage::Packing => "Packing cave…",
        BackupStage::Encrypting => "Encrypting…",
        BackupStage::Uploading => "Uploading…",
    }
}

#[component]
pub fn BackupSettings(form: RwSignal<SettingsForm>) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let config = ctx.config;

    // Passphrase management state (saved immediately, outside the form's
    // Save button — the derived key lives in .granit/backup.key, not in
    // config.yml).
    let key_set = RwSignal::new(false);
    let passphrase = RwSignal::new(String::new());
    let passphrase_confirm = RwSignal::new(String::new());
    let setting_passphrase = RwSignal::new(false);
    let passphrase_message = RwSignal::new(None::<Result<String, String>>);

    // Running-backup state, fed by the backup:* events.
    let running_stage = RwSignal::new(None::<BackupStage>);
    let backup_message = RwSignal::new(None::<Result<String, String>>);

    // Snapshot list from the backend.
    let backups = RwSignal::new(None::<Result<Vec<BackupInfo>, String>>);

    let refresh_backups = move || {
        let configured = {
            let f = form.get_untracked();
            !f.backup_backend_url.trim().is_empty() && !f.backup_api_key.trim().is_empty()
        };
        if !configured {
            backups.set(None);
            return;
        }
        leptos::task::spawn_local_scoped_with_cancellation(async move {
            backups.set(Some(ipc::list_backups().await));
        });
    };

    // Initial loads on mount.
    leptos::task::spawn_local_scoped_with_cancellation(async move {
        if let Ok(set) = ipc::has_backup_key().await {
            key_set.set(set);
        }
    });
    refresh_backups();

    // Keep the event handles alive for the component's lifetime; unmount
    // cancels the future, dropping the handles and unlistening.
    leptos::task::spawn_local_scoped_with_cancellation(async move {
        let _progress = ipc::listen_backup_progress(move |progress| {
            running_stage.set(Some(progress.stage));
        })
        .await;
        let _done = ipc::listen_backup_done(move |info| {
            running_stage.set(None);
            backup_message.set(Some(Ok(format!(
                "Backed up {} ({})",
                info.cave_name,
                format_size(info.size_bytes)
            ))));
            refresh_backups();
        })
        .await;
        let _error = ipc::listen_backup_error(move |message| {
            running_stage.set(None);
            backup_message.set(Some(Err(message)));
        })
        .await;
        std::future::pending::<()>().await;
    });

    let on_set_passphrase = move |_| {
        let pass = passphrase.get_untracked();
        if pass != passphrase_confirm.get_untracked() {
            passphrase_message.set(Some(Err("Passphrases do not match".to_string())));
            return;
        }
        setting_passphrase.set(true);
        passphrase_message.set(None);
        leptos::task::spawn_local_scoped_with_cancellation(async move {
            match ipc::set_backup_passphrase(&pass).await {
                Ok(()) => {
                    key_set.set(true);
                    passphrase.set(String::new());
                    passphrase_confirm.set(String::new());
                    passphrase_message.set(Some(Ok("Passphrase set".to_string())));
                }
                Err(e) => passphrase_message.set(Some(Err(e))),
            }
            setting_passphrase.set(false);
        });
    };

    let on_backup_now = move |_| {
        backup_message.set(None);
        running_stage.set(Some(BackupStage::Packing));
        leptos::task::spawn_local_scoped_with_cancellation(async move {
            // The backend command reads the *saved* config, so persist the
            // connection fields first — otherwise an edited URL or key would
            // require Save (closing the modal) before backing up. Only the
            // backup section is saved here; other pending form edits still
            // belong to the Save button.
            let f = form.get_untracked();
            let next_backup = BackupConfig {
                backend_url: f.backup_backend_url,
                api_key: f.backup_api_key,
            };
            let mut next_config = config.get_untracked();
            if next_config.backup != next_backup {
                next_config.backup = next_backup;
                match ipc::save_config(next_config).await {
                    Ok(new_config) => config.set(new_config),
                    Err(e) => {
                        running_stage.set(None);
                        backup_message.set(Some(Err(format!("Could not save settings: {e}"))));
                        return;
                    }
                }
            }
            // Progress and the final outcome arrive via the backup:* events;
            // the invoke result only matters if the command fails before the
            // run starts (e.g. another backup in progress).
            if let Err(e) = ipc::backup_now().await {
                running_stage.set(None);
                backup_message.set(Some(Err(e)));
            }
        });
    };

    let connection_configured = move || {
        let f = form.get();
        !f.backup_backend_url.trim().is_empty() && !f.backup_api_key.trim().is_empty()
    };
    let can_backup =
        move || connection_configured() && key_set.get() && running_stage.get().is_none();

    view! {
        <fieldset class="fieldset space-y-3">
            <legend class="fieldset-legend">"Backup"</legend>

            // ── Connection ─────────────────────────────────────────
            <div class="space-y-1">
                <label class="label text-xs text-base-content/50" for="backup-url">"Backend URL"</label>
                // Long help text must not use the `label` class: DaisyUI's
                // .label is white-space:nowrap and would force the pane wide.
                <p class="text-xs text-base-content/35">"Public origin of your granit-server deployment, e.g. http://localhost:8080"</p>
                <input
                    id="backup-url"
                    type="text"
                    class="input input-bordered input-sm w-full"
                    placeholder="https://backup.example.com"
                    prop:value=move || form.get().backup_backend_url
                    on:input=move |ev| form.update(|f| f.backup_backend_url = event_target_value(&ev))
                />
            </div>
            <div class="space-y-1">
                <label class="label text-xs text-base-content/50" for="backup-api-key">"API key"</label>
                <p class="text-xs text-base-content/35">"Created with: granit-server create-key --name <name>"</p>
                <input
                    id="backup-api-key"
                    type="password"
                    class="input input-bordered input-sm w-full"
                    placeholder="grnt_…"
                    prop:value=move || form.get().backup_api_key
                    on:input=move |ev| form.update(|f| f.backup_api_key = event_target_value(&ev))
                />
                <p class="text-xs text-base-content/35">"Both fields are stored with the cave settings — Save and Back up now both apply them."</p>
            </div>

            // ── Encryption passphrase ──────────────────────────────
            <div class="space-y-1 pt-2">
                <label class="label text-xs text-base-content/50">"Encryption passphrase"</label>
                <p class="text-xs text-base-content/35">
                    {move || if key_set.get() {
                        "A passphrase is set for this cave. Setting a new one only affects future backups — older snapshots still need the passphrase they were made with."
                    } else {
                        "Backups are encrypted before upload. The passphrase never leaves this machine — and a forgotten passphrase makes the backups unrecoverable, so store it safely."
                    }}
                </p>
                <div class="space-y-2">
                    <input
                        type="password"
                        class="input input-bordered input-sm w-full"
                        placeholder="Passphrase (min 8 characters)"
                        prop:value=move || passphrase.get()
                        on:input=move |ev| passphrase.set(event_target_value(&ev))
                    />
                    <div class="flex gap-2">
                        <input
                            type="password"
                            class="input input-bordered input-sm flex-1"
                            placeholder="Repeat passphrase"
                            prop:value=move || passphrase_confirm.get()
                            on:input=move |ev| passphrase_confirm.set(event_target_value(&ev))
                        />
                        <button
                            type="button"
                            class="btn btn-sm shrink-0"
                            disabled=move || setting_passphrase.get() || passphrase.get().is_empty()
                            on:click=on_set_passphrase
                        >
                            {move || if setting_passphrase.get() {
                                view! { <span class="loading loading-spinner loading-xs"></span> }.into_any()
                            } else if key_set.get() {
                                view! { "Change" }.into_any()
                            } else {
                                view! { "Set" }.into_any()
                            }}
                        </button>
                    </div>
                </div>
                {move || passphrase_message.get().map(|result| match result {
                    Ok(msg) => view! { <p class="text-xs text-success">{msg}</p> }.into_any(),
                    Err(msg) => view! { <p class="text-xs text-error">{msg}</p> }.into_any(),
                })}
            </div>

            // ── Back up now ────────────────────────────────────────
            <div class="space-y-2 pt-2">
                <div class="flex items-center gap-3">
                    <button
                        type="button"
                        class="btn btn-sm btn-primary"
                        disabled=move || !can_backup()
                        on:click=on_backup_now
                    >
                        "Back up now"
                    </button>
                    <Show when=move || running_stage.get().is_some()>
                        <span class="flex items-center gap-2 text-xs text-base-content/60">
                            <span class="loading loading-spinner loading-xs"></span>
                            {move || running_stage.get().map(stage_label).unwrap_or_default()}
                        </span>
                    </Show>
                </div>
                <Show when=move || !connection_configured() || !key_set.get()>
                    <p class="text-xs text-base-content/35">"Fill in the backend URL, API key, and passphrase to enable backups."</p>
                </Show>
                {move || backup_message.get().map(|result| match result {
                    Ok(msg) => view! { <div class="alert alert-success py-2 text-xs">{msg}</div> }.into_any(),
                    Err(msg) => view! { <div class="alert alert-error py-2 text-xs">{msg}</div> }.into_any(),
                })}
            </div>

            // ── Snapshots ──────────────────────────────────────────
            <div class="space-y-1 pt-2">
                <label class="label text-xs text-base-content/50">"Snapshots"</label>
                {move || match backups.get() {
                    None => view! {
                        <p class="text-xs text-base-content/35">"Configure and save the connection to list snapshots."</p>
                    }.into_any(),
                    Some(Err(e)) => view! {
                        <p class="text-xs text-error">{format!("Could not list snapshots: {e}")}</p>
                    }.into_any(),
                    Some(Ok(list)) if list.is_empty() => view! {
                        <p class="text-xs text-base-content/35">"No snapshots yet."</p>
                    }.into_any(),
                    Some(Ok(list)) => view! {
                        <div class="overflow-x-auto">
                            <table class="table table-xs">
                                <thead>
                                    <tr>
                                        <th>"Taken (UTC)"</th>
                                        <th>"Cave"</th>
                                        <th>"Size"</th>
                                        <th>"State"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {list.into_iter().map(|info| {
                                        let state = match info.state {
                                            BackupState::Complete => view! {
                                                <span class="badge badge-success badge-xs">"complete"</span>
                                            }.into_any(),
                                            BackupState::Pending => view! {
                                                <span class="badge badge-warning badge-xs">"pending"</span>
                                            }.into_any(),
                                        };
                                        view! {
                                            <tr>
                                                <td>{info.created_at.format("%Y-%m-%d %H:%M").to_string()}</td>
                                                <td>{info.cave_name}</td>
                                                <td>{format_size(info.size_bytes)}</td>
                                                <td>{state}</td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    }.into_any(),
                }}
            </div>
        </fieldset>
    }
}
