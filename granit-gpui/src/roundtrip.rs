//! Lossless round-trip check for gate G2 (Bears task `baa.6x`).
//!
//! Every note body of a cave (and every template body) is loaded into a real
//! [`EditorState`] with `set_text`, painted once, and read back with `text()`.
//! The result is compared byte for byte with the body read from disk; each
//! mismatch is listed with its slug, the first differing byte offset and a
//! short excerpt of both sides.
//!
//! An optional save pass then writes every note back through
//! `Cave::save_note` — either the body exactly as read (the backend's own
//! behaviour) or the editor's read-back text (what the first real edit would
//! carry) — and classifies each changed file into frontmatter and body
//! changes.
//!
//! Run it as the hidden CLI mode
//! `granit-gpui --roundtrip <cave> [--save body|editor] [--out <file>]`.
//! The mode needs a GPUI window for `EditorState::new` and for painting, so it
//! opens one hidden window, runs the pass and exits (status 1 when anything
//! mismatched or failed).

use std::{collections::BTreeMap, ffi::OsString, fmt::Write as _, path::PathBuf};

use gpui::{
    px, size, AnyWindowHandle, App, AppContext, Bounds, WindowBounds, WindowHandle, WindowOptions,
};
use granit_lib::{
    cave::{Cave, CaveError},
    markdown::Markdown,
};
use zorite_editor::EditorState;

/// Bytes of context shown before a differing offset in an excerpt.
const EXCERPT_BEFORE: usize = 16;
/// Bytes of context shown from a differing offset on in an excerpt.
const EXCERPT_AFTER: usize = 32;

const USAGE: &str =
    "Usage: granit-gpui --roundtrip <cave> [--save body|editor] [--out <report-file>]";

#[derive(Debug, thiserror::Error)]
pub enum RoundTripError {
    #[error("{0}")]
    Usage(String),
    #[error(transparent)]
    Cave(#[from] CaveError),
    #[error("could not write the report: {0}")]
    Report(#[from] std::io::Error),
}

/// One document whose read-back text differs from its input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    /// The note slug, or `template:<slug>` for a template.
    pub name: String,
    /// First differing byte offset into the body.
    pub offset: usize,
    pub input_len: usize,
    pub output_len: usize,
    /// Escaped bytes around `offset` in the input and in the output.
    pub input_excerpt: String,
    pub output_excerpt: String,
}

/// Result of the editor round-trip over a cave.
#[derive(Debug, Default)]
pub struct RoundTripReport {
    pub notes: usize,
    pub templates: usize,
    pub bytes: usize,
    pub mismatches: Vec<Mismatch>,
    /// Documents that could not be read: `(name, error)`.
    pub errors: Vec<(String, String)>,
}

impl RoundTripReport {
    pub fn passed(&self) -> bool {
        self.mismatches.is_empty() && self.errors.is_empty()
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "Editor round-trip (set_text -> paint -> text()): {} notes, {} templates, {} body bytes",
            self.notes, self.templates, self.bytes
        );
        let _ = writeln!(
            out,
            "Identical: {}  Mismatched: {}  Unreadable: {}",
            self.notes + self.templates - self.mismatches.len() - self.errors.len(),
            self.mismatches.len(),
            self.errors.len()
        );
        for m in &self.mismatches {
            let _ = writeln!(
                out,
                "MISMATCH {}: first difference at byte {} (input {} bytes, output {} bytes)\n  input:  {}\n  output: {}",
                m.name, m.offset, m.input_len, m.output_len, m.input_excerpt, m.output_excerpt
            );
        }
        for (name, err) in &self.errors {
            let _ = writeln!(out, "UNREADABLE {name}: {err}");
        }
        out
    }
}

/// The first byte offset at which `a` and `b` differ, or `None` if equal.
/// A strict prefix differs at the shorter one's length.
pub fn first_difference(a: &str, b: &str) -> Option<usize> {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    match a.iter().zip(b).position(|(x, y)| x != y) {
        Some(i) => Some(i),
        None if a.len() != b.len() => Some(a.len().min(b.len())),
        None => None,
    }
}

