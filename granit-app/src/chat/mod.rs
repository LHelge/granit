//! The agent chat panel, beside the main area.
//!
//! The conversation is one Markdown document ([`transcript::Transcript`])
//! rendered by a single GPUI Kit `TextView`: replies stream into it and are
//! formatted while they arrive, the view follows the bottom, and a
//! selection can run across messages. `[[wiki-links]]` are rewritten to
//! `wiki:` links on the stream ([`wiki_links`]) and resolved against the
//! cave ([`links`]); clicking one emits [`ChatEvent::OpenNote`], which the
//! workspace handles. Broken links are drawn as plain text, task-list
//! checkboxes are not clickable, and raw HTML shows as text.
//!
//! A message goes to the core's agent (`Agent::stream_with_history`) on the
//! tokio runtime; every stream item is brought to the panel with an entity
//! update. Tool results announce `CaveEvent::NotesChanged`, so the explorer
//! and the open note follow what agent tools change. Changing the provider,
//! model or mode saves the cave's config, resets the agent and cancels a
//! running reply, as the Tauri app does; "New conversation" clears the
//! agent's history. History lives in the agent, per session.

mod links;
mod tasks;
pub mod transcript;
pub mod wiki_links;

use std::future::Future;

use gpui::{
    div, prelude::FluentBuilder as _, px, AppContext as _, Context, Entity, EventEmitter,
    FollowMode, InteractiveElement as _, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled, Subscription, Task, Window,
};
use gpui_kit::component::{
    button::{Button, ButtonVariants},
    h_flex,
    input::{InputEvent, MoveDown, MoveUp, Textarea, TextareaState},
    menu::{DropdownMenu, PopupMenuItem},
    resizable::HandleEdge,
    text::{MarkdownExtensions, TextView, TextViewState},
    v_flex, ActiveTheme, Disableable, Icon, IconName, Sizable,
};
use granit_core::agent::{build_agent_prompt, AgentError, AgentStream, AgentStreamItem};
use granit_types::{
    AgentDocInfo, AgentMode, AttachedNote, ModelInfo, ProviderConfig, ToolCallInfo,
};
use log::warn;

use crate::agent_ops;
use crate::assets::{note_icon_path, AppIcon};
use crate::cave_ops;
use crate::config_ops::{self, ConfigEvent, ConfigEvents};
use crate::events::{CaveEvent, CaveEvents};
use crate::host::Host;
use crate::resize::PanelResize;
use crate::runtime;
use links::{classify, LinkTarget, LinkTargets};
use tasks::{matching_tasks, parse_task_invocation};
use transcript::{Transcript, ERROR_SURFACE, TOOL_SURFACE, USER_SURFACE};

/// What the chat asks of the workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatEvent {
    /// A wiki-link was clicked: open the note `slug`, at the heading
    /// `anchor` when there is one.
    OpenNote {
        slug: String,
        anchor: Option<String>,
    },
    /// The user asked to attach the open note, with the editor's selected
    /// text: the workspace answers with [`ChatPanel::attach`].
    AttachActiveNote,
}

/// The narrowest and widest the panel gets, whatever the config says.
const MIN_WIDTH: f32 = 240.;
const MAX_WIDTH: f32 = 640.;

/// The note open in the main area, offered as an attachment.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveNote {
    slug: String,
    /// Its frontmatter icon id.
    icon: Option<String>,
}

