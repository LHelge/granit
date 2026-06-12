---
title: Templates
category: Notes & Writing
tags: [templates, notes]
---

Templates are reusable note scaffolds stored per cave. When you create a note from a template, Granit copies the template's content into the new note so you start from a consistent structure instead of a blank page. Templates are especially useful together with [[daily-notes]], which can seed each new day's note from a template.

# Where templates live

Templates live in the cave's `.granit/templates/` directory, alongside the cave's `config.yml`. They sit **outside** the normal note tree, so they never appear in the [[explorer]] file tree or in cave note scans, and they are not valid [[wiki-links]] targets.

Each template is a Markdown file. Templates use their own **flat slug namespace** — separate from note slugs and not nested into subfolders. A template's slug is its filename stem, just like a note's.

> [!NOTE]
> Because templates are stored under `.granit/`, they are part of the cave's configuration rather than its content. See [[cave-rules]] for what `.granit/` contains and [[configuration]] for the per-cave config model.

# Creating notes from templates

When you create a new note, you can choose a template to seed its contents. Granit copies the selected template's body into the new note, and you continue editing from there as with any other note — see [[notes-and-markdown]] for the editing flow.

A template is just Markdown, so it can include frontmatter fields, headings, task lists, and placeholder text that you fill in each time.

# Related pages

- [[daily-notes]] — seed each daily note from a template.
- [[notes-and-markdown]] — edit notes created from a template.
- [[configuration]] — per-cave settings, including the default daily-note template.
- [[cave-rules]] — the role of the `.granit/` directory.
