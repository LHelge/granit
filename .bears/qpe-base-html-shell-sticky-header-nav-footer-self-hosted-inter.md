---
id: qpe
title: "base.html: shell, sticky header, nav, footer, self-hosted Inter"
status: done
priority: P1
created: "2026-06-12T11:36:46.231762622Z"
updated: "2026-06-12T12:12:11.694626562Z"
tags:
  - docs
  - theme
depends_on:
  - fge
parent: f9w
---

## Summary

Rework the base layout every page extends: branded sticky header, nav, footer, fonts, and meta tags.

## Acceptance Criteria

- [ ] Sticky header: pebble `logo.png` + "Granit" wordmark linking home; nav rendered from `nav_pages` (Download, About) plus a hardcoded "Wiki" link to `/wiki/`; GitHub icon linked from `socials`.
- [ ] `<head>`: `{{ favicon_tags | safe }}`, canonical URL, `<meta name="theme-color" content="#181825">`, stylesheet link.
- [ ] Inter self-hosted: woff2 files in `docs/theme/static/fonts/` with `@font-face` rules — no third-party font requests.
- [ ] Footer: license note + GitHub link + "Built with aphid {{ version }}".
- [ ] Mermaid script block gated on `contains_mermaid`, loading `/static/js/mermaid.min.js`.

## Implementation Notes

- Template variables documented in `.claude/skills/aphid-theme/SKILL.md` (base.html section: site_title, nav_pages, socials, favicon_tags, feed urls).
- Use Tera blocks (`page_title`, `content`) so child templates override cleanly — keep the block names the default theme already uses.
- Inter source: download Latin woff2 subsets (400/500/700 + maybe 600) from the official Inter releases (rsms.me/inter or GitHub rsms/inter).

## Testing

- `aphid build` passes; serve and check header/nav/footer on home, a wiki page, and 404; verify no external requests in devtools network tab.