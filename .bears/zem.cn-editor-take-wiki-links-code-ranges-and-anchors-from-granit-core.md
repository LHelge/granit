---
id: zem.cn
title: "Editor: take wiki-links, code ranges and anchors from granit-core"
status: open
priority: P2
created: "2026-11-27T01:10:30.400904439Z"
updated: "2026-11-27T01:10:30.400904439Z"
tags:
  - gpui
  - port
  - frontend
  - editor
  - core
depends_on:
  - zem.4k
---

## Summary

`granit-app/src/note_view/wiki_links.rs` (zem.4k) finds wiki-links, code ranges (fences, indented code, code spans) and heading anchors with a hand-rolled CommonMark approximation. granit-core already finds these exactly with pulldown-cmark offsets (`Markdown::wiki_link_targets`, `anchor_ids`, `rename_wiki_links`), and CLAUDE.md makes the backend the single source of truth. The two disagree, e.g. a setext heading with `{#id}` is an anchor to the core but the editor does not scroll to it; `[[x]]` in an HTML block or an indented block in a blockquote is marked differently from the core's backlinks and broken-link list.

## Acceptance Criteria

- [ ] A granit-core function returns, for a body: wiki-link byte ranges with targets, code byte ranges, and anchor heading offsets; the editor uses it for broken-link marking, link clicks, completion suppression and anchor scrolling
- [ ] Only the completion query, completion edit and ranking stay in the app
- [ ] Setext headings with `{#id}` scroll on anchor navigation
- [ ] `LinkIndex::from_cave` (`wiki_links.rs:452-466`) is no longer quadratic in the number of anchors (it calls `resolve_link` per anchor and is rebuilt on every `NotesChanged`, i.e. every autosave)

## Testing

- Core tests for the new function over code/HTML/blockquote/setext cases; the editor's existing link and completion tests still pass