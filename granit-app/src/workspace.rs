//! The root view of the main window: the sidebar with the explorer, and the
//! main area with the open note.
//!
//! The workspace owns the open note: it opens what the explorer asks for,
//! tells the core (for the agent's tools) and the explorer (for the
//! selection), and follows renames and deletions announced as
//! [`CaveEvent`]s. A new sidebar panel becomes a field here and a child in
//! [`Workspace::render_sidebar`]; the main area is [`NoteView`], the note
//! editor. Other panels open a note with [`Workspace::open_note`], or at a
//! heading, line or offset with [`Workspace::open_note_at`]. The agent chat
//! ([`ChatPanel`]) sits right of the main area while it is shown
//! ([`ToggleChat`]); its visibility is kept in the cave's config
//! (`agent_panel.visible`). The settings ([`SettingsView`],
//! [`ToggleSettings`]) replace the note in the main area while they are
//! open; opening a note or another cave closes them.

use std::path::PathBuf;

use gpui::{
    div, prelude::FluentBuilder as _, px, App, AppContext as _, Context, Entity, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, MouseButton, ParentElement, PathPromptOptions,
    Render, SharedString, Styled, Subscription, Window,
};
use gpui_kit::component::{
    button::{Button, ButtonVariants},
    h_flex, ActiveTheme, Disableable as _, IconName, Sizable,
};
use log::warn;

use crate::agent_ops;
use crate::assets::AppIcon;
use crate::cave_ops;
use crate::chat::{ChatEvent, ChatPanel};
use crate::config_ops::{ConfigEvent, ConfigEvents};
use crate::events::{CaveEvent, CaveEvents};
use crate::explorer::{Explorer, ExplorerEvent};
use crate::host::Host;
use crate::note_view::{NoteView, NoteViewEvent, Reveal};
use crate::settings::{SettingsEvent, SettingsView};
use crate::theme;

gpui_kit::actions!(granit, [OpenCave, ToggleChat, ToggleSettings]);

/// Which cave to open when the window opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartupCave {
    /// Open this cave.
    Open(PathBuf),
    /// Ask for a cave with the folder picker (first run).
    Pick,
    /// Start with no cave open.
    None,
}

/// The state of the cave as the window sees it.
enum CaveStatus {
    /// No cave is open (and none is being opened).
    Closed,
    /// A cave is being opened.
    Opening(PathBuf),
    /// The cave at this path is open.
    Open(PathBuf),
}

