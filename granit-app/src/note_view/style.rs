//! The editor's look: live-preview styling from the active theme, shared
//! code-block highlighting, and keeping the caret in view.

use std::{cell::RefCell, collections::HashMap, ops::Range, rc::Rc, sync::Arc};

use gpui::{font, point, px, HighlightStyle, Hsla, Pixels, ScrollHandle, SharedString};
use gpui_kit::component::{
    highlighter::{HighlightTheme, SyntaxHighlighter},
    input::Rope,
    Theme,
};
use zorite_editor::SyntaxStyle;

/// Highlighted code blocks kept per highlighter; cleared when full, so a
/// block being typed into cannot grow it without bound.
const HIGHLIGHT_CACHE_LIMIT: usize = 64;

type Highlights = Vec<(Range<usize>, HighlightStyle)>;

/// Space kept between the caret and the viewport edge when scrolling it
/// into view.
const CARET_MARGIN: Pixels = px(24.);

fn with_alpha(mut color: Hsla, alpha: f32) -> Hsla {
    color.a = alpha;
    color
}

/// The editor's font family and size. The editor takes them from the
/// ambient text style, which the note view sets from this.
pub fn editor_font(theme: &Theme) -> (SharedString, Pixels) {
    (theme.font_family.clone(), theme.font_size)
}

/// Live-preview styling built from the active theme, so the editor follows
/// the Catppuccin light/dark palette. Rebuilt on every theme change.
pub fn markdown_style(theme: &Theme) -> SyntaxStyle {
    SyntaxStyle {
        marker: with_alpha(theme.muted_foreground, 0.6),
        code: theme.foreground,
        code_bg: theme.accent,
        link: theme.link,
        tag: theme.info,
        quote: theme.muted_foreground,
        alert_note: theme.info,
        alert_tip: theme.success,
        alert_important: theme.primary,
        alert_warning: theme.warning,
        alert_caution: theme.danger,
        alert_icons: None,
        rule: theme.border,
        mark_bg: with_alpha(theme.warning, 0.35),
        block_label: None,
        block_label_gen: 0,
        block_ref_count: None,
        popover_bg: theme.popover,
        popover_border: theme.border,
        popover_fg: theme.popover_foreground,
        popover_hover: theme.accent,
        popover_divider: theme.border,
        popover_danger: theme.danger,
        mono: font(theme.mono_font_family.clone()),
        property_icon: None,
    }
}

/// Token colors for fenced code blocks, from GPUI Kit's tree-sitter
/// highlighter and the theme's syntax colors.
///
/// One instance is shared by every editor the note view creates. It keeps
/// one highlighter per language, re-used for every block, because creating
/// one compiles the language's tree-sitter queries, which is far too slow to
/// repeat per block, keystroke or opened note (it froze the spike on notes
/// with several code blocks). Results are cached by language and block text,
/// since the editor calls this while shaping.
pub struct CodeHighlighter {
    theme: RefCell<Arc<HighlightTheme>>,
    highlighters: RefCell<HashMap<String, SyntaxHighlighter>>,
    cache: RefCell<HashMap<(String, String), Highlights>>,
}

impl CodeHighlighter {
    pub fn new(theme: Arc<HighlightTheme>) -> Rc<Self> {
        Rc::new(Self {
            theme: RefCell::new(theme),
            highlighters: RefCell::default(),
            cache: RefCell::default(),
        })
    }

    /// Switch to `theme`'s colors. The parsed highlighters are kept; only
    /// the resolved styles are dropped.
    pub fn set_theme(&self, theme: Arc<HighlightTheme>) {
        if *self.theme.borrow() != theme {
            *self.theme.borrow_mut() = theme;
            self.cache.borrow_mut().clear();
        }
    }

