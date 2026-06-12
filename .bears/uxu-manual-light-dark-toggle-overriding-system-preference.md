---
id: uxu
title: Manual light/dark toggle overriding system preference
status: in_progress
priority: P2
created: "2026-06-12T15:31:33.463627198Z"
updated: "2026-06-12T15:31:49.851149344Z"
tags:
  - docs
  - theme
  - css
---

## Summary

A sun/moon toggle button in the docs site header that overrides the system color-scheme preference, persisted in localStorage.

## Acceptance Criteria

- [ ] Tokens refactored to `light-dark()` with `color-scheme: light dark` on :root; `[data-theme="light"|"dark"]` forces the scheme
- [ ] Toggle button in the header nav; icon reflects the effective theme (sun shown in dark, moon in light), correct with and without an override
- [ ] Choice persists across page loads via localStorage with a no-FOUC restore script in head
- [ ] theme-color meta and mermaid theme respect the override
- [ ] No override stored → site follows system preference exactly as before