---
id: exw
title: 'Modal: Convert settings modal to dialog/modal-box'
status: done
priority: P1
created: 2026-04-04T21:14:30.316271720Z
updated: 2026-04-04T21:50:28.702314825Z
tags:
- frontend
depends_on:
- 54k
- 8wf
parent: 9fd
---

## Summary

Convert the settings modal from a hand-rolled fixed-position backdrop + panel to a DaisyUI `modal` using the HTML `<dialog>` element. This gives proper focus trapping and native accessibility.

## Acceptance Criteria

- [ ] Settings modal uses `<dialog class="modal">` element
- [ ] Modal panel uses `modal-box` class
- [ ] Backdrop click-to-close uses `modal-backdrop`
- [ ] Footer actions area uses `modal-action`
- [ ] Modal open state controlled via `.showModal()` / `method="dialog"`
- [ ] Existing sidebar nav + content pane layout preserved inside modal-box

## Implementation Notes

- DaisyUI modal syntax: `<dialog class="modal"><div class="modal-box">{content}</div><form method="dialog" class="modal-backdrop"><button>close</button></form></dialog>`
- The `showModal()` call needs to happen from Leptos — use a `NodeRef<Dialog>` and call `.show_modal()` in an Effect
- Cancel/Save buttons: cancel can use `method="dialog"` form, save still needs custom handler

## Files to Modify

- `src/app/components/settings/mod.rs` — modal structure
- `src/app/mod.rs` — modal trigger (may need to pass NodeRef instead of bool signal)
