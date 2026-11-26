---
id: baa.jq
title: "Fallback: assess alternative live-preview editor routes"
status: cancelled
priority: P3
created: "2026-11-26T12:16:03.418389Z"
updated: "2026-11-26T21:06:31.287933Z"
tags:
  - spike
  - gpui
  - editor
depends_on:
  - baa.6x
  - baa.9s
  - baa.q8
---

## Summary

Only needed if `zorite-editor` fails gate G2, G3 or G4 with a blocker. Cancel this task if all three pass. Time-box to one session: decide whether another editor route is viable before calling the port a no-go.

## Acceptance Criteria

- [ ] The specific blocker(s) from the earlier tasks are restated
- [ ] Each route below gets a short verdict (viable / not viable / unknown) with the reason:
  - Patch or fork `zorite-editor` (MIT) to fix the blocker; estimate the size of the change and whether upstream would take it
  - Build live preview on GPUI Kit's `Editor` using its decoration APIs (the Dotnote app has an Obsidian-style Live mode on GPUI Kit; read its source to see how)
  - `gpui-kit-markdown-editor` (block-based; known to normalize markdown on save and to lack wiki-link extension points, so only viable if those are acceptable)
  - Keep CodeMirror: stay on Tauri and add live-preview decorations to `js/editor.ts`, giving up the toolkit goals but meeting the editing goal
- [ ] A recommendation for the write-up task

## Implementation Notes

- Reading and small experiments only; do not start building a custom editor in the spike. Velotype's native block editor is roughly 37,000 lines of Rust, which is the scale of a from-scratch attempt.
- License check for anything read or copied: Zorite app is GPL-3.0 (its editor crate is MIT), Dotnote is MIT, GPUI Kit is Apache-2.0.

## Testing

- None; the verdict table is the deliverable

## Cancelled (2026-11-26)

Not needed. G3 and G4 pass by the owner's verdict. G2 fails as written, but not because of an editor blocker: most of it is the backend's existing frontmatter rewrite on save (the Tauri app does the same), and the editor's own two failures (a `$$` fence sharing a line with words is split; Enter in a CRLF note writes LF) are fixable bugs, not reasons to change editor. The owner called a go.

