---
id: 5d2
title: Design nested note identity and folder invariants
status: done
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

## Design Decisions (Settled)

### Identity model

- **Primary key** for all operations = `relative_path` — the POSIX-style path from the cave root (e.g. `notes/foo.md`, or just `foo.md` for root notes). This is what every backend command and IPC call uses to address a note.
- **Slug** = filename without `.md` extension (e.g. `foo`). Globally unique across the entire cave tree. Used for wiki-link resolution only.
- **Title** = slug (filename-derived), unchanged from current behaviour.

### Uniqueness rule

- No two `.md` files anywhere in the cave may share the same filename (slug). This is enforced at `create_note` and future `rename_note` / `move_note` time.
- The in-memory index (`Cave.notes`) maps `slug → absolute PathBuf`. Slug-uniqueness is the invariant; relative_path is derived from the absolute path.

### Path representation

- `NoteMeta.relative_path` carries the full relative path from cave root with forward slashes (e.g. `folder/sub/note.md`). For root notes this is just `note.md` (same as before).
- `NoteMeta.slug` stays filename-only (no directory component).
- IPC commands that previously took `name: String` (slug) are changed to take `path: String` (relative_path). The frontend always has the `NoteMeta` in hand, so it passes `meta.relative_path`.

### list_notes

- Returns a flat `Vec<NoteMeta>` with `relative_path` fully populated for all notes recursively. The frontend builds the display tree from these flat data.

### Folder operations

- New command `create_folder(path: String)` creates a subdirectory within the cave. Path is relative from cave root.
- No explicit folder-delete command in this iteration (deferred).

### Backend index

- `Cave.notes` remains `HashMap<String, PathBuf>` keyed by slug. Recursive scan populates it.
- `scan_dir` becomes recursive, visiting subdirectories (skipping `.granit/` and hidden dirs).
- Duplicate slug detection on scan: log a warning, skip the duplicate (first one wins).

### Error variants

- No new error variants needed for this phase; `CaveError::AlreadyExists` and `CaveError::NotFound` continue to carry the slug string.

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