pub struct Workspace {
    status: CaveStatus,
    /// Why the last attempt to open a cave failed. Any cave that was open
    /// stays open.
    open_error: Option<SharedString>,
    explorer: Entity<Explorer>,
    note_view: Entity<NoteView>,
    chat: Entity<ChatPanel>,
    /// Whether the chat panel is shown.
    chat_visible: bool,
    /// The settings view, while it replaces the note in the main area, and
    /// its close subscription.
    settings: Option<(Entity<SettingsView>, Subscription)>,
    /// The open note.
    active: Option<String>,
    /// Keeps the workspace on the key-dispatch path while nothing inside it
    /// has focus, so its actions (`OpenCave`) work from a fresh window.
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    pub fn new(startup: StartupCave, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);
        let explorer = cx.new(Explorer::new);
        let note_view = cx.new(|cx| NoteView::new(window, cx));
        let chat = cx.new(|cx| ChatPanel::new(window, cx));
        let subscriptions = vec![
            cx.subscribe(&chat, |this, _, event: &ChatEvent, cx| match event {
                ChatEvent::OpenNote { slug, anchor } => {
                    let reveal = anchor.clone().map(Reveal::Anchor);
                    this.open_note_at(slug.clone(), reveal, cx)
                }
            }),
            cx.subscribe(
                &note_view,
                |this, _, event: &NoteViewEvent, cx| match event {
                    NoteViewEvent::OpenNote { slug, reveal } => {
                        this.open_note_at(slug.clone(), reveal.clone(), cx)
                    }
                },
            ),
            cx.observe_window_appearance(window, |_, window, cx| theme::sync(window, cx)),
            // The theme follows the open cave's setting.
            cx.subscribe_in(
                &ConfigEvents::entity(cx),
                window,
                |_, _, event: &ConfigEvent, window, cx| {
                    if event.theme {
                        theme::sync(window, cx);
                    }
                },
            ),
            cx.subscribe_in(
                &CaveEvents::entity(cx),
                window,
                |_, _, event: &CaveEvent, window, cx| {
                    if *event == CaveEvent::Opened {
                        theme::sync(window, cx);
                    }
                },
            ),
            cx.subscribe(
                &explorer,
                |this, _, event: &ExplorerEvent, cx| match event {
                    ExplorerEvent::OpenNote(slug) => {
                        this.close_settings(cx);
                        this.open_note(Some(slug.clone()), cx)
                    }
                },
            ),
            cx.subscribe(&CaveEvents::entity(cx), |this, _, event, cx| {
                this.on_cave_event(event, cx)
            }),
        ];
        let mut this = Self {
            status: CaveStatus::Closed,
            open_error: None,
            explorer,
            note_view,
            chat,
            chat_visible: false,
            settings: None,
            active: None,
            focus_handle,
            _subscriptions: subscriptions,
        };
        match startup {
            StartupCave::Open(path) => this.open_cave(path, cx),
            StartupCave::Pick => this.pick_cave(cx),
            StartupCave::None => {}
        }
        this
    }

    /// The open note.
    #[cfg(test)]
    pub fn active_note(&self) -> Option<&str> {
        self.active.as_deref()
    }

    /// Why the last attempt to open a cave failed, while that is shown.
    #[cfg(test)]
    pub fn failure(&self) -> Option<SharedString> {
        self.open_error.clone()
    }

    #[cfg(test)]
    pub fn explorer(&self) -> &Entity<Explorer> {
        &self.explorer
    }

    #[cfg(test)]
    pub fn note_view(&self) -> &Entity<NoteView> {
        &self.note_view
    }

    #[cfg(test)]
    pub fn chat(&self) -> &Entity<ChatPanel> {
        &self.chat
    }

    /// Whether the chat panel is shown.
    #[cfg(test)]
    pub fn chat_visible(&self) -> bool {
        self.chat_visible
    }

    /// Show or hide the chat panel, remembering it in the cave's config.
    pub fn toggle_chat(&mut self, cx: &mut Context<Self>) {
        self.set_chat_visible(!self.chat_visible, cx);
        if let Err(err) = agent_ops::set_chat_visible(self.chat_visible, cx) {
            warn!("Could not remember the chat panel: {err}");
        }
    }

    fn set_chat_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        self.chat_visible = visible;
        if visible {
            self.chat.update(cx, |chat, cx| chat.shown(cx));
        }
        cx.notify();
    }

    fn on_toggle_chat(&mut self, _: &ToggleChat, _: &mut Window, cx: &mut Context<Self>) {
        self.toggle_chat(cx);
    }

    /// The settings view, while it is shown.
    #[cfg(test)]
    pub fn settings(&self) -> Option<&Entity<SettingsView>> {
        self.settings.as_ref().map(|(view, _)| view)
    }

    /// Show the settings in the main area instead of the note, or close
    /// them. Settings need an open cave (they are its config).
    pub fn toggle_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.is_some() {
            self.close_settings(cx);
            window.focus(&self.focus_handle, cx);
            return;
        }
        if !matches!(self.status, CaveStatus::Open(_)) {
            return;
        }
        let view = cx.new(|cx| SettingsView::new(window, cx));
        let subscription =
            cx.subscribe_in(&view, window, |this, _, event, window, cx| match event {
                SettingsEvent::Close => {
                    this.close_settings(cx);
                    window.focus(&this.focus_handle, cx);
                }
            });
        window.focus(&view.focus_handle(cx), cx);
        self.settings = Some((view, subscription));
        cx.notify();
    }

    /// Show the note again in the main area.
    fn close_settings(&mut self, cx: &mut Context<Self>) {
        if let Some((view, _)) = self.settings.take() {
            // Edits still waiting for a typing pause are written now.
            view.update(cx, |view, cx| view.flush(cx));
            cx.notify();
        }
    }

    fn on_toggle_settings(
        &mut self,
        _: &ToggleSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_settings(window, cx);
    }

    /// Open the cave at `path`, replacing the open one when it succeeds.
    pub fn open_cave(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // Settings edits waiting for a typing pause belong to this cave.
        if let Some((settings, _)) = &self.settings {
            settings.update(cx, |settings, cx| settings.flush(cx));
        }
        // Pending edits go to the cave they were made in. If they cannot be
        // written, the cave is not switched (like a note switch).
        if let Err(err) = self
            .note_view
            .update(cx, |view, cx| view.save_before_leaving(cx))
        {
            warn!("Not opening {}: {err}", path.display());
            self.open_error = Some(format!("Could not open {}: {err}", path.display()).into());
            cx.notify();
            return;
        }
        self.status = CaveStatus::Opening(path.clone());
        cx.notify();
        // Edits made while the cave is prepared are written right before the
        // core swaps caves.
        let note_view = self.note_view.downgrade();
        let before_install = move |cx: &mut App| {
            note_view
                .update(cx, |view, cx| view.save_before_leaving(cx))
                .unwrap_or(Ok(()))
        };
        let task = cave_ops::open_cave(path.clone(), before_install, cx);
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                // On success `CaveEvent::Opened` has already reset the note
                // and the explorer.
                this.open_error = result.err().map(|err| {
                    warn!("Could not open the cave at {}: {err}", path.display());
                    format!("Could not open {}: {err}", path.display()).into()
                });
                this.status = Host::cave_path(cx).map_or(CaveStatus::Closed, CaveStatus::Open);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Ask for a cave folder and open it.
    pub fn pick_cave(&mut self, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open cave".into()),
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
                this.update(cx, |this, cx| this.open_cave(path, cx)).ok();
            }
        })
        .detach();
    }

    fn on_open_cave(&mut self, _: &OpenCave, _: &mut Window, cx: &mut Context<Self>) {
        self.pick_cave(cx);
    }

    /// Show `slug` in the main area (`None`: nothing) and make it the
    /// active note. The open note's pending edits are written first; if
    /// that fails, it stays open (and active).
    pub fn open_note(&mut self, slug: Option<String>, cx: &mut Context<Self>) {
        let shown = self
            .note_view
            .update(cx, |view, cx| view.open(slug.as_deref(), cx));
        cave_ops::set_active_note(shown.clone(), cx);
        self.explorer
            .update(cx, |explorer, cx| explorer.set_active(shown.clone(), cx));
        self.active = shown;
        cx.notify();
    }

    /// Open `slug` and put the caret at `reveal` in it (a heading anchor, a
    /// line or an offset), scrolled to the top: for following links and
    /// opening search or todo matches.
    pub fn open_note_at(&mut self, slug: String, reveal: Option<Reveal>, cx: &mut Context<Self>) {
        self.close_settings(cx);
        self.open_note(Some(slug.clone()), cx);
        if let Some(reveal) = reveal {
            if self.active.as_deref() == Some(slug.as_str()) {
                self.note_view
                    .update(cx, |view, cx| view.reveal(reveal, cx));
            }
        }
    }

    /// Close the open note without writing it (it no longer exists).
    fn close_removed_note(&mut self, cx: &mut Context<Self>) {
        self.note_view.update(cx, |view, cx| view.forget(cx));
        self.open_note(None, cx);
    }

    fn on_cave_event(&mut self, event: &CaveEvent, cx: &mut Context<Self>) {
        match event {
            CaveEvent::Opened => {
                self.status = Host::cave_path(cx).map_or(CaveStatus::Closed, CaveStatus::Open);
                // The settings were the previous cave's.
                self.close_settings(cx);
                // The previous cave's note was written before the switch.
                self.close_removed_note(cx);
                let visible = Host::core(cx).lock_config().agent_panel.visible;
                self.set_chat_visible(visible, cx);
            }
            CaveEvent::NoteRenamed { from, to } if self.active.as_deref() == Some(from) => {
                // Pending edits follow the note to its new name.
                self.note_view
                    .update(cx, |view, cx| view.note_renamed(from, to, cx));
                self.open_note(Some(to.clone()), cx);
                self.note_view.update(cx, |view, cx| view.reload(cx));
            }
            CaveEvent::NotesRemoved(slugs)
                if self
                    .active
                    .as_ref()
                    .is_some_and(|active| slugs.contains(active)) =>
            {
                self.close_removed_note(cx);
            }
            _ => {
                // The active note may have been removed or changed by
                // someone else (an agent tool).
                let exists = self
                    .active
                    .as_deref()
                    .is_some_and(|slug| cave_ops::read_note(slug, cx).is_ok());
                if self.active.is_some() && !exists {
                    self.close_removed_note(cx);
                } else {
                    self.note_view.update(cx, |view, cx| view.reload(cx));
                }
            }
        }
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let cave_name = match &self.status {
            CaveStatus::Open(path) | CaveStatus::Opening(path) => path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string()),
            CaveStatus::Closed => "No cave open".to_string(),
        };
        // With no cave open the main area shows the error instead.
        let open_error = match self.status {
            CaveStatus::Open(_) => self.open_error.clone(),
            CaveStatus::Closed | CaveStatus::Opening(_) => None,
        };
        div()
            .w(px(260.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(theme.sidebar)
            .text_color(theme.sidebar_foreground)
            .border_r_1()
            .border_color(theme.sidebar_border)
            .child(
                h_flex()
                    .flex_none()
                    .px_3()
                    .py_2()
                    .gap_1()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(cave_name),
                    )
                    .child(
                        Button::new("open-cave")
                            .ghost()
                            .xsmall()
                            .icon(IconName::FolderOpen)
                            .tooltip("Open cave…")
                            .on_click(cx.listener(|this, _, _, cx| this.pick_cave(cx))),
                    )
                    .child(
                        Button::new("toggle-chat")
                            .ghost()
                            .xsmall()
                            .icon(AppIcon::PanelRight)
                            .tooltip("Toggle agent chat")
                            .toggled(self.chat_visible)
                            .disabled(!matches!(self.status, CaveStatus::Open(_)))
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_chat(cx))),
                    )
                    .child(
                        Button::new("toggle-settings")
                            .ghost()
                            .xsmall()
                            .icon(AppIcon::Settings)
                            .tooltip("Settings")
                            .toggled(self.settings.is_some())
                            .disabled(!matches!(self.status, CaveStatus::Open(_)))
                            .on_click(
                                cx.listener(|this, _, window, cx| this.toggle_settings(window, cx)),
                            ),
                    ),
            )
            .children(open_error.map(|error| {
                div()
                    .flex_none()
                    .px_3()
                    .pb_2()
                    .text_sm()
                    .text_color(theme.danger)
                    .child(error)
            }))
            .child(div().flex_1().min_h_0().child(self.explorer.clone()))
    }

    fn render_main(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme();
        let message = |text: SharedString, color| {
            div()
                .flex_1()
                .h_full()
                .flex()
                .flex_col()
                .gap_3()
                .items_center()
                .justify_center()
                .p_6()
                .child(div().text_color(color).child(text))
        };
        let open_button = || {
            Button::new("open-cave-main")
                .primary()
                .label("Open cave…")
                .on_click(cx.listener(|this, _, _, cx| this.pick_cave(cx)))
        };
        match &self.status {
            CaveStatus::Open(_) => match &self.settings {
                Some((settings, _)) => div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(settings.clone())
                    .into_any_element(),
                None => self.note_view.clone().into_any_element(),
            },
            CaveStatus::Opening(path) => message(
                format!("Opening {}…", path.display()).into(),
                theme.muted_foreground,
            )
            .into_any_element(),
            CaveStatus::Closed => match &self.open_error {
                Some(err) => message(err.clone(), theme.danger),
                None => message(
                    "Open a folder of markdown notes to start.".into(),
                    theme.muted_foreground,
                ),
            }
            .child(open_button())
            .into_any_element(),
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .size_full()
            .flex()
            .flex_row()
            .bg(theme.background)
            .text_color(theme.foreground)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_open_cave))
            .on_action(cx.listener(Self::on_toggle_chat))
            .on_action(cx.listener(Self::on_toggle_settings))
            // A click that focused nothing inside (empty space, or after a
            // focused input went away) takes focus back, so the
            // workspace's keys keep working.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    if window.focused(cx).is_none() {
                        window.focus(&this.focus_handle, cx);
                    }
                }),
            )
            .child(self.render_sidebar(cx))
            .child(self.render_main(cx))
            .when(
                self.chat_visible && matches!(self.status, CaveStatus::Open(_)),
                |this| this.child(self.chat.clone()),
            )
    }
}

impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Bind the workspace's keys. Call once at startup.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        gpui::KeyBinding::new(OPEN_CAVE_KEY, OpenCave, None),
        gpui::KeyBinding::new(TOGGLE_CHAT_KEY, ToggleChat, None),
        gpui::KeyBinding::new(TOGGLE_SETTINGS_KEY, ToggleSettings, None),
    ]);
    crate::note_view::bind_keys(cx);
}

// `KeyBinding::new` rejects GPUI's `secondary-` prefix, so the platform's
// command modifier is spelled out: `cmd-` on macOS, `ctrl-` elsewhere.

/// The key that opens a cave.
#[cfg(target_os = "macos")]
pub const OPEN_CAVE_KEY: &str = "cmd-o";
#[cfg(not(target_os = "macos"))]
pub const OPEN_CAVE_KEY: &str = "ctrl-o";

/// The key that shows or hides the agent chat.
#[cfg(target_os = "macos")]
pub const TOGGLE_CHAT_KEY: &str = "cmd-shift-a";
#[cfg(not(target_os = "macos"))]
pub const TOGGLE_CHAT_KEY: &str = "ctrl-shift-a";

/// The key that shows or hides the settings.
#[cfg(target_os = "macos")]
pub const TOGGLE_SETTINGS_KEY: &str = "cmd-,";
#[cfg(not(target_os = "macos"))]
pub const TOGGLE_SETTINGS_KEY: &str = "ctrl-,";

/// The key that quits the app.
#[cfg(target_os = "macos")]
pub const QUIT_KEY: &str = "cmd-q";
#[cfg(not(target_os = "macos"))]
pub const QUIT_KEY: &str = "ctrl-q";

/// Open the main window with the Catppuccin theme installed and a
/// [`Workspace`] as its content (inside GPUI Kit's root, which hosts menus
/// and popovers). Call after [`Host::init`].
pub fn open_main_window(
    startup: StartupCave,
    cx: &mut App,
) -> gpui::Result<(gpui::AnyWindowHandle, Entity<Workspace>)> {
    let options = gpui::WindowOptions {
        window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds::centered(
            None,
            gpui::size(px(1100.), px(720.)),
            cx,
        ))),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some(crate::host::PRODUCT_NAME.into()),
            ..Default::default()
        }),
        app_id: Some(crate::host::APP_ID.into()),
        ..Default::default()
    };
    gpui_kit::open_window(options, cx, |window, cx| {
        theme::install(window, cx);
        cx.new(|cx| Workspace::new(startup, window, cx))
    })
}
