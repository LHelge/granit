---
id: qa7
title: Use HashMap Entry API in cave scanning
status: open
priority: P2
created: 2026-04-04T21:41:19.399196196Z
updated: 2026-04-04T21:41:19.399196196Z
tags:
- idiomatic
- backend
- cave
parent: drc
---

## Summary
Cave scanning uses `#[allow(clippy::map_entry)]` to suppress the lint about get-then-insert. Use the idiomatic `Entry` API instead.

## What to do
- Replace `if let Some(existing) = notes.get(&slug)` + `notes.insert()` with `match notes.entry(slug) { Occupied => Err(..), Vacant(v) => { v.insert(..) } }`
- Remove the `#[allow(clippy::map_entry)]` attribute

## Files
- `src-tauri/src/cave/mod.rs` — `scan_recursive()` function