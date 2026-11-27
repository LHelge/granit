//! The Backup page: the backend connection (saved with the cave config),
//! the encryption passphrase (its derived key cached in
//! `.granit/backup.key`; the passphrase itself is never stored), backing up
//! now, and the snapshot list with restore and delete, as in the Tauri
//! app's Backup settings.
//!
//! The page's own state (passphrase inputs, the listed snapshots, the
//! restore flow of one snapshot) lives in a [`BackupPanel`] owned by the
//! settings view. Backups and restores run through [`backup_ops`]; a
//! restore is run by the workspace (it opens the restored cave), so it
//! finishes even when the settings close. Restoring over the current cave
//! and deleting a snapshot ask first, with a dialog.

use gpui::{
    div, prelude::FluentBuilder as _, px, AnyElement, App, AppContext as _, Context, Entity,
    IntoElement, ParentElement as _, PathPromptOptions, PromptLevel, SharedString, Styled as _,
    Subscription, Window,
};
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    setting::{SettingGroup, SettingItem, SettingPage},
    v_flex, ActiveTheme as _, Disableable as _, Sizable as _,
};
use granit_core::backup::{BackupInfo, BackupState, RestoreRequest};
use granit_types::{BackupConfig, RestoreTarget};
use log::warn;

use super::fields::{text_field, TextFieldOptions};
use super::{getter, SettingsCtx};
use crate::assets::AppIcon;
use crate::backup_ops::{self, BackupEvent, BackupOps, Running};
use crate::config_ops::{ConfigEvent, ConfigEvents};
use crate::host::Host;

/// The answer of the delete dialog that deletes.
pub const DELETE_ANSWER: &str = "Delete snapshot";
/// The answer of the replace dialog that replaces the cave.
pub const REPLACE_ANSWER: &str = "Replace current cave";

/// The Backup page.
pub fn page(ctx: &SettingsCtx, panel: &Entity<BackupPanel>) -> SettingPage {
    let section =
        |render: fn(&mut BackupPanel, &mut Window, &mut Context<BackupPanel>) -> AnyElement| {
            let panel = panel.clone();
            SettingItem::render(move |_, window, cx| {
                panel.update(cx, |panel, cx| render(panel, window, cx))
            })
        };
    SettingPage::new("Backup")
        .icon(AppIcon::CloudUpload)
        .resettable(false)
        .group(
            SettingGroup::new()
                .title("Connection")
                .description("Stored with the cave settings in .granit/config.yml.")
                .item(
                    SettingItem::new(
                        "Backend URL",
                        text_field(
                            "backup-url",
                            TextFieldOptions {
                                masked: false,
                                placeholder: "https://backup.example.com",
                            },
                            getter(|config| config.backup.backend_url.clone()),
                            ctx.typed_setter(|config, url: String| config.backup.backend_url = url),
                        ),
                    )
                    .description(
                        "Public origin of your granit-server deployment, e.g. \
                         http://localhost:8080",
                    ),
                )
                .item(
                    SettingItem::new(
                        "API key",
                        text_field(
                            "backup-api-key",
                            TextFieldOptions {
                                masked: true,
                                placeholder: "grnt_…",
                            },
                            getter(|config| config.backup.api_key.clone()),
                            ctx.typed_setter(|config, key: String| config.backup.api_key = key),
                        ),
                    )
                    .description("Created with: granit-server create-key --name <name>"),
                ),
        )
        .group(
            SettingGroup::new()
                .title("Encryption passphrase")
                .item(section(BackupPanel::render_passphrase)),
        )
        .group(
            SettingGroup::new()
                .title("Back up")
                .item(section(BackupPanel::render_backup)),
        )
        .group(
            SettingGroup::new()
                .title("Snapshots")
                .item(section(BackupPanel::render_snapshots)),
        )
}

/// The snapshot list as last loaded.
enum Snapshots {
    /// Not asked for yet (or the connection is not filled in).
    NotLoaded,
    Loading,
    Loaded(Vec<BackupInfo>),
    Failed(SharedString),
}