/// Largest char boundary of `s` at or below `i`.
fn floor_boundary(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Escaped text of `s` around byte `at`, with `|` marking `at`.
pub fn excerpt(s: &str, at: usize) -> String {
    let at = floor_boundary(s, at);
    let start = floor_boundary(s, at.saturating_sub(EXCERPT_BEFORE));
    let end = floor_boundary(s, at + EXCERPT_AFTER);
    format!(
        "\"{}|{}\"",
        s[start..at].escape_debug(),
        s[at..end].escape_debug()
    )
}

fn mismatch(name: &str, input: &str, output: &str) -> Option<Mismatch> {
    let offset = first_difference(input, output)?;
    Some(Mismatch {
        name: name.to_string(),
        offset,
        input_len: input.len(),
        output_len: output.len(),
        input_excerpt: excerpt(input, offset),
        output_excerpt: excerpt(output, offset),
    })
}

/// Run every note body and template body of `cave` through `read_back` and
/// compare the result with the input.
pub fn check_cave(
    cave: &Cave,
    mut read_back: impl FnMut(&str) -> String,
) -> Result<RoundTripReport, CaveError> {
    let mut report = RoundTripReport::default();
    let mut notes = cave.list_notes()?;
    notes.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    for meta in notes {
        report.notes += 1;
        match cave.read_note(&meta.slug) {
            Ok(doc) => {
                report.bytes += doc.content.len();
                let output = read_back(&doc.content);
                report
                    .mismatches
                    .extend(mismatch(&meta.slug, &doc.content, &output));
            }
            Err(err) => report.errors.push((meta.slug, err.to_string())),
        }
    }
    for meta in cave.list_templates()? {
        report.templates += 1;
        let name = format!("template:{}", meta.slug);
        match cave.read_template(&meta.slug) {
            Ok(doc) => {
                report.bytes += doc.content.len();
                let output = read_back(&doc.content);
                report
                    .mismatches
                    .extend(mismatch(&name, &doc.content, &output));
            }
            Err(err) => report.errors.push((name, err.to_string())),
        }
    }
    Ok(report)
}

/// Which text the save pass writes for each note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveText {
    /// The body exactly as `read_note` returned it: measures `save_note`.
    Body,
    /// The editor's read-back of that body: what a first edit would carry.
    Editor,
}

/// How one file changed in the save pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub slug: String,
    /// Frontmatter change kinds, e.g. `rewritten: modified_at`.
    pub frontmatter: Vec<String>,
    /// First differing byte offset of the body, if the body changed.
    pub body_offset: Option<usize>,
    pub body_excerpts: Option<(String, String)>,
}

/// Result of the save pass.
#[derive(Debug, Default)]
pub struct SaveReport {
    pub saved: usize,
    pub identical: usize,
    pub changes: Vec<FileChange>,
    pub errors: Vec<(String, String)>,
}

impl SaveReport {
    pub fn render(&self, mode: SaveText) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "Save pass ({mode:?}): {} notes saved through Cave::save_note, {} byte-identical, {} changed, {} failed",
            self.saved,
            self.identical,
            self.changes.len(),
            self.errors.len()
        );
        let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
        for change in &self.changes {
            for kind in &change.frontmatter {
                *kinds.entry(kind).or_default() += 1;
            }
            if change.body_offset.is_some() {
                *kinds.entry("body changed").or_default() += 1;
            }
        }
        for (kind, count) in &kinds {
            let _ = writeln!(out, "  {count:>4} x {kind}");
        }
        for change in &self.changes {
            let _ = write!(out, "CHANGED {}:", change.slug);
            if !change.frontmatter.is_empty() {
                let _ = write!(out, " frontmatter [{}]", change.frontmatter.join(", "));
            }
            if let (Some(offset), Some((before, after))) =
                (change.body_offset, &change.body_excerpts)
            {
                let _ = write!(
                    out,
                    " body at byte {offset}\n  before: {before}\n  after:  {after}"
                );
            }
            let _ = writeln!(out);
        }
        for (slug, err) in &self.errors {
            let _ = writeln!(out, "FAILED {slug}: {err}");
        }
        out
    }
}

/// Split a raw note into its frontmatter block (fences included, empty when
/// absent) and its body, the way `Cave::read_note` splits it.
fn split_raw(raw: &str) -> (&str, &str) {
    let body = Markdown::new(raw).body();
    (&raw[..raw.len() - body.len()], body)
}

