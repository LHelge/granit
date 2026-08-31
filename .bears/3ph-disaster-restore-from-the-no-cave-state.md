---
id: "3ph"
title: Disaster restore from the no-cave state
status: open
priority: P2
created: "2026-08-31T11:08:06.428469Z"
updated: "2026-08-31T11:08:06.428469Z"
tags:
  - ui
depends_on:
  - pkx
parent: bb4
---

## Summary
The lost-machine flow: restore a cave on a machine with no cave open and no saved credentials. Granit has no welcome screen — the no-cave state is the empty reader plus the CaveSelector footer (src/app/explorer/cave_selector.rs), so add a "Restore from backup" affordance there (e.g. in the empty main area or beside the cave selector).

## Acceptance Criteria
- [ ] Reachable with no cave open; dialog collects backend URL + API key, lists that key's snapshots via the credential-override command
- [ ] Pick snapshot → pick empty target folder → passphrase prompt (no key file exists) → restore with progress → restored cave opens
- [ ] Errors (bad URL, bad key, wrong passphrase) surface in the dialog without closing it

## Implementation Notes
- Reuse the restore engine's credential-override commands from the engine task; no new backend work
- A modal component exists (src/app/components/modal.rs, used by SettingsModal) — follow it
- Owner is less experienced with Leptos/Tailwind — keep the dialog simple and explicit, copy patterns from settings/backup.rs