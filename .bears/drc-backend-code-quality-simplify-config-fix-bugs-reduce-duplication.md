---
id: drc
title: 'Backend code quality: simplify config, fix bugs, reduce duplication'
type: epic
status: done
priority: P1
created: 2026-04-04T21:39:37.165606817Z
updated: 2026-04-05T12:43:00.877768606Z
tags:
- backend
- refactor
---

## Scope

Thorough backend review found bugs, significant duplication, non-idiomatic patterns, and architectural smells. Key decision: **drop layered config** (global + cave override) — simplifies config module substantially.

## Phases

1. **Simplify config** — Remove RawConfig, MergeRaw, merge logic; single AppConfig with Default impl
2. **Fix error types** — Wrong error types in AppState helpers, fake IO errors, deadlock risk
3. **Reduce duplication** — IPC builder, agent provider match arms, cave helpers, agent tools
4. **Idiomatic Rust** — Entry API, silent fallbacks, dead code cleanup
5. **Security** — Symlink containment
6. **Tests** — Config round-trip, frontmatter, slug case-sensitivity

## Acceptance Criteria

- [ ] Single config file (`~/.config/granit/config.yml`), no cave-level config override
- [ ] No scattered default values — single `Default` impl for `AppConfig`
- [ ] Error types match their domain (no ConfigError for cave locks)
- [ ] Agent provider duplication eliminated via macro
- [ ] All duplicate agent tools consolidated
- [ ] `cargo fmt && cargo clippy && cargo test` clean after every task