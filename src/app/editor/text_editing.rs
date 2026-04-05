//! Pure text-editing logic for the markdown editor textarea.
//!
//! All cursor/selection positions use **char** (Unicode scalar value) offsets
//! internally. The DOM layer converts to/from UTF-16 at the boundary.

// ── UTF-16 ↔ char offset conversion ──────────────────────────────

/// Convert a UTF-16 code-unit offset (JS `selectionStart`/`selectionEnd`)
/// to a char (Unicode scalar value) index.
pub fn utf16_to_char_offset(s: &str, utf16_offset: usize) -> usize {
    let mut utf16_count = 0;
    let mut char_count = 0;
    for c in s.chars() {
        if utf16_count >= utf16_offset {
            break;
        }
        utf16_count += c.len_utf16();
        char_count += 1;
    }
    char_count
}

/// Convert a char index to a UTF-16 code-unit offset.
pub fn char_to_utf16_offset(s: &str, char_offset: usize) -> usize {
    s.chars().take(char_offset).map(|c| c.len_utf16()).sum()
}

// ── TextareaState ────────────────────────────────────────────────

/// Snapshot of a textarea's text content and cursor/selection positions.
/// All offsets are in char (Unicode scalar value) units.
pub struct TextareaState {
    value: String,
    chars: Vec<char>,
    start: usize,
    end: usize,
}

impl TextareaState {
    /// Create from a string value and char-based cursor positions.
    pub fn new(value: String, start: usize, end: usize) -> Self {
        let chars: Vec<char> = value.chars().collect();
        let start = start.min(chars.len());
        let end = end.min(chars.len());
        Self {
            value,
            chars,
            start,
            end,
        }
    }

    /// Create from a string and UTF-16 cursor positions (from the JS DOM).
    pub fn from_utf16(value: String, utf16_start: usize, utf16_end: usize) -> Self {
        let start = utf16_to_char_offset(&value, utf16_start);
        let end = utf16_to_char_offset(&value, utf16_end);
        Self::new(value, start, end)
    }

    pub fn has_selection(&self) -> bool {
        self.start != self.end
    }

    fn before(&self) -> String {
        self.chars[..self.start].iter().collect()
    }

    fn selected(&self) -> String {
        self.chars[self.start..self.end].iter().collect()
    }

    fn after_cursor(&self) -> String {
        self.chars[self.start..].iter().collect()
    }

    fn after_selection(&self) -> String {
        self.chars[self.end..].iter().collect()
    }

    fn char_at_cursor(&self) -> Option<char> {
        self.chars.get(self.start).copied()
    }

