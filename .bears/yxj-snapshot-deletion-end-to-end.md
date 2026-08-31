---
id: yxj
title: Snapshot deletion end-to-end
status: open
priority: P2
created: "2026-08-31T11:07:30.397820Z"
updated: "2026-08-31T11:07:30.397820Z"
tags:
  - server
  - api
  - ui
parent: bb4
---

## Summary
`DELETE /api/v1/backups/{id}` removes the object from the store and the row from postgres, plus a delete button with confirmation in the settings snapshot list. One focused commit covering server + client + UI.

## Acceptance Criteria
- [ ] Endpoint deletes S3 object then row; 404 for unknown/foreign ids; deleting a `pending` row is allowed (cleanup of abandoned uploads)
- [ ] `Storage::delete_object(key)` (internal client) + stub arm
- [ ] `BackupApiClient::delete_backup(id)` + `delete_backup` tauri command (BackupError conventions) + ipc wrapper
- [ ] Settings list row gets a delete action with an explicit confirm step; list refreshes after deletion

## Implementation Notes
- Missing S3 object during delete is not an error (already gone) — still delete the row
- UI: follow the existing snapshot table in src/app/settings/backup.rs; DaisyUI `btn btn-ghost btn-xs text-error` or similar, confirm via a small inline confirm (avoid a new modal component if possible)

## Testing
- `#[sqlx::test]`: delete removes row, 404 on foreign key's snapshot, idempotent object deletion via stub