---
id: xaz
title: Simplify IPC argument structs and invoke pattern
status: open
priority: P2
created: 2026-03-31T16:32:52.084708611Z
updated: 2026-03-31T16:32:52.084708611Z
tags:
- frontend
- refactor
parent: 4cm
---

## Summary
ipc.rs defines ~10 single-field argument structs and repeats the `to_value → invoke → from_value` pattern in every function. This is ~350 lines of boilerplate.

## Acceptance Criteria
- [ ] Create a macro or generic helper to reduce the invoke boilerplate
- [ ] Eliminate trivial single-field arg structs where possible
- [ ] Each IPC function should be ~3-5 lines max

## Implementation Notes
- Files: `src/app/ipc.rs`
- Option A: macro `invoke_cmd!("command_name", { field: value })` that handles serialization
- Option B: generic function `async fn invoke_typed<A: Serialize, R: DeserializeOwned>(cmd: &str, args: A) -> Result<R, String>`
- Keep the `js_err_to_string` helper but consider improving its error extraction

## Testing
- App compiles and IPC calls work as before