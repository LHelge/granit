//! The templates panel: the note templates of `.granit/templates/` (their
//! own flat slug namespace), as the Tauri app's Templates tab lists them.
//! A click opens a template in the editor (the workspace's
//! [`crate::workspace::Workspace::open_template`]); the header button
//! creates one and opens it, and each row has rename (inline, like the
//! explorer) and delete buttons. Below the list, "Template parameters"
//! folds out the Tera variables a template can use.
//!
//! The list is the core's `Cave::list_templates`, an in-memory lookup redone
//! on every cave change. Template mutations go through [`crate::cave_ops`],
//! which announces them as `CaveEvent::Template*`.

use gpui::{
    div, prelude::FluentBuilder as _, AppContext as _, Context, Entity, EventEmitter,
    InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled, Subscription, Window,
};
use gpui_kit::component::{
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex, ActiveTheme, Icon, IconName, Sizable,
};
use granit_core::cave::{CaveError, DocumentMeta};
use log::warn;

use super::{empty_message, error_line, PanelEvent, ROW_HEIGHT};
use crate::assets::{note_icon_path, AppIcon};
use crate::cave_ops;
use crate::events::{CaveEvent, CaveEvents};

/// The Tera variables of every note template, with what they hold.
const PARAMETERS: [(&str, &str); 1] = [("{{ slug }}", "the new note's name")];

/// The variables added when the new note is a daily note (`YYYY-MM-DD`).
const DAILY_PARAMETERS: [&str; 8] = [
    "{{ date }}",
    "{{ tomorrow }}",
    "{{ yesterday }}",
    "{{ year }}",
    "{{ month }}",
    "{{ day }}",
    "{{ weekday }}",
    "{{ weekday_short }}",
];

/// The template being renamed inline, with its text input.
struct Rename {
    slug: String,
    input: Entity<InputState>,
    _events: Subscription,
}

