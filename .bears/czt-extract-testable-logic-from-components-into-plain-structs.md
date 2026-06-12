---
id: czt
title: Extract testable logic from components into plain structs
status: done
priority: P1
created: 2026-03-31T16:40:34.555211160Z
updated: 2026-03-31T19:35:08.173376632Z
tags:
- frontend
- refactor
parent: 82y
---

## Summary
Pull pure logic out of components into standalone structs with `impl` blocks, testable with normal `#[test]` (no WASM needed). Start with `TextareaState` which has the highest bug risk.

## Acceptance Criteria
- [ ] `TextareaState` and cursor manipulation logic extracted to a standalone module
- [ ] List prefix detection (`detect_prefix`) extracted and testable
- [ ] URL detection logic extracted and testable
- [ ] Each extracted module has `#[cfg(test)] mod tests` with unit tests
- [ ] Tests cover UTF-8 edge cases (emoji, CJK) for cursor logic
- [ ] `cargo test -p granit-ui` runs these tests natively (no WASM)

## Implementation Notes
- Files: `src/app/components/editor/writer.rs` → extract to `src/text_editing.rs` or similar
- `TextareaState` is already a struct — just needs to be moved to a non-component module and given `#[cfg(test)]` tests
- Note: the struct currently depends on `web_sys::HtmlTextAreaElement` for `From` impl — the pure logic (string manipulation, cursor math) should be separated from the DOM binding
- This overlaps with task `dnc` (fix char/byte index handling) — tests here will catch those bugs

## Testing
- `cargo test -p granit-ui` passes with new unit tests
- At least 10 test cases for cursor positioning including multi-byte chars