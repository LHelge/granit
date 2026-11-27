use chrono::{DateTime, SecondsFormat, Utc};
use granit_types::Frontmatter;

use super::frontmatter::{key_end, locate_frontmatter};
use super::Markdown;

/// Serialize frontmatter to YAML normalized to exactly one trailing newline,
/// so the closing `---` fence always lands on its own line regardless of how
/// the YAML library terminates its output (serde_yml 0.0.13 stopped emitting
/// a trailing newline).
fn frontmatter_yaml(fm: &Frontmatter) -> String {
    let mut yaml = serde_yml::to_string(fm).unwrap_or_default();
    while yaml.ends_with('\n') {
        yaml.pop();
    }
    if !yaml.is_empty() {
        yaml.push('\n');
    }
    yaml
}

impl Markdown<'_> {
    /// Generate the initial file content for a new note.
    ///
    /// Produces a YAML frontmatter block with `created_at` and `modified_at`
    /// set to the current UTC time. The note body starts empty.
    pub fn new_note() -> String {
        Self::new_note_with_body("", Vec::new(), None, None)
    }

    /// Generate initial file content with fresh frontmatter and a provided body.
    pub fn new_note_with_body(
        body: &str,
        tags: Vec<String>,
        icon: Option<String>,
        presentation: Option<String>,
    ) -> String {
        let now = Utc::now();
        let fm = Frontmatter {
            tags,
            created_at: Some(now),
            modified_at: Some(now),
            icon,
            favorite: None,
            presentation,
        };
        let yaml = frontmatter_yaml(&fm);
        format!("---\n{yaml}---\n{body}")
    }

    /// Put `new_body` under the frontmatter of `existing_raw`, applying the
    /// requested changes to the fields Granit owns.
    ///
    /// - `tags`: `None` = preserve existing tags; `Some(v)` = replace.
    /// - `icon`: `None` = preserve existing icon; `Some("")` = clear;
    ///   `Some(s)` = set to `s`.
    /// - `favorite`: `None` = preserve existing value; `Some(v)` = set to `v`.
    /// - `presentation`: `None` = preserve; `Some("")` = clear; `Some(s)` = set.
    ///
    /// The frontmatter is never re-serialized. A requested value equal to the
    /// current one is no change, and when neither the body nor any field
    /// changes, `existing_raw` is returned unchanged, byte for byte, so the
    /// caller can skip the write. Otherwise only the lines of the changed keys
    /// are rewritten (plus `modified_at`, stamped with the current time), and
    /// everything else in the block — unknown keys, key order, comments,
    /// quoting, flow-style lists, line endings — is copied verbatim.
    /// `created_at` is never touched.
    ///
    /// `new_body` is the body verbatim: a leading `---` block in it is body
    /// text, not frontmatter to strip.
    ///
    /// Frontmatter that is malformed (not YAML, or not the shape of
    /// [`Frontmatter`]) is kept verbatim on a body change and gets no
    /// `modified_at`; an explicit field change still rewrites just that key.
    /// A note without frontmatter gains a fresh block (with `created_at` and
    /// `modified_at`) only when a field is set.
    pub fn rebuild(
        existing_raw: &str,
        new_body: &str,
        tags: Option<Vec<String>>,
        icon: Option<String>,
        favorite: Option<bool>,
        presentation: Option<String>,
    ) -> String {
        let existing = Markdown::new(existing_raw);
        let parsed = existing.frontmatter().cloned();
        let current = parsed.clone().unwrap_or_default();
        // An empty string means "no value" on both sides: a file with
        // `icon: ""` and a request to clear the icon agree.
        let non_empty = |s: String| (!s.is_empty()).then_some(s);
        let current_icon = current.icon.clone().and_then(non_empty);
        let current_presentation = current.presentation.clone().and_then(non_empty);

        let mut edits: Vec<FieldEdit> = Vec::new();
        if let Some(tags) = tags.filter(|tags| *tags != current.tags) {
            edits.push(FieldEdit::Tags(tags));
        }
        if let Some(icon) = icon.map(non_empty).filter(|icon| *icon != current_icon) {
            edits.push(FieldEdit::Scalar("icon", icon));
        }
        if let Some(favorite) = favorite.filter(|f| Some(*f) != current.favorite) {
            edits.push(FieldEdit::Favorite(favorite));
        }
        if let Some(presentation) = presentation
            .map(non_empty)
            .filter(|p| *p != current_presentation)
        {
            edits.push(FieldEdit::Scalar("presentation", presentation));
        }

        if edits.is_empty() && existing.body() == new_body {
            return existing_raw.to_string();
        }

        let now = Utc::now();
        let Some(span) = locate_frontmatter(existing_raw) else {
            // Without a block, every edit sets a value (clearing compares
            // equal to the absent field), so any edit means a new block.
            if edits.is_empty() {
                return new_body.to_string();
            }
            let mut fm = Frontmatter {
                created_at: Some(now),
                modified_at: Some(now),
                ..Frontmatter::default()
            };
            for edit in edits {
                edit.apply_to(&mut fm);
            }
            let yaml = frontmatter_yaml(&fm);
            return format!("---\n{yaml}---\n{new_body}");
        };

        let mut yaml = span.yaml.to_string();
        for edit in &edits {
            yaml = edit.splice(&yaml, span.eol);
        }
        if parsed.is_some() {
            yaml = splice_modified_at(&yaml, now, span.eol);
        }
        format!(
            "{}{yaml}{}{new_body}",
            &existing_raw[..span.yaml_start],
            &existing_raw[span.yaml_end..span.body_start]
        )
    }
}

