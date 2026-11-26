//! GPUI frontend spike: opens the cave given on the command line, shows its
//! note tree in the sidebar and the selected note's slug in the main area.

mod sidebar;
mod theme;
mod tree_model;

use std::path::PathBuf;

use gpui::{
    div, px, size, App, AppContext, Bounds, Context, Entity, IntoElement, KeyBinding,
    ParentElement, Render, SharedString, Styled, Subscription, TitlebarOptions, Window,
    WindowBounds, WindowOptions,
};
use gpui_kit::component::{
    tree::{tree, TreeState},
    ActiveTheme,
};
use granit_lib::cave::{Cave, CaveError};

use tree_model::{build_tree, item_kind, to_tree_items, ItemKind, TreeNode};

gpui_kit::actions!(granit, [Quit]);

#[derive(Debug, thiserror::Error)]
enum OpenError {
    #[error("No cave path given. Usage: granit-gpui <cave-path>")]
    MissingPath,
    #[error(transparent)]
    Cave(#[from] CaveError),
}

/// Open and scan the cave, then build its display tree. Blocking file I/O;
/// runs on the background executor.
fn load_cave(path: PathBuf) -> Result<(Cave, Vec<TreeNode>), OpenError> {
    let cave = Cave::open(path)?;
    let tree = build_tree(cave.list_notes()?, cave.list_folders()?);
    Ok((cave, tree))
}

enum CaveStatus {
    Loading,
    Failed(SharedString),
    Open {
        cave: Box<Cave>,
        tree: Entity<TreeState>,
        _selection: Subscription,
    },
}

struct Workspace {
    status: CaveStatus,
    /// Slug of the note selected in the tree; the main area shows this note.
    selected_slug: Option<SharedString>,
    _appearance: Subscription,
}

impl Workspace {
    fn new(cave_path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let appearance = cx.observe_window_appearance(window, |_, window, cx| {
            theme::sync(window, cx);
        });
        let status = match cave_path {
            Some(path) => {
                cx.spawn(async move |this, cx| {
                    let result = cx.background_spawn(async move { load_cave(path) }).await;
                    this.update(cx, |this, cx| this.finish_loading(result, cx))
                        .ok();
                })
                .detach();
                CaveStatus::Loading
            }
            None => CaveStatus::Failed(OpenError::MissingPath.to_string().into()),
        };
        Self {
            status,
            selected_slug: None,
            _appearance: appearance,
        }
    }

    fn finish_loading(
        &mut self,
        result: Result<(Cave, Vec<TreeNode>), OpenError>,
        cx: &mut Context<Self>,
    ) {
        self.status = match result {
            Ok((cave, nodes)) => {
                let items = to_tree_items(&nodes);
                let tree = cx.new(|cx| TreeState::new(cx).items(items));
                let selection = cx.observe(&tree, Self::sync_selection);
                CaveStatus::Open {
                    cave: Box::new(cave),
                    tree,
                    _selection: selection,
                }
            }
            Err(err) => CaveStatus::Failed(format!("Could not open cave: {err}").into()),
        };
        cx.notify();
    }

    /// Follow the tree selection when it lands on a note. Selecting a folder
    /// keeps the current note open.
    fn sync_selection(&mut self, tree: Entity<TreeState>, cx: &mut Context<Self>) {
        let Some(item) = tree.read(cx).selected_item() else {
            return;
        };
        let Some(ItemKind::Note(slug)) = item_kind(&item.id) else {
            return;
        };
        if self.selected_slug.as_deref() != Some(slug) {
            self.selected_slug = Some(SharedString::from(slug.to_string()));
            cx.notify();
        }
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let content = match &self.status {
            CaveStatus::Open {
                cave, tree: state, ..
            } => {
                let name = cave
                    .path()
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| cave.path().display().to_string());
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(name),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .px_1()
                            .child(tree(state, sidebar::render_entry)),
                    )
                    .into_any_element()
            }
            CaveStatus::Loading | CaveStatus::Failed(_) => div().into_any_element(),
        };
        div()
            .w(px(260.))
            .h_full()
            .flex_none()
            .overflow_hidden()
            .bg(theme.sidebar)
            .text_color(theme.sidebar_foreground)
            .border_r_1()
            .border_color(theme.sidebar_border)
            .child(content)
    }

    fn render_main(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let message = match (&self.status, &self.selected_slug) {
            (CaveStatus::Failed(err), _) => div().text_color(theme.danger).child(err.clone()),
            (CaveStatus::Loading, _) => div()
                .text_color(theme.muted_foreground)
                .child("Opening cave…"),
            (CaveStatus::Open { .. }, Some(slug)) => div().child(slug.clone()),
            (CaveStatus::Open { .. }, None) => div()
                .text_color(theme.muted_foreground)
                .child("No note open"),
        };
        div()
            .flex_1()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .p_6()
            .child(message)
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
            .child(self.render_sidebar(cx))
            .child(self.render_main(cx))
    }
}

fn main() {
    let cave_path = std::env::args_os().nth(1).map(PathBuf::from);

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            zorite_editor::bind_keys(cx);

            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
            cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1100.), px(720.)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Granit".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                theme::install(window, cx);
                cx.new(|cx| Workspace::new(cave_path, window, cx))
            })
            .expect("failed to open window");
            cx.activate(true);
        });
}
