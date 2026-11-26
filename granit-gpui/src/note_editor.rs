//! Theme mapping and host-side helpers for the zorite live-preview editor.

use std::{cell::RefCell, collections::HashMap, ops::Range, sync::Arc};

use gpui::{font, point, px, HighlightStyle, Hsla, Pixels, ScrollHandle};
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

/// Live-preview styling built from the active Kit theme, so the editor
/// follows the Catppuccin light/dark palette. Rebuild it after every theme
/// change.
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

/// Token colors for fenced code blocks, from the Kit's tree-sitter
/// highlighter and the theme's syntax colors. The editor calls this while
/// shaping text, so results are cached by language and block text.
///
/// One highlighter is kept per language and re-used for every block:
/// creating one compiles the language's tree-sitter queries, which is far
/// too slow to repeat for each block or keystroke.
pub fn code_highlighter(theme: Arc<HighlightTheme>) -> impl Fn(&str, &str) -> Highlights + 'static {
    let cache: RefCell<HashMap<(String, String), Highlights>> = RefCell::default();
    let highlighters: RefCell<HashMap<String, SyntaxHighlighter>> = RefCell::default();
    move |language, code| {
        let key = (language.to_lowercase(), code.to_string());
        if let Some(hit) = cache.borrow().get(&key) {
            return hit.clone();
        }
        let mut highlighters = highlighters.borrow_mut();
        let highlighter = highlighters
            .entry(key.0.clone())
            .or_insert_with(|| SyntaxHighlighter::new(&key.0));
        highlighter.update(None, &Rope::from_str(code), None);
        let styles: Highlights = highlighter
            .styles(&(0..code.len()), &*theme)
            .into_iter()
            .filter(|(range, style)| !range.is_empty() && *style != HighlightStyle::default())
            .collect();
        let mut cache = cache.borrow_mut();
        if cache.len() >= HIGHLIGHT_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, styles.clone());
        styles
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
    use std::{cell::Cell, rc::Rc};

    use gpui::{AppContext, EntityInputHandler, TestAppContext};
    use gpui_kit::component::{highlighter::HighlightTheme, Theme};
    use zorite_editor::{EditorEvent, EditorState};

    use super::{code_highlighter, markdown_style};
    use crate::save::{FlushOutcome, NoteSession};

    /// Bodies covering the constructs the live preview restyles.
    const SAMPLES: &[&str] = &[
        "",
        "# Only a heading",
        "# Heading\n\nSome **bold**, *italic*, `code` and a [[wiki-link]].\n",
        "- [ ] open\n- [x] done\n\n| a | b |\n| - | - |\n| 1 | 2 |\n",
        "```rust\nfn main() {}\n```\n\n> quote\n\n---\n\ntrailing spaces  \n\n\n",
        "tabs\tand CRLF\r\nlines\r\n",
        "Åäö, émoji 😀, かな\nno trailing newline",
        "# Volvo {#volvo}\n\n[[note|label]] ![img](a.png) #tag\n",
    ];

    fn open_editor(cx: &mut TestAppContext) -> gpui::WindowHandle<EditorState> {
        cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut editor = EditorState::new(window, cx);
                    editor.set_markdown_style(markdown_style(&Theme::default()), cx);
                    editor
                })
            })
            .unwrap()
        })
    }

    #[test]
    fn code_blocks_get_token_colors() {
        let highlight = code_highlighter(HighlightTheme::default_light());
        let code = "fn main() {\n    let x = \"s\";\n}";
        let styles = highlight("rust", code);
        assert!(
            styles.iter().any(|(range, _)| &code[range.clone()] == "fn"),
            "{styles:?}"
        );
        assert!(styles.windows(2).all(|w| w[0].0.end <= w[1].0.start));
        // Aliases resolve; unknown languages stay plain.
        assert!(!highlight("rs", code).is_empty());
        // One highlighter serves every block of a language: a second block
        // gets its own tokens, and the first still highlights as before.
        let other = "struct S;";
        let other_styles = highlight("rust", other);
        assert!(other_styles
            .iter()
            .any(|(range, _)| &other[range.clone()] == "struct"));
        assert!(other_styles
            .iter()
            .all(|(range, _)| range.end <= other.len()));
        let fresh = code_highlighter(HighlightTheme::default_light());
        assert_eq!(fresh("rust", code), styles);
        assert_eq!(fresh("rust", other), other_styles);
        assert_eq!(
            fresh("rust", &format!("{code}\n")),
            highlight("rust", &format!("{code}\n"))
        );
        assert!(highlight("nosuchlang", code).is_empty());
    }

    #[gpui::test]
    fn loaded_text_reads_back_unchanged(cx: &mut TestAppContext) {
        let editor = open_editor(cx);
        for sample in SAMPLES {
            editor
                .update(cx, |editor, _, cx| {
                    editor.set_text(*sample, cx);
                    assert_eq!(editor.text(), *sample);
                })
                .unwrap();
        }
    }

    #[gpui::test]
    fn display_math_fences_are_normalized_on_load(cx: &mut TestAppContext) {
        // Recorded behaviour, not a requirement: the editor splits `$$`
        // fences that share a line with words onto their own lines when it
        // loads text, so the first edit of such a note also rewrites them.
        let editor = open_editor(cx);
        editor
            .update(cx, |editor, _, cx| {
                editor.set_text("$$x$$\n", cx);
                assert_eq!(editor.text(), "$$x$$\n");
                editor.set_text("text $$x$$ text\n", cx);
                assert_eq!(editor.text(), "text\n$$x$$\ntext\n");
            })
            .unwrap();
    }

    #[gpui::test]
    fn typing_and_deleting_a_character_leaves_nothing_to_save(cx: &mut TestAppContext) {
        let editor = open_editor(cx);
        let body = "# Heading\n\nText\n";
        let changes = Rc::new(Cell::new(0));
        let entity = editor.root(cx).unwrap();
        let _subscription = cx.update(|cx| {
            let changes = changes.clone();
            cx.subscribe(&entity, move |_, event: &EditorEvent, _| {
                if *event == EditorEvent::Changed {
                    changes.set(changes.get() + 1);
                }
            })
        });

        editor
            .update(cx, |editor, _, cx| editor.set_text(body, cx))
            .unwrap();
        cx.run_until_parked();
        assert_eq!(changes.get(), 0, "loading is not an edit");

        let mut session = NoteSession::new("note", body);
        editor
            .update(cx, |editor, window, cx| {
                let end = body.len();
                editor.set_cursor(end, cx);
                editor.replace_text_in_range(None, "x", window, cx);
                assert_eq!(editor.text(), "# Heading\n\nText\nx");
                editor.replace_text_in_range(Some(end..end + 1), "", window, cx);
            })
            .unwrap();
        cx.run_until_parked();
        assert!(changes.get() >= 2, "typing emits Changed");
        session.mark_edited();

        let text = editor
            .update(cx, |editor, _, _| editor.text().to_string())
            .unwrap();
        assert_eq!(text, body);
        let outcome = session.flush(&text, |_, _| -> Result<(), String> {
            panic!("an unchanged body must not be written")
        });
        assert_eq!(outcome, FlushOutcome::Unchanged);
    }
}
