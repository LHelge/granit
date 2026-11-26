//! GPUI frontend spike: a themed window with a sidebar placeholder and a
//! main area.

mod theme;

use gpui::{
    div, px, size, App, AppContext, Bounds, Context, IntoElement, KeyBinding, ParentElement,
    Render, Styled, Subscription, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use gpui_kit::component::ActiveTheme;

gpui_kit::actions!(granit, [Quit]);

struct Workspace {
    _appearance: Subscription,
}

impl Workspace {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let appearance = cx.observe_window_appearance(window, |_, window, cx| {
            theme::sync(window, cx);
        });
        Self {
            _appearance: appearance,
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
            .child(
                div()
                    .w(px(260.))
                    .h_full()
                    .flex_none()
                    .p_3()
                    .bg(theme.sidebar)
                    .text_color(theme.sidebar_foreground)
                    .border_r_1()
                    .border_color(theme.sidebar_border)
                    .child("Sidebar"),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .child("No note open"),
                    ),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx: &mut App| {
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
            cx.new(|cx| Workspace::new(window, cx))
        })
        .expect("failed to open window");
        cx.activate(true);
    });
}
