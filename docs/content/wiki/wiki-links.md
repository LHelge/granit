---
title: Wiki-links
category: Notes & Writing
tags: [wiki-links, linking, navigation]
---

Wiki-links connect your notes into a web of references. Granit resolves them by filename across the entire cave rather than by relative path, so a link works no matter where the target note lives in the folder tree. This page covers link syntax, heading anchors, broken links, and backlinks. For how filenames define a note's identity, see [[cave-rules]]; for how links render in the reader, see [[notes-and-markdown]].

# Linking to a note

A wiki-link wraps a note's slug (its filename without the `.md` extension) in double brackets. To link to a note named `Volvo`, you would write `[[Volvo]]` in your Markdown.

To show different link text, add a label after a pipe. Writing `[[Volvo|the car]]` links to the `Volvo` note but displays "the car".

> [!IMPORTANT]
> Links resolve by **filename across the whole cave**, not by relative path. Because filenames are globally unique (see [[cave-rules]]), the slug alone always identifies exactly one note, regardless of which subfolder it sits in.

# Heading anchors

Any heading can be made a link target by giving it an explicit id with a pandoc-style attribute. Writing a heading as `# Volvo {#volvo}` registers the anchor `volvo`.

What makes Granit distinctive is that anchor ids live in the **same global namespace as note filenames**. Once a heading declares an id, you can link to it with a plain wiki-link — `[[Volvo]]` — exactly as if it were a note. The link resolves to that heading and scrolls the reader to it.

Plain headings without a `{#id}` attribute are not link targets.

> [!WARNING]
> Anchor ids must be globally unique against both note slugs and all other anchors. A duplicate id causes Granit to refuse to open the cave until the conflict is resolved.

# Broken links

A wiki-link whose slug matches no note and no anchor is a broken link. Granit styles broken links distinctly in the reader so you can spot them at a glance — for example, after renaming or deleting a note that other notes still reference.

# Backlinks

Granit tracks backlinks: for any note, it knows which other notes link to it. Backlinks are maintained automatically as you edit, so the connections between notes stay current without manual bookkeeping.

# Related pages

- [[notes-and-markdown]] — how links render and how notes are edited.
- [[cave-rules]] — globally unique filenames and the slug model.
- [[explorer]] — search and browse to find notes to link.
- [[templates]] — scaffold notes that follow your linking conventions.
