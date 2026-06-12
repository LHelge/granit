---
id: s7d
title: Document and validate nested folder behavior
status: done
priority: P3
created: 2026-03-27T12:42:05.458718Z
updated: 2026-03-30T12:47:57.665348Z
depends_on:
- 8b3
parent: ph5
---

## Summary

Document the final nested-folder behavior and validate the end-to-end workflows once the feature work is complete. This closes the loop between the design rules, backend behavior, frontend UX, and contributor documentation.

## Acceptance Criteria

- [x] Documentation explains nested folders, global filename uniqueness, and filename-based titles accurately.
- [x] End-to-end manual validation covers the main nested-folder workflows.
- [x] Any implementation caveats or deferred limitations are documented clearly.

## Implementation Notes

- Update README and any relevant contributor instructions only after the implementation is stable.
- Cross-check the final behavior against the parent epic's product decisions.

## Testing

- Manual validation is required.
- Record any gaps that remain intentionally deferred.

## Validation Notes

All acceptance criteria from epic `ph5` checked against the implementation:

| Rule | Status |
|------|--------|
| Nested subdirs scanned recursively | ✅ `scan_recursive` in `cave/mod.rs` |
| Globally unique filenames enforced on scan | ✅ logs warning + skips duplicate slugs |
| Globally unique filenames enforced on create | ✅ `CaveError::SlugAlreadyExists` returned |
| Filename stem is slug and title | ✅ `note_meta_from_relative_path` uses `file_stem` |
| Frontmatter/headings do not override title | ✅ title not read from content |
| Hidden dirs and `.granit/` excluded | ✅ `starts_with('.')` check in `scan_recursive` |
| `create_note` accepts optional folder | ✅ `folder: Option<&Path>` |
| `create_folder` creates nested dirs | ✅ `std::fs::create_dir_all` |
| Sidebar displays tree with collapsible folders | ✅ `note_list.rs` `TreeNode` + `build_tree` |

### Deferred Limitations (documented in README)

- UI always creates notes at cave root; selecting a target folder during creation is not yet exposed in the UI.
- Wiki-link resolution (`[[slug]]`) is planned but not yet implemented.
