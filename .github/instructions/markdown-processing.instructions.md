---
applyTo: "src-tauri/src/markdown/**"
---

# Markdown Processing

Instructions for the markdown parsing pipeline in the backend.

## Pipeline

Raw markdown → frontmatter extraction → pulldown-cmark (with `ENABLE_WIKILINKS`) → HTML

### 1. Frontmatter Extraction

Strip YAML between `---` fences before passing to pulldown-cmark. Parse with `serde_yml`:

```rust
struct Frontmatter {
    // title is intentionally absent — derived from the filename (slug)
    #[serde(default)]
    tags: Vec<String>,
    created_at: Option<DateTime<Utc>>,
    modified_at: Option<DateTime<Utc>>,
}
```

The note title is always derived from the filename. It is included in `RenderedNote` for the frontend to render as a page header above the HTML body.

### 2. Rendering with Wiki-Link Resolution

Wiki-links (`[[note-name]]`) are resolved **during** pulldown-cmark event processing
using the `ENABLE_WIKILINKS` parser option. This means code blocks and inline code
are handled automatically by the parser — no separate text preprocessing step needed.

The `render_with_wiki_links` function intercepts `Event::Start(Tag::Link { link_type: WikiLink { .. }, .. })` events:

- **Resolved**: lookup matches a cave note → rewrite to `LinkType::Inline` with the slug as href, collect into `outgoing_links`
- **Unresolved**: no match → render with `class="broken-link"` for frontend styling
- **Piped links**: `[[target|display text]]` are supported natively (`has_pothole: true`)

Options enabled: tables, strikethrough, task lists, footnotes, **wikilinks**.

For plain markdown without wiki-link resolution (e.g., agent chat messages), use
`render_html()` which does not enable `ENABLE_WIKILINKS`.

## Return Type

Backend commands return a struct with both rendered HTML and metadata:

```rust
struct RenderedNote {
    title: String,                // slug (filename without .md)
    html: String,
    frontmatter: Option<Frontmatter>,
    outgoing_links: Vec<String>,  // resolved [[links]]
}
```