/// Set the top-level string key `key` of the frontmatter of `raw` to
/// `value`, rewriting only that key's lines (in the quoting style of the
/// value it replaces; a new key goes at the end of the block). Every other
/// byte of the file — other keys, comments, order, line endings, the body —
/// is kept. A file without a frontmatter block gains one holding just the
/// key, followed by a blank line. Nothing is compared: the caller decides
/// whether the value changed.
pub(crate) fn splice_frontmatter_scalar(raw: &str, key: &str, value: &str) -> String {
    let Some(span) = locate_frontmatter(raw) else {
        return format!(
            "---\n{key}: {}\n---\n\n{raw}",
            render_scalar(value, Quote::Plain)
        );
    };
    let entry = find_entry(span.yaml, key);
    let key_text = entry.as_ref().map_or(key, |e| e.key_text);
    let style = entry
        .as_ref()
        .map_or(Quote::Plain, |e| quote_style(e.value));
    let line = format!("{key_text}: {}{}", render_scalar(value, style), span.eol);
    let yaml = replace_entry(span.yaml, entry.as_ref(), Some(&line), span.eol);
    format!("{}{yaml}{}", &raw[..span.yaml_start], &raw[span.yaml_end..])
}

/// A change to one Granit-owned frontmatter field. `None` removes the key.
enum FieldEdit {
    Tags(Vec<String>),
    Scalar(&'static str, Option<String>),
    Favorite(bool),
}

impl FieldEdit {
    fn apply_to(self, fm: &mut Frontmatter) {
        match self {
            FieldEdit::Tags(tags) => fm.tags = tags,
            FieldEdit::Scalar("icon", value) => fm.icon = value,
            FieldEdit::Scalar(_, value) => fm.presentation = value,
            FieldEdit::Favorite(favorite) => fm.favorite = Some(favorite),
        }
    }