/// The restore of one snapshot, from choosing it to its outcome.
struct RestoreFlow {
    info: BackupInfo,
    /// The target of the restore this panel started and is waiting for.
    started: Option<RestoreTarget>,
    /// The cached key does not fit: the target to retry with a passphrase.
    needs_passphrase: Option<RestoreTarget>,
    error: Option<SharedString>,
}

/// The Backup page's state.
pub struct BackupPanel {
    passphrase: Entity<InputState>,
    passphrase_confirm: Entity<InputState>,
    setting_passphrase: bool,
    passphrase_message: Option<Result<SharedString, SharedString>>,
    backup_message: Option<Result<SharedString, SharedString>>,
    snapshots: Snapshots,
    restore: Option<RestoreFlow>,
    restore_passphrase: Entity<InputState>,
    deleting: bool,
    delete_error: Option<SharedString>,
    /// The credentials the snapshots were last listed with.
    listed_with: BackupConfig,
    _subscriptions: Vec<Subscription>,
}

impl BackupPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let masked = |placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .masked(true)
                    .placeholder(placeholder)
            })
        };
        let passphrase = masked("Passphrase (min 8 characters)", window, cx);
        let passphrase_confirm = masked("Repeat passphrase", window, cx);
        let restore_passphrase = masked("Backup passphrase", window, cx);
        let ops = BackupOps::entity(cx);
        let mut subscriptions = vec![
            // Redraw the stage of a running backup or restore.
            cx.observe(&ops, |_, _, cx| cx.notify()),
            cx.subscribe(&ops, |this, _, event: &BackupEvent, cx| {
                this.on_backup_event(event, cx)
            }),
        ];
        // The buttons follow whether the inputs are empty.
        for input in [&passphrase, &passphrase_confirm, &restore_passphrase] {
            subscriptions.push(cx.subscribe(input, |_, _, _: &InputEvent, cx| cx.notify()));
        }
        // A config event follows a commit, so the config then holds the
        // saved credentials (typed ones only after the typing pause, Enter
        // or blur): list again when they changed.
        subscriptions.push(cx.subscribe(
            &ConfigEvents::entity(cx),
            |this, _, _: &ConfigEvent, cx| {
                if Host::core(cx).lock_config().backup != this.listed_with {
                    this.refresh(cx);
                }
            },
        ));
        let mut this = Self {
            passphrase,
            passphrase_confirm,
            setting_passphrase: false,
            passphrase_message: None,
            backup_message: None,
            snapshots: Snapshots::NotLoaded,
            restore: None,
            restore_passphrase,
            deleting: false,
            delete_error: None,
            listed_with: BackupConfig::default(),
            _subscriptions: subscriptions,
        };
        // The snapshots are listed once, when the settings open (with the
        // saved credentials: pending edits were committed when the settings
        // last closed).
        this.refresh(cx);
        this
    }

    fn on_backup_event(&mut self, event: &BackupEvent, cx: &mut Context<Self>) {
        match event {
            BackupEvent::BackedUp(Ok(info)) => {
                self.backup_message = Some(Ok(format!(
                    "Backed up {} ({})",
                    info.cave_name,
                    backup_ops::format_size(info.size_bytes)
                )
                .into()));
                self.refresh(cx);
            }
            BackupEvent::BackedUp(Err(err)) => {
                self.backup_message = Some(Err(err.clone().into()));
            }
            BackupEvent::RestoreFailed(failed) => {
                let Some(flow) = &mut self.restore else {
                    return;
                };
                let Some(target) = flow.started.take() else {
                    return;
                };
                if failed.needs_passphrase {
                    flow.needs_passphrase = Some(target);
                } else {
                    // A wrong passphrase on a retry keeps the prompt open
                    // (`needs_passphrase` stays set) with this error.
                    flow.error = Some(failed.message.clone().into());
                }
            }
            BackupEvent::RestoreRequested(_) => {}
        }
        cx.notify();
    }

    /// List the snapshots again, if the connection is filled in.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.listed_with = Host::core(cx).lock_config().backup.clone();
        if !self.listed_with.is_configured() {
            self.snapshots = Snapshots::NotLoaded;
            cx.notify();
            return;
        }
        self.snapshots = Snapshots::Loading;
        let task = backup_ops::list(None, cx);
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.snapshots = match result {
                    Ok(list) => Snapshots::Loaded(list),
                    Err(err) => {
                        Snapshots::Failed(format!("Could not list snapshots: {err}").into())
                    }
                };
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Show `snapshots` as if they had been listed.
    #[cfg(test)]
    pub fn set_snapshots(&mut self, snapshots: Vec<BackupInfo>, cx: &mut Context<Self>) {
        self.snapshots = Snapshots::Loaded(snapshots);
        cx.notify();
    }

    /// The listed snapshots.
    #[cfg(test)]
    pub fn snapshots(&self) -> Option<&[BackupInfo]> {
        match &self.snapshots {
            Snapshots::Loaded(list) => Some(list),
            _ => None,
        }
    }

    /// Why listing the snapshots failed.
    #[cfg(test)]
    pub fn list_error(&self) -> Option<&SharedString> {
        match &self.snapshots {
            Snapshots::Failed(err) => Some(err),
            _ => None,
        }
    }

    /// The snapshot whose restore is shown, and its error.
    #[cfg(test)]
    pub fn restore_state(&self) -> Option<(&BackupInfo, Option<&SharedString>)> {
        self.restore
            .as_ref()
            .map(|flow| (&flow.info, flow.error.as_ref()))
    }

    /// Why the last deletion failed.
    #[cfg(test)]
    pub fn delete_error(&self) -> Option<&SharedString> {
        self.delete_error.as_ref()
    }

    /// Whether a deletion is running.
    #[cfg(test)]
    pub fn is_deleting(&self) -> bool {
        self.deleting
    }

    fn set_passphrase(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let passphrase = self.passphrase.read(cx).value().to_string();
        if passphrase != self.passphrase_confirm.read(cx).value().as_ref() {
            self.passphrase_message = Some(Err("Passphrases do not match".into()));
            cx.notify();
            return;
        }
        self.setting_passphrase = true;
        self.passphrase_message = None;
        let task = backup_ops::set_passphrase(passphrase, cx);
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            this.update_in(cx, |this, window, cx| {
                this.setting_passphrase = false;
                this.passphrase_message = Some(match result {
                    Ok(()) => {
                        for input in [&this.passphrase, &this.passphrase_confirm] {
                            input.update(cx, |input, cx| input.set_value("", window, cx));
                        }
                        Ok("Passphrase set".into())
                    }
                    Err(err) => Err(err.into()),
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Open the restore flow of `info`.
    pub fn choose_restore(
        &mut self,
        info: BackupInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.delete_error = None;
        self.restore_passphrase
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.restore = Some(RestoreFlow {
            info,
            started: None,
            needs_passphrase: None,
            error: None,
        });
        cx.notify();
    }

    /// Ask the workspace to restore the chosen snapshot into `target`.
    fn start_restore(
        &mut self,
        target: RestoreTarget,
        passphrase: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if let Some(request) = self.begin_restore(target, passphrase, cx) {
            backup_ops::request_restore(request, cx);
        }
    }

    /// The flow's state for a restore into `target` about to start, and its
    /// request.
    pub(crate) fn begin_restore(
        &mut self,
        target: RestoreTarget,
        passphrase: Option<String>,
        cx: &mut Context<Self>,
    ) -> Option<RestoreRequest> {
        let flow = self.restore.as_mut()?;
        flow.error = None;
        // A retry with a passphrase keeps its prompt, so a wrong one shows
        // its error under it.
        if passphrase.is_none() {
            flow.needs_passphrase = None;
        }
        flow.started = Some(target.clone());
        let request = RestoreRequest {
            id: flow.info.id,
            target,
            passphrase,
            credentials: None,
        };
        cx.notify();
        Some(request)
    }

    /// Whether the passphrase prompt is shown.
    #[cfg(test)]
    pub fn asks_for_passphrase(&self) -> bool {
        self.restore
            .as_ref()
            .is_some_and(|flow| flow.needs_passphrase.is_some())
    }

    /// The restore's passphrase input.
    #[cfg(test)]
    pub fn restore_passphrase(&self) -> &Entity<InputState> {
        &self.restore_passphrase
    }

    /// Whether the snapshots are being listed.
    #[cfg(test)]
    pub fn is_loading(&self) -> bool {
        matches!(self.snapshots, Snapshots::Loading)
    }

    /// Ask for an empty folder and restore the chosen snapshot into it.
    fn restore_to_new_folder(&mut self, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Restore here".into()),
        });
        cx.spawn(async move |this, cx| {
            let picked = match paths.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) | Err(_) => None,
                Ok(Err(err)) => {
                    warn!("The folder picker failed: {err}");
                    None
                }
            };
            if let Some(path) = picked {
                this.update(cx, |this, cx| {
                    let target = RestoreTarget::NewDirectory {
                        path: path.display().to_string(),
                    };
                    this.start_restore(target, None, cx)
                })
                .ok();
            }
        })
        .detach();
    }

    /// Ask before replacing the current cave with the chosen snapshot.
    pub fn request_replace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(flow) = &self.restore else {
            return;
        };
        let detail = format!(
            "This replaces everything in the current cave with the snapshot from {}. The \
             current contents are kept beside the cave folder as a \
             \"<cave>.pre-restore-<timestamp>\" copy until you delete it.",
            backup_ops::summary(&flow.info)
        );
        let answer = window.prompt(
            PromptLevel::Warning,
            "Replace the current cave with this snapshot?",
            Some(&detail),
            &[REPLACE_ANSWER, "Cancel"],
            cx,
        );
        cx.spawn(async move |this, cx| {
            if answer.await == Ok(0) {
                this.update(cx, |this, cx| {
                    this.start_restore(RestoreTarget::CurrentCave, None, cx)
                })
                .ok();
            }
        })
        .detach();
    }

    /// Retry the restore that needed a passphrase, with the typed one.
    fn restore_with_passphrase(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self
            .restore
            .as_ref()
            .and_then(|flow| flow.needs_passphrase.clone())
        else {
            return;
        };
        let passphrase = self.restore_passphrase.read(cx).value().to_string();
        self.start_restore(target, Some(passphrase), cx);
    }

    /// Ask before deleting `info` from the backend.
    pub fn request_delete(
        &mut self,
        info: BackupInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.restore = None;
        self.delete_error = None;
        let message = format!("Delete the snapshot from {}?", backup_ops::summary(&info));
        let answer = window.prompt(
            PromptLevel::Warning,
            &message,
            Some("This permanently removes the snapshot from the backend. It cannot be recovered."),
            &[DELETE_ANSWER, "Cancel"],
            cx,
        );
        cx.spawn(async move |this, cx| {
            if answer.await == Ok(0) {
                this.update(cx, |this, cx| this.delete(info, cx)).ok();
            }
        })
        .detach();
        cx.notify();
    }

    fn delete(&mut self, info: BackupInfo, cx: &mut Context<Self>) {
        self.deleting = true;
        let task = backup_ops::delete(&info, cx);
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.deleting = false;
                match result {
                    Ok(()) => this.refresh(cx),
                    Err(err) => {
                        this.delete_error = Some(format!("Could not delete snapshot: {err}").into())
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn render_passphrase(&mut self, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let key_set = backup_ops::has_key(cx);
        let hint = if key_set {
            "A passphrase is set for this cave. Setting a new one only affects future backups \
             — older snapshots still need the passphrase they were made with."
        } else {
            "Backups are encrypted before upload. The passphrase never leaves this machine — and \
             a forgotten passphrase makes the backups unrecoverable, so store it safely."
        };
        let empty = self.passphrase.read(cx).value().is_empty();
        v_flex()
            .w_full()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(hint),
            )
            .child(Input::new(&self.passphrase).small().mask_toggle())
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.passphrase_confirm).small().mask_toggle()),
                    )
                    .child(
                        Button::new("set-backup-passphrase")
                            .small()
                            .label(if key_set { "Change" } else { "Set" })
                            .loading(self.setting_passphrase)
                            .disabled(self.setting_passphrase || empty)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.set_passphrase(window, cx)),
                            ),
                    ),
            )
            .children(message(&self.passphrase_message, cx))
            .into_any_element()
    }

    fn render_backup(&mut self, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let configured = backup_ops::is_configured(cx);
        let key_set = backup_ops::has_key(cx);
        let running = BackupOps::running(cx);
        let backing_up = matches!(running, Some(Running::Backup(_)));
        v_flex()
            .w_full()
            .gap_2()
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Button::new("backup-now")
                            .primary()
                            .small()
                            .label("Back up now")
                            .loading(backing_up)
                            .disabled(!configured || !key_set || running.is_some())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.backup_message = None;
                                backup_ops::backup_now(cx);
                                cx.notify();
                            })),
                    )
                    .children(running.filter(|_| backing_up).map(|running| {
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(running.label())
                    })),
            )
            .when(!configured || !key_set, |this| {
                this.child(
                    div().text_sm().text_color(theme.muted_foreground).child(
                        "Fill in the backend URL, API key, and passphrase to enable backups.",
                    ),
                )
            })
            .children(message(&self.backup_message, cx))
            .into_any_element()
    }

    fn render_snapshots(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let muted = |text: SharedString| {
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(text)
                .into_any_element()
        };
        let busy = BackupOps::running(cx).is_some() || self.deleting;
        let list = match &self.snapshots {
            Snapshots::NotLoaded => {
                muted("Fill in the backend URL and API key to list snapshots.".into())
            }
            Snapshots::Loading => muted("Loading snapshots…".into()),
            Snapshots::Failed(err) => div()
                .text_sm()
                .text_color(theme.danger)
                .child(err.clone())
                .into_any_element(),
            Snapshots::Loaded(list) if list.is_empty() => muted("No snapshots yet.".into()),
            Snapshots::Loaded(list) => v_flex()
                .w_full()
                .child(snapshot_header(cx))
                .children(list.iter().enumerate().map(|(index, info)| {
                    let restore_info = info.clone();
                    let delete_info = info.clone();
                    snapshot_row(info, cx)
                        .child(
                            h_flex()
                                .gap_1()
                                // Only verified snapshots can be restored;
                                // the backend answers 409 for pending ones.
                                .when(info.state == BackupState::Complete, |this| {
                                    this.child(
                                        Button::new(("restore-snapshot", index))
                                            .ghost()
                                            .xsmall()
                                            .label("Restore…")
                                            .disabled(busy)
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.choose_restore(
                                                    restore_info.clone(),
                                                    window,
                                                    cx,
                                                )
                                            })),
                                    )
                                })
                                .child(
                                    Button::new(("delete-snapshot", index))
                                        .ghost()
                                        .xsmall()
                                        .icon(AppIcon::Trash)
                                        .label("Delete")
                                        .disabled(busy)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.request_delete(delete_info.clone(), window, cx)
                                        })),
                                ),
                        )
                        .into_any_element()
                }))
                .into_any_element(),
        };
        v_flex()
            .w_full()
            .gap_2()
            .child(
                h_flex().child(
                    Button::new("refresh-snapshots")
                        .outline()
                        .xsmall()
                        .label("Refresh")
                        .disabled(!backup_ops::is_configured(cx) || busy)
                        .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                ),
            )
            .child(list)
            .children(
                self.delete_error
                    .clone()
                    .map(|err| div().text_sm().text_color(theme.danger).child(err)),
            )
            .children(self.render_restore(window, cx))
            .into_any_element()
    }

    /// The restore flow of the chosen snapshot.
    fn render_restore(&self, _: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let flow = self.restore.as_ref()?;
        let theme = cx.theme();
        let restoring = match BackupOps::running(cx) {
            Some(running @ Running::Restore(_)) => Some(running),
            _ => None,
        };
        let text = |text: &'static str| {
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(text)
        };
        let body = if let Some(running) = restoring {
            text(running.label()).into_any_element()
        } else if flow.needs_passphrase.is_some() {
            v_flex()
                .gap_2()
                .child(text(
                    "This cave's cached key does not fit this snapshot. Enter the passphrase \
                     the snapshot was encrypted with.",
                ))
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .child(Input::new(&self.restore_passphrase).small().mask_toggle()),
                        )
                        .child(
                            Button::new("restore-with-passphrase")
                                .primary()
                                .small()
                                .label("Restore")
                                .disabled(self.restore_passphrase.read(cx).value().is_empty())
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.restore_with_passphrase(cx)),
                                ),
                        ),
                )
                .into_any_element()
        } else {
            let busy = BackupOps::running(cx).is_some();
            v_flex()
                .gap_2()
                .child(text(
                    "Restore into a new empty folder and switch to it, or replace the contents \
                     of the current cave.",
                ))
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("restore-new-folder")
                                .small()
                                .label("Restore to new folder…")
                                .disabled(busy)
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.restore_to_new_folder(cx)),
                                ),
                        )
                        .child(
                            Button::new("restore-replace")
                                .warning()
                                .small()
                                .label("Replace current cave…")
                                .disabled(busy)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.request_replace(window, cx)
                                })),
                        ),
                )
                .into_any_element()
        };
        Some(
            v_flex()
                .w_full()
                .gap_2()
                .p_3()
                .border_1()
                .border_color(theme.border)
                .rounded(theme.radius)
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_sm()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child(format!(
                                    "Restore snapshot from {}",
                                    backup_ops::summary(&flow.info)
                                )),
                        )
                        .child(
                            Button::new("cancel-restore")
                                .ghost()
                                .xsmall()
                                .label("Cancel")
                                .disabled(restoring.is_some())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.restore = None;
                                    cx.notify();
                                })),
                        ),
                )
                .child(body)
                .children(
                    flow.error
                        .clone()
                        .map(|err| div().text_sm().text_color(theme.danger).child(err)),
                )
                .into_any_element(),
        )
    }
}

