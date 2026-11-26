---
id: bgy
title: Bring the user docs in line with the app
type: epic
status: open
priority: P2
created: "2026-11-26T23:41:09.038880167Z"
updated: "2026-11-26T23:41:09.038880167Z"
tags:
  - docs
---

## Scope

A November 2026 audit compared every page in `docs/content/` against the code. All the links resolve and the config defaults are right. But many pages state things the code contradicts, leave out shipped features, and repeat each other. This epic fixes the wiki pages, the home and download pages, and `backend/README.md`. `CLAUDE.md` and the root `README.md` were already fixed alongside the `implement-epic` skill.

Each task owns a group of pages, so the tasks are independent and can run in parallel. Every finding below cites the code it was checked against. The implementer re-checks each claim against the code before writing; it does not copy the finding blindly.

Note: `zem.sp` (docs for the GPUI app) will revisit UI wording after the GPUI port. This epic documents the current Leptos app.

## Acceptance Criteria

- [ ] Every page states only what the code does: labels, shortcuts, defaults and behaviour are checked against the source.
- [ ] Shipped features named in the tasks are documented once, in the page that owns them, and linked from elsewhere.
- [ ] No fact is explained on two pages. Fonts, auto-update, the backup server, checkbox behaviour and the calendar each have one home.
- [ ] Pages open with the point, not "This page covers…". "Related pages" lists that repeat links already in the body are removed.
- [ ] `aphid build -o <scratch>` from `docs/` succeeds with every wiki-link resolving.