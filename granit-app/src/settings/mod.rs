//! The settings view: the open cave's `.granit/config.yml`, edited in the
//! main area with GPUI Kit's `Settings` component (a page list with search
//! on the left, the selected page's groups on the right).
//!
//! Every field reads the core's config when it renders. Switches, pickers
//! and buttons write through [`SettingsCtx::setter`] / [`SettingsCtx::update`]
//! ([`config_ops::update_config`]): saved at once and applied live (theme,
//! fonts, agent reset, RAG index), so there is no Save button. Text and
//! number fields write through [`SettingsCtx::typed_setter`]
//! ([`config_ops::edit_config`]): shown at once, saved and applied when
//! typing pauses, on Enter or blur, or when the view closes. A failed write
//! shows above the pages.
//!
//! Each page is a module with a `page(&SettingsCtx, cx) -> SettingPage`
//! function; adding a section is a new module and a line in
//! [`SettingsView::pages`]. [`fields`] has the bound text input (masked for
//! secrets) the Kit's own fields lack.
//!
//! Not here, by design: theme choices beyond Catppuccin light and dark,
//! `markdown_font` (the Tauri app's raw-source editor font; the live-preview
//! editor uses `reading_font`, shown as the note font), presentation and Mermaid settings
//! (dropped from this app).
//!
//! The Backup page ([`backup`]) keeps its own state (passphrase inputs,
//! snapshots, a restore flow) in a [`backup::BackupPanel`] the view owns.

mod agent;
mod appearance;
pub mod backup;
pub mod fields;
mod notes;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use gpui::{
    div, App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Task, WeakEntity,
    Window,
};
use gpui_kit::component::{
    button::{Button, ButtonVariants},
    group_box::GroupBoxVariant,
    h_flex,
    input::{InputEvent, TextareaState},
    setting::{SelectIndex, SettingPage, Settings},
    v_flex, ActiveTheme, IconName, Sizable,
};
use granit_types::AppConfig;

use crate::config_ops::{self, ConfigEvent, ConfigEvents, ConfigSaveFailed};
use crate::host::Host;

/// What the settings view asks of the workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsEvent {
    /// The close button was clicked.
    Close,
}

pub struct SettingsView {
    /// The last failed write, shown until the next one succeeds.
    error: Option<SharedString>,
    /// The installed font families, listed once when the view opens.
    fonts: Vec<SharedString>,
    /// The agent's system prompt (`.granit/agent/system.md`).
    prompt: Entity<TextareaState>,
    /// Writes the prompt after a typing pause; replaced by every edit.
    prompt_commit: Option<Task<()>>,
    /// The Backup page's state.
    backup: Entity<backup::BackupPanel>,
    /// The cave these settings belong to.
    cave: Option<PathBuf>,
    /// The page shown when the view opens.
    start_page: usize,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl gpui::EventEmitter<SettingsEvent> for SettingsView {}

impl SettingsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut fonts: Vec<SharedString> = cx
            .text_system()
            .all_font_names()
            .into_iter()
            .filter(|name| !name.starts_with('.'))
            .map(SharedString::from)
            .collect();
        fonts.sort();
        fonts.dedup();

