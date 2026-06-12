---
title: Todos
category: Notes & Writing
tags: [todos, tasks, checkboxes]
---

Granit turns Markdown task checkboxes scattered across your notes into a single, cave-wide todo list. Tasks live inline in the notes where they belong, and the Todo tab in the [[explorer]] gathers them so you can see everything outstanding in one place. Toggle a task from the reader and the change is written straight back to its note.

# Task checkboxes

A todo is an ordinary Markdown task-list item:

```markdown
- [ ] Outstanding task
- [x] Completed task
```

You can write tasks anywhere — in project notes, in [[daily-notes]], or in scratch notes. Granit parses them from the note body; see [[notes-and-markdown]] for how task lists render in the reader.

# The Todo tab

The Todo tab in the [[explorer]] aggregates task-list items from across the whole cave into one view. Instead of opening each note to check what is left, you get a consolidated list of outstanding and completed tasks drawn from every note.

# Toggling from the reader

Task checkboxes are interactive in the reader. Checking or unchecking a box updates the underlying Markdown in the note file, so the note, the reader, and the Todo tab stay in sync. Checkboxes in agent-rendered Markdown are disabled and display state only.

# Related pages

- [[notes-and-markdown]] — task-list syntax and interactive checkboxes.
- [[explorer]] — the Todo tab and the other sidebar views.
- [[daily-notes]] — capture recurring tasks in each day's note.
