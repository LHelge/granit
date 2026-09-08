---
id: aub
title: Custom URI scheme serving files from the open cave
status: open
priority: P2
created: "2026-09-08T22:02:02.408190287Z"
updated: "2026-09-08T22:02:02.408190287Z"
tags:
  - presentation
  - backend
parent: h7p
---

Register a custom URI scheme on the Tauri app (`register_uri_scheme_protocol`) with two routes:

- `granit://cave/<relative-path>` serves any file inside the open cave root. Used by note images in the reader (relative `<img src>` paths do not load today) and later by the presentation page.
- `granit://presentation/<relative-path>` serves files from `.granit/presentations/` (template CSS and its assets). The presentation page route itself is added in the window task.

Requirements:
- Resolve against the cave held in `AppState`; return 404-style responses when no cave is open or the file is missing.
- Path scoping: canonicalize and verify the result stays under the allowed root. Reject `..` segments and symlinks that escape. Tests for traversal attempts, outside-root files, and normal files.
- Content type from extension for the common cases (css, svg, png, jpg, gif, webp, woff, woff2, ttf, otf, html).
- Rewrite relative image sources in reader-rendered markdown to the `granit://cave/` route so note images load. Absolute http(s) URLs untouched.
- Add the scheme to the capability/CSP setup as needed so both the main window and a future presentation window can load from it.

Standalone value: fixes local images in the reader. Prerequisite for the presentation window.