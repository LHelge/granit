---
id: vuk
title: Linux install script served from the docs site
status: open
priority: P2
created: "2026-06-12T16:30:03.148612883Z"
updated: "2026-06-12T16:30:03.148612883Z"
tags:
  - release
  - distribution
  - linux
  - docs
parent: bym
---

## Summary

`docs/static/install.sh` — a readable POSIX sh script installing the latest AppImage with desktop integration, served at https://granit.lhelge.se/static/install.sh.

## Acceptance Criteria

- [ ] Detects OS/arch; exits with a clear message on anything but x86_64 Linux (points macOS at brew/dmg, others at the releases page)
- [ ] Fetches the latest release via the GitHub API, downloads the AppImage to `~/.local/bin/granit`, chmod +x
- [ ] Installs an icon + `.desktop` file under `~/.local/share` so launchers pick it up; prints a PATH hint if `~/.local/bin` isn't on PATH
- [ ] Idempotent re-run = upgrade; `set -eu`, no bashisms, curl-or-wget fallback
- [ ] Script passes shellcheck

## Implementation Notes

- AppImage is the right artifact: it self-updates via the built-in updater afterward.
- No sha256 sums are published with releases (only Tauri updater .sig); v1 is HTTPS-trust — note it in a script comment. Verifying minisign sigs is a possible follow-up.
- Verify the AppImage asset naming pattern from the latest release.
- Mind the docs cache-bust convention only applies to theme.css; the script URL is fetched fresh by curl anyway.