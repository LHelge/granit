use leptos::prelude::*;
use leptos::web_sys::wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use super::frontmatter::FrontmatterEditor;
use super::{use_editor_ctx, EditorCtx};

fn request_animation_frame(f: impl FnOnce() + 'static) {
    let cb = Closure::once_into_js(f);
    let _ = leptos::web_sys::window()
        .unwrap()
        .request_animation_frame(cb.as_ref().unchecked_ref());
}

// ── Textarea manipulation helpers ──────────────────────────────────

/// Build a String from a char slice.
fn chars_to_string(chars: &[char]) -> String {
    chars.iter().collect()
}

/// Snapshot of a textarea's state at the moment a key event fires.
struct TextareaState {
    value: String,
    chars: Vec<char>,
    start: usize,
    end: usize,
}

impl TextareaState {
    fn from(textarea: &web_sys::HtmlTextAreaElement) -> Self {
        let value = textarea.value();
        let chars: Vec<char> = value.chars().collect();
        let start = textarea.selection_start().ok().flatten().unwrap_or(0) as usize;
        let end = textarea.selection_end().ok().flatten().unwrap_or(0) as usize;
        Self {
            value,
            chars,
            start,
            end,
        }
    }

    fn has_selection(&self) -> bool {
        self.start != self.end
    }

    fn before(&self) -> String {
        chars_to_string(&self.chars[..self.start])
    }

    fn selected(&self) -> String {
        chars_to_string(&self.chars[self.start..self.end])
    }

    fn after_cursor(&self) -> String {
        chars_to_string(&self.chars[self.start..])
    }

    fn after_selection(&self) -> String {
        chars_to_string(&self.chars[self.end..])
    }

    fn char_at_cursor(&self) -> Option<char> {
        self.chars.get(self.start).copied()
    }

    /// The current line (text from last newline up to cursor).
    fn current_line(&self) -> (&str, usize) {
        let text_before = &self.value[..self
            .value
            .char_indices()
            .nth(self.start)
            .map(|(i, _)| i)
            .unwrap_or(self.value.len())];
        let line_start = text_before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        (&text_before[line_start..], line_start)
    }
}

/// Apply the textarea value + cursor position and sync the content signal.
fn apply(
    textarea: &web_sys::HtmlTextAreaElement,
    new_value: &str,
    cursor_start: u32,
    cursor_end: u32,
    ctx: EditorCtx,
) {
    textarea.set_value(new_value);
    let _ = textarea.set_selection_range(cursor_start, cursor_end);
    ctx.content.set(textarea.value());
}

// ── Line prefix detection ──────────────────────────────────────────

/// Recognised line prefix for continuation (list, checkbox, numbered, blockquote).
enum LinePrefix {
    Bullet(String),   // e.g. "  - "
    Checkbox(String), // e.g. "  - [ ] "
    Numbered {
        indent: String,
        next_num: u32,
        suffix: String, // ". " or ") "
    },
    Blockquote(String), // e.g. "> "
}

impl LinePrefix {
    /// The string to insert on the next line.
    fn continuation(&self) -> String {
        match self {
            Self::Bullet(p) | Self::Checkbox(p) | Self::Blockquote(p) => p.clone(),
            Self::Numbered {
                indent,
                next_num,
                suffix,
            } => format!("{indent}{next_num}{suffix}"),
        }
    }
}