    /// Rewrite this field's key in `yaml`, leaving every other line as is.
    fn splice(&self, yaml: &str, eol: &str) -> String {
        let key = match self {
            FieldEdit::Tags(_) => "tags",
            FieldEdit::Scalar(key, _) => key,
            FieldEdit::Favorite(_) => "favorite",
        };
        let entry = find_entry(yaml, key);
        let key_text = entry.as_ref().map_or(key, |e| e.key_text);
        let rendered = match self {
            FieldEdit::Tags(tags) if tags.is_empty() => None,
            FieldEdit::Tags(tags) => Some(render_tags(key_text, tags, entry.as_ref(), eol)),
            FieldEdit::Scalar(_, None) => None,
            FieldEdit::Scalar(_, Some(value)) => {
                let style = entry
                    .as_ref()
                    .map_or(Quote::Plain, |e| quote_style(e.value));
                Some(format!("{key_text}: {}{eol}", render_scalar(value, style)))
            }
            FieldEdit::Favorite(favorite) => Some(format!("{key_text}: {favorite}{eol}")),
        };
        replace_entry(yaml, entry.as_ref(), rendered.as_deref(), eol)
    }
}

/// Set `modified_at` to `now`, keeping the quoting of the existing value (or,
/// for a new key, that of `created_at`).
fn splice_modified_at(yaml: &str, now: DateTime<Utc>, eol: &str) -> String {
    let entry = find_entry(yaml, "modified_at");
    let style = match &entry {
        Some(e) => quote_style(e.value),
        None => find_entry(yaml, "created_at").map_or(Quote::Double, |e| quote_style(e.value)),
    };
    let key_text = entry.as_ref().map_or("modified_at", |e| e.key_text);
    let value = now.to_rfc3339_opts(SecondsFormat::AutoSi, true);
    let line = format!("{key_text}: {}{eol}", render_scalar(&value, style));
    replace_entry(yaml, entry.as_ref(), Some(&line), eol)
}

/// One top-level key of a frontmatter block: its key line plus the indented
/// (or `- ` sequence) continuation lines that belong to its value.
struct Entry<'a> {
    /// The key as written, quotes included.
    key_text: &'a str,
    /// The inline value after the colon, trimmed (may be empty).
    value: &'a str,
    /// Byte range of the entry's lines in the YAML text, line endings included.
    start: usize,
    end: usize,
    /// The continuation lines (the value's block, if any).
    continuation: &'a str,
}

/// Find the first top-level entry whose key is `key`.
fn find_entry<'a>(yaml: &'a str, key: &str) -> Option<Entry<'a>> {
    let mut found: Option<Entry<'a>> = None;
    let mut key_line_end = 0;
    let mut pos = 0;
    for line in yaml.split_inclusive('\n') {
        let line_end = pos + line.len();
        let text = line.trim_end_matches(['\n', '\r']);
        if let Some(colon) = key_end(line) {
            if found.is_some() {
                break;
            }
            let key_text = text[..colon].trim_end();
            if key_text.trim_matches(['"', '\'']) == key {
                key_line_end = line_end;
                found = Some(Entry {
                    key_text,
                    value: text[colon + 1..].trim(),
                    start: pos,
                    end: line_end,
                    continuation: "",
                });
            }
        } else if let Some(entry) = found.as_mut() {
            let blank = text.trim().is_empty();
            let continues =
                (text.starts_with([' ', '\t']) && !blank) || text == "-" || text.starts_with("- ");
            if continues {
                // Blank lines in between are only taken along when a
                // continuation line follows them.
                entry.end = line_end;
            } else if !blank {
                // A top-level comment or stray line ends the value.
                break;
            }
        }
        pos = line_end;
    }
    found.map(|mut entry| {
        entry.continuation = &yaml[key_line_end..entry.end];
        entry
    })
}

/// Replace `entry` with `rendered` (or remove it when `None`); append a new
/// entry at the end of the block when the key is absent.
fn replace_entry(yaml: &str, entry: Option<&Entry>, rendered: Option<&str>, eol: &str) -> String {
    match (entry, rendered) {
        (Some(entry), rendered) => format!(
            "{}{}{}",
            &yaml[..entry.start],
            rendered.unwrap_or_default(),
            &yaml[entry.end..]
        ),
        (None, Some(rendered)) => {
            let sep = if yaml.is_empty() || yaml.ends_with('\n') {
                ""
            } else {
                eol
            };
            format!("{yaml}{sep}{rendered}")
        }
        (None, None) => yaml.to_string(),
    }
}

/// Render a `tags` entry in the style of the one it replaces: a flow list
/// stays a flow list, a block list keeps its item indentation.
fn render_tags(key_text: &str, tags: &[String], entry: Option<&Entry>, eol: &str) -> String {
    if entry.is_some_and(|e| e.value.starts_with('[')) {
        let items: Vec<String> = tags.iter().map(|t| render_flow_item(t)).collect();
        return format!("{key_text}: [{}]{eol}", items.join(", "));
    }
    let indent = entry
        .and_then(|e| {
            e.continuation.lines().find_map(|line| {
                let item = line.trim_start();
                item.starts_with('-')
                    .then(|| &line[..line.len() - item.len()])
            })
        })
        .unwrap_or("");
    let mut out = format!("{key_text}:{eol}");
    for tag in tags {
        out.push_str(&format!(
            "{indent}- {}{eol}",
            render_scalar(tag, Quote::Plain)
        ));
    }
    out
}