pub struct TemplatesPanel {
    templates: Vec<DocumentMeta>,
    /// The template open in the editor.
    active: Option<String>,
    rename: Option<Rename>,
    /// Whether the template parameters are folded out.
    show_parameters: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PanelEvent> for TemplatesPanel {}

impl TemplatesPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.subscribe(&CaveEvents::entity(cx), |this, _, event, cx| {
            if *event == CaveEvent::Opened {
                this.rename = None;
                this.error = None;
            }
            this.refresh(cx);
        })];
        let mut this = Self {
            templates: Vec::new(),
            active: None,
            rename: None,
            show_parameters: false,
            error: None,
            _subscriptions: subscriptions,
        };
        this.refresh(cx);
        this
    }

    /// Re-read the templates.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        match cave_ops::list_templates(cx) {
            Ok(templates) => self.templates = templates,
            Err(CaveError::NoCaveOpen) => self.templates.clear(),
            Err(err) => {
                warn!("Could not list the templates: {err}");
                self.error = Some(format!("Could not list the templates: {err}").into());
            }
        }
        cx.notify();
    }

    /// The template open in the editor changed.
    pub fn set_active(&mut self, slug: Option<String>, cx: &mut Context<Self>) {
        self.active = slug;
        cx.notify();
    }

    /// The listed template slugs, for tests.
    #[cfg(test)]
    pub fn slugs(&self) -> Vec<String> {
        self.templates.iter().map(|t| t.slug.clone()).collect()
    }

    #[cfg(test)]
    pub fn error(&self) -> Option<&SharedString> {
        self.error.as_ref()
    }

    /// The template being renamed, and the text typed so far, for tests.
    #[cfg(test)]
    pub fn renaming(&self, cx: &gpui::App) -> Option<(String, SharedString)> {
        let rename = self.rename.as_ref()?;
        Some((rename.slug.clone(), rename.input.read(cx).value()))
    }

    /// Type `name` into the open rename input and press Enter, for tests.
    #[cfg(test)]
    pub fn type_and_commit(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(rename) = &self.rename {
            let name = name.to_string();
            rename
                .input
                .update(cx, |input, cx| input.set_value(name, window, cx));
        }
        self.commit_rename(cx);
    }

    /// Show the result of an operation in the error line.
    fn report<T>(&mut self, action: &str, result: Result<T, CaveError>) -> Option<T> {
        match result {
            Ok(value) => {
                self.error = None;
                Some(value)
            }
            Err(err) => {
                warn!("{action} failed: {err}");
                self.error = Some(format!("{action} failed: {err}").into());
                None
            }
        }
    }

    /// Ask the workspace to open the template `slug`.
    pub fn open(&mut self, slug: String, cx: &mut Context<Self>) {
        cx.emit(PanelEvent::OpenTemplate(slug));
    }

    /// Create a template, open it and start renaming it.
    pub fn new_template(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let result = cave_ops::create_template(cx);
        if let Some(meta) = self.report("Creating the template", result) {
            cx.emit(PanelEvent::OpenTemplate(meta.slug.clone()));
            self.start_rename(meta.slug, window, cx);
        }
        cx.notify();
    }

    /// Replace the row of `slug` with a text input holding its name. Enter
    /// or leaving the input renames it, Escape cancels.
    pub fn start_rename(&mut self, slug: String, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| InputState::new(window, cx).default_value(slug.clone()));
        let events = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                this.commit_rename(cx);
            }
        });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        self.rename = Some(Rename {
            slug,
            input,
            _events: events,
        });
        cx.notify();
    }

    /// Stop renaming without changing anything.
    pub fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.rename = None;
        cx.notify();
    }

    /// Rename the template to the typed name; an empty or unchanged name
    /// changes nothing.
    pub fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(rename) = self.rename.take() else {
            return;
        };
        let new_name = rename.input.read(cx).value().trim().to_string();
        cx.notify();
        if new_name.is_empty() || new_name == rename.slug {
            return;
        }
        let result = cave_ops::rename_template(&rename.slug, &new_name, cx);
        self.report("Renaming the template", result);
    }

    /// Delete the template `slug`.
    pub fn delete(&mut self, slug: &str, cx: &mut Context<Self>) {
        let result = cave_ops::delete_template(slug, cx);
        self.report("Deleting the template", result);
        cx.notify();
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        h_flex()
            .flex_none()
            .px_3()
            .py_1()
            .gap_1()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .truncate()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Note templates"),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_xs()
                            .text_color(theme.muted_foreground.opacity(0.7))
                            .child("Stored in .granit/templates"),
                    ),
            )
            .child(
                Button::new("new-template")
                    .ghost()
                    .xsmall()
                    .icon(AppIcon::FilePlus)
                    .tooltip("New note template")
                    .on_click(cx.listener(|this, _, window, cx| this.new_template(window, cx))),
            )
    }

    fn render_row(
        &self,
        ix: usize,
        meta: &DocumentMeta,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme();
        let slug = meta.slug.clone();
        let active = self.active.as_deref() == Some(slug.as_str());
        let rename_input = self
            .rename
            .as_ref()
            .filter(|rename| rename.slug == slug)
            .map(|rename| rename.input.clone());
        let icon = Icon::default()
            .path(note_icon_path(meta.icon.as_deref()))
            .small()
            .text_color(theme.primary);
        let row = h_flex()
            .id(("template", ix))
            .group("template-row")
            .h(ROW_HEIGHT)
            .px_2()
            .gap_1()
            .rounded_md()
            .text_sm()
            .when(active, |row| {
                row.bg(theme.accent).text_color(theme.accent_foreground)
            })
            .when(!active, |row| row.hover(|row| row.bg(theme.muted)))
            .child(icon);
        if let Some(input) = rename_input {
            return row
                .child(div().flex_1().min_w_0().child(Input::new(&input).xsmall()))
                .capture_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    if event.keystroke.key == "escape" {
                        this.cancel_rename(cx);
                    }
                }))
                .into_any_element();
        }
        let open_slug = slug.clone();
        let rename_slug = slug.clone();
        let delete_slug = slug.clone();
        row.cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.open(open_slug.clone(), cx)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(SharedString::from(slug)),
            )
            .child(
                h_flex()
                    .flex_none()
                    .invisible()
                    .group_hover("template-row", |buttons| buttons.visible())
                    .child(
                        Button::new(("rename-template", ix))
                            .ghost()
                            .xsmall()
                            .icon(AppIcon::Pencil)
                            .tooltip("Rename template")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.start_rename(rename_slug.clone(), window, cx)
                            })),
                    )
                    .child(
                        Button::new(("delete-template", ix))
                            .ghost()
                            .xsmall()
                            .icon(AppIcon::Trash)
                            .tooltip("Delete template")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.delete(&delete_slug, cx)
                            })),
                    ),
            )
            .into_any_element()
    }

    fn render_parameters(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let code = |text: &'static str| {
            div()
                .px_1()
                .rounded_sm()
                .bg(theme.muted)
                .font_family(theme.mono_font_family.clone())
                .text_xs()
                .child(text)
        };
        let body =
            v_flex()
                .gap_2()
                .px_3()
                .pb_2()
                .text_xs()
                .text_color(muted)
                .children(PARAMETERS.iter().map(|(name, what)| {
                    h_flex().gap_1().child(code(name)).child(div().child(*what))
                }))
                .child(div().child(
                    "Added when the new note's name is a date (YYYY-MM-DD), as for daily notes:",
                ))
                .child(
                    h_flex()
                        .flex_wrap()
                        .gap_1()
                        .children(DAILY_PARAMETERS.iter().map(|name| code(name))),
                );
        v_flex()
            .flex_none()
            .border_t_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .id("template-parameters")
                    .px_3()
                    .py_1()
                    .gap_1()
                    .cursor_pointer()
                    .text_sm()
                    .text_color(muted)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_parameters = !this.show_parameters;
                        cx.notify();
                    }))
                    .child(
                        Icon::new(if self.show_parameters {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .xsmall(),
                    )
                    .child("Template parameters"),
            )
            .when(self.show_parameters, |this| this.child(body))
    }
}

impl Render for TemplatesPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<gpui::AnyElement> = self
            .templates
            .clone()
            .iter()
            .enumerate()
            .map(|(ix, meta)| self.render_row(ix, meta, cx))
            .collect();
        let list = if rows.is_empty() {
            empty_message("No note templates yet", cx).into_any_element()
        } else {
            div()
                .id("templates-list")
                .size_full()
                .overflow_y_scroll()
                .px_1()
                .children(rows)
                .into_any_element()
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(self.render_header(cx))
            .child(div().flex_1().min_h_0().child(list))
            .child(self.render_parameters(cx))
            .children(error_line(self.error.clone(), cx))
    }
}
