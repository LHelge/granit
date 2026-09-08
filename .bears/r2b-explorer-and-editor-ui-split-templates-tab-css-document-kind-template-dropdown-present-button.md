---
id: r2b
title: "Explorer and editor UI: split Templates tab, CSS document kind, template dropdown, Present button"
status: open
priority: P2
created: "2026-09-08T22:03:04.991561433Z"
updated: "2026-09-08T22:03:04.991561433Z"
tags:
  - presentation
  - frontend
depends_on:
  - "4ff"
  - jqd
  - bh6
parent: h7p
---

Frontend work in the Leptos app. The repo owner is less experienced with Leptos/Tailwind/DaisyUI, so keep changes explicit and follow existing patterns closely.

Explorer (`src/app/explorer/templates.rs`):
- Split the Templates tab into two stacked sections, "Note templates" and "Presentation templates", each with its own header row (title + directory subtitle) and its own new button. No new tab icon. Each section scrolls independently and shares the vertical space.
- Presentation rows mirror note-template rows: click opens in the editor, delete button with the same confirmation behaviour. Rename happens via the editor title as for note templates.
- `AppCtx` gains a `presentations` signal and a refresh, like `templates`.

Editor:
- New `DocumentKind::Presentation` with its persist path (`save_presentation`), rename via title, and doc-key handling, following the `Template` arm in `src/app/editor/mod.rs`.
- Edit-only: the reader/edit toggle is disabled for this kind and it always opens in edit mode. No render request.
- Create the CodeMirror instance in CSS language mode (from the CSS language-mode task). Font follows the markdown font setting.
- No frontmatter panel, tag editor, or icon picker for this kind. Header shows name, save state, and the standard save action only.
- Autosave behaves as for other kinds.

Frontmatter panel (`src/app/editor/frontmatter.rs`):
- Add a "Presentation" dropdown (DaisyUI `select`) listing presentation templates plus a "None" entry. Selecting writes the `presentation` field through the same save path as tags. If the note's value matches no template, show it as a broken entry (e.g. "corporate (missing)") rather than clearing it.

Editor header:
- Add a "Present" button next to edit/save/copy, visible only when the active note has a `presentation` value. Calls `start_presentation(slug)`; errors surface as toasts via `push_error`.

Wasm test only if the dropdown gains logic worth testing.