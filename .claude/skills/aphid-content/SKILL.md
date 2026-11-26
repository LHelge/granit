---
name: aphid-content
description: Reference for writing Granit's documentation site (aphid). Use when writing or editing markdown under docs/content/ (wiki pages, standalone pages, home/wiki/404 intros) or changing docs/aphid.toml.
---

Granit's docs site is built with aphid from `docs/`: config in `docs/aphid.toml`, content
in `docs/content/`, theme in `docs/theme/` (pebble; see the `aphid-theme` skill). The site
has no blog.

# Content files

- `docs/content/wiki/<stem>.md` — the user documentation, at `/wiki/<stem>/`. Frontmatter
  (all optional): `title` (else the stem), `category`, `tags` (list), `created`, `updated`
  (YYYY-MM-DD), `draft`. `category` must be one of the `[[wiki_categories]]` names in
  `aphid.toml` — Getting Started, Notes & Writing, AI Agent, Reference — or the page lands
  in the default, Reference.
- `docs/content/pages/<stem>.md` — standalone pages (About, Download) at `/<stem>/`.
  Frontmatter: `title` (required), `order` (nav position, lower first), `draft`.
- `docs/content/home.md`, `wiki.md`, `404.md` — no frontmatter: the bodies of the home
  page, the intro above the wiki index, and the 404 page.

Scaffold a page with `aphid wiki new "Title"` or `aphid page new "Title"`, run from `docs/`.

# Headings

The page title comes from frontmatter and renders as `<h1>`; the pipeline shifts markdown
headings one level down. Use `#` for top-level sections, `##` for subsections; never a `#`
for the page title. `{#custom-id}` after a heading sets its id.

# Links

Link pages with `[[stem]]`, `[[stem|text]]`, `[[stem#section]]` or `[[#section]]`. Links are
checked at build time, so prefer them over hand-written URLs. Check the page exists first.

# Markdown

Tables, strikethrough, task lists, footnotes, fenced code with highlighting, ` ```mermaid `
diagrams, and GitHub alerts (`> [!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`,
`[!CAUTION]`). Images and files go in `docs/static/` and are referenced as `/static/…`.

# Building

From `docs/`: `aphid serve` for a live preview on :3000, `aphid build -o <dir>` to build and
validate links. Build into a scratch directory: a bare `aphid build` writes `docs/dist/`,
which is gitignored but easy to confuse with the app's `dist/` at the repo root.

# Style

Neutral reference tone, written for someone using the app, not developing it: name the UI
by its labels, give the real keyboard shortcuts and defaults. Open with the point, not
"This page covers…"; skip "Related pages" lists that repeat links already in the text.
Each fact has one home: link to the page that owns it instead of repeating it.
