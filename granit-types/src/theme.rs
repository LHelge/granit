use serde::{Deserialize, Serialize};

/// Summary of a theme for listing purposes (id, name, dark flag).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMeta {
    pub id: String,
    pub name: String,
    pub is_dark: bool,
}

/// Full theme color palette with application-semantic names.
///
/// All colors are hex strings, e.g. `"#1e1e2e"`.
///
/// Field names describe the *role* each color plays in the UI, making it
/// straightforward to create new themes without knowledge of any specific
/// palette naming convention (like Catppuccin).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub is_dark: bool,

    // ── Backgrounds ────────────────────────────────────────────────
    /// Deepest background — main window and editor area.
    pub window_bg: String,
    /// Panel background — sidebar, titlebar, agent panel.
    pub panel_bg: String,
    /// Elevated card background — modals, dropdowns, inputs, popovers.
    pub card_bg: String,
    /// Hover / selected item background — buttons, tags, selected rows.
    pub item_hover_bg: String,
    /// Active / pressed item background — prominent selections, user bubbles.
    pub item_active_bg: String,
    /// Strongest background highlight — resize handles, save button hover.
    pub highlight_bg: String,

    // ── Text / foreground ──────────────────────────────────────────
    /// Primary readable text — body content, input text.
    pub text_primary: String,
    /// Secondary text — titles, labels, button text, tag pills.
    pub text_secondary: String,
    /// Muted text — icons, section headings, interactive defaults.
    pub text_muted: String,
    /// Faintest text — placeholders, descriptions, empty-state hints.
    pub text_faint: String,

    // ── Borders ────────────────────────────────────────────────────
    /// Default component border — inputs, cards, dropdowns.
    pub border_color: String,
    /// Subtle structural border — panel dividers, section separators.
    pub border_subtle: String,
    /// Hover-state border — interactive element hover.
    pub border_hover: String,
    /// Focus ring border — keyboard/focus indicator on inputs.
    pub border_focus: String,

    // ── Semantic accent colors ─────────────────────────────────────
    /// Primary accent — links, focus indicators, active elements.
    pub accent: String,
    /// Error / destructive action color.
    pub error: String,
    /// Success indication color.
    pub success: String,
    /// Warning / caution color.
    pub warning: String,
}

impl Theme {
    pub fn meta(&self) -> ThemeMeta {
        ThemeMeta {
            id: self.id.clone(),
            name: self.name.clone(),
            is_dark: self.is_dark,
        }
    }
}

/// All built-in themes.
pub fn builtin_themes() -> Vec<Theme> {
    vec![
        theme_default(),
        theme_latte(),
        theme_frappe(),
        theme_macchiato(),
        theme_mocha(),
    ]
}

/// Default theme — neutral stone palette matching Granit's original dark look.
pub fn theme_default() -> Theme {
    Theme {
        id: "default".into(),
        name: "Default".into(),
        is_dark: true,
        window_bg: "#1c1917".into(),
        panel_bg: "#231f1e".into(),
        card_bg: "#292524".into(),
        item_hover_bg: "#44403c".into(),
        item_active_bg: "#57534e".into(),
        highlight_bg: "#78716c".into(),
        text_primary: "#e7e5e4".into(),
        text_secondary: "#d6d3d1".into(),
        text_muted: "#a8a29e".into(),
        text_faint: "#78716c".into(),
        border_color: "#57534e".into(),
        border_subtle: "#44403c".into(),
        border_hover: "#78716c".into(),
        border_focus: "#a8a29e".into(),
        accent: "#7098d8".into(),
        error: "#f87171".into(),
        success: "#7ab870".into(),
        warning: "#c8a048".into(),
    }
}

/// Catppuccin Latte — light flavour.
pub fn theme_latte() -> Theme {
    Theme {
        id: "latte".into(),
        name: "Catppuccin Latte".into(),
        is_dark: false,
        window_bg: "#eff1f5".into(),
        panel_bg: "#e6e9ef".into(),
        card_bg: "#ccd0da".into(),
        item_hover_bg: "#bcc0cc".into(),
        item_active_bg: "#acb0be".into(),
        highlight_bg: "#8c8fa1".into(),
        text_primary: "#4c4f69".into(),
        text_secondary: "#5c5f77".into(),
        text_muted: "#7c7f93".into(),
        text_faint: "#8c8fa1".into(),
        border_color: "#acb0be".into(),
        border_subtle: "#bcc0cc".into(),
        border_hover: "#8c8fa1".into(),
        border_focus: "#7c7f93".into(),
        accent: "#1e66f5".into(),
        error: "#d20f39".into(),
        success: "#40a02b".into(),
        warning: "#df8e1d".into(),
    }
}

/// Catppuccin Frappé — medium-dark flavour.
pub fn theme_frappe() -> Theme {
    Theme {
        id: "frappe".into(),
        name: "Catppuccin Frappé".into(),
        is_dark: true,
        window_bg: "#303446".into(),
        panel_bg: "#292c3c".into(),
        card_bg: "#414559".into(),
        item_hover_bg: "#51576d".into(),
        item_active_bg: "#626880".into(),
        highlight_bg: "#838ba7".into(),
        text_primary: "#c6d0f5".into(),
        text_secondary: "#b5bfe2".into(),
        text_muted: "#949cbb".into(),
        text_faint: "#838ba7".into(),
        border_color: "#626880".into(),
        border_subtle: "#51576d".into(),
        border_hover: "#838ba7".into(),
        border_focus: "#949cbb".into(),
        accent: "#8caaee".into(),
        error: "#e78284".into(),
        success: "#a6d189".into(),
        warning: "#e5c890".into(),
    }
}

/// Catppuccin Macchiato — darker flavour.
pub fn theme_macchiato() -> Theme {
    Theme {
        id: "macchiato".into(),
        name: "Catppuccin Macchiato".into(),
        is_dark: true,
        window_bg: "#24273a".into(),
        panel_bg: "#1e2030".into(),
        card_bg: "#363a4f".into(),
        item_hover_bg: "#494d64".into(),
        item_active_bg: "#5b6078".into(),
        highlight_bg: "#8087a2".into(),
        text_primary: "#cad3f5".into(),
        text_secondary: "#b8c0e0".into(),
        text_muted: "#939ab7".into(),
        text_faint: "#8087a2".into(),
        border_color: "#5b6078".into(),
        border_subtle: "#494d64".into(),
        border_hover: "#8087a2".into(),
        border_focus: "#939ab7".into(),
        accent: "#8aadf4".into(),
        error: "#ed8796".into(),
        success: "#a6da95".into(),
        warning: "#eed49f".into(),
    }
}

/// Catppuccin Mocha — darkest flavour.
pub fn theme_mocha() -> Theme {
    Theme {
        id: "mocha".into(),
        name: "Catppuccin Mocha".into(),
        is_dark: true,
        window_bg: "#1e1e2e".into(),
        panel_bg: "#181825".into(),
        card_bg: "#313244".into(),
        item_hover_bg: "#45475a".into(),
        item_active_bg: "#585b70".into(),
        highlight_bg: "#7f849c".into(),
        text_primary: "#cdd6f4".into(),
        text_secondary: "#bac2de".into(),
        text_muted: "#9399b2".into(),
        text_faint: "#7f849c".into(),
        border_color: "#585b70".into(),
        border_subtle: "#45475a".into(),
        border_hover: "#7f849c".into(),
        border_focus: "#9399b2".into(),
        accent: "#89b4fa".into(),
        error: "#f38ba8".into(),
        success: "#a6e3a1".into(),
        warning: "#f9e2af".into(),
    }
}
