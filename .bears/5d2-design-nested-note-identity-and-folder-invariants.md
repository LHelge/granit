---
id: 5d2
title: Design nested note identity and folder invariants
status: open
priority: P1
created: 2026-03-27T12:42:05.040118Z
updated: 2026-03-27T12:44:25.734535Z
depends_on:
- bjf
- c2z
- m9t
- duy
- vc2
- 3s4
- urx
- nst
- tx2
- n59
parent: ph5
---

## Summary

Define the exact note, path, and folder invariants for nested cave support before implementation begins. This task should settle the identity model, uniqueness rules, path semantics, and behavioral constraints that every later nested-folder change will rely on.

## Acceptance Criteria

- [ ] The note identity model is documented clearly enough to drive backend, shared-type, and frontend work.
- [ ] The filename-uniqueness rule across the entire cave is specified precisely.
- [ ] The relationship between filename, slug, relative path, folder path, and displayed title is defined.
- [ ] Expected behavior for moves, renames, folder creation, and duplicate-name rejection is agreed.
- [ ] The design is recorded in the epic or linked task notes so downstream tasks can implement against it directly.

## Implementation Notes

- Use the decisions already captured in the parent epic as hard constraints.
- Revisit current assumptions in src-tauri/src/cave/mod.rs, src-tauri/src/cave/note.rs, src/app/components/note_list.rs, and the planned shared IPC crate work.
- Be explicit about whether the slug remains filename-based and how relative paths are represented.

## Edge Cases

- Moving a note between folders without changing its filename.
- Renaming a note to an already-used filename elsewhere in the cave.
- Notes at the cave root versus nested subfolders.
- Future wiki-link resolution with globally unique filenames.

## Testing

- Design-only task: acceptance is a complete and unambiguous design, not code changes.
