---
title: Getting Started
category: Getting Started
tags: [basics, cave, ui]
---

Granit organizes your notes around a single concept called a cave: a local directory of markdown files that the app opens, reads, and writes. This guide explains what a cave is, how to open your first one, what Granit stores alongside your notes, and how to find your way around the interface. If you have not installed Granit yet, start with [[installation]].

# What is a cave?

A cave is any directory on your computer that holds your markdown notes. Granit does not impose its own database or file format — your notes stay as plain `.md` files that you can read, back up, or edit with any other tool. Notes may live directly in the cave or in nested subfolders.

When Granit opens a directory as a cave, it also manages a small `.granit/` folder inside it for app-specific data. Each cave is self-contained: its settings travel with it, so different caves can have different themes, daily-note folders, and agent configurations.

# Opening your first cave

When you launch Granit, use the cave selector to choose a directory. Any folder works — pick an existing collection of markdown notes, or create a new empty directory to start fresh. Granit scans the directory, indexes your notes, and opens the explorer.

Granit remembers the last cave you opened and restores it automatically on the next launch, so you only need to select a cave once.

> [!TIP]
> Filenames are the identity of your notes in Granit, and they must be unique across the whole cave. Read [[cave-rules]] before you start organizing notes into subfolders.

# What `.granit/` contains

The first time Granit opens a directory, it creates a `.granit/` folder for cave-local data:

```text
<cave>/
  .granit/
    config.yml
    templates/
```

- **`config.yml`** stores this cave's settings: sidebar widths and visibility, theme, fonts, the daily-note folder, an optional daily-note template, and agent/provider settings. See [[configuration]] for the full list of options.
- **`templates/`** holds reusable note templates in their own flat namespace. See [[templates]] for how to create and use them.

The `.granit/` directory and any other hidden directories are excluded from note scans, so nothing inside them appears as a note. The path of the currently open cave is remembered separately by the app, not inside `config.yml`.

# A quick tour of the interface

Granit's window is split into the explorer sidebar on the left, the main reader or editor in the center, and an optional AI agent panel.

## Explorer sidebar

The left pane is the [[explorer]], which provides several tabs for navigating your cave:

- **Tree** — the folder and note hierarchy, with drag-and-drop moves, inline rename, and context-menu actions for notes and folders.
- **Search** — full-text search across every note in the cave.
- **Todo** — task-list checkboxes collected from your notes; see [[todos]].
- **Tags** — notes grouped by the tags declared in their frontmatter.
- **Favorites** — notes you have marked as favorite.
- **Templates** — the templates stored in `.granit/templates/`.

A compact daily-note calendar strip sits alongside the tabs for jumping to [[daily-notes]].

## Reader and edit mode

Selecting a note opens it in the rendered reader, which displays your markdown as formatted HTML — headings, lists, tables, code blocks, and resolved [[wiki-links]]. Task-list checkboxes in the reader are interactive and can be toggled directly.

Switching to edit mode replaces the reader with a CodeMirror text editor for writing raw markdown. For more on the note format, frontmatter, and supported syntax, see [[notes-and-markdown]].

## Agent panel

The [[ai-agent]] panel hosts a streaming chat assistant that can read and modify notes in your cave through a set of tools. You choose the provider and model, and in Ask mode the agent can draw on retrieval over your cave. See [[agent-tools-and-rag]] for what the agent can do and how retrieval works.

# Next steps

- Learn the naming and identity model in [[cave-rules]].
- Start writing in [[notes-and-markdown]] and connect notes with [[wiki-links]].
- Tune the app to your cave in [[configuration]].
