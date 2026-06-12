---
id: u4r
title: "Wiki content: Getting Started (installation, getting-started, cave-rules)"
status: open
priority: P1
created: "2026-06-12T11:37:44.520616519Z"
updated: "2026-06-12T11:37:44.520616519Z"
tags:
  - docs
  - content
depends_on:
  - jfv
parent: sm3
---

## Summary

The three `category: Getting Started` wiki pages, expanded from README.md.

## Acceptance Criteria

- [ ] `wiki/installation.md` — download/install per OS from GitHub releases, Linux packaging caveats (AppImage self-updates; deb/rpm manual), auto-update behavior.
- [ ] `wiki/getting-started.md` — opening your first cave, what a cave is, the `.granit/` directory, tour of the three-pane UI. (Replaces/extends the scaffold stub.)
- [ ] `wiki/cave-rules.md` — identity model: globally unique filenames across the cave, filename stem = title/slug, frontmatter does not override title, hidden dirs and `.granit/` excluded from scans.
- [ ] Cross-linked: installation ↔ getting-started ↔ cave-rules, links into [[notes-and-markdown]], [[configuration]] etc. (targets must exist by the time the epic merges — coordinate with sibling tasks; `aphid build` enforces).

## Implementation Notes

- Source: README.md "Cave rules" + features list; CLAUDE.md "Cave model".
- Authoring rules in `.claude/skills/aphid-content/SKILL.md`: title in frontmatter only, body sections start at `#`, use GitHub alerts for caveats (e.g. `> [!IMPORTANT]` on unique-filename rule).