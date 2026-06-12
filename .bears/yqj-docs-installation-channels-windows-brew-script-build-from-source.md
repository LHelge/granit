---
id: yqj
title: "Docs: installation channels (windows, brew, script, build from source)"
status: open
priority: P2
created: "2026-06-12T16:30:08.748465079Z"
updated: "2026-06-12T16:30:23.087621857Z"
tags:
  - docs
  - content
depends_on:
  - "7gm"
  - gfq
  - vuk
parent: bym
---

## Summary

Update the docs site and README for all installation channels.

## Acceptance Criteria

- [ ] `docs/content/wiki/installation.md`: Windows section (.exe/.msi, SmartScreen `> [!NOTE]` since builds are unsigned, auto-update works), Homebrew section (`brew install --cask lhelge/tap/granit`), install-script section with the curl one-liner, build-from-source section (git clone, npm ci && npm run build, cargo tauri build + prerequisites)
- [ ] `docs/content/pages/download.md`: artifact table gains Windows; quick brew + curl one-liners
- [ ] README install section mentions the new channels briefly, linking to the docs
- [ ] `aphid build` passes (wiki-links intact)

## Implementation Notes

- Depends on the other four tasks for accurate commands/asset names.
- Per cave/aphid authoring rules: title from frontmatter, sections start at `#`, alerts for caveats.