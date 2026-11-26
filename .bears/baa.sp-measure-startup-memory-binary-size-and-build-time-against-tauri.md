---
id: baa.sp
title: Measure startup, memory, binary size and build time against Tauri
status: done
priority: P1
created: "2026-11-26T12:15:47.292409Z"
updated: "2026-11-26T21:01:23.230477Z"
tags:
  - spike
  - gpui
depends_on:
  - baa.cs
attempts: 1
---

## Summary

Gate G5. Put numbers on the goals behind the port: compare the spike against the current Tauri release build, on the same machine and the same cave, with a method that can be repeated.

## Acceptance Criteria

A table with both apps for each row:

- [ ] Cold start to a visible note (app not run since login) and warm start, median of 5 runs each
- [ ] Idle memory with one note open, after 30 s: for Tauri, the app process plus its WebKit WebContent, GPU and Networking helper processes; for GPUI, the single process. Report physical footprint, not only RSS
- [ ] Idle CPU with a blinking caret, and CPU while scrolling a long note
- [ ] Release binary / app bundle size
- [ ] Clean build time and incremental build time (one-line change in the UI crate), plus the number of toolchains and steps needed (cargo only vs cargo + npm + trunk + wasm target)

## Implementation Notes

- Build both in release mode. Current app: `cd src-tauri && cargo tauri build`. Spike: `cargo build --release -p granit-gpui`.
- Fairness: the Tauri app builds the RAG vector index and loads the embedding model in the background when a cave opens; the spike does not. Measure the Tauri app with RAG disabled in the cave's `.granit/config.yml`, and say so in the table. Optionally add a second Tauri row with RAG enabled to show how much of today's footprint is toolkit-independent.
- The spike binary links `granit_lib` (Tauri, fastembed and the rest) even though it does not use most of it, so its binary size overstates a real port. Report it with that caveat.
- Tools on macOS: `footprint <pid>` or `vmmap --summary <pid>` for memory; `ps -o rss` as a cross-check; helper processes are children named `com.apple.WebKit.*`. For startup, timestamp at process spawn and log a line from the first frame that paints a note (both apps), or screen-record at 60 fps and count frames if instrumenting the Tauri app is awkward.
- The clean and incremental build times of the spike were already noted in the scaffold task; re-measure only if dependencies changed.

## Edge Cases

- Disk cache and Spotlight noise: discard the first run after a build
- Thermal state: run the two apps interleaved, not all of one then all of the other

## Testing

- The table is the deliverable; keep the raw numbers
## Recorded from the scaffold task (baa.7f, 2026-11-26)

Spike build times, debug profile, Apple M4 (10 cores), crates already downloaded, measured by the implementing subagent with `/usr/bin/time -p` into a fresh `CARGO_TARGET_DIR`:

- Clean build of `granit-gpui`: 53.0 s wall (user 249 s, sys 29 s)
- Incremental (touch `granit-gpui/src/main.rs`, rebuild): 0.9-1.7 s over three runs
- Debug binary 106 MB; clean target directory 3.2 GB

Measured before `granit_lib` was added as a dependency (baa.85), with gpui-kit 0.10.0, zorite-editor 0.11.7, gpui-pre 0.3.12. Re-measure once the dependency set is final.

## GPUI side measured (2026-11-26)

Apple M4 (10 cores, 16 GB), macOS, release profile, test cave `~/tmp/test_cave` (389 notes). Stack: gpui-kit 0.10.0, gpui-pre 0.3.12, zorite-editor 0.11.7 from the fork branch (baa.ac), `granit_lib` linked. The Tauri side is not measured yet: the app reopens the live cave, so the owner has to switch it to the test cave first.

Method: `GRANIT_T0_NS=<spawn time, Unix ns> GRANIT_OPEN=<slug> granit-gpui <cave>` prints `first-note-frame ms=…` on stderr, the time from spawn to the frame after the one that drew the note (`log_first_note_frame` in `granit-gpui/src/main.rs`). A script spawned the binary 6 times per note, killed it after the line, and waited 1 s between runs.

| Measure | GPUI spike | Raw |
|---|---|---|
| Warm start, small note (`Test note`) | 291 ms median of 5 | 275.8, 291.1, 307.9, 292.1, 291.1 |
| First launch after a rebuild (same note) | 1,012 ms | one run; not a true cold start |
| Warm start, 6,910-line note | 1,337 ms median of 5 | 1336.7 (first), 1306.4, 1315.3, 1340.4, 1336.7, 1370.1 |
| Idle memory, small note open, after 30 s | 103 MB physical footprint (`footprint`), peak 157 MB | RSS 102 MB, 1 process, 5 threads |
| Idle CPU over the next 30 s | 0.0 % | `ps` CPU time unchanged at 0.27 s since start; editor not focused, so no caret blink |
| Release binary | 30.0 MB, 24.3 MB stripped | 30,030,848 / 24,283,384 bytes |
| Clean release build | 194 s wall | user 1,436 s, sys 87 s; target dir 3.1 GB |
| Incremental release build (touch `main.rs`) | 28-32 s | 31.8, 28.5, 28.3 |
| Clean debug build | 110 s wall | user 598 s, sys 81 s; target dir 7.1 GB; binary 126 MB |
| Incremental debug build (touch `main.rs`) | 4.7-5.5 s | 5.5, 4.7, 5.1 |

Build toolchain: cargo only (crates already downloaded for the timings). The debug numbers are higher than the scaffold task's (53 s clean, 0.9-1.7 s incremental) because `granit_lib` is now linked.

Not measured on the GPUI side:

- A true cold start (app not run since login).
- CPU with a blinking caret and CPU while scrolling a long note: both need the editor focused or driven by hand.

Reference without launching it: the installed Tauri app `/Applications/Granit.app` (0.10.1) is a 91 MB bundle with a 70 MB main binary.

## Decision: Tauri side skipped (2026-11-26)

The owner decided not to measure the Tauri app: from using both, the spike is obviously snappier, and getting comparable numbers would mean switching the installed app off the live cave. The acceptance criteria above are therefore met for the GPUI column only. For the write-up (baa.ng), G5 rests on the GPUI numbers above plus the owner's judgement, not on a side-by-side table.
