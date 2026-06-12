use super::helpers::validate_name;
use super::{Cave, CaveError};
use crate::markdown::Markdown;
use granit_types::{TodoItem, TodoList};

/// A parsed todo checkbox line, e.g. `- [ ] text` or `* [x] text`.
struct Checkbox<'a> {
    /// The list marker: `-`, `*`, or `+`.
    marker: char,
    checked: bool,
    /// The todo text after the `"[x] "` prefix (unstripped markdown).
    rest: &'a str,
}

/// Parse a line (already trimmed of leading whitespace) as a todo checkbox.
///
/// Recognises `-`/`*`/`+` followed by ` [ ] `, ` [x] `, or ` [X] ` —
/// the trailing space is required, matching how the markdown renderer
/// treats task-list items.
fn parse_checkbox(trimmed: &str) -> Option<Checkbox<'_>> {
    let marker = match trimmed.chars().next() {
        Some(m @ ('-' | '*' | '+')) => m,
        _ => return None,
    };
    let after_marker = &trimmed[1..];
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
    /// in a note. This is used by the reader view, which counts checkboxes in
    /// rendered HTML and needs to map back to a source line.
    pub fn toggle_todo_by_index(&self, slug: &str, index: usize) -> Result<(), CaveError> {
        validate_name(slug)?;
        let abs_path = self
            .notes
            .get(slug)
            .ok_or_else(|| CaveError::NotFound(slug.to_string()))?;

        let raw = std::fs::read_to_string(abs_path)?;
        let mut checkbox_count = 0usize;
        let mut target_line: Option<usize> = None;

        for (idx, line) in raw.lines().enumerate() {
            if parse_checkbox(line.trim_start()).is_some() {
                if checkbox_count == index {
                    target_line = Some(idx + 1); // 1-based
                    break;
                }
                checkbox_count += 1;
            }
        }

        let line = target_line.ok_or(CaveError::InvalidTodoLine(index))?;
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
            ("- [ ] task", false, '-'),
            ("* [x] task", true, '*'),
            ("+ [X] task", true, '+'),
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
        assert!(parse_checkbox("1. [ ] numbered").is_none());
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
    fn test_toggle_todo_by_index_out_of_range() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "- [ ] only\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let err = cave.toggle_todo_by_index("note", 5).unwrap_err();
        assert!(matches!(err, CaveError::InvalidTodoLine(_)));
    }
}
