use super::helpers::validate_name;
use super::{Cave, CaveError};
use crate::markdown::Markdown;
use granit_types::{TodoItem, TodoList};

/// A parsed todo checkbox line, e.g. `- [ ] text` or `1. [x] text`.
struct Checkbox<'a> {
    /// The list marker: `-`, `*`, `+`, or an ordered marker like `3.` / `3)`.
    marker: &'a str,
    checked: bool,
    /// The todo text after the `"[x] "` prefix (unstripped markdown).
    rest: &'a str,
}

/// Parse a line (already trimmed of leading whitespace) as a todo checkbox.
///
/// Recognises `-`/`*`/`+` and ordered markers (`1.` / `1)`, up to 9 digits
/// per CommonMark) followed by ` [ ] `, ` [x] `, or ` [X] ` — the trailing
/// space is required, matching how the markdown renderer treats task-list
/// items.
fn parse_checkbox(trimmed: &str) -> Option<Checkbox<'_>> {
    let marker_len = match trimmed.chars().next()? {
        '-' | '*' | '+' => 1,
        c if c.is_ascii_digit() => {
            let digits = trimmed
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(trimmed.len());
            match (digits <= 9, trimmed[digits..].chars().next()) {
                (true, Some('.' | ')')) => digits + 1,
                _ => return None,
            }
        }
        _ => return None,
    };
    let (marker, after_marker) = trimmed.split_at(marker_len);
    let checked = if after_marker.starts_with(" [ ] ") {
        false
    } else if after_marker.starts_with(" [x] ") || after_marker.starts_with(" [X] ") {
        true
    } else {
        return None;
    };
    Some(Checkbox {
        marker,
        checked,
        rest: &after_marker[5..],
    })
}