#[derive(Clone, Copy)]
enum Quote {
    Plain,
    Single,
    Double,
}

fn quote_style(value: &str) -> Quote {
    match value.chars().next() {
        Some('"') => Quote::Double,
        Some('\'') => Quote::Single,
        _ => Quote::Plain,
    }
}

/// Render a string scalar in `style`, falling back to double quotes whenever
/// the requested style could not represent the value as that string.
fn render_scalar(value: &str, style: Quote) -> String {
    match style {
        Quote::Plain if is_plain_safe(value, false) => value.to_string(),
        Quote::Single if !value.chars().any(char::is_control) => {
            format!("'{}'", value.replace('\'', "''"))
        }
        _ => double_quoted(value),
    }
}

fn render_flow_item(value: &str) -> String {
    if is_plain_safe(value, true) {
        value.to_string()
    } else {
        double_quoted(value)
    }
}

fn double_quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Whether `value` can be written as a plain YAML scalar and read back as
/// the same string. Conservative: anything doubtful is quoted.
fn is_plain_safe(value: &str, in_flow: bool) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };
    let reserved = [
        "true", "false", "yes", "no", "on", "off", "y", "n", "null", "~",
    ];
    !(first.is_whitespace()
        || value.ends_with(char::is_whitespace)
        || "-?:,[]{}#&*!|>'\"%@`".contains(first)
        || value.chars().any(char::is_control)
        || value.contains(": ")
        || value.contains(" #")
        || value.ends_with(':')
        || (in_flow && value.contains([',', '[', ']', '{', '}']))
        || reserved.contains(&value.to_ascii_lowercase().as_str())
        || value.parse::<f64>().is_ok())
}

#[cfg(test)]
mod tests {
    use crate::markdown::Markdown;

    #[test]
    fn test_rebuild_keeps_a_leading_block_in_the_new_body() {
        let existing =
            "---\ntags:\n- original\ncreated_at: \"2026-01-01T00:00:00Z\"\nmodified_at: \"2026-01-01T00:00:00Z\"\n---\nOld body";
        let new_body = "---\nkey: value\n---\nNew body content";
        let result = Markdown::rebuild(existing, new_body, None, None, None, None);
        assert!(result.starts_with("---\ntags:\n- original\n"), "{result}");
        assert!(result.ends_with("\n---\n---\nkey: value\n---\nNew body content"));
        // Read back, the body is exactly what was saved.
        assert_eq!(Markdown::new(&result).body(), new_body);
    }

    #[test]
    fn test_rebuild_unchanged_is_identical() {
        let existing = "---\r\n# comment\r\ntags: [a, b]\r\ncustom: 1\r\ncreated_at: 2026-01-01T00:00:00Z\r\n---\r\nBody\r\n";
        let body = Markdown::new(existing).body().to_string();
        // The values the editor sends on every save equal the current ones.
        let result = Markdown::rebuild(
            existing,
            &body,
            Some(vec!["a".into(), "b".into()]),
            Some(String::new()),
            None,
            Some(String::new()),
        );
        assert_eq!(result, existing);
    }

    #[test]
    fn test_rebuild_body_change_only_stamps_modified_at() {
        let existing = "---\r\n# comment\r\ntitle: Custom\r\ntags: [a, b]\r\ncreated_at: 2026-01-01T00:00:00.000Z\r\nmodified_at: '2026-01-01T00:00:00Z'\r\nz: last\r\n---\r\nBody\r\n";
        let result = Markdown::rebuild(existing, "New\r\n", None, None, None, None);
        let (head, rest) = result.split_once("modified_at: '").unwrap();
        assert_eq!(
            head,
            "---\r\n# comment\r\ntitle: Custom\r\ntags: [a, b]\r\ncreated_at: 2026-01-01T00:00:00.000Z\r\n"
        );
        let (stamp, tail) = rest.split_once('\'').unwrap();
        assert!(
            stamp.parse::<chrono::DateTime<chrono::Utc>>().is_ok(),
            "{stamp}"
        );
        assert_ne!(stamp, "2026-01-01T00:00:00Z");
        assert_eq!(tail, "\r\nz: last\r\n---\r\nNew\r\n");
    }

