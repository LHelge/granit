---
id: 4cm
title: Architectural refactoring and bug fixes
type: epic
status: done
priority: P1
created: 2026-03-31T16:31:55.818959479Z
updated: 2026-03-31T20:22:21.142128946Z
---

## Scope

Address findings from the architectural review: correctness bugs, code quality improvements, idiomatic Rust patterns, reduced duplication, and better modularization.

## Acceptance Criteria

- [ ] All cave mutation operations are atomic (filesystem-first, then index)
- [ ] Slug uniqueness enforced as invariant at cave open
- [ ] Agent message history is bounded
- [ ] Config merge uses trait-based approach instead of manual field chaining
- [ ] Mutex lock boilerplate eliminated via extension trait or wrapper
- [ ] IPC layer simplified with less boilerplate
- [ ] Prop drilling replaced with context where appropriate
- [ ] Error handling unified (single error signal, structured types)
- [ ] Large handlers decomposed into smaller functions
- [ ] Wiki-link resolution respects code blocks
- [ ] TextareaState uses safe char/byte handling
- [ ] Undocumented async patterns documented
- [ ] Silent error swallowing replaced with logging
- [ ] Tuple types replaced with named structs
- [ ] Platform detection done once at startup
- [ ] All changes pass `cargo fmt`, `cargo clippy`, `cargo test`