/// Try to detect a list / quote prefix on the given line.
fn detect_prefix(line: &str) -> Option<LinePrefix> {
    let indent_len = line.len() - line.trim_start().len();
    let indent = &line[..indent_len];
    let trimmed = line.trim_start();

    // Checkbox: - [ ] or - [x]
    if trimmed.starts_with("- [ ] ")
        || trimmed.starts_with("- [x] ")
        || trimmed.starts_with("- [X] ")
    {
        return Some(LinePrefix::Checkbox(format!("{indent}- [ ] ")));
    }

    // Bullet: - text
    if trimmed.starts_with("- ") {
        return Some(LinePrefix::Bullet(format!("{indent}- ")));
    }

    // Numbered list: 1. or 1)
    if let Some(num_end) = trimmed.find(|c: char| !c.is_ascii_digit()) {
        if num_end > 0 {
            let rest = &trimmed[num_end..];
            if rest.starts_with(". ") || rest.starts_with(") ") {
                let num: u32 = trimmed[..num_end].parse().unwrap_or(1);
                let suffix = &rest[..2];
                return Some(LinePrefix::Numbered {
                    indent: indent.to_string(),
                    next_num: num + 1,
                    suffix: suffix.to_string(),
                });
            }
        }
    }

    // Blockquote: > text
    if trimmed.starts_with("> ") {
        return Some(LinePrefix::Blockquote(format!("{indent}> ")));
    }

    None
}

/// Returns true if the line is an empty continuation prefix with no real content.
fn is_empty_prefix(line: &str) -> bool {
    let trimmed = line.trim();
    // Bullet / checkbox
    if trimmed == "-" || trimmed == "- [ ]" || trimmed == "- [x]" || trimmed == "- [X]" {
        return true;
    }
    // Numbered: just "1." or "1)" with no text
    if let Some(num_end) = trimmed.find(|c: char| !c.is_ascii_digit()) {
        if num_end > 0 {
            let rest = trimmed[num_end..].trim();
            if rest == "." || rest == ")" {
                return true;
            }
        }
    }
    // Blockquote: just ">"
    if trimmed == ">" {
        return true;
    }
    false
}

// ── Individual keydown handlers ────────────────────────────────────

/// Auto-close a bracket pair: `[` → `[]`, `(` → `()`.
/// With selection, wraps the selected text.
fn handle_bracket(
    ev: &leptos::ev::KeyboardEvent,
    textarea: &web_sys::HtmlTextAreaElement,
    s: &TextareaState,
    ctx: EditorCtx,
    open: char,
    close: char,
) {
    ev.prevent_default();
    if s.has_selection() {
        let new_value = format!(
            "{}{open}{}{close}{}",
            s.before(),
            s.selected(),
            s.after_selection()
        );
        apply(
            textarea,
            &new_value,
            s.start as u32 + 1,
            s.end as u32 + 1,
            ctx,
        );
    } else {
        let new_value = format!("{}{open}{close}{}", s.before(), s.after_cursor());
        apply(
            textarea,
            &new_value,
            s.start as u32 + 1,
            s.start as u32 + 1,
            ctx,
        );
    }
}

/// Skip over a closing bracket/char if it's already at the cursor.
fn handle_skip_close(
    ev: &leptos::ev::KeyboardEvent,
    textarea: &web_sys::HtmlTextAreaElement,
    s: &TextareaState,
    expected: char,
) {
    if !s.has_selection() && s.char_at_cursor() == Some(expected) {
        ev.prevent_default();
        let _ = textarea.set_selection_range(s.start as u32 + 1, s.start as u32 + 1);
    }
}

/// Auto-close a formatting character (`*`, `_`, `` ` ``, `~`).
/// With selection: wrap. Without: skip-over if next char matches, else insert pair.
fn handle_formatting_char(
    ev: &leptos::ev::KeyboardEvent,
    textarea: &web_sys::HtmlTextAreaElement,
    s: &TextareaState,
    ctx: EditorCtx,
    ch: char,
) {
    if s.has_selection() {
        ev.prevent_default();
        let new_value = format!(
            "{}{ch}{}{ch}{}",
            s.before(),
            s.selected(),
            s.after_selection()
        );
        apply(
            textarea,
            &new_value,
            s.start as u32 + 1,
            s.end as u32 + 1,
            ctx,
        );
    } else if s.char_at_cursor() == Some(ch) {
        ev.prevent_default();
        let _ = textarea.set_selection_range(s.start as u32 + 1, s.start as u32 + 1);
    } else {
        ev.prevent_default();
        let new_value = format!("{}{ch}{ch}{}", s.before(), s.after_cursor());
        apply(
            textarea,
            &new_value,
            s.start as u32 + 1,
            s.start as u32 + 1,
            ctx,
        );
    }
}