    pub fn highlight(&self, language: &str, code: &str) -> Highlights {
        let key = (language.to_lowercase(), code.to_string());
        if let Some(hit) = self.cache.borrow().get(&key) {
            return hit.clone();
        }
        let mut highlighters = self.highlighters.borrow_mut();
        let highlighter = highlighters
            .entry(key.0.clone())
            .or_insert_with(|| SyntaxHighlighter::new(&key.0));
        highlighter.update(None, &Rope::from_str(code), None);
        let theme = self.theme.borrow();
        let styles: Highlights = highlighter
            .styles(&(0..code.len()), &**theme)
            .into_iter()
            .filter(|(range, style)| !range.is_empty() && *style != HighlightStyle::default())
            .collect();
        let mut cache = self.cache.borrow_mut();
        if cache.len() >= HIGHLIGHT_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, styles.clone());
        styles
    }

    /// The hook for `EditorState::set_code_highlighter`.
    pub fn hook(self: &Rc<Self>) -> impl Fn(&str, &str) -> Highlights + 'static {
        let this = self.clone();
        move |language, code| this.highlight(language, code)
    }

    /// How many languages have a parsed highlighter.
    #[cfg(test)]
    pub fn languages(&self) -> usize {
        self.highlighters.borrow().len()
    }
}

/// Scroll `scroll` so the caret's window-space row (`caret_top..caret_bottom`)
/// is inside the scroll container's viewport. The editor grows to its content
/// height and does not scroll itself, so the host does this after edits and
/// caret moves. Returns whether the offset changed.
pub fn scroll_caret_into_view(
    scroll: &ScrollHandle,
    caret_top: Pixels,
    caret_bottom: Pixels,
) -> bool {
    let viewport = scroll.bounds();
    let offset = scroll.offset();
    let mut y = offset.y;
    if caret_bottom + CARET_MARGIN > viewport.bottom() {
        y -= caret_bottom + CARET_MARGIN - viewport.bottom();
    } else if caret_top - CARET_MARGIN < viewport.top() {
        y += viewport.top() - (caret_top - CARET_MARGIN);
    }
    set_scroll_y(scroll, y)
}

/// Scroll so the window-space `top` of a row sits `margin` below the top of
/// the viewport. Returns whether the offset changed.
pub fn scroll_row_to_top(scroll: &ScrollHandle, top: Pixels, margin: Pixels) -> bool {
    let viewport = scroll.bounds();
    let y = scroll.offset().y - (top - margin - viewport.top());
    set_scroll_y(scroll, y)
}

fn set_scroll_y(scroll: &ScrollHandle, y: Pixels) -> bool {
    let offset = scroll.offset();
    // Offsets run from 0 (top) to -max (bottom).
    let y = y.min(px(0.)).max(-scroll.max_offset().y);
    if y == offset.y {
        return false;
    }
    scroll.set_offset(point(offset.x, y));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_blocks_get_token_colors_from_one_highlighter_per_language() {
        let shared = CodeHighlighter::new(HighlightTheme::default_light());
        let highlight = shared.hook();
        let code = "fn main() {\n    let x = \"s\";\n}";
        let styles = highlight("rust", code);
        assert!(
            styles.iter().any(|(range, _)| &code[range.clone()] == "fn"),
            "{styles:?}"
        );
        assert!(styles.windows(2).all(|w| w[0].0.end <= w[1].0.start));
        // A second block of the language re-uses its highlighter.
        let other = "struct S;";
        let other_styles = highlight("rust", other);
        assert!(other_styles
            .iter()
            .any(|(range, _)| &other[range.clone()] == "struct"));
        assert_eq!(shared.languages(), 1);
        // Unknown languages stay plain.
        assert!(highlight("nosuchlang", code).is_empty());

        // A theme switch recolors without re-creating highlighters.
        let parsed = shared.languages();
        shared.set_theme(HighlightTheme::default_dark());
        assert_eq!(shared.languages(), parsed);
        let dark = shared.highlight("rust", code);
        assert_eq!(
            dark.iter().map(|(r, _)| r.clone()).collect::<Vec<_>>(),
            styles.iter().map(|(r, _)| r.clone()).collect::<Vec<_>>()
        );
        assert_ne!(dark, styles);
    }
}
