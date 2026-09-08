---
id: bh6
title: "Presentation window: start command, page route, navigation, reload and close rules"
status: open
priority: P2
created: "2026-09-08T22:02:44.217136182Z"
updated: "2026-09-08T22:11:23.191388561Z"
tags:
  - presentation
  - backend
depends_on:
  - aub
  - bss
  - "4ff"
parent: h7p
---

Open and manage the presentation window from the backend. No Leptos in this window and no IPC capability; it is a plain webview showing a backend-rendered page.

Start command `start_presentation(slug)`:
- Read the note, require `frontmatter.presentation`, and require the named template file to exist. Missing field or missing template returns a `CaveError` that the frontend shows as a toast naming the template. Never fall back to a default.
- Create a `WebviewWindow` with a fixed label (e.g. `presentation`) loading `granit://presentation/<slug>` (page route added to the scheme handler; it returns the assembled page from the markdown module with the template CSS linked). If the window already exists, navigate it to the new URL and focus it. One presentation window at a time.
- Opens as a normal window, not fullscreen. Add a capability entry for the window label with the minimum needed (likely none beyond core defaults).

Navigation script (inline in the page):
- Arrow keys, space, PgUp/PgDn, Home/End move between slides; click/touch on the right half advances, left half goes back (clicker-friendly).
- `F` and `F11` toggle fullscreen via the Fullscreen API. Escape leaves fullscreen; a second Escape closes the window (`window.close()` or an equivalent that works in the Tauri webview).
- Current slide index kept in the URL hash and restored on load, so reloads keep position.
- Toggle the `active` class on `section.slide`; no other DOM manipulation.
- **Canvas scaling.** On load and on `resize`, compute `min(viewportWidth / 1280, viewportHeight / 720)` and write it to the CSS custom property the base CSS uses for its `transform: scale()`. The canvas size itself comes from the base CSS custom properties defined in the rendering task; do not hardcode it twice.
- Never introduce scrolling: no scrollable containers, and key handling must `preventDefault` on space and the page keys so the webview does not scroll.

Reload and close rules (backend):
- After a note save, if the presentation window is showing that slug, `eval("location.reload()")` on it.
- After a presentation template save, if the shown note uses that template, reload likewise.
- On note rename or delete of the shown slug, and on cave close, close the window.
- Track the currently shown slug and template on `AppState` for these checks.

Verification is manual (see the run skill): start, navigate, fullscreen, save keeps position, template save reloads, rename closes, resize the window and confirm the slide scales with letterboxing and an oversized slide clips with no scrollbar. Unit tests only for the page route's error paths where they are cheap.