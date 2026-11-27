//! The agent files panel: the skills (`.granit/agent/skills/<name>/SKILL.md`)
//! and tasks (`.granit/agent/tasks/<name>.md`) of the cave, as the Tauri
//! app's Agent tab lists them, each section with its own name namespace.
//! A click opens a skill or task in the editor (the workspace's
//! [`crate::workspace::Workspace::open_agent_doc`]), where the header edits
//! its description; each section's header button creates one and opens it,
//! and each row has rename (inline) and delete buttons. Below the lists,
//! "Agent files help" folds out what skills and tasks are and the Tera
//! variables a task can use. (The system prompt is edited in the settings'
//! Agent page.)
//!
//! The lists are the core's `Cave::list_skills` / `Cave::list_tasks`,
//! re-read on every cave change. Mutations go through [`crate::cave_ops`],
//! which announces them as `CaveEvent::AgentDoc*`.

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
use granit_core::cave::CaveError;
use granit_types::AgentDocInfo;
use log::warn;

use super::{error_line, PanelEvent, ROW_HEIGHT};
use crate::assets::AppIcon;
use crate::cave_ops;
use crate::events::{AgentDocKind, CaveEvent, CaveEvents};

/// The Tera variables a task's body can use, with what they hold.
const TASK_PARAMETERS: [(&str, &str); 10] = [
    ("{{ input }}", "the text typed after /name"),
    (
        "{{ active_note }}",
        "the open note (only while one is open)",
    ),
    ("{{ today }}", "YYYY-MM-DD"),
    ("{{ tomorrow }}", "YYYY-MM-DD"),
    ("{{ yesterday }}", "YYYY-MM-DD"),
    ("{{ year }}", "a number"),
    ("{{ month }}", "a number"),
    ("{{ day }}", "a number"),
    ("{{ weekday }}", "Monday, …"),
    ("{{ weekday_short }}", "Mon, …"),
];

const SKILLS_HELP: &str = "Each skill is a folder with a SKILL.md in the Agent Skills format. \
     Its name (lowercase letters, digits and hyphens) is the folder's; its description is \
     edited in the header. The agent sees every name and description and loads the full \
     instructions on demand with the use_skill tool. Only SKILL.md is used.";

const TASKS_HELP: &str = "Prompts sent from the chat by typing / and the task's name, with \
     the text after it as input; the description is edited in the header. The body is a Tera \
     template rendered when the task is sent. Available variables:";

const SYSTEM_PROMPT_HELP: &str =
    "The system prompt (.granit/agent/system.md) is edited in Settings, on the Agent page.";

/// What tells the two sections apart on screen.
struct Section {
    label: &'static str,
    subtitle: &'static str,
    empty: &'static str,
    icon: AppIcon,
    /// Element ids, unique per section.
    new_id: &'static str,
    row_id: &'static str,
    rename_id: &'static str,
    delete_id: &'static str,
    group: &'static str,
}

fn section(kind: AgentDocKind) -> Section {
    match kind {
        AgentDocKind::Skill => Section {
            label: "Skills",
            subtitle: "Stored in .granit/agent/skills",
            empty: "No skills yet",
            icon: AppIcon::Brain,
            new_id: "new-skill",
            row_id: "skill",
            rename_id: "rename-skill",
            delete_id: "delete-skill",
            group: "skill-row",
        },
        AgentDocKind::Task => Section {
            label: "Tasks",
            subtitle: "Stored in .granit/agent/tasks",
            empty: "No tasks yet",
            icon: AppIcon::Zap,
            new_id: "new-task",
            row_id: "task",
            rename_id: "rename-task",
            delete_id: "delete-task",
            group: "task-row",
        },
    }
}

/// The skill or task being renamed inline, with its text input.
struct Rename {
    kind: AgentDocKind,
    name: String,
    input: Entity<InputState>,
    _events: Subscription,
}