    /// Byte offset in `self.value` for a given char offset.
    fn char_to_byte(&self, char_offset: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_offset)
            .map(|(i, _)| i)
            .unwrap_or(self.value.len())
    }

    /// The current line (text from the last newline up to the cursor)
    /// and its starting char offset.
    fn current_line(&self) -> (&str, usize) {
        let cursor_byte = self.char_to_byte(self.start);
        let text_before = &self.value[..cursor_byte];
        let line_start_byte = text_before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_start_char = self.value[..line_start_byte].chars().count();
        (&self.value[line_start_byte..cursor_byte], line_start_char)
    }

    // ── Edit operations ─────────────────────────────────────────

    /// Auto-close a bracket pair (e.g. `[` → `[]`).
    /// With a selection, wraps the selected text.
    pub fn bracket(&self, open: char, close: char) -> EditResult {
        if self.has_selection() {
            let new_value = format!(
                "{}{open}{}{close}{}",
                self.before(),
                self.selected(),
                self.after_selection()
            );
            EditResult::new(new_value, self.start + 1, self.end + 1)
        } else {
            let new_value = format!("{}{open}{close}{}", self.before(), self.after_cursor());
            EditResult::new(new_value, self.start + 1, self.start + 1)
        }
    }

    /// Skip over a closing bracket if it's already at the cursor.
    pub fn skip_close(&self, expected: char) -> Option<EditResult> {
        if !self.has_selection() && self.char_at_cursor() == Some(expected) {
            Some(EditResult::new(
                self.value.clone(),
                self.start + 1,
                self.start + 1,
            ))
        } else {
            None
        }
    }

    /// Auto-close or skip a formatting character (`*`, `_`, `` ` ``, `~`).
    pub fn formatting_char(&self, ch: char) -> EditResult {
        if self.has_selection() {
            let new_value = format!(
                "{}{ch}{}{ch}{}",
                self.before(),
                self.selected(),
                self.after_selection()
            );
            EditResult::new(new_value, self.start + 1, self.end + 1)
        } else if self.char_at_cursor() == Some(ch) {
            EditResult::new(self.value.clone(), self.start + 1, self.start + 1)
        } else {
            let new_value = format!("{}{ch}{ch}{}", self.before(), self.after_cursor());
            EditResult::new(new_value, self.start + 1, self.start + 1)
        }
    }

    /// Handle Enter: continue list/quote prefixes or end empty ones.
    /// Returns `None` when the browser should handle Enter normally.
    pub fn enter(&self) -> Option<EditResult> {
        if self.has_selection() {
            return None;
        }
        let (line_text, line_start_char) = self.current_line();

        if is_empty_prefix(line_text) {
            let before_line: String = self.chars[..line_start_char].iter().collect();
            let after = self.after_cursor();
            let new_value = format!("{before_line}\n{after}");
            let new_cursor = line_start_char + 1;
            Some(EditResult::new(new_value, new_cursor, new_cursor))
        } else if let Some(prefix) = detect_prefix(line_text) {
            let cont = prefix.continuation();
            let before = self.before();
            let after = self.after_cursor();
            let new_value = format!("{before}\n{cont}{after}");
            let cont_chars = cont.chars().count();
            let new_cursor = self.start + 1 + cont_chars;
            Some(EditResult::new(new_value, new_cursor, new_cursor))
        } else {
            None
        }
    }

    /// Handle Tab/Shift+Tab: indent or outdent list items.
    /// Returns `None` when the line is not a list item.
    pub fn tab(&self, shift: bool) -> Option<EditResult> {
        let (line_text, line_start_char) = self.current_line();
        detect_prefix(line_text)?;

        let before_line: String = self.chars[..line_start_char].iter().collect();
        let after = self.after_cursor();

        if shift {
            let stripped = line_text
                .strip_prefix("  ")
                .unwrap_or_else(|| line_text.trim_start_matches(' '));
            let removed = line_text.chars().count() - stripped.chars().count();
            let new_value = format!("{before_line}{stripped}{after}");
            let new_cursor = self.start.saturating_sub(removed);
            Some(EditResult::new(new_value, new_cursor, new_cursor))
        } else {
            let new_value = format!("{before_line}  {line_text}{after}");
            let new_cursor = self.start + 2;
            Some(EditResult::new(new_value, new_cursor, new_cursor))
        }
    }

    /// Wrap a URL as a markdown link.
    /// With selection: `[selected](url)`. Without: `[url](url)`.
    pub fn paste_url(&self, url: &str) -> EditResult {
        let display = if self.has_selection() {
            self.selected()
        } else {
            url.to_string()
        };
        let new_value = format!(
            "{}[{}]({}){}",
            self.before(),
            display,
            url,
            self.after_selection()
        );
        let display_chars = display.chars().count();
        let url_chars = url.chars().count();
        let new_cursor = self.start + 1 + display_chars + 2 + url_chars + 1;
        EditResult::new(new_value, new_cursor, new_cursor)
    }
}

// ── EditResult ───────────────────────────────────────────────────

/// The result of a text editing operation: new text and cursor position.
pub struct EditResult {
    pub value: String,
    /// Cursor start as a char offset.
    pub cursor_start: usize,
    /// Cursor end as a char offset.
    pub cursor_end: usize,
}

