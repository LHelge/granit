---
id: tfv
title: Add wasm-bindgen-test and CI integration
status: open
priority: P1
created: 2026-03-31T16:40:34.547431916Z
updated: 2026-03-31T16:40:34.547431916Z
tags:
- frontend
- infra
parent: 82y
---

## Summary
Add `wasm-bindgen-test` as a dev-dependency and configure `wasm-pack test` to run headless browser tests. Update CI to run frontend tests alongside backend tests.

## Acceptance Criteria
- [ ] `wasm-bindgen-test` added as dev-dependency in root `Cargo.toml`
- [ ] `tests/` directory created under workspace root with a trivial passing test
- [ ] `wasm-pack test --headless --firefox --target wasm32-unknown-unknown` runs and passes
- [ ] CI workflow updated to run frontend WASM tests
- [ ] Document the test command in copilot-instructions.md

## Implementation Notes
- Files: `Cargo.toml`, `tests/web.rs` (new), `.github/workflows/release.yml`
- Add to Cargo.toml:
  ```toml
  [dev-dependencies]
  wasm-bindgen-test = "0.3"
  ```
- Trivial first test:
  ```rust
  use wasm_bindgen_test::*;
  wasm_bindgen_test_configure!(run_in_browser);
  
  #[wasm_bindgen_test]
  fn pass() {
      assert_eq!(1 + 1, 2);
  }
  ```
- Requires `wasm-pack` installed in CI (`cargo install wasm-pack` or use action)

## Testing
- `wasm-pack test --headless --firefox` passes locally and in CI