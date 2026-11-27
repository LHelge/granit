//! Catppuccin Latte (light) and Frappé (dark), following the OS appearance.
//!
//! The palette is taken from the matching `@plugin "daisyui/theme"` blocks in
//! the web frontend's `styles.css`, mapped onto GPUI Kit's theme keys:
//! `base-100` is the window background, `base-200` the sidebar and secondary
//! surfaces, `base-300` the accent (hover) surface, `neutral` the borders and
//! `neutral-content` the muted text. Code-block token colors follow the
//! Catppuccin syntax palette of each flavour (`highlight` in `themes.json`).
//!
//! Views read colors with `cx.theme()` (`gpui_kit::component::ActiveTheme`),
//! e.g. `cx.theme().sidebar`. Which flavour shows is the cave config's
//! [`ThemeChoice`] (`AppConfig::theme_choice`, settable in the settings
//! view): Latte, Frappé, or the one matching the window's appearance. The
//! root view calls [`sync`] when the appearance, the open cave or the theme
//! setting changes. Fonts from the config go through [`resolve_font`].

use std::rc::Rc;

use gpui::{px, App, Pixels, SharedString, Window, WindowAppearance};
use gpui_kit::component::{Theme, ThemeConfig, ThemeMode, ThemeSet};
use granit_types::{FontConfig, ThemeChoice};

use crate::host::Host;

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

/// Re-apply the theme the config asks for, the window's appearance deciding
/// when it follows the system.
pub fn sync(window: &mut Window, cx: &mut App) {
    let mode = mode_for(configured(cx), window.appearance());
    Theme::change(mode, None, cx);
}

/// The open cave's theme choice (following the system before the host is
/// set up).
fn configured(cx: &App) -> ThemeChoice {
    if cx.has_global::<Host>() {
        Host::core(cx).lock_config().theme_choice()
    } else {
        ThemeChoice::System
    }
}

/// The theme mode for `choice` in a window of `appearance`.
pub fn mode_for(choice: ThemeChoice, appearance: WindowAppearance) -> ThemeMode {
    match choice {
        ThemeChoice::System => appearance.into(),
        ThemeChoice::Light => ThemeMode::Light,
        ThemeChoice::Dark => ThemeMode::Dark,
    }
}

/// The stored family for the theme's UI font (the default).
pub const SANS_FONT: &str = "sans-serif";
/// The stored family for the theme's monospace font.
pub const MONO_FONT: &str = "monospace";

/// The family and size to draw `font` with. The CSS generics the Tauri app
/// stores (`sans-serif`, `serif`, `system-ui`, or none) mean the theme's UI
/// font, `monospace` its monospace font; a size of 0 the theme's size.
pub fn resolve_font(font: &FontConfig, theme: &Theme) -> (SharedString, Pixels) {
    let family = match font.font_family.trim() {
        MONO_FONT | "ui-monospace" => theme.mono_font_family.clone(),
        "" | SANS_FONT | "serif" | "system-ui" | "ui-sans-serif" => theme.font_family.clone(),
        name => SharedString::from(name.to_string()),
    };
    let size = if font.font_size > 0 {
        px(f32::from(font.font_size))
    } else {
        theme.font_size
    };
    (family, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_choice_overrides_the_appearance_unless_it_follows_the_system() {
        assert_eq!(
            mode_for(ThemeChoice::System, WindowAppearance::Dark),
            ThemeMode::Dark
        );
        assert_eq!(
            mode_for(ThemeChoice::System, WindowAppearance::Light),
            ThemeMode::Light
        );
        assert_eq!(
            mode_for(ThemeChoice::Light, WindowAppearance::Dark),
            ThemeMode::Light
        );
        assert_eq!(
            mode_for(ThemeChoice::Dark, WindowAppearance::Light),
            ThemeMode::Dark
        );
    }

    #[test]
    fn css_generic_fonts_resolve_to_the_theme_fonts() {
        let theme = Theme::default();
        let font = |family: &str, size| FontConfig {
            font_family: family.into(),
            font_size: size,
        };
        assert_eq!(
            resolve_font(&font("monospace", 14), &theme),
            (theme.mono_font_family.clone(), px(14.))
        );
        assert_eq!(
            resolve_font(&font("sans-serif", 0), &theme),
            (theme.font_family.clone(), theme.font_size)
        );
        assert_eq!(
            resolve_font(&font("Iosevka", 18), &theme),
            (SharedString::from("Iosevka"), px(18.))
        );
    }

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