/// A success or failure line.
fn message(message: &Option<Result<SharedString, SharedString>>, cx: &App) -> Option<AnyElement> {
    let theme = cx.theme();
    message.as_ref().map(|message| {
        let (text, color) = match message {
            Ok(text) => (text.clone(), theme.success),
            Err(text) => (text.clone(), theme.danger),
        };
        div()
            .text_sm()
            .text_color(color)
            .child(text)
            .into_any_element()
    })
}

/// The column widths of the snapshot table.
const TAKEN_WIDTH: f32 = 130.;
const CAVE_WIDTH: f32 = 140.;
const SIZE_WIDTH: f32 = 80.;
const STATE_WIDTH: f32 = 80.;

fn snapshot_header(cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    h_flex()
        .w_full()
        .gap_2()
        .py_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .border_b_1()
        .border_color(theme.border)
        .child(div().w(px(TAKEN_WIDTH)).child("Taken (UTC)"))
        .child(div().w(px(CAVE_WIDTH)).child("Cave"))
        .child(div().w(px(SIZE_WIDTH)).child("Size"))
        .child(div().w(px(STATE_WIDTH)).child("State"))
}

/// One snapshot's cells; the caller adds the actions.
pub fn snapshot_row(info: &BackupInfo, cx: &App) -> gpui::Div {
    let theme = cx.theme();
    let (state, color) = match info.state {
        BackupState::Complete => ("complete", theme.success),
        BackupState::Pending => ("pending", theme.warning),
    };
    h_flex()
        .w_full()
        .gap_2()
        .py_1()
        .text_sm()
        .child(
            div()
                .w(px(TAKEN_WIDTH))
                .child(info.created_at.format("%Y-%m-%d %H:%M").to_string()),
        )
        .child(
            div()
                .w(px(CAVE_WIDTH))
                .truncate()
                .child(info.cave_name.clone()),
        )
        .child(
            div()
                .w(px(SIZE_WIDTH))
                .child(backup_ops::format_size(info.size_bytes)),
        )
        .child(div().w(px(STATE_WIDTH)).text_color(color).child(state))
}
