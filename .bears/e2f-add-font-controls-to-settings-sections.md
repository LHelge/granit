---
id: e2f
title: Add font controls to settings sections
status: in_progress
priority: P1
created: 2026-03-27T21:38:06.579772716Z
updated: 2026-03-29T21:21:35.333196917Z
tags:
- frontend
depends_on:
- 5js
parent: bp2
---

## Summary

Add font family and font size controls to the Markdown, Reading, and Agent sections of the settings modal. Wire save to include font configs.

## Acceptance Criteria

- [ ] Each section has a font family text input and font size number input
- [ ] Values initialized from config on modal open
- [ ] Save persists all font configs via updated `save_config` IPC
- [ ] Font preview or description text in each section