impl Cave {
    /// Collect all todo items (`- [ ]` / `- [x]`) across every note in the cave.
    ///
    /// Todos are extracted from raw file lines (including frontmatter so that
    /// line numbers are stable and can be used with [`toggle_todo`]). The
    /// checkbox patterns `- [ ]`, `- [x]`, `- [X]`, `* [ ]`, `* [x]`,
    /// `+ [ ]`, `+ [x]` are all supported, with any leading whitespace.
    ///
    /// Results are split into two sorted lists: incomplete and completed,
    /// each sorted alphabetically by slug then by line number.
    pub fn list_todos(&self) -> Result<TodoList, CaveError> {
        let mut incomplete: Vec<TodoItem> = Vec::new();
        let mut completed: Vec<TodoItem> = Vec::new();

        for (slug, abs_path) in &self.notes {
            let raw = std::fs::read_to_string(abs_path)?;
            let rel = self.relative_path(abs_path);
            let rel_str = rel
                .components()
                .filter_map(|c| {
                    if let std::path::Component::Normal(s) = c {
                        Some(s.to_string_lossy().into_owned())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("/");

            for (idx, line) in raw.lines().enumerate() {
                let Some(checkbox) = parse_checkbox(line.trim_start()) else {
                    continue;
                };
                let item = TodoItem {
                    slug: slug.clone(),
                    relative_path: rel_str.clone(),
                    line: idx + 1, // 1-based
                    text: Markdown::strip(checkbox.rest),
                };
                if checkbox.checked {
                    completed.push(item);
                } else {
                    incomplete.push(item);
                }
            }
        }

        incomplete.sort_by(|a, b| a.slug.cmp(&b.slug).then(a.line.cmp(&b.line)));
        completed.sort_by(|a, b| a.slug.cmp(&b.slug).then(a.line.cmp(&b.line)));
        Ok(TodoList {
            incomplete,
            completed,
        })
    }

    /// Toggle the checkbox on a specific line (1-based) in a note.
    ///
    /// `[ ]` → `[x]`, `[x]`/`[X]` → `[ ]`.  The surrounding marker prefix
    /// (`-`, `*`, `+`) and any leading whitespace are preserved.
    ///
    /// Returns an error if the line does not contain a recognised checkbox pattern.
    pub fn toggle_todo(&self, slug: &str, line: usize) -> Result<(), CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;

        let raw = std::fs::read_to_string(abs_path)?;
        let mut lines: Vec<String> = raw.lines().map(str::to_owned).collect();

        if line == 0 || line > lines.len() {
            return Err(CaveError::InvalidTodoLine(line));
        }

        let target = lines[line - 1].clone();
        let trimmed = target.trim_start();
        let leading_ws = &target[..target.len() - trimmed.len()];

        let Some(checkbox) = parse_checkbox(trimmed) else {
            return Err(CaveError::InvalidTodoLine(line));
        };
        let new_state = if checkbox.checked { ' ' } else { 'x' };
        let toggled = format!(
            "{leading_ws}{} [{new_state}] {}",
            checkbox.marker, checkbox.rest
        );

        lines[line - 1] = toggled;
        // Preserve trailing newline if original had one
        let mut new_content = lines.join("\n");
        if raw.ends_with('\n') {
            new_content.push('\n');
        }

        let updated =
            crate::markdown::Markdown::rebuild(&new_content, &new_content, None, None, None);
        super::helpers::write_atomic(abs_path, updated)?;
        Ok(())
    }

    /// Toggle the checkbox identified by its 0-based index among all checkboxes
    /// in a note. This is used by the reader view, where the renderer tags each
    /// checkbox with `data-index` counted over pulldown-cmark events.
    ///
    /// The index→line mapping must therefore come from the same parse
    /// ([`Markdown::checkbox_source_lines`]). Counting raw `- [ ]` lines here
    /// would diverge — e.g. a checkbox-looking line inside a fenced code block
    /// is no checkbox to the renderer, so every index after it would shift and
    /// toggle the wrong line.
    pub fn toggle_todo_by_index(&self, slug: &str, index: usize) -> Result<(), CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;

        let raw = std::fs::read_to_string(abs_path)?;
        let line = Markdown::new(&raw)
            .checkbox_source_lines()
            .get(index)
            .copied()
            .ok_or(CaveError::InvalidTodoLine(index))?;
        self.toggle_todo(slug, line)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_checkbox;
    use crate::cave::{Cave, CaveError};

    // ── parse_checkbox ─────────────────────────────────────────────

    #[test]
    fn test_parse_checkbox_markers_and_state() {
        for (line, checked, marker) in [
            ("- [ ] task", false, "-"),
            ("* [x] task", true, "*"),
            ("+ [X] task", true, "+"),
            // The renderer emits checkboxes for ordered task items too.
            ("1. [ ] task", false, "1."),
            ("12) [x] task", true, "12)"),
        ] {
            let cb = parse_checkbox(line).unwrap();
            assert_eq!(cb.checked, checked, "{line}");
            assert_eq!(cb.marker, marker, "{line}");
            assert_eq!(cb.rest, "task", "{line}");
        }
    }

    #[test]
    fn test_parse_checkbox_requires_trailing_space() {
        assert!(parse_checkbox("- [ ]").is_none());
        assert!(parse_checkbox("- [x]task").is_none());
    }

    #[test]
    fn test_parse_checkbox_rejects_non_checkbox_lines() {
        assert!(parse_checkbox("plain text").is_none());
        assert!(parse_checkbox("- regular list item").is_none());
        assert!(parse_checkbox("1.[ ] no space after marker").is_none());
        // CommonMark caps ordered markers at 9 digits.
        assert!(parse_checkbox("1234567890. [ ] too many digits").is_none());
        assert!(parse_checkbox("").is_none());
    }

    #[test]
    fn test_list_todos_finds_checkboxes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("note.md"),
            "- [ ] unchecked\n- [x] checked\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let todos = cave.list_todos().unwrap();
        assert_eq!(todos.incomplete.len() + todos.completed.len(), 2);
        assert_eq!(todos.incomplete.len(), 1);
        assert_eq!(todos.completed.len(), 1);
    }

    #[test]
    fn test_list_todos_empty_cave() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "# No todos here\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let todos = cave.list_todos().unwrap();
        assert!(todos.incomplete.is_empty() && todos.completed.is_empty());
    }