        let (prompt_text, prompt_error) = match config_ops::read_system_prompt(cx) {
            Ok(text) => (text, None),
            Err(err) => (String::new(), Some(err.into())),
        };
        let prompt = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(8, 24)
                .default_value(prompt_text)
        });
        let backup = cx.new(|cx| backup::BackupPanel::new(window, cx));
        let subscriptions = vec![
            cx.observe(&backup, |_, _, cx| cx.notify()),
            // The prompt is written once typing pauses, on blur, or when
            // the view closes: one edit, one agent reset.
            cx.subscribe(&prompt, |this, _, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    this.prompt_commit = Some(cx.spawn(async move |this, cx| {
                        cx.background_executor()
                            .timer(config_ops::COMMIT_DELAY)
                            .await;
                        // A failure shows above the pages.
                        let _ = this.update(cx, |this, cx| this.flush_prompt(cx));
                    }));
                }
                InputEvent::Blur => {
                    let _ = this.flush_prompt(cx);
                }
                _ => {}
            }),
            // Re-read every field after any change, including one made
            // elsewhere (the chat's provider and model pickers).
            cx.subscribe(&ConfigEvents::entity(cx), |_, _, _: &ConfigEvent, cx| {
                cx.notify()
            }),
            cx.subscribe(
                &ConfigEvents::entity(cx),
                |this, _, failed: &ConfigSaveFailed, cx| this.report(Err(failed.0.clone()), cx),
            ),
            cx.on_release(|this, cx| this.flush_on_release(cx)),
            cx.on_app_quit(|this, cx| {
                if let Err(err) = this.flush(cx) {
                    log::warn!("Could not save the settings on quit: {err}");
                }
                async {}
            }),
        ];
        Self {
            error: prompt_error,
            fonts,
            prompt,
            prompt_commit: None,
            backup,
            cave: Host::cave_path(cx),
            start_page: 0,
            focus_handle: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }

    /// The error shown above the pages, if any.
    #[cfg(test)]
    pub fn error(&self) -> Option<&SharedString> {
        self.error.as_ref()
    }

    /// Show page `index` (in the order of [`Self::pages`]).
    #[cfg(test)]
    pub fn show_page(&mut self, index: usize, cx: &mut Context<Self>) {
        self.start_page = index;
        cx.notify();
    }

    /// The system prompt editor.
    #[cfg(test)]
    pub fn prompt(&self) -> &Entity<TextareaState> {
        &self.prompt
    }

    /// The Backup page's state.
    #[cfg(test)]
    pub fn backup(&self) -> &Entity<backup::BackupPanel> {
        &self.backup
    }

    /// Commit what is still pending: field edits ([`config_ops::flush_config`])
    /// and the system prompt. Nothing is written once another cave is open.
    /// A failure is shown and returned (the first, when both fail); what
    /// could not be written stays pending for the next flush.
    pub fn flush(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        let config = config_ops::flush_config(cx).map(|_| ());
        let prompt = if self.prompt_commit.is_some() {
            self.flush_prompt(cx)
        } else {
            // A prompt typed earlier that could not be written.
            let result = config_ops::flush_system_prompt(cx);
            self.report(result.clone(), cx);
            result
        };
        if config.is_err() {
            self.report(config.clone(), cx);
        }
        config.and(prompt)
    }

    /// Write the system prompt if an edit is pending. A failure is shown
    /// and returned; the text stays pending in [`config_ops`], so it is
    /// written (or a cave switch refused) after the view is gone.
    fn flush_prompt(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        if self.prompt_commit.take().is_none() || Host::cave_path(cx) != self.cave {
            return Ok(());
        }
        let text = self.prompt.read(cx).value().to_string();
        let result = config_ops::write_system_prompt(&text, cx);
        self.report(result.clone(), cx);
        result
    }

    /// [`Self::flush`] for a view being dropped, which has only the app.
    /// What cannot be written stays pending in [`config_ops`].
    fn flush_on_release(&mut self, cx: &mut App) {
        if let Err(err) = config_ops::flush_config(cx) {
            log::warn!("Could not save the settings: {err}");
        }
        if self.prompt_commit.take().is_none() || Host::cave_path(cx) != self.cave {
            return;
        }
        let text = self.prompt.read(cx).value().to_string();
        if let Err(err) = config_ops::write_system_prompt(&text, cx) {
            log::warn!("Could not save the system prompt: {err}");
        }
    }

    /// Show a failed write; a successful one clears the error only once
    /// nothing waits to be saved (an earlier failure still pending stays
    /// shown).
    fn report(&mut self, result: Result<(), String>, cx: &mut Context<Self>) {
        match result {
            Err(err) => self.error = Some(err.into()),
            Ok(()) if !config_ops::has_unsaved(cx) => self.error = None,
            Ok(()) => {}
        }
        cx.notify();
    }

    /// The settings pages, in the order the sidebar lists them.
    fn pages(&self, cx: &mut Context<Self>) -> Vec<SettingPage> {
        let ctx = SettingsCtx {
            view: cx.entity().downgrade(),
        };
        vec![
            appearance::page(&ctx, &self.fonts),
            notes::page(&ctx, cx),
            agent::page(&ctx, &self.prompt, cx),
            agent::providers_page(&ctx, cx),
            backup::page(&ctx, &self.backup),
        ]
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let cave_name = Host::cave_path(cx)
            .and_then(|path| path.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "Current cave".to_string());
        let header = h_flex()
            .flex_none()
            .gap_3()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(theme.border)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(format!("{cave_name} settings")),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!("Saved in {cave_name}/.granit/config.yml")),
                    ),
            )
            .child(
                Button::new("close-settings")
                    .ghost()
                    .small()
                    .icon(IconName::Close)
                    .tooltip("Close settings")
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(SettingsEvent::Close))),
            );
        let error = self.error.clone().map(|error| {
            div()
                .flex_none()
                .px_4()
                .py_2()
                .text_sm()
                .text_color(theme.danger)
                .child(error)
        });
        let pages = self.pages(cx);
        v_flex()
            .id("settings-view")
            .key_context("SettingsView")
            .track_focus(&self.focus_handle)
            .size_full()
            .child(header)
            .children(error)
            .child(
                div().flex_1().min_h_0().child(
                    // A new id drops the component's state (selected page,
                    // search), so `start_page` applies again.
                    Settings::new(("granit-settings", self.start_page))
                        .default_selected_index(SelectIndex {
                            page_ix: self.start_page,
                            group_ix: None,
                        })
                        .with_group_variant(GroupBoxVariant::Outline)
                        .sidebar_width(gpui::px(200.))
                        .pages(pages),
                ),
            )
    }
}