pub struct AgentFilesPanel {
    skills: Vec<AgentDocInfo>,
    tasks: Vec<AgentDocInfo>,
    /// The skill or task open in the editor.
    active: Option<(AgentDocKind, String)>,
    rename: Option<Rename>,
    /// Whether the help is folded out.
    show_help: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PanelEvent> for AgentFilesPanel {}

impl AgentFilesPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.subscribe(&CaveEvents::entity(cx), |this, _, event, cx| {
            if *event == CaveEvent::Opened {
                this.rename = None;
                this.error = None;
            }
            this.refresh(cx);
        })];
        let mut this = Self {
            skills: Vec::new(),
            tasks: Vec::new(),
            active: None,
            rename: None,
            show_help: false,
            error: None,
            _subscriptions: subscriptions,
        };
        this.refresh(cx);
        this
    }

    /// Re-read the skills and tasks.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        for kind in [AgentDocKind::Skill, AgentDocKind::Task] {
            let docs = match cave_ops::list_agent_docs(kind, cx) {
                Ok(docs) => docs,
                Err(CaveError::NoCaveOpen) => Vec::new(),
                Err(err) => {
                    warn!("Could not list the {}s: {err}", kind.noun());
                    self.error = Some(format!("Could not list the {}s: {err}", kind.noun()).into());
                    continue;
                }
            };
            *self.docs_mut(kind) = docs;
        }
        cx.notify();
    }

    fn docs(&self, kind: AgentDocKind) -> &[AgentDocInfo] {
        match kind {
            AgentDocKind::Skill => &self.skills,
            AgentDocKind::Task => &self.tasks,
        }
    }

    fn docs_mut(&mut self, kind: AgentDocKind) -> &mut Vec<AgentDocInfo> {
        match kind {
            AgentDocKind::Skill => &mut self.skills,
            AgentDocKind::Task => &mut self.tasks,
        }
    }

    /// The skill or task open in the editor changed.
    pub fn set_active(&mut self, active: Option<(AgentDocKind, String)>, cx: &mut Context<Self>) {
        self.active = active;
        cx.notify();
    }

    /// The listed names of `kind`, for tests.
    #[cfg(test)]
    pub fn names(&self, kind: AgentDocKind) -> Vec<String> {
        self.docs(kind).iter().map(|doc| doc.name.clone()).collect()
    }

    /// The listed descriptions of `kind`, for tests.
    #[cfg(test)]
    pub fn descriptions(&self, kind: AgentDocKind) -> Vec<String> {
        self.docs(kind)
            .iter()
            .map(|doc| doc.description.clone())
            .collect()
    }

    #[cfg(test)]
    pub fn error(&self) -> Option<&SharedString> {
        self.error.as_ref()
    }

    /// What is being renamed, and the text typed so far, for tests.
    #[cfg(test)]
    pub fn renaming(&self, cx: &gpui::App) -> Option<(AgentDocKind, String, SharedString)> {
        let rename = self.rename.as_ref()?;
        Some((
            rename.kind,
            rename.name.clone(),
            rename.input.read(cx).value(),
        ))
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

    /// Fold the help out or in.
    pub fn toggle_help(&mut self, cx: &mut Context<Self>) {
        self.show_help = !self.show_help;
        cx.notify();
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

    /// Ask the workspace to open the skill or task `name`.
    pub fn open(&mut self, kind: AgentDocKind, name: String, cx: &mut Context<Self>) {
        cx.emit(PanelEvent::OpenAgentDoc { kind, name });
    }

    /// Create a skill or task, open it and start renaming it.
    pub fn new_doc(&mut self, kind: AgentDocKind, window: &mut Window, cx: &mut Context<Self>) {
        let result = cave_ops::create_agent_doc(kind, cx);
        if let Some(meta) = self.report(&format!("Creating the {}", kind.noun()), result) {
            cx.emit(PanelEvent::OpenAgentDoc {
                kind,
                name: meta.slug.clone(),
            });
            self.start_rename(kind, meta.slug, window, cx);
        }
        cx.notify();
    }

    /// Replace the row of `name` with a text input holding it. Enter or
    /// leaving the input renames it, Escape cancels.
    pub fn start_rename(
        &mut self,
        kind: AgentDocKind,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name.clone()));
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
            kind,
            name,
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

    /// Rename to the typed name; an empty or unchanged name changes
    /// nothing. (Pending edits of the open document follow it.)
    pub fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(rename) = self.rename.take() else {
            return;
        };
        let new_name = rename.input.read(cx).value().trim().to_string();
        cx.notify();
        if new_name.is_empty() || new_name == rename.name {
            return;
        }
        let result = cave_ops::rename_agent_doc(rename.kind, &rename.name, &new_name, cx);
        self.report(&format!("Renaming the {}", rename.kind.noun()), result);
    }

    /// Delete the skill (with its folder) or task `name`.
    pub fn delete(&mut self, kind: AgentDocKind, name: &str, cx: &mut Context<Self>) {
        let result = cave_ops::delete_agent_doc(kind, name, cx);
        self.report(&format!("Deleting the {}", kind.noun()), result);
        cx.notify();
    }

    fn render_section_header(
        &self,
        kind: AgentDocKind,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let section = section(kind);
        h_flex()
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
                            .child(section.label),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_xs()
                            .text_color(theme.muted_foreground.opacity(0.7))
                            .child(section.subtitle),
                    ),
            )
            .child(
                Button::new(section.new_id)
                    .ghost()
                    .xsmall()
                    .icon(AppIcon::FilePlus)
                    .tooltip(format!("New {}", kind.noun()))
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.new_doc(kind, window, cx)),
                    ),
            )
    }

    fn render_row(
        &self,
        kind: AgentDocKind,
        ix: usize,
        doc: &AgentDocInfo,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme();
        let section = section(kind);
        let name = doc.name.clone();
        let active = self
            .active
            .as_ref()
            .is_some_and(|(active_kind, active)| *active_kind == kind && *active == name);
        let rename_input = self
            .rename
            .as_ref()
            .filter(|rename| rename.kind == kind && rename.name == name)
            .map(|rename| rename.input.clone());
        let icon = Icon::new(section.icon).small().text_color(theme.primary);
        let row = h_flex()
            .id((section.row_id, ix))
            .group(section.group)
            .min_h(ROW_HEIGHT)
            .px_2()
            .py_1()
            .gap_2()
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
        let description = (!doc.description.is_empty()).then(|| {
            div()
                .truncate()
                .text_xs()
                .text_color(if active {
                    theme.accent_foreground.opacity(0.7)
                } else {
                    theme.muted_foreground
                })
                .child(SharedString::from(doc.description.clone()))
        });
        let open_name = name.clone();
        let rename_name = name.clone();
        let delete_name = name.clone();
        row.cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.open(kind, open_name.clone(), cx)))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(div().truncate().child(SharedString::from(name)))
                    .children(description),
            )
            .child(
                h_flex()
                    .flex_none()
                    .invisible()
                    .group_hover(section.group, |buttons| buttons.visible())
                    .child(
                        Button::new((section.rename_id, ix))
                            .ghost()
                            .xsmall()
                            .icon(AppIcon::Pencil)
                            .tooltip(format!("Rename {}", kind.noun()))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.start_rename(kind, rename_name.clone(), window, cx)
                            })),
                    )
                    .child(
                        Button::new((section.delete_id, ix))
                            .ghost()
                            .xsmall()
                            .icon(AppIcon::Trash)
                            .tooltip(format!("Delete {}", kind.noun()))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.delete(kind, &delete_name, cx)
                            })),
                    ),
            )
            .into_any_element()
    }

    fn render_section(&self, kind: AgentDocKind, cx: &mut Context<Self>) -> impl IntoElement {
        let docs = self.docs(kind).to_vec();
        let rows: Vec<gpui::AnyElement> = docs
            .iter()
            .enumerate()
            .map(|(ix, doc)| self.render_row(kind, ix, doc, cx))
            .collect();
        let empty = rows.is_empty().then(|| {
            div()
                .px_3()
                .py_1()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(section(kind).empty)
        });
        v_flex()
            .pb_2()
            .child(self.render_section_header(kind, cx))
            .children(empty)
            .child(v_flex().px_1().children(rows))
    }

    fn render_help(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
        let heading = |text: &'static str| div().text_color(theme.foreground).child(text);
        let body =
            v_flex()
                .gap_2()
                .px_3()
                .pb_2()
                .text_xs()
                .text_color(muted)
                .child(heading("Skills"))
                .child(div().child(SKILLS_HELP))
                .child(heading("Tasks"))
                .child(div().child(TASKS_HELP))
                .children(TASK_PARAMETERS.iter().map(|(name, what)| {
                    h_flex().gap_1().child(code(name)).child(div().child(*what))
                }))
                .child(heading("System prompt"))
                .child(div().child(SYSTEM_PROMPT_HELP));
        v_flex()
            .flex_none()
            .border_t_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .id("agent-files-help")
                    .px_3()
                    .py_1()
                    .gap_1()
                    .cursor_pointer()
                    .text_sm()
                    .text_color(muted)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_help(cx)))
                    .child(
                        Icon::new(if self.show_help {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .xsmall(),
                    )
                    .child("Agent files help"),
            )
            .when(self.show_help, |this| {
                this.child(
                    div()
                        .id("agent-files-help-body")
                        .max_h(gpui::px(320.))
                        .overflow_y_scroll()
                        .child(body),
                )
            })
    }
}

impl Render for AgentFilesPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let skills = self.render_section(AgentDocKind::Skill, cx);
        let tasks = self.render_section(AgentDocKind::Task, cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("agent-files-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(skills)
                    .child(tasks),
            )
            .child(self.render_help(cx))
            .children(error_line(self.error.clone(), cx))
    }
}
