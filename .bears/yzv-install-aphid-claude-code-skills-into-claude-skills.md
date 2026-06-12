---
id: yzv
title: Install aphid Claude Code skills into .claude/skills/
status: done
priority: P2
created: "2026-06-12T11:35:57.247999233Z"
updated: "2026-06-12T11:53:50.125761205Z"
tags:
  - docs
  - tooling
parent: nbd
---

## Summary

Copy the two repo-local Claude Code skills from the aphid repository into this repo so all docs authoring and theming work has the reference material loaded automatically.

## Acceptance Criteria

- [ ] `.claude/skills/aphid-content/SKILL.md` exists, content identical to upstream
- [ ] `.claude/skills/aphid-theme/SKILL.md` exists, content identical to upstream

## Implementation Notes

- Sources:
  - https://raw.githubusercontent.com/LHelge/aphid/main/.claude/skills/aphid-content/SKILL.md
  - https://raw.githubusercontent.com/LHelge/aphid/main/.claude/skills/aphid-theme/SKILL.md
- Each skill directory upstream contains only SKILL.md — no extra reference files to copy.
- Keep frontmatter (`name`, `description`) unchanged so trigger conditions match.