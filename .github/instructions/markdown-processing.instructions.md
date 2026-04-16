---
applyTo: "src-tauri/src/markdown/**"
---

# Markdown Processing

Instructions for backend markdown rendering.

## Rules

- Parse YAML frontmatter separately from the markdown body.
- The note title comes from the filename slug, not from frontmatter.
- Frontmatter currently includes tags, timestamps, icon, and optional favorite flag.
- Resolve wiki-links by filename during markdown event processing.
- Preserve broken links distinctly so the frontend can style them.
- Sanitize raw HTML before it reaches the webview.
- Task checkboxes are interactive in the reader and disabled in agent-rendered markdown.
- Rendered responses should include the HTML plus metadata needed by the frontend, including frontmatter, outgoing links, timestamps, and backlinks when available.