impl EditResult {
    fn new(value: String, cursor_start: usize, cursor_end: usize) -> Self {
        Self {
            value,
            cursor_start,
            cursor_end,
        }
    }

    /// Cursor start as a UTF-16 offset for the JS DOM.
    pub fn cursor_start_utf16(&self) -> u32 {
        char_to_utf16_offset(&self.value, self.cursor_start) as u32
    }

    /// Cursor end as a UTF-16 offset for the JS DOM.
    pub fn cursor_end_utf16(&self) -> u32 {
        char_to_utf16_offset(&self.value, self.cursor_end) as u32
    }
}

// ── Line prefix detection ────────────────────────────────────────

/// A recognised line prefix for continuation.
pub enum LinePrefix {
    Bullet(String),
    Checkbox(String),
    Numbered {
        indent: String,
        next_num: u32,
        suffix: String,
    },
    Blockquote(String),
}

impl LinePrefix {
    /// The string to insert on the next line.
    pub fn continuation(&self) -> String {
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

/// Try to detect a list/quote prefix on the given line text.
pub fn detect_prefix(line: &str) -> Option<LinePrefix> {
    let indent_len = line.len() - line.trim_start().len();
    let indent = &line[..indent_len];
    let trimmed = line.trim_start();

    if trimmed.starts_with("- [ ] ")
        || trimmed.starts_with("- [x] ")
        || trimmed.starts_with("- [X] ")
    {
        return Some(LinePrefix::Checkbox(format!("{indent}- [ ] ")));
    }

    if trimmed.starts_with("- ") {
        return Some(LinePrefix::Bullet(format!("{indent}- ")));
    }

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

    if trimmed.starts_with("> ") {
        return Some(LinePrefix::Blockquote(format!("{indent}> ")));
    }

    None
}

/// Returns `true` if the line is an empty continuation prefix with no content.
pub fn is_empty_prefix(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed == "-" || trimmed == "- [ ]" || trimmed == "- [x]" || trimmed == "- [X]" {
        return true;
    }
    if let Some(num_end) = trimmed.find(|c: char| !c.is_ascii_digit()) {
        if num_end > 0 {
            let rest = trimmed[num_end..].trim();
            if rest == "." || rest == ")" {
                return true;
            }
        }
    }
    if trimmed == ">" {
        return true;
    }
    false
}

/// Returns `true` if `text` looks like a URL.
pub fn is_url(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with("http://") || trimmed.starts_with("https://")
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    // ── UTF-16 conversion ──

    #[wasm_bindgen_test]
    fn utf16_offset_ascii() {
        assert_eq!(utf16_to_char_offset("hello", 3), 3);
        assert_eq!(char_to_utf16_offset("hello", 3), 3);
    }

    #[wasm_bindgen_test]
    fn utf16_offset_emoji() {
        // 🎉 is U+1F389: 4 bytes UTF-8, 2 code units UTF-16, 1 char
        let s = "a🎉b";
        assert_eq!(utf16_to_char_offset(s, 0), 0);
        assert_eq!(utf16_to_char_offset(s, 1), 1); // after 'a'
        assert_eq!(utf16_to_char_offset(s, 3), 2); // after 🎉
        assert_eq!(utf16_to_char_offset(s, 4), 3); // after 'b'

        assert_eq!(char_to_utf16_offset(s, 0), 0);
        assert_eq!(char_to_utf16_offset(s, 1), 1);
        assert_eq!(char_to_utf16_offset(s, 2), 3);
        assert_eq!(char_to_utf16_offset(s, 3), 4);
    }

    #[wasm_bindgen_test]
    fn utf16_offset_cjk() {
        // CJK chars are BMP: 3 bytes UTF-8, 1 code unit UTF-16, 1 char
        let s = "你好世界";
        assert_eq!(utf16_to_char_offset(s, 2), 2);
        assert_eq!(char_to_utf16_offset(s, 2), 2);
    }

    #[wasm_bindgen_test]
    fn utf16_offset_mixed() {
        // "hi🎉你" = h(1) i(1) 🎉(2) 你(1) = 5 UTF-16 units, 4 chars
        let s = "hi🎉你";
        assert_eq!(utf16_to_char_offset(s, 4), 3);
        assert_eq!(char_to_utf16_offset(s, 3), 4);
        assert_eq!(utf16_to_char_offset(s, 5), 4);
        assert_eq!(char_to_utf16_offset(s, 4), 5);
    }

    // ── TextareaState basics ──

    #[wasm_bindgen_test]
    fn state_before_after_ascii() {
        let s = TextareaState::new("hello world".into(), 5, 5);
        assert_eq!(s.before(), "hello");
        assert_eq!(s.after_cursor(), " world");
        assert!(!s.has_selection());
    }

    #[wasm_bindgen_test]
    fn state_selection_with_emoji() {
        let s = TextareaState::new("ab🎉cd".into(), 2, 3);
        assert!(s.has_selection());
        assert_eq!(s.before(), "ab");
        assert_eq!(s.selected(), "🎉");
        assert_eq!(s.after_selection(), "cd");
    }

    #[wasm_bindgen_test]
    fn state_from_utf16_with_emoji() {
        // cursor after emoji: UTF-16 offset 3, char offset 2
        let s = TextareaState::from_utf16("a🎉b".into(), 3, 3);
        assert_eq!(s.before(), "a🎉");
        assert_eq!(s.after_cursor(), "b");
    }

    #[wasm_bindgen_test]
    fn state_clamps_out_of_bounds() {
        let s = TextareaState::new("hi".into(), 100, 200);
        assert_eq!(s.start, 2);
        assert_eq!(s.end, 2);
    }

    // ── current_line ──

    #[wasm_bindgen_test]
    fn current_line_first_line() {
        let s = TextareaState::new("hello world".into(), 5, 5);
        let (line, start) = s.current_line();
        assert_eq!(line, "hello");
        assert_eq!(start, 0);
    }

    #[wasm_bindgen_test]
    fn current_line_second_line() {
        let s = TextareaState::new("first\nsecond".into(), 9, 9);
        let (line, start) = s.current_line();
        assert_eq!(line, "sec");
        assert_eq!(start, 6);
    }

    #[wasm_bindgen_test]
    fn current_line_with_emoji() {
        // "🎉\nhello" — cursor at char 4 (first 'l')
        // current_line returns text from line start up to cursor
        let s = TextareaState::new("🎉\nhello".into(), 4, 4);
        let (line, start) = s.current_line();
        assert_eq!(line, "he");
        assert_eq!(start, 2);
    }

    // ── Edit operations ──

    #[wasm_bindgen_test]
    fn bracket_no_selection() {
        let s = TextareaState::new("hello".into(), 2, 2);
        let r = s.bracket('[', ']');
        assert_eq!(r.value, "he[]llo");
        assert_eq!(r.cursor_start, 3);
        assert_eq!(r.cursor_end, 3);
    }

    #[wasm_bindgen_test]
    fn bracket_with_selection() {
        let s = TextareaState::new("hello".into(), 1, 4);
        let r = s.bracket('[', ']');
        assert_eq!(r.value, "h[ell]o");
        assert_eq!(r.cursor_start, 2);
        assert_eq!(r.cursor_end, 5);
    }

    #[wasm_bindgen_test]
    fn bracket_with_emoji_utf16_conversion() {
        // "a🎉b" — cursor after emoji (char 2)
        let s = TextareaState::new("a🎉b".into(), 2, 2);
        let r = s.bracket('(', ')');
        assert_eq!(r.value, "a🎉()b");
        assert_eq!(r.cursor_start, 3);
        // UTF-16: a(1) 🎉(2) ((1) = 4
        assert_eq!(r.cursor_start_utf16(), 4);
    }

    #[wasm_bindgen_test]
    fn skip_close_matches() {
        let s = TextareaState::new("he]llo".into(), 2, 2);
        let r = s.skip_close(']').unwrap();
        assert_eq!(r.cursor_start, 3);
    }

    #[wasm_bindgen_test]
    fn skip_close_no_match() {
        let s = TextareaState::new("hello".into(), 2, 2);
        assert!(s.skip_close(']').is_none());
    }

    #[wasm_bindgen_test]
    fn formatting_wrap_selection() {
        let s = TextareaState::new("hello world".into(), 6, 11);
        let r = s.formatting_char('*');
        assert_eq!(r.value, "hello *world*");
        assert_eq!(r.cursor_start, 7);
        assert_eq!(r.cursor_end, 12);
    }

    #[wasm_bindgen_test]
    fn formatting_insert_pair() {
        let s = TextareaState::new("hello".into(), 5, 5);
        let r = s.formatting_char('`');
        assert_eq!(r.value, "hello``");
        assert_eq!(r.cursor_start, 6);
    }

    #[wasm_bindgen_test]
    fn formatting_skip_over() {
        let s = TextareaState::new("he*llo".into(), 2, 2);
        let r = s.formatting_char('*');
        assert_eq!(r.value, "he*llo");
        assert_eq!(r.cursor_start, 3);
    }

    // ── Enter ──

    #[wasm_bindgen_test]
    fn enter_continues_bullet() {
        let s = TextareaState::new("- hello".into(), 7, 7);
        let r = s.enter().unwrap();
        assert_eq!(r.value, "- hello\n- ");
        assert_eq!(r.cursor_start, 10);
    }

    #[wasm_bindgen_test]
    fn enter_continues_checkbox() {
        let s = TextareaState::new("- [ ] task".into(), 10, 10);
        let r = s.enter().unwrap();
        assert_eq!(r.value, "- [ ] task\n- [ ] ");
        assert_eq!(r.cursor_start, 17);
    }

    #[wasm_bindgen_test]
    fn enter_continues_numbered() {
        let s = TextareaState::new("1. first".into(), 8, 8);
        let r = s.enter().unwrap();
        assert_eq!(r.value, "1. first\n2. ");
        assert_eq!(r.cursor_start, 12);
    }

    #[wasm_bindgen_test]
    fn enter_continues_blockquote() {
        let s = TextareaState::new("> quoted".into(), 8, 8);
        let r = s.enter().unwrap();
        assert_eq!(r.value, "> quoted\n> ");
        assert_eq!(r.cursor_start, 11);
    }

    #[wasm_bindgen_test]
    fn enter_removes_empty_bullet() {
        let s = TextareaState::new("- ".into(), 2, 2);
        let r = s.enter().unwrap();
        assert_eq!(r.value, "\n");
        assert_eq!(r.cursor_start, 1);
    }

    #[wasm_bindgen_test]
    fn enter_removes_empty_bullet_after_content() {
        let s = TextareaState::new("hello\n- ".into(), 8, 8);
        let r = s.enter().unwrap();
        assert_eq!(r.value, "hello\n\n");
        assert_eq!(r.cursor_start, 7);
    }

    #[wasm_bindgen_test]
    fn enter_no_prefix() {
        let s = TextareaState::new("plain text".into(), 5, 5);
        assert!(s.enter().is_none());
    }

    #[wasm_bindgen_test]
    fn enter_with_selection() {
        let s = TextareaState::new("- hello".into(), 2, 5);
        assert!(s.enter().is_none());
    }

    #[wasm_bindgen_test]
    fn enter_bullet_with_emoji() {
        let s = TextareaState::new("- 🎉hello".into(), 3, 3);
        let r = s.enter().unwrap();
        assert_eq!(r.value, "- 🎉\n- hello");
        assert_eq!(r.cursor_start, 6);
        // UTF-16: "- 🎉\n- " = -(1) (1) 🎉(2) \n(1) -(1) (1) = 7
        assert_eq!(r.cursor_start_utf16(), 7);
    }

    // ── Tab ──

    #[wasm_bindgen_test]
    fn tab_indent_bullet() {
        let s = TextareaState::new("- hello".into(), 7, 7);
        let r = s.tab(false).unwrap();
        assert_eq!(r.value, "  - hello");
        assert_eq!(r.cursor_start, 9);
    }

    #[wasm_bindgen_test]
    fn tab_outdent_bullet() {
        let s = TextareaState::new("  - hello".into(), 9, 9);
        let r = s.tab(true).unwrap();
        assert_eq!(r.value, "- hello");
        assert_eq!(r.cursor_start, 7);
    }

    #[wasm_bindgen_test]
    fn tab_non_list_line() {
        let s = TextareaState::new("plain text".into(), 5, 5);
        assert!(s.tab(false).is_none());
    }

    // ── Paste URL ──

    #[wasm_bindgen_test]
    fn paste_url_no_selection() {
        let s = TextareaState::new("text ".into(), 5, 5);
        let r = s.paste_url("https://example.com");
        assert_eq!(r.value, "text [https://example.com](https://example.com)");
    }

    #[wasm_bindgen_test]
    fn paste_url_with_selection() {
        let s = TextareaState::new("click here please".into(), 6, 10);
        let r = s.paste_url("https://example.com");
        assert_eq!(r.value, "click [here](https://example.com) please");
    }

    #[wasm_bindgen_test]
    fn paste_url_with_emoji_selection() {
        let s = TextareaState::new("go 🎉 now".into(), 3, 4);
        let r = s.paste_url("https://x.com");
        assert_eq!(r.value, "go [🎉](https://x.com) now");
    }

    // ── detect_prefix ──

    #[wasm_bindgen_test]
    fn detect_prefix_bullet() {
        let p = detect_prefix("  - hello").unwrap();
        assert_eq!(p.continuation(), "  - ");
    }

    #[wasm_bindgen_test]
    fn detect_prefix_checkbox() {
        let p = detect_prefix("- [ ] task").unwrap();
        assert_eq!(p.continuation(), "- [ ] ");
    }

    #[wasm_bindgen_test]
    fn detect_prefix_numbered() {
        let p = detect_prefix("  3. item").unwrap();
        assert_eq!(p.continuation(), "  4. ");
    }

    #[wasm_bindgen_test]
    fn detect_prefix_blockquote() {
        let p = detect_prefix("> text").unwrap();
        assert_eq!(p.continuation(), "> ");
    }

    #[wasm_bindgen_test]
    fn detect_prefix_none() {
        assert!(detect_prefix("plain text").is_none());
        assert!(detect_prefix("").is_none());
    }

    // ── is_empty_prefix ──

    #[wasm_bindgen_test]
    fn empty_prefix_bullet() {
        assert!(is_empty_prefix("-"));
        assert!(is_empty_prefix("  -"));
    }

    #[wasm_bindgen_test]
    fn empty_prefix_numbered() {
        assert!(is_empty_prefix("1."));
        assert!(is_empty_prefix("  1."));
    }

    #[wasm_bindgen_test]
    fn empty_prefix_blockquote() {
        assert!(is_empty_prefix(">"));
    }

    #[wasm_bindgen_test]
    fn not_empty_prefix() {
        assert!(!is_empty_prefix("- hello"));
        assert!(!is_empty_prefix("plain"));
        assert!(!is_empty_prefix(""));
    }

    // ── is_url ──

    #[wasm_bindgen_test]
    fn url_detection() {
        assert!(is_url("https://example.com"));
        assert!(is_url("http://example.com"));
        assert!(is_url("  https://example.com  "));
        assert!(!is_url("not a url"));
        assert!(!is_url("ftp://example.com"));
    }
}
