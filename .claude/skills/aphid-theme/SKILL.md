---
name: aphid-theme
description: Reference for editing the docs site's aphid theme (pebble). Use when modifying Tera templates under docs/theme/templates/, the theme CSS or JS in docs/theme/static/, or template variables.
---

The docs site's theme is `docs/theme/` ("pebble": Catppuccin Mocha, Inter). Templates in
`templates/` are Tera (Jinja2-style) and extend `base.html`; read the existing templates
before changing one. Static files in `static/` are served at `/static/…`; a file of the same
path in `docs/static/` wins.

**Cache busting:** `base.html` loads `theme.css?v=<version>`. When the CSS changes, bump
`version` in `theme.toml` and the `?v=` query together.

The site has no blog: `blog_post.html`, `blog_index.html` and `pagination.html` exist only
because aphid requires them.

# Variables

Every template gets: `site_title`, `site_description`, `social_image_url`, `version` (aphid's),
`nav_pages` (`{title, url}` by `order`), `socials` (`{platform, url}`), `favicon_tags`
(pre-rendered HTML), `feed_atom_url`, `feed_rss_url`.

| Template | Variables |
|----------|-----------|
| `page.html` | `title`, `url`, `canonical_url`, `content` (HTML, use `\| safe`), `toc` (`{level, text, id}`), `contains_mermaid` |
| `wiki_page.html` | everything `page.html` has, plus `category` (always a string), `backlinks` (`{title, url}`), `wiki_categories` (see below) |
| `wiki_index.html` | `categories` (see below), `wiki_intro` (optional `{content}` from `content/wiki.md`), `contains_mermaid` |
| `home.html` | `home` (optional `{content}` from `content/home.md`), `popular_tags` (`{name, slug, count}`), `posts`, `contains_mermaid` |
| `tag.html` | `tag`, `tag_slug`, `blog_posts`, `wiki_pages` (entries `{title, url, created?, image?, description?, tags}`) |
| `tags_index.html` | `tags` (`{name, slug, count}`) |
| `404.html` | `not_found` (optional `{content}` from `content/404.md`), `contains_mermaid` |

A category is `{name, description?, icon?, pages: [{title, url}]}`: description and icon come
from `[[wiki_categories]]` in `docs/aphid.toml`. Categories come in config order, then
alphabetical, with the default category (Reference) last; pages sort by title. Highlight
the current page in a sidebar with `page.url == url`.

# Rendered content

- Code blocks carry `hl-` token classes (`hl-keyword`, `hl-string`, `hl-comment`,
  `hl-type`, `hl-function`, `hl-number`, …), which the theme CSS colours.
- Mermaid blocks render as `<pre class="mermaid">`; when `contains_mermaid` is true the
  template loads aphid's bundled `/static/js/mermaid.min.js`.
- Body headings start at `<h2>`.

Check a change against a page with no tags or TOC, a very long page, many backlinks, the
404 page, and mobile width. Build from `docs/` with `aphid build -o <scratch dir>`.
