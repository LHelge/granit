---
applyTo: "src-tauri/src/markdown/**"
---

# Markdown Processing

Instructions for the markdown parsing pipeline in the backend.

## Pipeline

Raw markdown → frontmatter extraction → wiki-link resolution → pulldown-cmark → HTML

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

### 2. Wiki-Link Resolution

Before parsing markdown, find `[[note-name]]` patterns and resolve them:

- Search the entire cave for a file matching `note-name.md` (case-insensitive, any subdirectory)
- Replace `[[note-name]]` with a standard markdown link: `[note-name](resolved-path)`
- If no match is found, render as a "broken link" (e.g., red text or strikethrough)
- Filenames are unique across the cave — if duplicates exist, match the first found

### 3. Rendering

Use `pulldown-cmark` with these options enabled:
- Tables
- Strikethrough
- Task lists
- Footnotes (if needed)

```rust
use pulldown_cmark::{Parser, Options, html};

let mut options = Options::empty();
options.insert(Options::ENABLE_TABLES);
options.insert(Options::ENABLE_STRIKETHROUGH);
options.insert(Options::ENABLE_TASKLISTS);

let parser = Parser::new_ext(&markdown, options);
let mut html_output = String::new();
html::push_html(&mut html_output, parser);
```

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