/// Top-level frontmatter entries: key -> the lines belonging to it. Comment
/// lines are keyed `#comment`; the fence lines are skipped.
fn frontmatter_entries(block: &str) -> BTreeMap<String, Vec<String>> {
    let mut entries: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in block.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        if bare == "---" {
            continue;
        }
        let top_level = !bare.starts_with([' ', '\t', '-']);
        if bare.starts_with('#') {
            current = Some("#comment".to_string());
        } else if top_level {
            if let Some((key, _)) = bare.split_once(':') {
                current = Some(key.trim().to_string());
            }
        }
        let key = current.clone().unwrap_or_else(|| "(no key)".to_string());
        entries.entry(key).or_default().push(line.to_string());
    }
    entries
}

/// Describe how the frontmatter block changed, one kind per changed key.
fn frontmatter_changes(before: &str, after: &str) -> Vec<String> {
    if before == after {
        return Vec::new();
    }
    if after.is_empty() {
        return vec!["frontmatter removed".to_string()];
    }
    if before.is_empty() {
        return vec!["frontmatter added".to_string()];
    }
    let (old, new) = (frontmatter_entries(before), frontmatter_entries(after));
    let mut kinds = Vec::new();
    for (key, lines) in &old {
        match new.get(key) {
            None => kinds.push(format!("dropped: {key}")),
            Some(other) if other != lines => kinds.push(format!("rewritten: {key}")),
            Some(_) => {}
        }
    }
    for key in new.keys().filter(|key| !old.contains_key(*key)) {
        kinds.push(format!("added: {key}"));
    }
    let order = |block: &str| -> Vec<String> {
        frontmatter_entries(block)
            .into_keys()
            .filter(|key| key != "#comment")
            .collect()
    };
    if kinds.is_empty() && order(before) != order(after) {
        kinds.push("keys reordered".to_string());
    }
    if before.contains('\r') != after.contains('\r') {
        kinds.push("frontmatter line endings changed".to_string());
    }
    if kinds.is_empty() {
        kinds.push("frontmatter reformatted".to_string());
    }
    kinds
}

/// Compare a note's raw file before and after a save.
pub fn classify_change(slug: &str, before: &str, after: &str) -> Option<FileChange> {
    if before == after {
        return None;
    }
    let (fm_before, body_before) = split_raw(before);
    let (fm_after, body_after) = split_raw(after);
    let body_offset = first_difference(body_before, body_after);
    Some(FileChange {
        slug: slug.to_string(),
        frontmatter: frontmatter_changes(fm_before, fm_after),
        body_offset,
        body_excerpts: body_offset
            .map(|offset| (excerpt(body_before, offset), excerpt(body_after, offset))),
    })
}

/// Save every note of `cave` once, unedited, through `Cave::save_note` and
/// record how each file changed. Writes to the cave: run it on a copy.
pub fn save_every_note(
    cave: &mut Cave,
    mode: SaveText,
    mut read_back: impl FnMut(&str) -> String,
) -> Result<SaveReport, CaveError> {
    let mut report = SaveReport::default();
    let mut notes = cave.list_notes()?;
    notes.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    for meta in notes {
        let slug = meta.slug;
        let result = (|| -> Result<Option<FileChange>, CaveError> {
            let before = cave.read_note_raw(&slug)?;
            let body = cave.read_note(&slug)?.content;
            let text = match mode {
                SaveText::Body => body,
                SaveText::Editor => read_back(&body),
            };
            cave.save_note(&slug, &text)?;
            let after = cave.read_note_raw(&slug)?;
            Ok(classify_change(&slug, &before, &after))
        })();
        match result {
            Ok(None) => {
                report.saved += 1;
                report.identical += 1;
            }
            Ok(Some(change)) => {
                report.saved += 1;
                report.changes.push(change);
            }
            Err(err) => report.errors.push((slug, err.to_string())),
        }
    }
    Ok(report)
}

/// Open a window whose root is an editor styled like the workspace's. GPUI's
/// window-opening error is returned as text.
pub fn open_probe_window(
    options: WindowOptions,
    cx: &mut App,
) -> Result<WindowHandle<EditorState>, String> {
    cx.open_window(options, |window, cx| {
        crate::theme::install(window, cx);
        let editor = cx.new(|cx| EditorState::new(window, cx));
        let highlighter = crate::note_editor::CodeHighlighter::new(
            gpui_kit::component::ActiveTheme::theme(cx)
                .highlight_theme
                .clone(),
        );
        crate::Workspace::apply_theme(&editor, &highlighter, cx);
        editor
    })
    .map_err(|err| err.to_string())
}

