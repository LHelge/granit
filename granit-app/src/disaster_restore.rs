//! Disaster restore: recover a cave from the backup backend when no cave is
//! open (a fresh machine, a lost disk), as the Tauri app's "Restore from
//! backup" dialog does. Shown in the main area of the no-cave screen.
//!
//! The backend URL and API key are typed here and only ride along with the
//! list and restore requests; nothing is saved. With no cave open there is
//! no cached key, so the snapshot's passphrase is always asked for. The
//! snapshot is restored into an empty folder, which then opens as the cave
//! (the workspace runs the restore, see [`backup_ops`]).

use std::path::PathBuf;

use gpui::{
    div, prelude::FluentBuilder as _, px, App, AppContext as _, Context, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement as _, IntoElement, ParentElement as _,
    PathPromptOptions, Render, SharedString, StatefulInteractiveElement as _, Styled as _,
    Subscription, Window,
};
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex, ActiveTheme as _, Disableable as _, IconName, Sizable as _,
};
use granit_core::backup::{BackupInfo, BackupState, RestoreRequest};
use granit_types::{BackupConfig, RestoreTarget};
use log::warn;

use crate::backup_ops::{self, BackupEvent, BackupOps, Running};
use crate::settings::backup::snapshot_row;

/// What the disaster-restore view asks of the workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisasterRestoreEvent {
    /// The close button was clicked.
    Close,
}

pub struct DisasterRestore {
    backend_url: Entity<InputState>,
    api_key: Entity<InputState>,
    passphrase: Entity<InputState>,
    /// Snapshots are being listed.
    listing: bool,
    /// The API key's snapshots, once listed.
    snapshots: Option<Vec<BackupInfo>>,
    /// The snapshot to restore.
    selected: Option<BackupInfo>,
    /// The folder to restore into.
    target: Option<PathBuf>,
    /// A restore this view asked for is running.
    started: bool,
    error: Option<SharedString>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<DisasterRestoreEvent> for DisasterRestore {}

impl DisasterRestore {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = |masked: bool,
                     placeholder: &'static str,
                     window: &mut Window,
                     cx: &mut Context<Self>| {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .masked(masked)
                    .placeholder(placeholder)
            })
        };
        let backend_url = input(false, "https://backup.example.com", window, cx);
        let api_key = input(true, "grnt_…", window, cx);
        let passphrase = input(true, "Backup passphrase", window, cx);
        let ops = BackupOps::entity(cx);
        let mut subscriptions = vec![
            cx.observe(&ops, |_, _, cx| cx.notify()),
            cx.subscribe(&ops, |this, _, event: &BackupEvent, cx| {
                if let BackupEvent::RestoreFailed(failed) = event {
                    if std::mem::take(&mut this.started) {
                        this.error = Some(failed.message.clone().into());
                        cx.notify();
                    }
                }
            }),
        ];
        // The buttons follow whether the inputs are empty.
        for input in [&backend_url, &api_key, &passphrase] {
            subscriptions.push(cx.subscribe(input, |_, _, _: &InputEvent, cx| cx.notify()));
        }
        Self {
            backend_url,
            api_key,
            passphrase,
            listing: false,
            snapshots: None,
            selected: None,
            target: None,
            started: false,
            error: None,
            focus_handle: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }

    fn credentials(&self, cx: &App) -> BackupConfig {
        BackupConfig {
            backend_url: self.backend_url.read(cx).value().to_string(),
            api_key: self.api_key.read(cx).value().to_string(),
        }
    }

    /// The listed snapshots.
    #[cfg(test)]
    pub fn snapshots(&self) -> Option<&[BackupInfo]> {
        self.snapshots.as_deref()
    }

    /// The error shown, if any.
    #[cfg(test)]
    pub fn error(&self) -> Option<&SharedString> {
        self.error.as_ref()
    }

    /// Whether a listing is running.
    #[cfg(test)]
    pub fn is_listing(&self) -> bool {
        self.listing
    }

    /// The inputs: backend URL, API key, passphrase.
    #[cfg(test)]
    pub fn inputs(&self) -> [Entity<InputState>; 3] {
        [
            self.backend_url.clone(),
            self.api_key.clone(),
            self.passphrase.clone(),
        ]
    }

    /// Show `message` (the restored cave failed to open, say); a restore
    /// this view asked for is over.
    pub fn show_error(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.started = false;
        self.error = Some(message.into());
        cx.notify();
    }

    /// List the snapshots of the typed credentials.
    pub fn list(&mut self, cx: &mut Context<Self>) {
        self.listing = true;
        self.error = None;
        let task = backup_ops::list(Some(self.credentials(cx)), cx);
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.listing = false;
                match result {
                    Ok(list) => this.snapshots = Some(list),
                    Err(err) => {
                        this.error = Some(format!("Could not list snapshots: {err}").into())
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Choose the snapshot to restore.
    pub fn select(&mut self, info: Option<BackupInfo>, cx: &mut Context<Self>) {
        self.selected = info;
        self.target = None;
        self.error = None;
        cx.notify();
    }

    /// Restore into `path` (the folder picker's answer).
    #[cfg(test)]
    pub fn set_target(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.target = Some(path);
        cx.notify();
    }

    fn pick_target(&mut self, cx: &mut Context<Self>) {
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
                    this.target = Some(path);
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    /// Ask the workspace to restore the chosen snapshot into the chosen
    /// folder with the typed passphrase; the cave opens when it is done.
    pub fn restore(&mut self, cx: &mut Context<Self>) {
        let (Some(info), Some(target)) = (&self.selected, &self.target) else {
            return;
        };
        let request = RestoreRequest {
            id: info.id,
            target: RestoreTarget::NewDirectory {
                path: target.display().to_string(),
            },
            passphrase: Some(self.passphrase.read(cx).value().to_string()),
            credentials: Some(self.credentials(cx)),
        };
        self.error = None;
        self.started = true;
        cx.notify();
        backup_ops::request_restore(request, cx);
    }

    fn render_connection(&self, busy: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let credentials = self.credentials(cx);
        let label = |text: &'static str| {
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(text)
        };
        v_flex()
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(label("Backend URL"))
                    .child(Input::new(&self.backend_url).small()),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(label("API key"))
                    .child(Input::new(&self.api_key).small().mask_toggle()),
            )
            .child(
                h_flex().child(
                    Button::new("disaster-list")
                        .primary()
                        .small()
                        .label("List snapshots")
                        .loading(self.listing)
                        .disabled(!credentials.is_configured() || busy)
                        .on_click(cx.listener(|this, _, _, cx| this.list(cx))),
                ),
            )
            .children(self.snapshots.as_ref().map(|list| {
                if list.is_empty() {
                    return label("This key has no snapshots.").into_any_element();
                }
                v_flex()
                    .w_full()
                    .children(list.iter().enumerate().map(|(index, info)| {
                        let pick = info.clone();
                        snapshot_row(info, cx)
                            // Pending uploads were never verified and
                            // cannot be restored.
                            .when(info.state == BackupState::Complete, |row| {
                                row.child(
                                    Button::new(("disaster-pick", index))
                                        .ghost()
                                        .xsmall()
                                        .label("Restore…")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.select(Some(pick.clone()), cx)
                                        })),
                                )
                            })
                    }))
                    .into_any_element()
            }))
    }

    fn render_target(
        &self,
        info: &BackupInfo,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let label = |text: &'static str| {
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(text)
        };
        let target = self.target.as_ref().map_or_else(
            || "No folder chosen".to_string(),
            |path| path.display().to_string(),
        );
        let ready = self.target.is_some() && !self.passphrase.read(cx).value().is_empty();
        let running =
            BackupOps::running(cx).filter(|running| matches!(running, Running::Restore(_)));
        v_flex()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(format!("Snapshot from {}", backup_ops::summary(info))),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(label("Restore into"))
                    .child(label(
                        "Pick an empty folder; the restored cave opens from there.",
                    ))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("disaster-target")
                                    .small()
                                    .label("Choose folder…")
                                    .disabled(busy)
                                    .on_click(cx.listener(|this, _, _, cx| this.pick_target(cx))),
                            )
                            .child(div().min_w_0().truncate().text_sm().child(target)),
                    ),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(label("Backup passphrase"))
                    .child(label("The passphrase this snapshot was encrypted with."))
                    .child(Input::new(&self.passphrase).small().mask_toggle()),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Button::new("disaster-restore")
                            .primary()
                            .small()
                            .label("Restore")
                            .loading(running.is_some())
                            .disabled(!ready || busy)
                            .on_click(cx.listener(|this, _, _, cx| this.restore(cx))),
                    )
                    .child(
                        Button::new("disaster-back")
                            .ghost()
                            .small()
                            .label("Back")
                            .disabled(busy)
                            .on_click(cx.listener(|this, _, _, cx| this.select(None, cx))),
                    )
                    .children(running.map(|running| label(running.label()))),
            )
    }
}

