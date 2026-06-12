---
id: ruw
title: Add ModelInfo shared type to granit-types
status: done
priority: P1
created: 2026-03-29T00:06:28.789991735Z
updated: 2026-04-02T12:36:36.276530436Z
tags:
- core
parent: xd5
---

## Summary

Add a `ModelInfo` struct to `granit-types` so both frontend and backend share a common model representation.

## Acceptance Criteria

- [ ] `ModelInfo` struct with `id: String` and `name: Option<String>` fields
- [ ] Derives `Debug, Clone, Serialize, Deserialize`
- [ ] `display_name()` method returns `name` if set, otherwise `id`

## Implementation Notes

- File: `granit-types/src/lib.rs`
- Keep it minimal — just id and display name for now