---
title: Notes & Markdown
category: Notes & Writing
tags: [markdown, notes, editing]
---

Every note in a cave is a plain Markdown file. Granit renders notes to HTML for reading and gives you a dedicated editor for writing. The filename stem is the note's title and identity — see [[cave-rules]] for the full naming model. This page covers how notes are read, edited, and what frontmatter fields Granit understands.

# Reader and editor

Granit has two views of a note: a rendered reader and an editor.

The **reader** shows the note as formatted HTML rendered by the backend. Headings, lists, tables, links, code blocks, and other Markdown render the way you would expect, and [[wiki-links]] become clickable navigation.

The **editor** is a CodeMirror editing surface for the raw Markdown source. Switch to it when you want to change a note's text. Granit does not use a live-preview editor — you write in the editor and read in the reader, rather than seeing both at once.

## Saving is automatic

You never need to save manually. While you edit, Granit persists your changes a couple of seconds after typing pauses, and closing the editor saves anything still pending before showing the updated preview. Explicit save (`Cmd/Ctrl+S`) still works and also returns you to the reader.

Renaming is the one deliberate exception: a changed title is applied when you leave the editor, save explicitly, or switch notes — not mid-typing — because a rename also rewrites every wiki-link pointing at the note (see [[cave-rules]]).

# Editing shortcuts

The editor carries markdown-aware keybindings:

- **Structure** — `Enter` continues lists, task lists, and blockquotes (numbered lists renumber); `Backspace` on an empty item dissolves the marker.
- **Formatting** — `Cmd/Ctrl+B` and `Cmd/Ctrl+I` toggle bold and italic around the selection or the word under the cursor. `Cmd/Ctrl+K` wraps the selection as a wiki-link (a selected URL becomes a markdown link instead). `Cmd/Ctrl+L` toggles the task checkbox on the selected lines.
- **Find and replace** — `Cmd/Ctrl+F` opens a search popover in the editor's top-right corner with match-case, regular-expression, and whole-word toggles. `Cmd/Ctrl+G` / `Shift+Cmd/Ctrl+G` step through matches, and `Cmd/Ctrl+D` selects the next occurrence of the selection for multi-cursor edits.
- **Links** — hold `Cmd/Ctrl` and click any link to follow it; see [[wiki-links#following-links-while-editing]].

The full list is always available in the app: the keyboard icon in the sidebar footer opens a shortcuts reference.

# Copying a note

The copy button in the top-right action bar (available in both reader and editor) puts the rendered note on the clipboard as rich text. Pasting into Word, Teams, or a presentation keeps headings, lists, and formatting; pasting into a plain-text target yields the raw Markdown source instead.

Because wiki-links only mean something inside your cave, they are flattened to their visible text in the copied rich text, while regular web links stay clickable. Task checkboxes are converted to ☐/☑ symbols so their state survives the paste.

# Frontmatter

Notes may begin with a YAML frontmatter block delimited by `---`. Granit parses the frontmatter separately from the body and recognizes these fields:

- `tags` — a list of strings, indexed cave-wide and surfaced in the Tags tab of the [[explorer]].
- timestamps — created and updated times for the note.
- `icon` — an optional icon shown next to the note.
- `favorite` — a boolean flag; favorited notes appear in the Favorites tab of the [[explorer]].

A minimal example:

```markdown
---
tags: [project, draft]
favorite: true
---

# Section heading

Body text starts here.
```

> [!IMPORTANT]
> Frontmatter does **not** set the note's title. The title always comes from the filename stem. See [[cave-rules]] for why filenames are the single source of identity.

# Raw HTML is sanitized

You can write raw HTML inside a note, but Granit sanitizes it before it reaches the reader. Unsafe markup is stripped, so embedded scripts and dangerous attributes will not run. Rely on Markdown and the supported extensions below rather than arbitrary HTML.

# Task-list checkboxes

Markdown task lists render as interactive checkboxes in the reader:

```markdown
- [ ] Draft the outline
- [x] Collect references
```

Toggling a checkbox in the reader writes the change back to the note file. These same tasks are aggregated in the Todo tab — see [[todos]] for details. Note that checkboxes in agent-rendered Markdown are disabled and act as a static display only.

# Mermaid diagrams

Fenced code blocks tagged `mermaid` render as diagrams in the reader:

````markdown
```mermaid
graph TD
  A[Note] --> B[Reader]
  A --> C[Editor]
```
````

This lets you keep flowcharts and other diagrams inline in your notes as plain text.

# Related pages

- [[wiki-links]] — linking notes together and to heading anchors.
- [[templates]] — start new notes from reusable scaffolds.
- [[explorer]] — browse, search, and filter your notes.
- [[configuration]] — fonts, themes, and per-cave settings.
