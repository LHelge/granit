---
title: Explorer
category: Notes & Writing
tags: [explorer, sidebar, navigation, search]
---

The explorer is the sidebar on the left of the Granit window. It collects several ways to navigate your cave — a file tree, full-text search, tags, favorites, todos, templates, and a calendar — into tabbed views. This page summarizes each tab; several have their own dedicated pages for detail.

# File tree

The file tree shows your notes and folders, including nested subfolders. It supports drag-and-drop to move notes and folders, inline rename, and context-menu actions for both notes and folders.

> [!NOTE]
> Renaming a note changes its slug, since the filename is the note's identity. Wiki-links resolve by filename, so renames can affect existing links — see [[wiki-links]] and [[cave-rules]].

# Search

The search tab runs a full-text search across the notes in your cave, letting you find notes by their content rather than only by name.

# Tags

The Tags tab indexes the `tags` frontmatter field cave-wide and lets you filter notes by tag. See [[notes-and-markdown]] for how to declare tags in a note's frontmatter.

# Favorites

The Favorites tab lists notes whose frontmatter sets the `favorite` flag, giving you quick access to the notes you return to most.

# Calendar

The calendar lets you browse [[daily-notes]] by date — jump to a previous day's note or see which days already have one.

# Other tabs

The explorer also surfaces the [[todos|Todo tab]], which aggregates task checkboxes from across the cave, and a templates view for the cave's [[templates]].

# Related pages

- [[notes-and-markdown]] — frontmatter fields that drive the Tags and Favorites tabs.
- [[wiki-links]] — how renaming notes interacts with links.
- [[todos]] — the aggregated Todo tab.
- [[daily-notes]] — the calendar tab in depth.
- [[configuration]] — sidebar width and visibility settings.
