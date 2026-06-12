---
id: sq6
title: Eliminate agent provider match duplication with macro
status: done
priority: P1
created: 2026-04-04T21:40:56.411585243Z
updated: 2026-04-04T22:40:52.485139466Z
tags:
- duplication
- backend
- agent
parent: drc
---

## Summary
Massive duplication in agent/mod.rs:
- `stream_with_history()`: 4 identical match arms (~60 duplicated lines)
- `list_models()`: 4 identical match arms (~40 lines)
- Four `build_*()` functions are 95% identical
- Manual `Clone` impl repeats the same pattern 4 times

## What to do
- Create a macro `dispatch_provider!` that expands into the match arms
- Factor shared builder logic (client → agent → wrap in enum) into a helper or macro
- Try `#[derive(Clone)]` — if rig agent types support it, remove manual impl
- For `list_models()`, consider extracting client construction into a helper

## Files
- `src-tauri/src/agent/mod.rs`

## Impact
Single highest-ROI task — eliminates ~150 lines of pure boilerplate