impl Render for DisasterRestore {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.listing || BackupOps::running(cx).is_some();
        let body = match self.selected.clone() {
            None => self.render_connection(busy, cx).into_any_element(),
            Some(info) => self.render_target(&info, busy, cx).into_any_element(),
        };
        let theme = cx.theme();
        div()
            .id("disaster-restore")
            .track_focus(&self.focus_handle)
            .size_full()
            .overflow_y_scroll()
            .flex()
            .justify_center()
            .p_6()
            .child(
                v_flex()
                    .w_full()
                    .max_w(px(560.))
                    .gap_4()
                    .child(
                        h_flex()
                            .gap_3()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child("Restore from backup"),
                                    )
                                    .child(
                                        div().text_sm().text_color(theme.muted_foreground).child(
                                            "Recover a cave from your backup backend onto this \
                                             machine",
                                        ),
                                    ),
                            )
                            .child(
                                Button::new("close-disaster-restore")
                                    .ghost()
                                    .small()
                                    .icon(IconName::Close)
                                    .tooltip("Close")
                                    .on_click(cx.listener(|_, _, _, cx| {
                                        cx.emit(DisasterRestoreEvent::Close)
                                    })),
                            ),
                    )
                    .child(body)
                    .children(
                        self.error
                            .clone()
                            .map(|err| div().text_sm().text_color(theme.danger).child(err)),
                    ),
            )
    }
}

impl Focusable for DisasterRestore {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