    #[test]
    fn test_list_todos_multiple_markers() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "- [ ] a\n* [x] b\n+ [ ] c\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let todos = cave.list_todos().unwrap();
        assert_eq!(todos.incomplete.len() + todos.completed.len(), 3);
        assert_eq!(todos.incomplete.len(), 2);
        assert_eq!(todos.completed.len(), 1);
    }

    #[test]
    fn test_list_todos_uppercase_x() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "- [X] done\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let todos = cave.list_todos().unwrap();
        assert_eq!(todos.completed.len(), 1);
        assert!(todos.incomplete.is_empty());
        assert_eq!(todos.completed[0].text, "done");
    }

    #[test]
    fn test_list_todos_line_numbers() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("note.md"),
            "# Title\nSome text\n- [ ] item at line 3\n\n- [x] item at line 5\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let todos = cave.list_todos().unwrap();
        assert_eq!(todos.incomplete.len(), 1);
        assert_eq!(todos.completed.len(), 1);
        assert_eq!(todos.incomplete[0].line, 3);
        assert_eq!(todos.completed[0].line, 5);
    }

    #[test]
    fn test_list_todos_strips_markdown() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("note.md"),
            "- [ ] **bold** and *italic*\n- [x] a `code` [link](http://example.com)\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let todos = cave.list_todos().unwrap();
        assert_eq!(todos.incomplete[0].text, "bold and italic");
        assert_eq!(todos.completed[0].text, "a code link");
    }

    #[test]
    fn test_list_todos_strips_wikilinks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("note.md"),
            "- [ ] see [[other-note]] for details\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let todos = cave.list_todos().unwrap();
        assert_eq!(todos.incomplete[0].text, "see other-note for details");
    }

    // ── toggle_todo ────────────────────────────────────────────────

    #[test]
    fn test_toggle_todo_unchecked_to_checked() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "- [ ] task\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.toggle_todo("note", 1).unwrap();

        let content = std::fs::read_to_string(dir.path().join("note.md")).unwrap();
        assert!(content.contains("[x]"));
    }

    #[test]
    fn test_toggle_todo_checked_to_unchecked() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "- [x] done\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.toggle_todo("note", 1).unwrap();

        let content = std::fs::read_to_string(dir.path().join("note.md")).unwrap();
        assert!(content.contains("[ ]"));
    }

    #[test]
    fn test_toggle_todo_uppercase_x() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "- [X] DONE\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.toggle_todo("note", 1).unwrap();

        let content = std::fs::read_to_string(dir.path().join("note.md")).unwrap();
        assert!(content.contains("[ ]"));
    }

    #[test]
    fn test_toggle_todo_invalid_line_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "no checkboxes\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.toggle_todo("note", 1).unwrap_err();
        assert!(matches!(err, CaveError::InvalidTodoLine(1)));
    }

    #[test]
    fn test_toggle_todo_out_of_bounds_line() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "- [ ] only line\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.toggle_todo("note", 999).unwrap_err();
        assert!(matches!(err, CaveError::InvalidTodoLine(999)));
    }

    // ── toggle_todo_by_index ───────────────────────────────────────

    #[test]
    fn test_toggle_todo_by_index_first() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "- [ ] first\n- [ ] second\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.toggle_todo_by_index("note", 0).unwrap();

        let content = std::fs::read_to_string(dir.path().join("note.md")).unwrap();
        assert!(content.contains("- [x] first"));
        assert!(content.contains("- [ ] second"));
    }

    #[test]
    fn test_toggle_todo_by_index_middle() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("note.md"),
            "- [ ] first\n- [ ] second\n- [ ] third\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.toggle_todo_by_index("note", 1).unwrap();

        let content = std::fs::read_to_string(dir.path().join("note.md")).unwrap();
        assert!(content.contains("- [ ] first"));
        assert!(content.contains("- [x] second"));
        assert!(content.contains("- [ ] third"));
    }

    #[test]
    fn test_toggle_todo_by_index_skips_checkbox_inside_code_block() {
        // The renderer does not render a checkbox for the code-block line, so
        // data-index 0 is the first *real* checkbox. The old raw-line counter
        // would have toggled the line inside the code block instead.
        let dir = tempfile::tempdir().unwrap();
        let content = "```\n- [ ] example in code\n```\n\n- [ ] real first\n- [ ] real second\n";
        std::fs::write(dir.path().join("note.md"), content).unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.toggle_todo_by_index("note", 0).unwrap();

        let content = std::fs::read_to_string(dir.path().join("note.md")).unwrap();
        assert!(
            content.contains("- [ ] example in code"),
            "code block must be untouched: {content}"
        );
        assert!(content.contains("- [x] real first"), "{content}");
        assert!(content.contains("- [ ] real second"), "{content}");
    }

    #[test]
    fn test_toggle_todo_by_index_counts_frontmatter_lines() {
        let dir = tempfile::tempdir().unwrap();
        let content = "---\ntags:\n  - daily\n---\n# Title\n\n- [ ] after frontmatter\n";
        std::fs::write(dir.path().join("note.md"), content).unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.toggle_todo_by_index("note", 0).unwrap();

        let content = std::fs::read_to_string(dir.path().join("note.md")).unwrap();
        assert!(content.contains("- [x] after frontmatter"), "{content}");
        assert!(content.contains("tags:"), "frontmatter intact: {content}");
    }

    #[test]
    fn test_toggle_todo_by_index_ordered_list_checkbox() {
        // pulldown-cmark treats `1. [ ] x` as a task-list item, so the reader
        // shows an interactive checkbox for it — toggling must work.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("note.md"),
            "1. [ ] ordered first\n2. [x] ordered second\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.toggle_todo_by_index("note", 1).unwrap();

        let content = std::fs::read_to_string(dir.path().join("note.md")).unwrap();
        assert!(content.contains("1. [ ] ordered first"), "{content}");
        assert!(content.contains("2. [ ] ordered second"), "{content}");
    }

    #[test]
    fn test_toggle_todo_by_index_out_of_range() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "- [ ] only\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.toggle_todo_by_index("note", 5).unwrap_err();
        assert!(matches!(err, CaveError::InvalidTodoLine(_)));
    }
}