impl Focusable for SettingsView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// What the page builders write through.
#[derive(Clone)]
pub struct SettingsCtx {
    view: WeakEntity<SettingsView>,
}

impl SettingsCtx {
    /// Change the config with `f`, save and apply it, and show the outcome.
    pub fn update(&self, cx: &mut App, f: impl FnOnce(&mut AppConfig)) {
        let result = config_ops::update_config(cx, f).map(|_| ());
        self.report(result, cx);
    }

    /// Show the outcome of a write.
    pub fn report(&self, result: Result<(), String>, cx: &mut App) {
        self.view
            .update(cx, |view, cx| view.report(result, cx))
            .ok();
    }

    /// A field setter that applies `f` with the new value.
    pub fn setter<T: 'static>(
        &self,
        f: impl Fn(&mut AppConfig, T) + 'static,
    ) -> impl Fn(T, &mut App) + 'static {
        let ctx = self.clone();
        move |value, cx| ctx.update(cx, |config| f(config, value))
    }

    /// A setter for text and number fields: the edit shows at once and is
    /// saved and applied once typing pauses ([`config_ops::edit_config`]),
    /// so a typed word resets the agent once, not per key.
    pub fn typed_setter<T: 'static>(
        &self,
        f: impl Fn(&mut AppConfig, T) + 'static,
    ) -> impl Fn(T, &mut App) + 'static {
        let view = self.view.clone();
        move |value, cx| {
            config_ops::edit_config(cx, |config| f(config, value));
            view.update(cx, |_, cx| cx.notify()).ok();
        }
    }
}

/// A field getter reading the config with `f`.
pub fn getter<T: 'static>(f: impl Fn(&AppConfig) -> T + 'static) -> impl Fn(&App) -> T + 'static {
    move |cx| f(&Host::core(cx).lock_config())
}
