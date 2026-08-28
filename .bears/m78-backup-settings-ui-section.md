---
id: m78
title: Backup settings UI section
status: done
priority: P2
created: "2026-08-28T12:46:02.771722Z"
updated: "2026-08-28T13:14:40.967221Z"
depends_on:
  - "2ws"
parent: psq
---

SettingsSection::Backup + settings/backup.rs (URL/API-key fields, passphrase set block, Back up now with stage spinner, snapshot list), SettingsForm + on_save mapping, ipc.rs wrappers + listen_backup_* listeners, granit-ui dep on granit-api. Commit: feat(backup).