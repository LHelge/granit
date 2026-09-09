---
id: bss
title: Slide splitting and presentation rendering in the markdown module
status: done
priority: P2
created: "2026-09-08T22:02:11.244147030Z"
updated: "2026-09-09T08:08:32.021967Z"
tags:
  - presentation
  - backend
parent: h7p
---

Add a presentation render mode to `Markdown` in `src-tauri/src/markdown/`.

Splitting:
- Split the pulldown-cmark event stream on `Event::Rule` (thematic break). `---`, `***`, `___` all split; a `---` inside a fenced code block or a table separator row never does.
- Drop empty slides (leading separator, consecutive separators, trailing separator).
- No separator gives a one-slide presentation. No automatic title slide. Frontmatter stripped as usual.

Per-slide content rules:
- Wiki-links flatten to their label text (reuse the export renderer behaviour).
- Task-list checkboxes rendered disabled (as agent-rendered markdown).
- `{#id}` heading attributes stripped; headings render plain.
- Raw HTML goes through the existing sanitizer.
- Code blocks keep the same class names as the reader so templates can reuse highlighting styles.
- Relative image sources rewritten to the `granit://cave/` route (same rewrite as the reader).

Page assembly (`render_presentation(template_css_href) -> String` or similar):
- Complete standalone HTML document: base mechanics CSS inline, then `<link>` to the template CSS on the `granit://presentation/` route, then one `<section class="slide" data-index="n">` per slide, first one `active`, then the navigation script (owned by the window task; this task provides the slot).
- Base CSS handles only mechanics, never colours, fonts, or spacing:
  - Showing the active slide and hiding the others by default.
  - **Fixed logical canvas.** Every slide is laid out at a fixed size, 1280 x 720 CSS pixels (16:9), and the base CSS scales that canvas with a `transform: scale()` to fit the viewport, centred and letterboxed. Templates design against known pixels and the result is identical on every screen. The scale factor is computed by the navigation script on load and resize and applied via a CSS custom property so the base CSS stays declarative. The canvas size is exposed as CSS custom properties (`--slide-width`, `--slide-height`) so templates can reference it.
  - **Overflow is clipped.** `overflow: hidden` on the slide and on the document so content that does not fit is cut off rather than scrolling. No scrollbars anywhere in the presentation window. The letterbox area outside the canvas is the template's to colour via the `html`/`body` background.
  - Hiding the cursor after a few seconds of inactivity.

Tests: separator variants, separators inside code blocks and tables, empty-slide dropping, single-slide case, link flattening, disabled checkboxes, anchor stripping, and a DOM-contract test asserting base CSS precedes template CSS, one section per slide, first active, and the canvas custom properties present.