/// Load `body` into the window's editor, paint the window once, and return
/// the editor's text. Call outside any update of the window's root.
pub fn load_and_read_back(window: WindowHandle<EditorState>, body: &str, cx: &mut App) -> String {
    let _ = window.update(cx, |editor, _, cx| editor.set_text(body.to_string(), cx));
    let handle: AnyWindowHandle = window.into();
    let _ = cx.update_window(handle, |_, window, cx| {
        let _ = window.draw(cx);
    });
    window
        .update(cx, |editor, _, _| editor.text().to_string())
        .unwrap_or_default()
}

/// Whether the last paint laid out every logical line of the editor's
/// current text, i.e. the text really went through layout and paint.
pub fn painted_current_text(window: WindowHandle<EditorState>, cx: &mut App) -> bool {
    window
        .update(cx, |editor, _, _| {
            editor.row_layout().len() == editor.text().split('\n').count()
        })
        .unwrap_or(false)
}

struct CliArgs {
    cave: PathBuf,
    save: Option<SaveText>,
    out: Option<PathBuf>,
}

fn parse_args(args: &[OsString]) -> Result<CliArgs, RoundTripError> {
    let usage = || RoundTripError::Usage(USAGE.to_string());
    let mut iter = args.iter();
    let cave = iter.next().map(PathBuf::from).ok_or_else(usage)?;
    let mut save = None;
    let mut out = None;
    while let Some(flag) = iter.next() {
        match flag.to_str() {
            Some("--save") => {
                save = Some(match iter.next().and_then(|v| v.to_str()) {
                    Some("body") => SaveText::Body,
                    Some("editor") => SaveText::Editor,
                    _ => return Err(usage()),
                });
            }
            Some("--out") => out = Some(iter.next().map(PathBuf::from).ok_or_else(usage)?),
            _ => return Err(usage()),
        }
    }
    Ok(CliArgs { cave, save, out })
}

/// Run the passes with a probe window's editor and return the report text
/// and whether everything round-tripped.
fn run_passes(
    args: &CliArgs,
    window: WindowHandle<EditorState>,
    cx: &mut App,
) -> Result<(String, bool), RoundTripError> {
    let mut cave = Cave::open(args.cave.clone())?;
    let started = std::time::Instant::now();
    let mut painted = 0;
    let report = check_cave(&cave, |body| {
        let text = load_and_read_back(window, body, cx);
        painted += usize::from(painted_current_text(window, cx));
        text
    })?;
    let mut text = format!("Cave: {}\n", args.cave.display());
    text.push_str(&report.render());
    let _ = writeln!(
        text,
        "Painted with every line laid out: {painted} of {} documents. Editor pass took {:?}",
        report.notes + report.templates - report.errors.len(),
        started.elapsed()
    );
    let mut ok = report.passed();
    if let Some(mode) = args.save {
        let save = save_every_note(&mut cave, mode, |body| load_and_read_back(window, body, cx))?;
        text.push('\n');
        text.push_str(&save.render(mode));
        ok &= save.changes.is_empty() && save.errors.is_empty();
    }
    if let Some(out) = &args.out {
        std::fs::write(out, &text)?;
    }
    Ok((text, ok))
}

/// Entry point of the hidden `--roundtrip` mode; `args` follow the flag.
/// Never returns: exits 0 when everything round-tripped, 1 otherwise, 2 on
/// a usage or I/O error.
pub fn run_cli(args: Vec<OsString>) -> ! {
    let args = match parse_args(&args) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    };
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(820.), px(720.)),
                    cx,
                ))),
                show: false,
                focus: false,
                ..Default::default()
            };
            let code = match open_probe_window(options, cx) {
                Ok(window) => match run_passes(&args, window, cx) {
                    Ok((text, ok)) => {
                        print!("{text}");
                        i32::from(!ok)
                    }
                    Err(err) => {
                        eprintln!("granit-gpui --roundtrip: {err}");
                        2
                    }
                },
                Err(err) => {
                    eprintln!("granit-gpui --roundtrip: could not open a window: {err}");
                    2
                }
            };
            std::process::exit(code);
        });
    std::process::exit(2);
}