pub struct ChatPanel {
    /// The rendered conversation.
    view: Entity<TextViewState>,
    transcript: Transcript,
    links: LinkTargets,
    input: Entity<TextareaState>,
    /// The cave's `/task` prompts.
    tasks: Vec<AgentDocInfo>,
    /// The highlighted entry of the task list shown while typing `/`.
    task_highlight: usize,
    /// Escape hid the task list until the input changes.
    tasks_dismissed: bool,
    /// The selected provider's models.
    models: Vec<ModelInfo>,
    models_loading: bool,
    models_error: Option<SharedString>,
    models_task: Option<Task<()>>,
    /// The provider `models` are (being) listed for.
    models_for: Option<ProviderConfig>,
    /// The running reply; dropping it cancels the stream.
    stream: Option<Task<()>>,
    /// The note open in the main area.
    active_note: Option<ActiveNote>,
    /// Notes attached to every message sent, as they were when attached.
    attached: Vec<AttachedNote>,
    /// The panel's width follows a drag of its left edge.
    resize: PanelResize,
    /// The prompt last handed to the agent, for tests.
    #[cfg(test)]
    last_prompt: Option<String>,
    /// The font the panel was last drawn with, for tests.
    #[cfg(test)]
    drawn_font: Option<(SharedString, gpui::Pixels)>,
    /// Records every link URL the view parses, for tests.
    #[cfg(test)]
    link_probe: std::sync::Arc<std::sync::Mutex<std::collections::BTreeSet<String>>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ChatEvent> for ChatPanel {}

impl ChatPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let view = cx.new(|cx| TextViewState::markdown("", cx));
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(3, 10)
                .submit_on_enter(true)
                .placeholder("Message… (/ for tasks)")
        });
        let subscriptions =
            vec![
                cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
                    match event {
                        InputEvent::PressEnter { shift: false, .. } => this.on_enter(window, cx),
                        InputEvent::Change => {
                            this.tasks_dismissed = false;
                            this.task_highlight = 0;
                            cx.notify();
                        }
                        _ => {}
                    }
                }),
                cx.subscribe(&CaveEvents::entity(cx), |this, _, event, cx| {
                    this.on_cave_event(event, cx)
                }),
                cx.subscribe(&ConfigEvents::entity(cx), |this, _, event, cx| {
                    this.on_config_event(*event, cx)
                }),
            ];
        let mut this = Self {
            view,
            transcript: Transcript::default(),
            links: LinkTargets::default(),
            input,
            tasks: Vec::new(),
            task_highlight: 0,
            tasks_dismissed: false,
            models: Vec::new(),
            models_loading: false,
            models_error: None,
            models_task: None,
            models_for: None,
            stream: None,
            active_note: None,
            attached: Vec::new(),
            resize: PanelResize::new("chat-panel-edge", HandleEdge::Leading, MIN_WIDTH, MAX_WIDTH),
            #[cfg(test)]
            last_prompt: None,
            #[cfg(test)]
            drawn_font: None,
            #[cfg(test)]
            link_probe: Default::default(),
            _subscriptions: subscriptions,
        };
        this.follow_tail(cx);
        this.tasks = agent_ops::list_tasks(cx);
        this
    }

    /// Whether a reply is streaming.
    pub fn is_streaming(&self) -> bool {
        self.stream.is_some()
    }

    /// The conversation's Markdown source, for tests.
    #[cfg(test)]
    pub fn transcript(&self) -> &str {
        self.transcript.source()
    }

    /// The rendered text of the conversation, for tests.
    #[cfg(test)]
    pub fn rendered_text(&self, cx: &gpui::App) -> String {
        self.view.read(cx).rendered_text().as_str().to_string()
    }

    /// Hand the running reply to a test to await.
    #[cfg(test)]
    pub fn take_stream(&mut self) -> Option<Task<()>> {
        self.stream.take()
    }

    /// The font family and size the panel was last drawn with.
    #[cfg(test)]
    pub fn drawn_font(&self) -> Option<(SharedString, gpui::Pixels)> {
        self.drawn_font.clone()
    }

    /// Every link URL the view has parsed, for tests.
    #[cfg(test)]
    pub fn parsed_links(&self) -> Vec<String> {
        self.link_probe.lock().unwrap().iter().cloned().collect()
    }

    /// The links' targets, for tests.
    #[cfg(test)]
    pub fn link_targets(&self) -> &LinkTargets {
        &self.links
    }

    /// The prompt last handed to the agent, for tests.
    #[cfg(test)]
    pub fn last_prompt(&self) -> Option<&str> {
        self.last_prompt.as_deref()
    }

    /// The panel's resizing, for tests.
    #[cfg(test)]
    pub fn resize(&self) -> &PanelResize {
        &self.resize
    }

    // ── Attached notes ───────────────────────────────────────────────

    /// The notes attached to the messages sent.
    #[cfg(test)]
    pub fn attached(&self) -> &[AttachedNote] {
        &self.attached
    }

    /// The note `slug` is open in the main area (`None`: no note): it is
    /// offered for attaching.
    pub fn set_active_note(&mut self, slug: Option<String>, cx: &mut Context<Self>) {
        self.active_note = slug.map(|slug| {
            let icon = cave_ops::read_note(&slug, cx)
                .ok()
                .and_then(|note| note.meta.icon);
            ActiveNote { slug, icon }
        });
        cx.notify();
    }

    /// Attach `note` to the messages sent from now on, as it is now. A note
    /// attached already stays as it was attached.
    pub fn attach(&mut self, note: AttachedNote, cx: &mut Context<Self>) {
        if !self.is_attached(&note.slug) {
            self.attached.push(note);
            cx.notify();
        }
    }

    /// Stop attaching the note `slug`.
    pub fn detach(&mut self, slug: &str, cx: &mut Context<Self>) {
        self.attached.retain(|note| note.slug != slug);
        cx.notify();
    }

    fn is_attached(&self, slug: &str) -> bool {
        self.attached.iter().any(|note| note.slug == slug)
    }

    /// Save the width the panel's edge was dragged to.
    fn resized(&mut self, width: f32, cx: &mut Context<Self>) {
        let width = width.round() as u16;
        if let Err(err) = config_ops::update_config(cx, |config| config.agent_panel.width = width) {
            warn!("Could not save the chat panel's width: {err}");
        }
    }

    /// The panel was shown (or a cave opened while it is): list the models
    /// if they are not listed for the selected provider yet. Listing asks
    /// the provider, so it waits until the panel is shown.
    pub fn shown(&mut self, cx: &mut Context<Self>) {
        let provider = agent_ops::selected_provider(cx);
        if provider.is_none() || provider != self.models_for || self.models_error.is_some() {
            self.load_models(cx);
        }
    }

    fn follow_tail(&self, cx: &mut Context<Self>) {
        self.view
            .read(cx)
            .list_state()
            .set_follow_mode(FollowMode::Tail);
    }

    fn on_cave_event(&mut self, event: &CaveEvent, cx: &mut Context<Self>) {
        self.tasks = agent_ops::list_tasks(cx);
        if *event == CaveEvent::Opened {
            // A new cave brings its own config; its agent starts empty.
            // The workspace calls `shown` next when the panel is shown.
            self.stream = None;
            self.attached.clear();
            self.clear(cx);
        } else {
            // The open note's icon may have changed.
            if let Some(active) = self.active_note.take() {
                self.set_active_note(Some(active.slug), cx);
            }
            // A note may have appeared or gone: links follow.
            let core = Host::core(cx);
            let _ = core.with_cave(|cave| {
                self.links.resolve_in(Vec::new(), true, cave);
                Ok(())
            });
        }
        cx.notify();
    }

    /// A settings change: the chat font redraws; a reset agent (its config
    /// or system prompt changed) stops the reply and starts over, and the
    /// models are listed again if the selected provider changed.
    fn on_config_event(&mut self, event: ConfigEvent, cx: &mut Context<Self>) {
        if event.agent {
            self.stream = None;
            self.clear(cx);
            if self.models_for.is_some() && agent_ops::selected_provider(cx) != self.models_for {
                self.load_models(cx);
            }
        }
        cx.notify();
    }

    // ── Sending ──────────────────────────────────────────────────────

    /// Whether the selected model is one the provider lists, which sending
    /// requires (as in the Tauri app).
    fn has_model(&self, cx: &gpui::App) -> bool {
        let core = Host::core(cx);
        let config = core.lock_config();
        let Some(selected) = config.agent.selected_model.as_deref() else {
            return false;
        };
        !self.models_loading && self.models.iter().any(|model| model.id == selected)
    }

    fn can_send(&self, cx: &gpui::App) -> bool {
        !self.is_streaming() && self.has_model(cx)
    }

    fn on_enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pick = self
            .visible_tasks(cx)
            .get(self.task_highlight)
            .map(|task| task.name.clone());
        match pick {
            Some(name) => self.complete_task(&name, window, cx),
            None => self.send(window, cx),
        }
    }

    /// Send what the input holds.
    pub fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self.input.read(cx).value().to_string();
        if raw.trim().is_empty() || !self.can_send(cx) {
            return;
        }
        self.input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.submit(raw, cx);
    }

    /// Send `raw` as typed: a `/task` invocation renders the task's prompt
    /// with the rest as its input, anything else is sent as it is.
    pub fn submit(&mut self, raw: String, cx: &mut Context<Self>) {
        let invocation = parse_task_invocation(&raw, &self.tasks);
        // A new message brings the view back to the bottom.
        self.follow_tail(cx);
        let shown = self.transcript.push_user(&raw);
        self.push_view(shown, cx);
        let message = match invocation {
            Some((name, input)) => match agent_ops::render_task(&name, &input, cx) {
                Ok(prompt) => prompt,
                Err(err) => return self.show_error(&format!("Task error: {err}"), cx),
            },
            None => raw,
        };
        // The attached notes go with every message, as in the Tauri app.
        let prompt = build_agent_prompt(&message, &self.attached);
        #[cfg(test)]
        {
            self.last_prompt = Some(prompt.clone());
        }
        let core = Host::core(cx);
        let built = {
            // Provider clients may expect to be built inside the runtime.
            let _runtime = runtime::handle().enter();
            core.ensure_agent()
        };
        if let Err(err) = built {
            return self.show_error(&err.to_string(), cx);
        }
        let generation = core.agent_generation();
        let Some((agent, history)) = core.lock_agent().as_ref().map(|agent| agent.snapshot())
        else {
            return self.show_error(&AgentError::NotInitialized.to_string(), cx);
        };
        let open = async move { agent.stream_with_history(&prompt, history).await };
        self.run_stream(open, message, generation, cx);
    }

    /// Stream the reply that `open` starts, recording `message` and the
    /// reply in the agent's history when it completes and the agent is
    /// still of `generation`.
    pub(crate) fn run_stream(
        &mut self,
        open: impl Future<Output = Result<AgentStream, AgentError>> + Send + 'static,
        message: String,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        let task = cx.spawn(async move |this, cx| {
            let mut stream = match runtime::spawn_abortable(open).await {
                Ok(Ok(stream)) => stream,
                Ok(Err(err)) => {
                    this.update(cx, |this, cx| this.fail(&err.to_string(), cx))
                        .ok();
                    return;
                }
                Err(err) => {
                    this.update(cx, |this, cx| this.fail(&err.to_string(), cx))
                        .ok();
                    return;
                }
            };
            let mut reply = String::new();
            loop {
                let next = runtime::spawn_abortable(async move {
                    let item = stream.next_item().await;
                    (stream, item)
                })
                .await;
                let item = match next {
                    Ok((returned, item)) => {
                        stream = returned;
                        item
                    }
                    Err(err) => {
                        this.update(cx, |this, cx| this.fail(&err.to_string(), cx))
                            .ok();
                        return;
                    }
                };
                let alive = match item {
                    Some(Ok(AgentStreamItem::Text(text))) => {
                        reply.push_str(&text);
                        this.update(cx, |this, cx| this.on_reply_text(&text, cx))
                    }
                    Some(Ok(AgentStreamItem::ToolCall(info))) => {
                        this.update(cx, |this, cx| this.on_tool_call(&info, cx))
                    }
                    Some(Ok(AgentStreamItem::ToolResult)) => this.update(cx, |_, cx| {
                        CaveEvents::emit(CaveEvent::NotesChanged, cx);
                    }),
                    Some(Ok(AgentStreamItem::Other)) => Ok(()),
                    Some(Ok(AgentStreamItem::Done)) | None => break,
                    Some(Err(err)) => {
                        this.update(cx, |this, cx| this.fail(&err.to_string(), cx))
                            .ok();
                        return;
                    }
                };
                if alive.is_err() {
                    return;
                }
            }
            this.update(cx, |this, cx| this.finish(&message, &reply, generation, cx))
                .ok();
        });
        self.stream = Some(task);
        cx.notify();
    }

    fn on_reply_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let pushed = self.transcript.push_reply(text);
        self.push_view(pushed, cx);
    }

    fn on_tool_call(&mut self, info: &ToolCallInfo, cx: &mut Context<Self>) {
        let pushed = self.transcript.push_tool_call(info);
        self.push_view(pushed, cx);
    }

    /// The reply is complete.
    fn finish(&mut self, message: &str, reply: &str, generation: u64, cx: &mut Context<Self>) {
        self.stream = None;
        let pushed = self.transcript.finish_reply();
        self.push_view(pushed, cx);
        let core = Host::core(cx);
        {
            let mut agent = core.lock_agent();
            if core.agent_generation() == generation && !reply.is_empty() {
                if let Some(agent) = agent.as_mut() {
                    agent.record_exchange(message, reply);
                }
            }
        }
        // Whatever the tools did, the views catch up once more at the end.
        CaveEvents::emit(CaveEvent::NotesChanged, cx);
        cx.notify();
    }

    /// The reply failed.
    fn fail(&mut self, message: &str, cx: &mut Context<Self>) {
        warn!("The agent reply failed: {message}");
        self.stream = None;
        self.show_error(message, cx);
    }

    fn show_error(&mut self, message: &str, cx: &mut Context<Self>) {
        let pushed = self.transcript.push_error(message);
        self.push_view(pushed, cx);
    }

    /// Append `text` to the view and resolve the links it brought.
    fn push_view(&mut self, text: String, cx: &mut Context<Self>) {
        if !text.is_empty() {
            self.view.update(cx, |view, cx| view.push_str(&text, cx));
        }
        let targets = self.transcript.take_targets();
        if !targets.is_empty() {
            let core = Host::core(cx);
            let _ = core.with_cave(|cave| {
                self.links.resolve_in(targets, false, cave);
                Ok(())
            });
        }
        cx.notify();
    }

    /// Empty the view (not the agent's history).
    fn clear(&mut self, cx: &mut Context<Self>) {
        self.transcript.clear();
        self.links.clear();
        self.view.update(cx, |view, cx| view.set_text("", cx));
        self.follow_tail(cx);
        cx.notify();
    }

    /// Start a new conversation: stop the reply, empty the view and the
    /// agent's history.
    pub fn new_conversation(&mut self, cx: &mut Context<Self>) {
        self.stream = None;
        self.clear(cx);
        if let Some(agent) = Host::core(cx).lock_agent().as_mut() {
            agent.clear_history();
        }
    }

    // ── Provider, model and mode ─────────────────────────────────────

    /// The agent was reset by a settings change: stop the reply and start
    /// over, as the history went with the old agent.
    /// Re-picking the current value (`Ok(false)`) changes nothing. Returns
    /// whether the agent was reset.
    fn agent_changed(&mut self, result: Result<bool, String>, cx: &mut Context<Self>) -> bool {
        match result {
            Ok(true) => {
                self.stream = None;
                self.clear(cx);
                true
            }
            Ok(false) => false,
            Err(err) => {
                self.show_error(&err, cx);
                false
            }
        }
    }

    fn select_provider(&mut self, index: usize, cx: &mut Context<Self>) {
        let result = agent_ops::select_provider(index, cx);
        if self.agent_changed(result, cx) {
            self.load_models(cx);
        }
    }

    fn select_model(&mut self, id: String, cx: &mut Context<Self>) {
        let result = agent_ops::select_model(id, cx);
        self.agent_changed(result, cx);
    }

    fn select_mode(&mut self, mode: AgentMode, cx: &mut Context<Self>) {
        let result = agent_ops::select_mode(mode, cx);
        self.agent_changed(result, cx);
    }

    /// List the selected provider's models, in the background.
    fn load_models(&mut self, cx: &mut Context<Self>) {
        self.models.clear();
        self.models_error = None;
        let provider = agent_ops::selected_provider(cx);
        self.models_for = provider.clone();
        // Without model listing (tests) the provider is never asked.
        let Some(provider) = provider.filter(|_| Host::lists_models(cx)) else {
            self.models_loading = false;
            self.models_task = None;
            cx.notify();
            return;
        };
        self.models_loading = true;
        self.models_task = Some(cx.spawn(async move |this, cx| {
            let listed = runtime::spawn_abortable(async move {
                granit_core::agent::list_models(&provider).await
            })
            .await;
            this.update(cx, |this, cx| {
                this.models_loading = false;
                match listed {
                    Ok(Ok(models)) => this.models = models,
                    Ok(Err(err)) => this.models_error = Some(err.to_string().into()),
                    Err(err) => this.models_error = Some(err.to_string().into()),
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    // ── Tasks ────────────────────────────────────────────────────────

    /// The tasks offered for what is typed, unless dismissed.
    fn visible_tasks(&self, cx: &gpui::App) -> Vec<AgentDocInfo> {
        if self.tasks_dismissed || self.is_streaming() {
            return Vec::new();
        }
        let value = self.input.read(cx).value();
        matching_tasks(&value, &self.tasks)
            .into_iter()
            .cloned()
            .collect()
    }

    fn complete_task(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let text = format!("/{name} ");
        self.input.update(cx, |input, cx| {
            // Inserting leaves the cursor after the name.
            input.set_value("", window, cx);
            input.insert(text, window, cx);
            input.focus(window, cx);
        });
        self.task_highlight = 0;
        cx.notify();
    }

    fn move_task_highlight(&mut self, down: bool, cx: &mut Context<Self>) -> bool {
        let count = self.visible_tasks(cx).len();
        if count == 0 {
            return false;
        }
        self.task_highlight = if down {
            (self.task_highlight + 1) % count
        } else {
            (self.task_highlight + count - 1) % count
        };
        cx.notify();
        true
    }

    // ── Rendering ────────────────────────────────────────────────────

    /// The Markdown extensions of the view: the surfaces of user messages,
    /// tool calls and errors, and no raw HTML.
    fn markdown_extensions(&self) -> MarkdownExtensions {
        let extensions = MarkdownExtensions::default()
            .parser_revision(1)
            .raw_html(false)
            .block_surface(USER_SURFACE, |_, cx| {
                let theme = cx.theme();
                div()
                    .ml_6()
                    .px_3()
                    .py_2()
                    .rounded_lg()
                    .bg(theme.secondary)
                    .text_color(theme.secondary_foreground)
            })
            .block_surface(TOOL_SURFACE, |_, cx| {
                let theme = cx.theme();
                h_flex()
                    .gap_1p5()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(Icon::new(AppIcon::Wrench).xsmall())
            })
            .block_surface(ERROR_SURFACE, |_, cx| {
                let theme = cx.theme();
                h_flex()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .rounded_lg()
                    .border_1()
                    .border_color(theme.danger)
                    .text_color(theme.danger)
                    .child(Icon::new(AppIcon::CircleAlert).small())
            });
        #[cfg(test)]
        let extensions = {
            let probe = self.link_probe.clone();
            extensions.block_parser(move |node, _| {
                record_links(node, &mut probe.lock().unwrap());
                None
            })
        };
        extensions
    }

    fn render_conversation(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme();
        if self.transcript.is_empty() {
            return div()
                .flex_1()
                .min_h_0()
                .p_4()
                .text_center()
                .italic()
                .text_color(theme.muted_foreground)
                .child("Ask me anything about your notes…")
                .into_any_element();
        }
        let links = self.links.clone();
        let this = cx.entity().downgrade();
        let view = TextView::new(&self.view)
            .scrollable(true)
            .selectable(true)
            .stream_fade(true)
            .markdown_extensions(self.markdown_extensions())
            .link(move |link, _| links.retarget(link))
            .on_link_click(move |url, _, _, cx| match classify(url) {
                LinkTarget::Note { slug, anchor } => {
                    this.update(cx, |_, cx| cx.emit(ChatEvent::OpenNote { slug, anchor }))
                        .ok();
                }
                LinkTarget::External(url) => cx.open_url(&url),
            });
        // `scrollable` needs a parent with a definite height.
        div()
            .flex_1()
            .min_h_0()
            .px_3()
            .py_2()
            .child(view.size_full())
            .into_any_element()
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let core = Host::core(cx);
        let (providers, selected) = {
            let config = core.lock_config();
            let names: Vec<SharedString> = config
                .agent
                .providers
                .iter()
                .map(|entry| entry.display_name().into())
                .collect();
            (names, config.agent.selected_provider)
        };
        let label: SharedString = providers
            .get(selected)
            .cloned()
            .unwrap_or_else(|| "No provider".into());
        let this = cx.entity().downgrade();
        h_flex()
            .flex_none()
            .px_2()
            .py_1()
            .gap_1()
            .child(
                Button::new("chat-provider")
                    .ghost()
                    .xsmall()
                    .label(label)
                    .dropdown_caret(true)
                    .disabled(providers.is_empty())
                    .dropdown_menu(move |menu, _, _| {
                        providers
                            .iter()
                            .enumerate()
                            .fold(menu, |menu, (index, name)| {
                                let this = this.clone();
                                menu.item(
                                    PopupMenuItem::new(name.clone())
                                        .checked(index == selected)
                                        .on_click(move |_, _, cx| {
                                            this.update(cx, |this, cx| {
                                                this.select_provider(index, cx)
                                            })
                                            .ok();
                                        }),
                                )
                            })
                    }),
            )
            .child(div().flex_1())
            .child(
                Button::new("chat-new")
                    .ghost()
                    .xsmall()
                    .icon(AppIcon::SquarePen)
                    .tooltip("New conversation")
                    .on_click(cx.listener(|this, _, _, cx| this.new_conversation(cx))),
            )
    }

    fn render_task_list(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let tasks = self.visible_tasks(cx);
        if tasks.is_empty() {
            return None;
        }
        let theme = cx.theme();
        let highlight = self.task_highlight.min(tasks.len() - 1);
        Some(
            v_flex()
                .id("chat-tasks")
                .mx_3()
                .mb_1()
                .p_1()
                .max_h(px(200.))
                .overflow_y_scroll()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .bg(theme.popover)
                .text_sm()
                .children(tasks.into_iter().enumerate().map(|(ix, task)| {
                    let name = task.name.clone();
                    h_flex()
                        .id(SharedString::from(format!("chat-task-{ix}")))
                        .gap_2()
                        .px_2()
                        .py_1()
                        .rounded_sm()
                        .cursor_pointer()
                        .when(ix == highlight, |row| row.bg(theme.accent))
                        .hover(|row| row.bg(theme.accent))
                        .child(
                            div()
                                .flex_none()
                                .font_family(theme.mono_font_family.clone())
                                .child(SharedString::from(format!("/{}", task.name))),
                        )
                        .when(!task.description.is_empty(), |row| {
                            row.child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(SharedString::from(task.description.clone())),
                            )
                        })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.complete_task(&name, window, cx);
                        }))
                })),
        )
    }

    /// The attached notes, each with a remove button, and the open note with
    /// an attach button while it is not attached; `None` when there is
    /// neither.
    fn render_attachments(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let offered = self
            .active_note
            .as_ref()
            .filter(|active| !self.is_attached(&active.slug));
        if self.attached.is_empty() && offered.is_none() {
            return None;
        }
        let theme = cx.theme();
        let streaming = self.is_streaming();
        let chip = |id: SharedString| {
            h_flex()
                .id(id)
                .max_w_full()
                .min_w_0()
                .gap_1()
                .pl_1p5()
                .rounded_md()
                .border_1()
                .text_xs()
        };
        let attached = self.attached.iter().enumerate().map(|(ix, note)| {
            let slug = note.slug.clone();
            chip(format!("chat-attached-{ix}").into())
                .border_color(theme.primary)
                .bg(theme.secondary)
                .child(
                    Icon::new(IconName::FileText)
                        .xsmall()
                        .text_color(theme.primary),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(SharedString::from(note.slug.clone())),
                )
                .when_some(note.selected.as_deref(), |chip, selected| {
                    chip.child(
                        div()
                            .min_w_0()
                            .truncate()
                            .italic()
                            .text_color(theme.muted_foreground)
                            .child(SharedString::from(format!(
                                "\"{}\"",
                                selection_preview(selected)
                            ))),
                    )
                })
                .child(
                    Button::new(SharedString::from(format!("chat-detach-{ix}")))
                        .ghost()
                        .xsmall()
                        .icon(IconName::Close)
                        .tooltip("Remove attached note")
                        .disabled(streaming)
                        .on_click(cx.listener(move |this, _, _, cx| this.detach(&slug, cx))),
                )
        });
        let offered = offered.map(|active| {
            chip("chat-offered".into())
                .border_color(theme.border)
                .text_color(theme.muted_foreground)
                .child(
                    Icon::default()
                        .path(note_icon_path(active.icon.as_deref()))
                        .xsmall()
                        .text_color(theme.primary),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(SharedString::from(active.slug.clone())),
                )
                .child(
                    Button::new("chat-attach")
                        .ghost()
                        .xsmall()
                        .icon(IconName::Plus)
                        .tooltip("Attach current note")
                        .disabled(streaming)
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(ChatEvent::AttachActiveNote))),
                )
        });
        Some(
            h_flex()
                .flex_wrap()
                .gap_1p5()
                .px_2()
                .pt_2()
                .children(attached)
                .children(offered),
        )
    }

    fn render_input(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let core = Host::core(cx);
        let (selected_model, mode) = {
            let config = core.lock_config();
            (config.agent.selected_model.clone(), config.agent.mode)
        };
        let model_label: SharedString = if self.models_loading {
            "Loading models…".into()
        } else if self.models.is_empty() {
            match &self.models_error {
                Some(_) => "Models unavailable".into(),
                None => "No models".into(),
            }
        } else {
            self.models
                .iter()
                .find(|model| Some(&model.id) == selected_model.as_ref())
                .map(|model| SharedString::from(model.display_name().to_string()))
                .unwrap_or_else(|| "Select a model…".into())
        };
        let models: Vec<(String, SharedString)> = self
            .models
            .iter()
            .map(|model| (model.id.clone(), model.display_name().to_string().into()))
            .collect();
        let this = cx.entity().downgrade();
        let model_button = Button::new("chat-model")
            .ghost()
            .xsmall()
            .label(model_label)
            .dropdown_caret(true)
            .disabled(models.is_empty())
            .when_some(self.models_error.clone(), |button, err| button.tooltip(err))
            .dropdown_menu({
                let this = this.clone();
                move |menu, _, _| {
                    models
                        .iter()
                        .fold(menu.scrollable(true), |menu, (id, name)| {
                            let this = this.clone();
                            let id = id.clone();
                            menu.item(
                                PopupMenuItem::new(name.clone())
                                    .checked(Some(&id) == selected_model.as_ref())
                                    .on_click(move |_, _, cx| {
                                        let id = id.clone();
                                        this.update(cx, |this, cx| this.select_model(id, cx)).ok();
                                    }),
                            )
                        })
                }
            });
        let mode_button = Button::new("chat-mode")
            .ghost()
            .xsmall()
            .label(mode.label())
            .dropdown_caret(true)
            .dropdown_menu(move |menu, _, _| {
                [AgentMode::Agent, AgentMode::Ask]
                    .into_iter()
                    .fold(menu, |menu, item| {
                        let this = this.clone();
                        menu.item(
                            PopupMenuItem::new(item.label())
                                .checked(item == mode)
                                .on_click(move |_, _, cx| {
                                    this.update(cx, |this, cx| this.select_mode(item, cx)).ok();
                                }),
                        )
                    })
            });
        let send = Button::new("chat-send")
            .ghost()
            .xsmall()
            .icon(AppIcon::Send)
            .tooltip("Send message")
            .loading(self.is_streaming())
            .disabled(!self.can_send(cx))
            .on_click(cx.listener(|this, _, window, cx| this.send(window, cx)));
        let attachments = self.render_attachments(cx);
        let theme = cx.theme();
        v_flex()
            .flex_none()
            .mx_3()
            .mb_3()
            .rounded_lg()
            .border_1()
            .border_color(theme.input)
            .bg(theme.background)
            .children(attachments)
            .child(
                Textarea::new(&self.input)
                    .appearance(false)
                    .disabled(self.is_streaming()),
            )
            .child(
                h_flex()
                    .px_1()
                    .pb_1()
                    .gap_1()
                    .child(model_button)
                    .child(mode_button)
                    .child(div().flex_1())
                    .child(send),
            )
    }
}

