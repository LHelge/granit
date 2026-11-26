---
id: baa.ng
title: Write up spike results and make the go/no-go call
status: done
priority: P1
created: "2026-11-26T12:16:13.865441Z"
updated: "2026-11-26T21:07:12.110256Z"
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

## Write-up (2026-11-26)

### Decision

**Go**, called by the owner on 2026-11-26 after using the spike: editing "feels great", streaming "looks awesome", wiki-links work, and the spike is obviously snappier than the Tauri app. Conditions carried into the port: the G2 round-trip failures below are fixed, and the zorite-editor fix (baa.ac) reaches a release or stays pinned to the fork.

### Gate results

| Gate | Result | Evidence |
|---|---|---|
| G1 Dependencies | pass, with gpui-kit 0.10 instead of 0.11 | baa.7f; one gpui-pre 0.3.12 for both crates |
| G2 Round-trip | fails as written; not an editor blocker | baa.6x |
| G3 Wiki-links | pass | baa.9s; owner: "works fine" |
| G4 Editing feel | pass, one gap | baa.q8; owner: "feels great" |
| G5 Numbers | pass on the GPUI numbers and the owner's judgement | baa.sp; no side-by-side Tauri table, by decision |
| G6 Streaming | pass | baa.pb; owner: "looks awesome" |

**G1.** gpui-kit 0.11 pins gpui-pre 0.3.14, which did not compile with zorite-editor 0.11.7 (through gpui-bidi 0.1.2), so the spike uses gpui-kit 0.10.0 with a single gpui-pre 0.3.12. Builds on macOS with cargo only.

**G2.** Editor round-trip on 389 notes: 387 byte-identical on load, including CRLF, BOM, lone CR, tabs, trailing whitespace, raw HTML, footnotes, tables and a 6,910-line note. Single edits stay local (typed character, checkbox click, table cell, alert fold, Enter in an LF list). Editor failures: a `$$` fence sharing a line with words is split onto its own lines on load, and Enter in a CRLF note writes a bare LF. Most of the gate's failure is the backend: `Cave::save_note` changes 385 of 389 files on an unedited save (re-stamped `modified_at`, reformatted `created_at`, dropped unknown keys and YAML comments, flow-style tags rewritten, CRLF frontmatter rewritten), and malformed or body-level `---` blocks are deleted. That behaviour is the same in the Tauri app today and is a port work item, not a GPUI finding.

**G3.** `[[note]]`, `[[note|label]]` and `{#id}` anchors render, navigate on click (anchors scroll to the heading), and `[[` completion of slugs and anchors is built in host code without patching the editor. Broken links are marked with a diagnostic underline because `SyntaxStyle` has one link colour. Gaps: indented code and multi-line inline code are not detected as code for completion; clicking a broken link does nothing.

**G4.** No blockers. One gap: a ```` ``` ```` fence turns into a rendered code card as soon as it is typed; the owner would prefer to edit it as markdown but can live with it. The checklist was not marked item by item.

**G5.** GPUI release build on an Apple M4: warm start to a painted small note 291 ms (median of 5), 1,337 ms into the 6,910-line note; idle physical footprint 103 MB in one process (peak 157 MB); idle CPU 0 %; release binary 30 MB (24 MB stripped). Clean release build 194 s, incremental 28-32 s; clean debug 110 s, incremental 5 s. The installed Tauri app is a 91 MB bundle with a 70 MB binary, plus its WebKit helper processes. The spike links all of `granit_lib`, so its binary and build times overstate a real port.

**G6.** GPUI Kit's `TextView` renders a streamed response incrementally (markdown formatting applies while streaming, which the current app does not do), follows the bottom, and delivers link clicks to host code. `[[wiki-links]]` work by rewriting them to `wiki:` links on the stream. Gaps: the rewriter ignores code spans, broken links cannot be styled differently, no `{#id}` anchor support.

### Editor robustness

zorite-editor 0.11.7 aborted the app when a press or an Up/Down key arrived between a content change and the next paint (baa.ac). Fixed in the owner's fork and sent upstream as packetThrower/zorite-editor#4 (issue) and #5 (PR); the spike is pinned to the fork branch. The spike also gives each opened note a fresh editor, which avoids the crash on note switches independently of the fix. Separately, one tree-sitter highlighter per language must be shared across editors, or opening a note with several code blocks recompiles the queries and freezes the app.

### Fallback

baa.jq was not needed and is cancelled: G3 and G4 pass, and G2 has no editor blocker.

### Dependencies

Used: gpui-pre 0.3.12 (aliased as `gpui`), gpui-kit 0.10.0, zorite-editor 0.11.7 from `LHelge/zorite-editor` branch `fix/stale-layout-input`. As of 2026-11-26 the official `gpui` crate is still 0.2.2, so no standalone GPUI release has landed; latest are gpui-pre 0.3.14 and gpui-kit 0.11.0. Moving to gpui-kit 0.11 needs a zorite-editor release that builds on gpui-pre 0.3.14.

### Tauri coupling in the backend

The spike only had to make `cave` and `markdown` public in `src-tauri/src/lib.rs`; it uses `Cave`, `CaveError` and `DocumentMeta`. Tauri reaches into the core modules in two places only:

- `cave/error.rs`: `impl From<tauri::Error> for CaveError` (the `Window` variant, used by the presentation window).
- `agent/vectordb.rs`: `tauri::async_runtime::spawn` for the debounced embedding update; a plain tokio spawn would do.

`markdown/` and `backup/` have no Tauri references. The rest lives where expected: 7 of 10 files in `commands/`, plus `lib.rs` and `scheme.rs`. Cave-adjacent features that depend on Tauri plugins: the last-open cave path (`tauri-plugin-store`), dialogs, the opener, the clipboard, logging and the updater. Extracting a Tauri-free core crate looks small.

### Draft port work items

Created on 2026-11-26 as Bears epic `zem` ("Port the frontend to GPUI"), 16 tasks.

1. Extract a Tauri-free core crate from `src-tauri/` (cave, markdown, agent, backup, config); replace the two Tauri touchpoints above.
2. Fix the backend's frontmatter rewrite on save (G2): keep unknown keys, comments, flow-style tags and line endings; never delete a body-level `---` block.
3. App shell and explorer: window, sidebar tree, open-cave flow, last-open cave without `tauri-plugin-store`.
4. Editor integration: note editor, save and autosave, wiki-link completion and navigation, broken-link marking, `$$` and CRLF round-trip fixes (upstream if they are in the editor), find in note, relative images.
5. Agent chat on `TextView` with streaming, tool calls, and wiki-link rewriting that respects code spans.
6. Settings with Catppuccin light and dark.
7. Templates and the remaining CSS-free features.
8. Todos, tags, calendar and search panels.
9. Backup UI.
10. Updater replacement for `tauri-plugin-updater`.
11. Bundling, signing and notarization.
12. Windows and Linux builds (Vulkan on Linux).
13. Remove Node, Tailwind, esbuild, Trunk and wasm from CI and git hooks.
14. Docs site updates.
15. Decide what happens to mermaid and presentations (dropped from the new frontend by the 2026-11-26 decision).
16. Track zorite-editor upstream: drop the fork pin when #5 is released; move to gpui-kit 0.11+.

The port starts from a clean branch and reuses spike code selectively; `spike/gpui` is not merged.

