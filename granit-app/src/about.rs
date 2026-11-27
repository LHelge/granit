//! The "About Granit" dialog: the version, the commit it was built from
//! and whether that tree had uncommitted changes, as in the Tauri app's
//! info modal. The commit and the dirty flag are set by `build.rs`
//! (`GRANIT_GIT_HASH`, `GRANIT_GIT_DIRTY`). Opened from the sidebar's info
//! button.
//!
//! The Tauri app's "Check for updates" button is not here: the GPUI app has
//! no updater yet.

use gpui::{div, px, App, IntoElement, ParentElement, SharedString, Styled, Window};
use gpui_kit::component::{
    button::{Button, ButtonVariants},
    h_flex,
    tag::Tag,
    v_flex, ActiveTheme, Sizable, WindowExt as _,
};
use granit_types::AppMetadata;

/// The app's name, repository, version and build.
pub fn metadata() -> AppMetadata {
    AppMetadata::new(
        option_env!("GRANIT_GIT_HASH").unwrap_or("unknown"),
        option_env!("GRANIT_GIT_DIRTY").unwrap_or("false") == "true",
    )
}

/// Open the about dialog in `window`.
pub fn open(window: &mut Window, cx: &mut App) {
    let metadata = metadata();
    let year = granit_core::calendar::Month::current().year();
    window.open_dialog(cx, move |dialog, _, cx| {
        dialog
            .title("About Granit")
            .w(px(420.))
            .child(details(&metadata, year, cx))
    });
}

/// The dialog's body.
fn details(metadata: &AppMetadata, year: i32, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let (muted, border) = (theme.muted_foreground, theme.border);
    let row = |label: &'static str, value: gpui::AnyElement| {
        h_flex()
            .justify_between()
            .gap_4()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(border)
            .child(div().text_sm().text_color(muted).child(label))
            .child(value)
    };
    let mono = |text: String| {
        div()
            .text_sm()
            .font_family(theme.mono_font_family.clone())
            .child(text)
            .into_any_element()
    };
    let url = metadata.repo_url.clone();
    let repository = Button::new("about-repository")
        .link()
        .small()
        .label(SharedString::from(metadata.repo_url.clone()))
        .on_click(move |_, _, cx| cx.open_url(&url))
        .into_any_element();
    let dirty = if metadata.git_dirty {
        Tag::warning().small().child("Yes")
    } else {
        Tag::success().small().child("No")
    };
    v_flex()
        .gap_4()
        .child(
            v_flex()
                .gap_1()
                .child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(metadata.app_name.clone()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(muted)
                        .child("Minimal desktop note-taking with an integrated AI agent."),
                ),
        )
        .child(
            v_flex()
                .border_1()
                .border_color(border)
                .rounded(theme.radius)
                .overflow_hidden()
                .child(row("Repository", repository))
                .child(row("Version", mono(metadata.version.clone())))
                .child(row("Commit", mono(metadata.git_commit_hash.clone())))
                .child(row("Dirty", dirty.into_any_element()).border_b_0()),
        )
        .child(
            div()
                .text_xs()
                .text_color(muted)
                .child(format!("Copyright © {year} LHelge")),
        )
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};
    use gpui_kit::component::WindowExt as _;

    use crate::host::HostOptions;
    use crate::tests::App;
    use crate::workspace::StartupCave;

    #[test]
    fn metadata_carries_the_build_of_build_rs() {
        let metadata = super::metadata();
        assert_eq!(metadata.version, env!("CARGO_PKG_VERSION"));
        // `build.rs` always sets both: a hash (or "unknown") and a flag.
        let hash = env!("GRANIT_GIT_HASH");
        assert_eq!(
            metadata.git_commit_hash,
            hash.chars().take(8).collect::<String>()
        );
        assert_eq!(metadata.git_dirty, env!("GRANIT_GIT_DIRTY") == "true");
        assert!(matches!(env!("GRANIT_GIT_DIRTY"), "true" | "false"));
    }

    #[gpui::test]
    fn the_about_dialog_opens_and_draws(cx: &mut TestAppContext) {
        let app = App::start(HostOptions::default(), StartupCave::None, cx);
        cx.update_window(app.window, |_, window, cx| super::open(window, cx))
            .unwrap();
        cx.run_until_parked();
        app.draw(cx);
        let open = cx
            .update_window(app.window, |_, window, cx| window.has_active_dialog(cx))
            .unwrap();
        assert!(open, "the about dialog is shown");
    }
}