/// The smallest byte span that differs between `before` and `after`:
/// `(offset, removed, inserted)`. `None` when they are equal. Used by the
/// single-edit tests.
#[cfg(test)]
fn changed_span<'a>(before: &'a str, after: &'a str) -> Option<(usize, &'a str, &'a str)> {
    let start = floor_boundary(before, first_difference(before, after)?);
    let (b, a) = (&before[start..], &after[start..]);
    let mut suffix = b
        .bytes()
        .rev()
        .zip(a.bytes().rev())
        .take_while(|(x, y)| x == y)
        .count()
        .min(b.len())
        .min(a.len());
    while !b.is_char_boundary(b.len() - suffix) || !a.is_char_boundary(a.len() - suffix) {
        suffix -= 1;
    }
    Some((start, &b[..b.len() - suffix], &a[..a.len() - suffix]))
}

/// Drivers for single structured edits through the editor's real input
/// paths (typed input, bound keys, mouse clicks on painted controls), and
/// the tests that record what each edit writes.
#[cfg(test)]
mod tests {
    use gpui::{point, Modifiers, Point, TestAppContext, VisualTestContext};

    use super::*;

    /// A tall window, so every row of a long note is painted and hittable.
    fn open(cx: &mut TestAppContext) -> WindowHandle<EditorState> {
        cx.update(|cx| {
            gpui_kit::init(cx);
            zorite_editor::bind_keys(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    Point::default(),
                    size(px(820.), px(6000.)),
                ))),
                ..Default::default()
            };
            open_probe_window(options, cx).unwrap()
        })
    }

    fn load(window: WindowHandle<EditorState>, body: &str, cx: &mut TestAppContext) -> String {
        cx.update(|cx| load_and_read_back(window, body, cx))
    }

    fn draw(window: WindowHandle<EditorState>, cx: &mut TestAppContext) {
        cx.update_window(window.into(), |_, window, cx| {
            let _ = window.draw(cx);
        })
        .unwrap();
    }

    fn text(window: WindowHandle<EditorState>, cx: &mut TestAppContext) -> String {
        window
            .update(cx, |editor, _, _| editor.text().to_string())
            .unwrap()
    }

    /// Byte offset of `needle` in `body` plus `at`.
    fn offset_of(body: &str, needle: &str, at: usize) -> usize {
        body.find(needle)
            .unwrap_or_else(|| panic!("{needle:?} not in the body"))
            + at
    }

    /// Load `body`, put the caret at `offset`, focus the editor and paint, so
    /// the focused editor's input handler is registered.
    fn place_caret(
        window: WindowHandle<EditorState>,
        body: &str,
        offset: usize,
        cx: &mut TestAppContext,
    ) {
        load(window, body, cx);
        window
            .update(cx, |editor, window, cx| {
                editor.set_cursor(offset, cx);
                editor.focus(window, cx);
            })
            .unwrap();
        cx.run_until_parked();
        draw(window, cx);
    }

    /// Type `input` as key presses at `offset`.
    pub(super) fn type_at(
        window: WindowHandle<EditorState>,
        body: &str,
        offset: usize,
        input: &str,
        cx: &mut TestAppContext,
    ) -> String {
        place_caret(window, body, offset, cx);
        VisualTestContext::from_window(window.into(), cx).simulate_input(input);
        text(window, cx)
    }

    /// Press bound keys (e.g. `"enter"`) at `offset`.
    pub(super) fn keys_at(
        window: WindowHandle<EditorState>,
        body: &str,
        offset: usize,
        keys: &str,
        cx: &mut TestAppContext,
    ) -> String {
        place_caret(window, body, offset, cx);
        VisualTestContext::from_window(window.into(), cx).simulate_keystrokes(keys);
        text(window, cx)
    }

    /// Click along the painted row of the line starting at byte `line_start`,
    /// left to right in 2px steps from the editor's left edge, reloading the
    /// body before each click, until a click changes the text. Finds a
    /// painted control (task checkbox, callout fold chevron) by hit-testing
    /// the editor's own mouse handling. Returns the click's x offset from the
    /// editor's edge and the new text, or `None` if no click changed it.
    pub(super) fn click_row_until_change(
        window: WindowHandle<EditorState>,
        body: &str,
        line_start: usize,
        cx: &mut TestAppContext,
    ) -> Option<(f32, String)> {
        for step in 0..200 {
            load(window, body, cx);
            let (left, row) = window
                .update(cx, |editor, _, _| {
                    let left = editor.caret_screen_bounds()?.left();
                    Some((left, editor.bounds_for_offset(line_start)?))
                })
                .unwrap()
                .expect("the row was painted");
            let dx = px(2. * step as f32);
            let position = point(left + dx, row.top() + row.size.height / 2.);
            let mut vcx = VisualTestContext::from_window(window.into(), cx);
            vcx.simulate_mouse_move(position, None, Modifiers::none());
            vcx.simulate_click(position, Modifiers::none());
            let after = text(window, cx);
            if after != body {
                return Some((f32::from(dx), after));
            }
        }
        None
    }

    /// The editor's text after the click that first changes the line
    /// starting with `line`.
    fn click_line(
        window: WindowHandle<EditorState>,
        body: &str,
        line: &str,
        cx: &mut TestAppContext,
    ) -> Option<String> {
        let start = offset_of(body, &format!("\n{line}"), 1);
        click_row_until_change(window, body, start, cx).map(|(_, text)| text)
    }

    fn span(before: &str, after: &str) -> (usize, String, String) {
        let (at, removed, inserted) = changed_span(before, after).expect("the edit changed text");
        (at, removed.to_string(), inserted.to_string())
    }

    #[test]
    fn first_difference_and_span() {
        assert_eq!(first_difference("abc", "abc"), None);
        assert_eq!(first_difference("abc", "abd"), Some(2));
        assert_eq!(first_difference("ab", "abc"), Some(2));
        assert_eq!(changed_span("a b", "a xb"), Some((2, "", "x")));
        assert_eq!(changed_span("- [ ] t", "- [x] t"), Some((3, " ", "x")));
        assert_eq!(changed_span("aa", "aaa"), Some((2, "", "a")));
        // Offsets snap to char boundaries.
        assert_eq!(changed_span("å", "ä"), Some((0, "å", "ä")));
        assert_eq!(excerpt("åäö", 1), "\"|åäö\"");
    }

    #[test]
    fn frontmatter_changes_are_classified() {
        let before = "---\n# c\ntitle: x\ntags:\n  - a\ncreated_at: \"2026-01-01T00:00:00.000000Z\"\n---\nbody";
        let after =
            "---\ntags:\n  - a\ncreated_at: \"2026-01-01T00:00:00Z\"\nmodified_at: x\n---\nbody";
        let change = classify_change("n", before, after).unwrap();
        assert_eq!(
            change.frontmatter,
            [
                "dropped: #comment",
                "rewritten: created_at",
                "dropped: title",
                "added: modified_at"
            ]
        );
        assert_eq!(change.body_offset, None);
        let change = classify_change("n", "---\ntags: [x\n---\nbody", "body").unwrap();
        assert_eq!(change.frontmatter, ["frontmatter removed"]);
        assert!(classify_change("n", "same", "same").is_none());
    }

    /// Bodies that must come back byte for byte, covering the task's edge
    /// cases: line endings, whitespace, raw HTML, footnotes, mixed list
    /// markers, hard breaks, Tera syntax and non-ASCII text.
    const LOSSLESS: &[&str] = &[
        "",
        "\n\n# Leading blank lines\n",
        "no trailing newline",
        "many trailing newlines\n\n\n\n\n",
        "crlf\r\nlines\r\n\r\n- [ ] task\r\n| a | b |\r\n| - | - |\r\n",
        "mixed\nendings\r\nhere\n",
        "lone\rcarriage\rreturns\r",
        "\u{feff}# BOM\n",
        "\tTab-led line\n- item\n\t- tab child\ntrailing   \ntab\t\n```\n\tcode\t \n```\n",
        "hard break  \nnext\nbackslash\\\nnext\n",
        "ref[^a] and[^b]\n\n[^a]:   spaced\n[^b]:\n    continued\n",
        "* star\n  - dash\n    + plus\n      1) paren\n1. one\n1. one\n10. ten\n",
        "<details>\n<summary>S</summary>\n\n*md*\n\n</details>\n<!-- comment -->\n\\* \\[x\\] &amp; &nbsp;\n",
        "{{ date | date(format=\"%Y-%m-%d\") }} {% if x %}y{% endif %}\n",
        "$E = mc^2$ inline and a block:\n\n$$\nx\n$$\n\n$$a$$ and $$b$$\n```\n$$ code $$\n```\n",
        "NBSP:\u{a0}. ZWJ: 👩‍💻. e\u{301}. RTL: שלום. ZWSP:\u{200b}.\n",
        "| Name   | Qty |\n| ------ | --: |\n| Apples |   3 |\n\n|Tight|t|\n|-|-|\n|no|p|\n",
        "> [!NOTE]-\n> folded\n\n> [!TIP]+\n> open\n",
        "Setext\n======\n\nSub\n---\n\n***\n",
    ];

    #[gpui::test]
    fn edge_case_bodies_round_trip(cx: &mut TestAppContext) {
        let window = open(cx);
        for &body in LOSSLESS {
            let output = load(window, body, cx);
            assert_eq!(
                first_difference(body, &output),
                None,
                "{body:?} came back as {output:?}"
            );
            assert!(cx.update(|cx| painted_current_text(window, cx)), "{body:?}");
        }
    }

    #[gpui::test]
    fn words_attached_math_fences_are_split_on_load(cx: &mut TestAppContext) {
        // Recorded behaviour: the editor's load normalization also reaches
        // into inline code spans.
        let window = open(cx);
        assert_eq!(
            load(window, "a $$x$$ b\n", cx),
            "a\n$$x$$\nb\n",
            "words-attached fence"
        );
        assert_eq!(
            load(window, "Inline `$$x$$` code\n", cx),
            "Inline `\n$$x$$\n` code\n"
        );
    }

    #[gpui::test]
    fn check_cave_lists_mismatches(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
        std::fs::write(dir.path().join("plain.md"), "# Plain\n\ntext\r\n").unwrap();
        std::fs::write(
            dir.path().join("math.md"),
            "---\ntags:\n  - t\n---\nSee $$x$$ here.\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join(".granit/templates/Tpl.md"),
            "{{ title }} $$y$$ z\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let window = open(cx);
        let report = check_cave(&cave, |body| load(window, body, cx)).unwrap();
        assert_eq!((report.notes, report.templates), (2, 1));
        let found: Vec<(&str, usize)> = report
            .mismatches
            .iter()
            .map(|m| (m.name.as_str(), m.offset))
            .collect();
        assert_eq!(found, [("math", 3), ("template:Tpl", 11)]);
        assert!(report
            .render()
            .contains("MISMATCH math: first difference at byte 3"));
    }

    const TOUR: &str =
        "# Tour\n\nA paragraph with **bold** text.\n\n- Bullet one\n- Bullet two\n\n\
                        - [ ] Open task\n- [x] Done task\n\n| Name   | Qty |\n| ------ | --: |\n\
                        | Apples |   3 |\n| Pears  |  12 |\n\n> [!NOTE]\n> plain\n\n\
                        > [!NOTE]-\n> folded\n";

    #[gpui::test]
    fn typing_one_character_mid_paragraph_is_a_one_byte_diff(cx: &mut TestAppContext) {
        let window = open(cx);
        let at = offset_of(TOUR, "paragraph", 4);
        let after = type_at(window, TOUR, at, "x", cx);
        assert_eq!(span(TOUR, &after), (at, String::new(), "x".to_string()));
    }

    #[gpui::test]
    fn clicking_a_task_checkbox_flips_one_byte(cx: &mut TestAppContext) {
        let window = open(cx);
        let after = click_line(window, TOUR, "- [ ] Open task", cx).expect("checkbox hit");
        let at = offset_of(TOUR, "- [ ] Open", 3);
        assert_eq!(span(TOUR, &after), (at, " ".to_string(), "x".to_string()));
    }

    #[gpui::test]
    fn typing_in_a_table_cell_touches_only_that_cell(cx: &mut TestAppContext) {
        let window = open(cx);
        let at = offset_of(TOUR, "Apples", 6);
        let after = type_at(window, TOUR, at, "!", cx);
        assert_eq!(span(TOUR, &after), (at, String::new(), "!".to_string()));
    }

    #[gpui::test]
    fn clicking_a_callout_chevron_flips_only_the_fold_char(cx: &mut TestAppContext) {
        let window = open(cx);
        let after = click_line(window, TOUR, "> [!NOTE]-", cx).expect("chevron hit");
        let at = offset_of(TOUR, "> [!NOTE]-", 9);
        assert_eq!(span(TOUR, &after), (at, "-".to_string(), "+".to_string()));
        // A plain `> [!NOTE]` has no fold control: no click on its row edits.
        assert_eq!(click_line(window, TOUR, "> [!NOTE]\n> plain", cx), None);
    }

    #[gpui::test]
    fn enter_in_a_list_continues_it(cx: &mut TestAppContext) {
        let window = open(cx);
        let at = offset_of(TOUR, "- Bullet two", 12);
        let after = keys_at(window, TOUR, at, "enter", cx);
        // The inserted "\n- " sits before another "\n- " line, so the
        // minimal span is ambiguous; compare the whole text instead.
        assert_eq!(after, format!("{}\n- {}", &TOUR[..at], &TOUR[at..]));
        assert_eq!(after.len(), TOUR.len() + 3);
    }

    /// Which single edit [`edit_a_note_of_a_cave_copy`] performs, by name:
    /// `(name, slug, kind, needle, offset into needle)`. Kinds: `type` types
    /// "x" at the offset, `keys:<keys>` presses bound keys there, `click`
    /// clicks along the needle's row until the text changes.
    const CAVE_EDITS: &[(&str, &str, &str, &str, usize)] = &[
        ("type", "Formatting tour", "type", "A paragraph", 6),
        ("checkbox", "Formatting tour", "click", "- [ ] Open task", 0),
        ("table", "Formatting tour", "type", "| Apples |", 8),
        ("alert-plain", "Formatting tour", "click", "> [!NOTE]", 0),
        ("alert-fold", "Foldable alerts", "click", "> [!NOTE]-", 0),
        ("alert-unfold", "Foldable alerts", "click", "> [!TIP]+", 0),
        ("enter", "Formatting tour", "keys:enter", "- Bullet two", 12),
        (
            "enter-task",
            "Formatting tour",
            "keys:enter",
            "- [ ] Open task",
            15,
        ),
        ("crlf-type", "CRLF line endings", "type", "Every line", 5),
        (
            "crlf-enter",
            "CRLF line endings",
            "keys:enter",
            "- [ ] a task",
            12,
        ),
        (
            "crlf-end-enter",
            "CRLF line endings",
            "keys:end enter",
            "- [ ] a task",
            0,
        ),
        (
            "math-para-type",
            "Raw HTML and oddities",
            "type",
            "Math-looking",
            4,
        ),
        (
            "math-note-type",
            "Raw HTML and oddities",
            "type",
            "# Raw HTML",
            5,
        ),
    ];

    /// Perform the single edit named by `GRANIT_RT_EDIT` on a note of the
    /// cave copy at `GRANIT_RT_CAVE` and save it through `Cave::save_note`,
    /// so `git diff` in the copy shows exactly what the edit wrote. Writes to
    /// that cave: point it at a throwaway copy.
    #[gpui::test]
    #[ignore = "needs GRANIT_RT_CAVE (a throwaway cave copy) and GRANIT_RT_EDIT"]
    fn edit_a_note_of_a_cave_copy(cx: &mut TestAppContext) {
        let cave_path = std::env::var("GRANIT_RT_CAVE").expect("GRANIT_RT_CAVE");
        let name = std::env::var("GRANIT_RT_EDIT").expect("GRANIT_RT_EDIT");
        let &(_, slug, kind, needle, at) = CAVE_EDITS
            .iter()
            .find(|edit| edit.0 == name)
            .unwrap_or_else(|| panic!("unknown edit {name}"));
        let mut cave = Cave::open(PathBuf::from(cave_path)).unwrap();
        let body = cave.read_note(slug).unwrap().content;
        let window = open(cx);
        let offset = offset_of(&body, needle, at);
        let after = match kind {
            "type" => Some(type_at(window, &body, offset, "x", cx)),
            "click" => {
                let start = body[..offset].rfind('\n').map_or(0, |i| i + 1);
                click_row_until_change(window, &body, start, cx).map(|(dx, text)| {
                    println!("{name}: the click {dx}px from the editor's edge changed the text");
                    text
                })
            }
            keys => Some(keys_at(
                window,
                &body,
                offset,
                keys.strip_prefix("keys:").expect("a keys: edit"),
                cx,
            )),
        };
        match after
            .as_deref()
            .and_then(|after| changed_span(&body, after))
        {
            Some((at, removed, inserted)) => {
                println!(
                    "{name} on {slug:?}: body span at byte {at}: removed {removed:?}, inserted {inserted:?}"
                );
                cave.save_note(slug, after.as_deref().unwrap()).unwrap();
            }
            None => println!("{name} on {slug:?}: no change to the text, nothing saved"),
        }
    }
}
