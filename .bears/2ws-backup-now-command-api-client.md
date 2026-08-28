---
id: "2ws"
title: backup_now command + API client
status: done
priority: P2
created: "2026-08-28T12:45:57.356667Z"
updated: "2026-08-28T13:14:39.273577Z"
depends_on:
  - kq2
  - q4z
parent: psq
---

backup/client.rs (reqwest: create/complete/list/upload-PUT), commands/backup.rs (backup_now with backup:progress/done/error events + BackupGuard, set_backup_passphrase, has_backup_key, list_backups), registration in lib.rs, BackupConfig on AppConfig + BackupStage/BackupProgress in granit-types. Commit: feat(backup).