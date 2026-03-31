---
id: qyq
title: Write reference component tests
status: open
priority: P2
created: 2026-03-31T16:40:34.556669077Z
updated: 2026-03-31T16:40:40.276335316Z
tags:
- frontend
depends_on:
- kk6
parent: 82y
---

## Summary
Write the first `wasm-bindgen-test` component tests as a reference pattern for future tests. Pick a simple component to validate the approach works end-to-end.

## Acceptance Criteria
- [ ] At least one component test that mounts a component, interacts with it, and asserts on DOM
- [ ] Test uses MockIpc (from task above) to avoid Tauri dependency
- [ ] Test demonstrates the pattern: mount → interact → tick().await → assert
- [ ] Test file serves as a documented template for writing more tests

## Implementation Notes
- Files: `tests/components.rs` (new)
- Good candidates for first test:
  - Agent panel: provide mock messages signal, verify message bubbles render
  - Sidebar: provide mock notes list, verify tree renders
- Pattern:
  ```rust
  #[wasm_bindgen_test]
  async fn sidebar_renders_notes() {
      let wrapper = document().create_element("div").unwrap();
      document().body().unwrap().append_child(&wrapper).unwrap();
      provide_context(MockIpc::new(/* canned notes */));
      mount_to(wrapper.unchecked_into(), || view! { <Sidebar /> });
      tick().await;
      // assert on DOM content
  }
  ```
- Keep tests minimal — goal is proving the pattern, not full coverage

## Testing
- `wasm-pack test --headless --firefox` passes with new component tests