/// Handle Enter: continue lists / blockquotes, or end empty ones.
fn handle_enter(
    ev: &leptos::ev::KeyboardEvent,
    textarea: &web_sys::HtmlTextAreaElement,
    s: &TextareaState,
    ctx: EditorCtx,
) {
    if s.has_selection() {
        return;
    }
    let (current_line, line_start) = s.current_line();

    if is_empty_prefix(current_line) {
        // Replace the empty prefix with a plain newline.
        ev.prevent_default();
        let byte_line_start = s
            .value
            .char_indices()
            .nth(line_start)
            .map(|(i, _)| i)
            .unwrap_or(s.value.len());
        let before = &s.value[..byte_line_start];
        let after = s.after_cursor();
        let new_value = format!("{before}\n{after}");
        let new_pos = line_start + 1;
        apply(textarea, &new_value, new_pos as u32, new_pos as u32, ctx);
    } else if let Some(prefix) = detect_prefix(current_line) {
        ev.prevent_default();
        let cont = prefix.continuation();
        let new_value = format!("{}\n{cont}{}", s.before(), s.after_cursor());
        let new_pos = s.start + 1 + cont.len();
        apply(textarea, &new_value, new_pos as u32, new_pos as u32, ctx);
    }
}

/// Handle Tab / Shift+Tab: indent or outdent list items.
fn handle_tab(
    ev: &leptos::ev::KeyboardEvent,
    textarea: &web_sys::HtmlTextAreaElement,
    s: &TextareaState,
    ctx: EditorCtx,
    shift: bool,
) {
    let (current_line, line_start) = s.current_line();
    // Only act on list lines.
    if detect_prefix(current_line).is_none() {
        return;
    }
    ev.prevent_default();

    let byte_line_start = s
        .value
        .char_indices()
        .nth(line_start)
        .map(|(i, _)| i)
        .unwrap_or(s.value.len());
    let before = &s.value[..byte_line_start];
    let after = s.after_cursor();

    if shift {
        // Outdent: remove up to 2 leading spaces.
        let stripped = if let Some(s) = current_line.strip_prefix("  ") {
            s
        } else {
            current_line.trim_start_matches(' ')
        };
        let removed = current_line.len() - stripped.len();
        let new_value = format!("{before}{stripped}{after}");
        let new_pos = s.start.saturating_sub(removed);
        apply(textarea, &new_value, new_pos as u32, new_pos as u32, ctx);
    } else {
        // Indent: add 2 spaces.
        let new_value = format!("{before}  {current_line}{after}");
        let new_pos = s.start + 2;
        apply(textarea, &new_value, new_pos as u32, new_pos as u32, ctx);
    }
}

// ── Paste handler ──────────────────────────────────────────────────

/// If text is selected and the clipboard contains a URL, wrap as a markdown link.
fn handle_paste(
    ev: leptos::ev::Event,
    content_ref: NodeRef<leptos::html::Textarea>,
    ctx: EditorCtx,
) {
    let ev: web_sys::ClipboardEvent = ev.unchecked_into();
    let Some(el) = content_ref.get() else { return };
    let textarea: &web_sys::HtmlTextAreaElement = el.as_ref();
    let s = TextareaState::from(textarea);

    if !s.has_selection() {
        return; // let browser handle normal paste
    }
    let Some(data) = ev.clipboard_data() else {
        return;
    };
    let Ok(clip_text) = data.get_data("text/plain") else {
        return;
    };
    let clip_trimmed = clip_text.trim();
    if !(clip_trimmed.starts_with("http://") || clip_trimmed.starts_with("https://")) {
        return; // not a URL, let browser paste normally
    }

    ev.prevent_default();
    let new_value = format!(
        "{}[{}]({}){}",
        s.before(),
        s.selected(),
        clip_trimmed,
        s.after_selection()
    );
    let new_pos = s.start + 1 + (s.end - s.start) + 2 + clip_trimmed.len() + 1;
    apply(textarea, &new_value, new_pos as u32, new_pos as u32, ctx);
}

