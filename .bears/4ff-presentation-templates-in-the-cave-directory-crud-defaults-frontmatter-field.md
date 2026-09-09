---
id: "4ff"
title: "Presentation templates in the cave: directory, CRUD, defaults, frontmatter field"
status: open
priority: P2
created: "2026-09-08T22:02:20.550328070Z"
updated: "2026-09-09T07:58:36.288884Z"
tags:
  - presentation
  - backend
parent: h7p
---

Cave-side support for presentation templates in `src-tauri/src/cave/`.

Directory and index:
- `.granit/presentations/` holds `<name>.css` files plus shared assets (logo, fonts). Flat, no subdirectories.
- `Cave` gains a `presentations` index (slug -> path) populated in `ensure_scanned()`, listing only `.css` files. Assets are not indexed.
- Seed `default-dark.css` and `default-light.css` into the directory when the directory does not exist. Existing directories are never touched, so deleted defaults stay deleted. Defaults are deliberately plain: title style, readable body text, small slide number in a corner, and a comment header pointing to the DOM contract.
  - **Decision (2026-09-09):** seeding happens in the open-cave flow (`open_cave_at` / restore), next to the system prompt seeding, best-effort with a warning on failure — not inside `ensure_scanned`, so `Cave::open` in agent tools and tests stays side-effect free.

Operations mirroring note templates (`templates.rs`), with the same name validation:
- `list_presentations() -> Vec<DocumentMeta>`
- `read_presentation(slug) -> Document` (raw CSS as content)
- `create_presentation(name)` seeded from the built-in dark default, `untitled` collision handling as for note templates
- `save_presentation(slug, content)` atomic write
- `rename_presentation(old, new)`
- `delete_presentation(slug)`
Rename and delete do not touch notes that reference the template.

Frontmatter:
- Add `presentation: Option<String>` to `granit_types::Frontmatter`, preserved on round-trip like `favorite` and `icon`.
- Thread it through `update_note` (command, IPC wrapper args) alongside tags/icon/favorite. `Some("")` clears the field, `None` preserves it (same convention as `icon`).
- **Decision (2026-09-09):** a note created from a note template inherits the template's `presentation` field, the same way tags and icon are inherited.

IPC:
- One `#[tauri::command]` per operation above, registered in `lib.rs`, plus typed wrappers in `src/app/ipc.rs`.

Tests: scanning finds CSS and ignores assets, seeding on missing directory only, each CRUD operation, name validation, frontmatter round-trip with the new field, template inheritance of the field.