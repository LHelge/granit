---
id: dnc
title: Fix TextareaState char/byte index handling
status: done
priority: P2
created: 2026-03-31T16:33:19.413929892Z
updated: 2026-03-31T19:35:08.182082285Z
tags:
- frontend
- bug
parent: 4cm
---

## Summary
TextareaState in writer.rs mixes `char_indices()` and byte offsets for cursor positioning. Multi-byte UTF-8 characters at cursor boundaries could cause off-by-one bugs or panics.

## Acceptance Criteria
- [ ] All cursor math uses a consistent indexing scheme (either chars or bytes, not mixed)
- [ ] Add tests with multi-byte UTF-8 strings (emoji, CJK, combining chars) at cursor
- [ ] No panics on any valid UTF-8 input + cursor position combination

## Implementation Notes
- Files: `src/app/components/editor/writer.rs` (TextareaState, lines ~21-65)
- The JS textarea `selectionStart`/`selectionEnd` use UTF-16 code units — need to convert properly
- Consider using a helper that maps UTF-16 offsets to byte offsets safely
- Property-based tests (proptest crate) would be ideal but manual edge cases work too

## Testing
- Test: cursor after emoji 🎉 (4 bytes, 2 UTF-16 units)
- Test: cursor in middle of CJK text
- Test: cursor at start/end of string