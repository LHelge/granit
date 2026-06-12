---
title: Cave Rules
category: Getting Started
tags: [cave, identity, slugs]
---

Granit's note model is built on a small set of strict rules about how notes are named and identified within a cave. Understanding these rules upfront avoids surprises when you organize notes into subfolders or link between them. This page covers the identity model: how filenames, titles, and slugs relate, and what Granit excludes from its scans. For an overview of the app, see [[getting-started]].

# Filenames are the identity

In Granit, a note's filename — without the `.md` extension and without any folder path — is its identity. That stem serves three roles at once:

- It is the note's **title**, shown in the explorer and the reader.
- It is the note's **slug**, used by wiki-links and internal IPC calls.
- It is the key Granit uses to **resolve links** to the note.

Because the title comes from the filename, **frontmatter does not override the displayed title**. To rename a note's title, rename its file. The frontmatter is still used for other metadata such as tags, timestamps, an optional icon, and the `favorite` flag — see [[notes-and-markdown]] for the full frontmatter format.

# Filenames must be globally unique

> [!IMPORTANT]
> Filenames are globally unique across the entire cave. Two notes in different subfolders **cannot** share the same filename. Because the filename stem is the note's slug, a duplicate would make link resolution ambiguous, so Granit treats the whole cave as one flat namespace of names even though notes can be nested in folders.

This is why [[wiki-links]] of the form `[[note]]` resolve by filename across the whole cave rather than by relative path: there is only ever one note with a given name, no matter where it sits in the folder tree.

# What is excluded from scans

Not every file in the cave directory becomes a note:

- **Hidden directories** (those whose name begins with a dot) are excluded from note scans.
- **The `.granit/` directory** is excluded as well. It holds the cave's `config.yml` and `templates/` folder rather than notes — see [[configuration]] for what lives there.

Templates are deliberately kept outside the note tree in `.granit/templates/` and use their own separate, flat slug namespace, so a template name never collides with a note name. See [[templates]] for details.

# Next steps

- Write your first notes with [[notes-and-markdown]].
- Connect notes together using [[wiki-links]].
- Adjust cave-local settings in [[configuration]].
