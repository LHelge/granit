---
id: h7p
title: "Presentations: markdown slides with per-cave CSS templates in a separate window"
type: epic
status: open
priority: P2
created: "2026-09-08T22:01:48.564292980Z"
updated: "2026-09-08T22:11:06.645331357Z"
tags:
  - presentation
  - backend
  - frontend
---

## Goal

A note can be presented as slides on a second screen while the main window stays free for note-taking. Slides are rendered from the note's markdown by the backend, styled by a cave-local CSS template, and shown in a separate Tauri window.

## Agreed design

**Binding.** A note is presentable when its frontmatter has `presentation: <name>`. `<name>` is the file stem of a CSS file in `.granit/presentations/`. The field joins tags/icon/favorite in `Frontmatter` and the `update_note` command.

**Templates.** CSS only (no HTML shell). Flat directory `.granit/presentations/`; assets (logo, fonts) sit beside the CSS files and are shared between templates by relative path. Two defaults, `default-dark.css` and `default-light.css`, are seeded into the cave on first scan when the directory is missing. They are ordinary deletable files; no hidden built-ins.

**DOM contract (public, documented).** One `<section class="slide" data-index="n">` per page containing reader-compatible rendered markdown (same code-block classes). Exactly one slide has class `active`. A built-in base stylesheet, placed before the template, handles mechanics only: active-slide toggle, canvas scaling, overflow clipping, cursor auto-hide. Everything visual belongs to the template.

**Canvas and overflow.** Slides are laid out on a fixed logical canvas of 1280 x 720 CSS pixels and scaled with a transform to fit the viewport, centred and letterboxed, so a template renders identically on every screen. Content that does not fit a slide is clipped; there are no scrollbars anywhere in the presentation window. Canvas size is exposed to templates as CSS custom properties.

**Splitting.** Split on the pulldown-cmark thematic-break event (`---`, `***`, `___` all split; `---` inside code blocks or tables never does). Empty slides dropped. No separator = one slide. No automatic title slide. Frontmatter stripped.

**Content rules.** Wiki-links flatten to label text (as in export). Task checkboxes disabled (as in agent markdown). `{#id}` heading anchors stripped. Raw HTML sanitized (so HTML comments work as speaker notes). Template CSS not sanitized.

**Serving.** Backend-registered custom URI scheme with two routes: `granit://cave/<path>` serves files from the open cave root (also fixes note images in the reader, which currently do not load), `granit://presentation/...` serves the presentation page and template assets from `.granit/presentations/`. Both scoped by canonicalized paths, rejecting `..` and escaping symlinks.

**Window.** Standalone backend-rendered HTML page (no Leptos, no IPC capability) loaded in a single Tauri window with a fixed label. Starting while open replaces content and refocuses. Opens as a normal window; `F`/`F11` toggles fullscreen; arrows, space, PgUp/PgDn, Home/End, and left/right half-clicks navigate; Escape leaves fullscreen, second press closes. Slide index in URL hash. Note save or template save (when that template is shown) triggers a reload via webview eval; hash keeps position. Rename, delete, and cave close shut the window; navigating to another note does not.

**Template editing.** Templates tab in the explorer splits into two stacked sections (note templates / presentation templates), each with its own header and new button, no new tab icon. New `DocumentKind::Presentation` with read/save/rename/delete commands mirroring note templates. Editor opens it edit-only (reader toggle disabled) in a CodeMirror CSS language mode with wiki-link decorations, slug completion, and Tera mode off; no frontmatter/tag/icon panels. New templates seeded from the dark default. Rename/delete leave note references hanging (visible as broken in the dropdown and as a start error).

**Main-window UI.** Template dropdown in the frontmatter editor panel with a "None" entry and a broken-reference state. "Present" button in the editor header next to edit/save/copy, visible only when the field is set. Missing template at start time is an error toast, never a fallback. No agent tool.

## Out of scope for v1 (candidates for later)

Blank-screen keys (B/W), jump-to-slide by number, print-to-PDF stylesheet, opening the window without stealing focus, per-slide layout classes via heading attributes (`{.title}`), presenter view with notes and timer, remembering the presentation monitor, HTML template shells, in-app asset management, in-app CSS preview, rewriting references on template rename.

## Verification

Backend unit tests for splitting, page assembly (DOM contract incl. canvas properties), scheme path scoping, template discovery and seeding. Window behaviour verified manually: start, navigate, fullscreen, save keeps position, rename closes, oversized slide is clipped with no scrollbar.