// ── Main keydown dispatcher ────────────────────────────────────────

fn handle_content_keydown(
    ev: leptos::ev::KeyboardEvent,
    content_ref: NodeRef<leptos::html::Textarea>,
    ctx: EditorCtx,
) {
    let Some(el) = content_ref.get() else { return };
    let textarea: &web_sys::HtmlTextAreaElement = el.as_ref();
    let s = TextareaState::from(textarea);

    match ev.key().as_str() {
        "[" => handle_bracket(&ev, textarea, &s, ctx, '[', ']'),
        "(" => handle_bracket(&ev, textarea, &s, ctx, '(', ')'),
        "]" => handle_skip_close(&ev, textarea, &s, ']'),
        ")" => handle_skip_close(&ev, textarea, &s, ')'),
        "*" | "_" | "`" | "~" => {
            handle_formatting_char(&ev, textarea, &s, ctx, ev.key().chars().next().unwrap())
        }
        "Enter" => handle_enter(&ev, textarea, &s, ctx),
        "Tab" => handle_tab(&ev, textarea, &s, ctx, ev.shift_key()),
        _ => {}
    }
}

/// Raw markdown editor with title input and content textarea.
#[component]
pub(super) fn Writer() -> impl IntoView {
    let ctx = use_editor_ctx();
    let title_ref = NodeRef::<leptos::html::Input>::new();
    let content_ref = NodeRef::<leptos::html::Textarea>::new();

    // Focus and select the title input when requested.
    Effect::new(move || {
        if ctx.focus_title.get() {
            ctx.focus_title.set(false);
            request_animation_frame(move || {
                if let Some(el) = title_ref.get() {
                    let input: &web_sys::HtmlInputElement = el.as_ref();
                    let _ = input.focus();
                    input.select();
                }
            });
        }
    });

    // Focus the content textarea when requested.
    Effect::new(move || {
        if ctx.focus_content.get() {
            ctx.focus_content.set(false);
            request_animation_frame(move || {
                if let Some(el) = content_ref.get() {
                    let textarea: &web_sys::HtmlTextAreaElement = el.as_ref();
                    let _ = textarea.focus();
                }
            });
        }
    });

    view! {
        <input
            type="text"
            node_ref=title_ref
            class="not-prose w-full bg-transparent text-white text-4xl font-extrabold leading-tight outline-none mb-2"
            placeholder="Untitled"
            prop:value=move || ctx.title_input.get()
            on:input=move |ev| ctx.title_input.set(event_target_value(&ev))
            on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                if ev.key() == "Enter" {
                    ev.prevent_default();
                    if let Some(el) = content_ref.get() {
                        let textarea: &web_sys::HtmlTextAreaElement = el.as_ref();
                        let _ = textarea.focus();
                    }
                }
            }
        />
        <FrontmatterEditor />
        <textarea
            node_ref=content_ref
            class="not-prose w-full flex-1 bg-transparent text-stone-300 resize-none outline-none leading-relaxed"
            placeholder="Start writing..."
            style:font-family=move || ctx.config.get().markdown_font.font_family
            style:font-size=move || format!("{}px", ctx.config.get().markdown_font.font_size)
            prop:value=move || ctx.content.get()
            on:input=move |ev| ctx.content.set(event_target_value(&ev))
            on:keydown=move |ev| handle_content_keydown(ev, content_ref, ctx)
            on:paste=move |ev| handle_paste(ev.into(), content_ref, ctx)
        />
    }
}