impl Render for ChatPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let task_list = self.render_task_list(cx);
        let theme = cx.theme();
        let (width, font) = {
            let core = Host::core(cx);
            let config = core.lock_config();
            (
                self.resize.width(f32::from(config.agent_panel.width)),
                config.agent_font.clone(),
            )
        };
        let (font_family, font_size) = crate::theme::resolve_font(&font, theme);
        #[cfg(test)]
        {
            self.drawn_font = Some((font_family.clone(), font_size));
        }
        let (sidebar, sidebar_foreground) = (theme.sidebar, theme.sidebar_foreground);
        // The resize handle on the left edge draws the border line.
        let panel = v_flex()
            .id("chat-panel")
            .w(px(width))
            .h_full()
            .flex_none()
            .overflow_hidden()
            .bg(sidebar)
            .text_color(sidebar_foreground);
        self.resize
            .attach(panel, |this: &mut Self| &mut this.resize, Self::resized, cx)
            .font_family(font_family)
            .text_size(font_size)
            // While the task list is open, the arrow keys move through it.
            .capture_action(cx.listener(|this, _: &MoveUp, _, cx| {
                if this.move_task_highlight(false, cx) {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &MoveDown, _, cx| {
                if this.move_task_highlight(true, cx) {
                    cx.stop_propagation();
                }
            }))
            .capture_action(
                cx.listener(|this, _: &gpui_kit::component::input::Escape, _, cx| {
                    if !this.visible_tasks(cx).is_empty() {
                        this.tasks_dismissed = true;
                        cx.notify();
                        cx.stop_propagation();
                    }
                }),
            )
            .child(self.render_header(cx))
            .child(self.render_conversation(cx))
            .children(task_list)
            .child(self.render_input(cx))
            .child(self.resize.handle())
    }
}

/// A selected text shortened to one line for its attachment chip.
fn selection_preview(selected: &str) -> String {
    const MAX_CHARS: usize = 28;
    let collapsed = selected.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= MAX_CHARS {
        return collapsed;
    }
    let truncated: String = collapsed.chars().take(MAX_CHARS).collect();
    format!("{truncated}…")
}

/// Add the URL of every link in `node` to `found`.
#[cfg(test)]
fn record_links(
    node: &gpui_kit::component::text::markdown_ast::Node,
    found: &mut std::collections::BTreeSet<String>,
) {
    use gpui_kit::component::text::markdown_ast::Node;
    if let Node::Link(link) = node {
        found.insert(link.url.clone());
    }
    for child in node.children().into_iter().flatten() {
        record_links(child, found);
    }
}

#[cfg(test)]
mod tests;
