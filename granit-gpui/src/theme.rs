//! Catppuccin Latte (light) and Frappé (dark), following the OS appearance.
//!
//! The palette is taken from the matching `@plugin "daisyui/theme"` blocks in
//! the web frontend's `styles.css`, mapped onto GPUI Kit's theme keys:
//! `base-100` is the window background, `base-200` the sidebar and secondary
//! surfaces, `base-300` the accent (hover) surface, `neutral` the borders and
//! `neutral-content` the muted text. Code-block token colors follow the
//! Catppuccin syntax palette of each flavour (`highlight` in `themes.json`).

use std::rc::Rc;

use gpui::{App, Window};
use gpui_kit::component::{Theme, ThemeConfig, ThemeSet};

const THEMES: &str = include_str!("themes.json");

/// Parse the embedded theme set into its (light, dark) pair.
fn themes() -> (ThemeConfig, ThemeConfig) {
    let set: ThemeSet = serde_json::from_str(THEMES).expect("embedded themes.json is valid");
    let mut light = None;
    let mut dark = None;
    for theme in set.themes {
        if theme.mode.is_dark() {
            dark = Some(theme);
        } else {
            light = Some(theme);
        }
    }
    (
        light.expect("themes.json has a light theme"),
        dark.expect("themes.json has a dark theme"),
    )
}

/// Install both themes and apply the one matching the window's appearance.
///
/// Call after `gpui_kit::init`.
pub fn install(window: &mut Window, cx: &mut App) {
    let (light, dark) = themes();
    Theme::update(cx, |theme| {
        theme.light_theme = Rc::new(light);
        theme.dark_theme = Rc::new(dark);
    });
    sync(window, cx);
}

/// Re-apply the theme for the window's current appearance.
pub fn sync(window: &mut Window, cx: &mut App) {
    Theme::change(window.appearance(), None, cx);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_themes_parse_into_light_and_dark() {
        let (light, dark) = themes();
        assert_eq!(light.name.as_ref(), "Catppuccin Latte");
        assert_eq!(dark.name.as_ref(), "Catppuccin Frappé");
        assert_eq!(light.colors.background.as_deref(), Some("#eff1f5"));
        assert_eq!(dark.colors.background.as_deref(), Some("#303446"));

        for (config, mauve) in [(&light, 0x8839ef), (&dark, 0xca9ee6)] {
            let keyword = config
                .highlight
                .as_ref()
                .and_then(|style| style.syntax.style("keyword"))
                .and_then(|style| style.color);
            assert_eq!(keyword, Some(gpui::rgb(mauve).into()), "{}", config.name);
        }
    }
}
