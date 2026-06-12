---
id: "92s"
title: Configure Pages + DNS for granit.lhelge.se and run first deployment
status: done
priority: P1
created: "2026-06-12T11:38:35.466471390Z"
updated: "2026-06-12T14:40:47.442496355Z"
tags:
  - docs
  - infra
  - manual
depends_on:
  - jz4
  - "7t6"
  - u4r
  - qn4
  - qje
  - fr4
  - q3x
parent: nbd
---

## Summary

The one-time launch step: GitHub Pages settings, DNS, and the first real deployment. Mostly **manual user actions** — this task tracks/documents them and verifies the result.

## Manual checklist (user)

- [ ] Repo Settings → Pages → Build and deployment → Source: **GitHub Actions**
- [ ] Settings → Pages → Custom domain: `granit.lhelge.se`; enable **Enforce HTTPS** once the cert issues
- [ ] DNS at the lhelge.se provider: `CNAME granit.lhelge.se → lhelge.github.io`
- [ ] (Recommended) Verify `lhelge.se` under account Settings → Pages → Verified domains (prevents domain takeover)

## Acceptance Criteria

- [ ] Docs workflow run green on main; deployment visible in the `github-pages` environment
- [ ] https://granit.lhelge.se resolves with valid HTTPS
- [ ] Deep links work: `/wiki/`, a wiki page URL, `/download/`; bogus URL serves the themed 404
- [ ] Custom domain survives a second deploy (if it resets, add `docs/static/CNAME` as fallback per plan §5)

## Implementation Notes

- No CNAME file in the artifact is needed for Actions-based deploys — the Settings entry is canonical. Only add `docs/static/CNAME` if the domain resets.