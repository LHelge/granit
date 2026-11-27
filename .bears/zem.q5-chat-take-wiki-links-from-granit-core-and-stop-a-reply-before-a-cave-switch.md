---
id: zem.q5
title: "Chat: take wiki-links from granit-core and stop a reply before a cave switch"
status: open
priority: P2
created: "2026-11-27T07:09:14.254219511Z"
updated: "2026-11-27T07:09:14.254219511Z"
tags:
  - gpui
  - port
  - frontend
  - agent
  - core
depends_on:
  - zem.up
  - zem.cn
---

## Summary

From the epic review of the GPUI port:

- `granit-app/src/chat/wiki_links.rs:19-23`: the chat's streaming rewriter decides what is a wiki-link with its own CommonMark approximation ("close to CommonMark but not exactly"), the kind of second parser zem.cn removed from the editor. Example: `- item\n\n      [[x]]` is indented code inside a list item for pulldown-cmark but becomes a clickable link in chat. Keep the streaming hold-back of undecided text, but decide links and code from `Markdown::link_spans` over the reply so far (cost is small, see the editor's `SpanCache`).
- Agent tools hold the same `SharedCave` Arc that `install_cave` swaps; the chat drops its stream only in its `Opened` handler, after the swap, so a tool call landing in that window runs against the new cave. Cancel the running reply in the workspace's `before_install` hook (or before the switch starts).

## Acceptance Criteria

- [ ] Chat links and code skipping agree with the reader (`link_spans`), including the list-indented-code case, while streaming
- [ ] A cave switch stops a running reply before the core swaps caves

## Testing

- Canned-stream tests for the list case and chunk splits; a test that a reply in flight is cancelled before `install_cave`