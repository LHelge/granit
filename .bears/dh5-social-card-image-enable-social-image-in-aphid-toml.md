---
id: dh5
title: Social card image + enable social_image in aphid.toml
status: open
priority: P2
created: "2026-06-12T11:38:43.577277996Z"
updated: "2026-06-12T11:38:43.577277996Z"
tags:
  - docs
  - polish
  - assets
depends_on:
  - u8w
parent: hb5
---

## Summary

A 1200x630 OpenGraph/Twitter card: pebble logo + "Granit" wordmark + tagline on the Mocha `--base-200` background.

## Acceptance Criteria

- [ ] `docs/static/social.png`, 1200x630px, on-brand (Mocha background #181825, Inter, pebble logo)
- [ ] `social_image = "static/social.png"` enabled in `docs/aphid.toml`
- [ ] Page source shows `og:image` / Twitter card meta pointing at `https://granit.lhelge.se/static/social.png`

## Implementation Notes

- Generate with ImageMagick from `src-tauri/icons/512x512.png` + text, or hand it off — keep it simple.
- Verify with a local build + view-source; optionally an OG preview tool after launch.