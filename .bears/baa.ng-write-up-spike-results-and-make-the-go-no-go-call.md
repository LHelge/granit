---
id: baa.ng
title: Write up spike results and make the go/no-go call
status: open
priority: P1
created: "2026-11-26T12:16:13.865441Z"
updated: "2026-11-26T12:16:13.865441Z"
tags:
  - spike
  - gpui
  - docs
depends_on:
  - baa.6x
  - baa.9s
  - baa.q8
  - baa.sp
  - baa.pb
---

## Summary

Collect the spike's findings against gates G1-G6, decide go or no-go on the GPUI port together with the owner, and, on a go, outline the port epic. The write-up is the only spike output that outlives the branch.

## Acceptance Criteria

- [ ] One results section per gate (G1-G6 in the epic body) with pass / fail and the evidence: round-trip output, wiki-link verdict, editing-feel table, measurement table, streaming notes
- [ ] The fallback task's recommendation is included if it ran
- [ ] Dependency notes: which `gpui-pre`, `gpui-kit` and `zorite-editor` versions were used, any version juggling needed, and whether the official standalone GPUI release has landed in the meantime
- [ ] Tauri coupling found in backend modules while linking `granit_lib` (sizes the core extraction)
- [ ] Explicit decision recorded: go, no-go, or go with conditions
- [ ] On a go: a draft list of port work items, at least: extract a Tauri-free core crate from `src-tauri/`; app shell and explorer; editor integration; agent chat; settings with Catppuccin light/dark; templates and CSS-free leftovers; todos/tags/calendar/search panels; backup UI; updater replacement; bundling, signing and notarization; Windows and Linux builds; removing Node/Trunk/wasm from CI and hooks; docs site updates. Created as a new Bears epic only after the owner agrees
- [ ] On a no-go: whether to do CodeMirror live preview in the current app instead

## Implementation Notes

- Put the write-up in this task's body or in the epic body; do not add a document to the repo unless the owner asks for one.
- The `spike/gpui` branch is not merged. If the port goes ahead, it starts from a clean branch and reuses code selectively.
- If the fallback task (`baa.jq`) is still open because a gate failed, finish it before this one.

## Testing

- None