    #[test]
    fn test_rebuild_adds_modified_at_in_created_at_style() {
        let existing = "---\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let result = Markdown::rebuild(existing, "New", None, None, None, None);
        assert!(
            result.starts_with("---\ncreated_at: \"2026-01-01T00:00:00Z\"\nmodified_at: \""),
            "{result}"
        );
        assert!(result.ends_with("\"\n---\nNew"), "{result}");
        assert!(Markdown::new(&result)
            .frontmatter()
            .unwrap()
            .modified_at
            .is_some());
    }

    #[test]
    fn test_rebuild_tags_keep_their_style() {
        let flow = "---\nx: 1\ntags: [a, b] \nmodified_at: x\n---\nBody";
        let result = Markdown::rebuild(
            flow,
            "Body",
            Some(vec!["c".into(), "d e".into()]),
            None,
            None,
            None,
        );
        assert!(
            result.starts_with("---\nx: 1\ntags: [c, d e]\n"),
            "{result}"
        );

        let block = "---\ntags:\n  - a\n  - b\n\nicon: Star\n---\nBody";
        let result = Markdown::rebuild(block, "Body", Some(vec!["yes".into()]), None, None, None);
        assert!(
            result.starts_with("---\ntags:\n  - \"yes\"\n\nicon: Star\nmodified_at: \""),
            "{result}"
        );
        assert_eq!(Markdown::new(&result).tags(), ["yes"]);

        let cleared = Markdown::rebuild(block, "Body", Some(Vec::new()), None, None, None);
        assert!(
            cleared.starts_with("---\n\nicon: Star\nmodified_at: "),
            "{cleared}"
        );
    }

    #[test]
    fn test_rebuild_empty_icon_and_presentation_are_unchanged() {
        let existing =
            "---\nicon: \"\"\npresentation: ''\nmodified_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        // The editor sends `Some("")` for an absent icon or presentation.
        let result = Markdown::rebuild(
            existing,
            "Body",
            Some(Vec::new()),
            Some(String::new()),
            None,
            Some(String::new()),
        );
        assert_eq!(result, existing);
    }

    #[test]
    fn test_rebuild_malformed_frontmatter_is_kept_verbatim() {
        let existing = "---\ntags: [unclosed\ncreated_at: \"2026-11-26T14:00:00Z\"\n---\n# Title\n";
        assert!(Markdown::new(existing).frontmatter().is_none());
        assert_eq!(Markdown::new(existing).body(), "# Title\n");
        let result = Markdown::rebuild(existing, "# Changed\n", Some(Vec::new()), None, None, None);
        assert_eq!(
            result,
            "---\ntags: [unclosed\ncreated_at: \"2026-11-26T14:00:00Z\"\n---\n# Changed\n"
        );
    }

    #[test]
    fn test_rebuild_with_tags_override() {
        let existing =
            "---\ntags:\n- old\ncreated_at: \"2026-01-01T00:00:00Z\"\nmodified_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let result =
            Markdown::rebuild(existing, "Body", Some(vec!["new".into()]), None, None, None);
        assert!(result.contains("new"), "new tag must be present");
        assert!(!result.contains("old"), "old tag must be removed");
    }

    #[test]
    fn test_rebuild_with_icon_set_and_clear() {
        let existing =
            "---\ntags: []\ncreated_at: \"2026-01-01T00:00:00Z\"\nmodified_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let with_icon = Markdown::rebuild(existing, "Body", None, Some("Star".into()), None, None);
        assert!(with_icon.contains("Star"), "icon must be set");

        let cleared = Markdown::rebuild(&with_icon, "Body", None, Some(String::new()), None, None);
        assert!(
            !cleared.contains("Star"),
            "icon must be cleared after empty string"
        );
    }

    #[test]
    fn test_rebuild_no_frontmatter_returns_body_unchanged() {
        let result = Markdown::rebuild("No frontmatter here", "new body", None, None, None, None);
        assert_eq!(result, "new body");
    }

    #[test]
    fn test_new_note_closing_fence_on_own_line() {
        // Guards against the YAML serializer dropping its trailing newline,
        // which would glue the closing fence onto the last frontmatter line.
        let result = Markdown::new_note_with_body("Body", vec![], Some("Star".into()), None);
        assert!(
            result.contains("icon: Star\n---\nBody"),
            "closing fence must sit on its own line: {result}"
        );
    }

