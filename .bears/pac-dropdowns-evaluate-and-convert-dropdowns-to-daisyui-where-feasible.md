---
id: pac
title: 'Dropdowns: Evaluate and convert dropdowns to DaisyUI where feasible'
status: open
priority: P2
created: 2026-04-04T21:15:13.916033712Z
updated: 2026-04-04T21:15:13.916033712Z
tags:
- frontend
depends_on:
- exw
parent: 9fd
---

## Summary

Evaluate each custom dropdown in the app and convert to DaisyUI `dropdown` where feasible. Some dropdowns need programmatic close-on-select which makes the `details` approach less clean — document trade-offs.

## Acceptance Criteria

- [ ] Each dropdown reviewed: cave selector, provider selector, model selector, font picker, provider type selector
- [ ] Dropdowns that can use `<details class="dropdown">` are converted
- [ ] Dropdowns that need JS state (e.g. close on selection + async action) are documented as kept
- [ ] Font picker searchable dropdown evaluated (may stay custom due to search+outside-click)

## Implementation Notes

- DaisyUI `details` dropdown auto-closes on blur, which may suffice for some cases
- Popover API approach (`popovertarget`) is another option but has browser compat concerns
- CSS focus approach is a third option that may work for simple cases
- The priority here is incremental: convert what's easy, document what stays

## Files to Modify

- `src/app/components/cave_selector.rs`
- `src/app/components/provider_selector.rs`
- `src/app/components/model_selector.rs`
- `src/app/components/settings/agent.rs` — provider type picker
- `src/app/components/settings/font_picker.rs`
