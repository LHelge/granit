---
id: bhq
title: "Docs: presentation guide, template DOM contract, keyboard reference"
status: done
priority: P3
created: "2026-09-08T22:02:49.522860434Z"
updated: "2026-09-09T08:36:42.367829Z"
tags:
  - presentation
  - docs
depends_on:
  - bss
  - "4ff"
parent: h7p
---

Document the presentation feature on the docs site and in the in-repo docs.

- User guide: setting `presentation:` in frontmatter (or via the dropdown), `---` as slide separator and what does and does not split, the Present button, window controls and keyboard reference, fullscreen, what closes the window.
- Template authoring guide: `.granit/presentations/` layout, shared assets by relative path, the two seeded defaults as starting points, the DOM contract (section.slide, data-index, active class, base CSS responsibilities vs template responsibilities, code-block classes), and an example of a title-slide style using `:first-child`.
- Note that rename/delete of a template leaves note references hanging and how that shows up.
- Update CLAUDE.md's cave model and markdown sections briefly so future sessions know the scheme, the presentations directory, and the DOM contract exist.

Can be written from the agreed contract before the window ships; final pass after the UI task lands.