    #[test]
    fn test_rebuild_preserves_icon() {
        let existing = "---\ntags:\n  - old\nicon: LuStar\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\nOld body";
        let result = Markdown::rebuild(existing, "New body", None, None, None, None);
        assert!(
            result.contains("icon: LuStar"),
            "icon should be preserved: {result}"
        );
        assert!(result.contains("New body"));
    }

    #[test]
    fn test_rebuild_overrides_icon() {
        let existing = "---\nicon: LuStar\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let result = Markdown::rebuild(
            existing,
            "Body",
            None,
            Some("LuFolder".to_string()),
            None,
            None,
        );
        assert!(
            result.contains("icon: LuFolder"),
            "icon should be updated: {result}"
        );
        assert!(
            !result.contains("LuStar"),
            "old icon should be gone: {result}"
        );
    }

    #[test]
    fn test_rebuild_clears_icon() {
        let existing = "---\nicon: LuStar\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let result = Markdown::rebuild(existing, "Body", None, Some(String::new()), None, None);
        assert!(
            !result.contains("icon:"),
            "icon should be cleared: {result}"
        );
    }

    #[test]
    fn test_rebuild_preserves_favorite() {
        let existing = "---\nfavorite: true\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let result = Markdown::rebuild(existing, "New body", None, None, None, None);
        assert!(
            result.contains("favorite: true"),
            "favorite should be preserved: {result}"
        );
        assert!(result.contains("New body"));
    }

    #[test]
    fn test_rebuild_overrides_favorite() {
        let existing = "---\nfavorite: true\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let result = Markdown::rebuild(existing, "Body", None, None, Some(false), None);
        assert!(
            result.contains("favorite: false"),
            "favorite should be updated: {result}"
        );
        assert!(
            !result.contains("favorite: true"),
            "old favorite should be gone: {result}"
        );
    }

    #[test]
    fn test_rebuild_creates_frontmatter_when_missing() {
        let result = Markdown::rebuild(
            "Legacy body",
            "Updated body",
            Some(vec!["legacy".into(), "migrated".into()]),
            Some("Star".into()),
            Some(true),
            None,
        );

        assert!(
            result.starts_with("---\n"),
            "frontmatter should be added: {result}"
        );
        assert!(
            result.contains("- legacy") && result.contains("- migrated"),
            "tags should be written: {result}"
        );
        assert!(
            result.contains("icon: Star"),
            "icon should be written: {result}"
        );
        assert!(
            result.contains("favorite: true"),
            "favorite should be written: {result}"
        );
        assert!(
            result.contains("created_at:"),
            "created_at should be initialized: {result}"
        );
        assert!(
            result.contains("modified_at:"),
            "modified_at should be initialized: {result}"
        );
        assert!(
            result.ends_with("Updated body"),
            "body should be preserved: {result}"
        );
    }

    #[test]
    fn test_rebuild_preserves_sets_and_clears_presentation() {
        let existing =
            "---\npresentation: corporate\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\nBody";
        let preserved = Markdown::rebuild(existing, "New body", None, None, None, None);
        assert!(
            preserved.contains("presentation: corporate"),
            "presentation should be preserved: {preserved}"
        );

        let replaced = Markdown::rebuild(
            existing,
            "Body",
            None,
            None,
            None,
            Some("default-dark".to_string()),
        );
        assert!(
            replaced.contains("presentation: default-dark") && !replaced.contains("corporate"),
            "presentation should be replaced: {replaced}"
        );

        let cleared = Markdown::rebuild(existing, "Body", None, None, None, Some(String::new()));
        assert!(
            !cleared.contains("presentation:"),
            "presentation should be cleared: {cleared}"
        );
    }

    #[test]
    fn test_rebuild_creates_frontmatter_for_presentation() {
        let result = Markdown::rebuild(
            "Legacy body",
            "Legacy body",
            None,
            None,
            None,
            Some("default-light".to_string()),
        );
        assert!(result.starts_with("---\n"), "got: {result}");
        assert!(
            result.contains("presentation: default-light"),
            "got: {result}"
        );
    }

    #[test]
    fn test_new_note_with_body_writes_presentation() {
        let result =
            Markdown::new_note_with_body("Body", vec![], None, Some("default-dark".to_string()));
        assert!(
            result.contains("presentation: default-dark\n"),
            "got: {result}"
        );
    }
}
