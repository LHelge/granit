---
id: duy
title: Align docs with implemented scope
status: open
priority: P2
created: 2026-03-27T12:15:24.294843Z
updated: 2026-03-27T12:39:19.905341Z
depends_on:
- tx2
- m9t
parent: wem
---

## Summary

Update the project documentation so it accurately describes what Granit currently implements and clearly separates shipped functionality from planned or deferred features. The current README and architecture notes overstate several capabilities, which makes planning and review work less reliable.

## Acceptance Criteria

- [ ] The README distinguishes implemented features from planned or deferred ones.
- [ ] Architecture descriptions match the actual current state ownership and module layout.
- [ ] Claims about markdown rendering, agent integration, nested folders, and wiki-links are either corrected or explicitly labeled as roadmap items.
- [ ] The resulting docs are useful for a new contributor trying to understand the current codebase.

## Implementation Notes

- Review README.md and compare it against the current frontend and backend implementation.
- Review .github/copilot-instructions.md and decide whether any wording should be updated to better match the repo’s current state.
- Prefer explicit wording such as “planned”, “not yet implemented”, or “current behavior” rather than vague future-facing language.

## Edge Cases

- Preserve long-term architectural intent without misrepresenting shipped functionality.
- Avoid deleting useful roadmap context if it can be reframed clearly.

## Testing

- Manual verification only: confirm the updated documentation matches the codebase after a quick implementation cross-check.
