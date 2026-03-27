---
id: 64k
title: Render agent messages with markdown formatting
status: open
priority: P2
created: 2026-03-27T21:34:20.424460003Z
updated: 2026-03-27T21:34:20.424460003Z
tags:
- frontend
depends_on:
- '443'
parent: ann
---

## Summary

Render agent (assistant) messages as formatted markdown/HTML, reusing the same pulldown-cmark pipeline from the markdown rendering epic. User messages stay as plain text.

## Acceptance Criteria

- [ ] Assistant messages rendered as HTML (headings, bold, italic, code blocks, lists, etc.)
- [ ] Code blocks have syntax-appropriate styling (monospace, background)
- [ ] Inline code formatted distinctly
- [ ] Markdown rendered client-side from the final message content (not during streaming)
- [ ] During streaming: show raw text; on stream-done: re-render as markdown HTML
- [ ] Styled consistently with the prose-invert theme used in the editor preview

## Implementation Notes

- Files: `src/app/components/agent_panel.rs`, possibly `src/app/ipc.rs`
- Option A: Call `render_markdown` backend command on stream-done to get HTML for the final message
- Option B: If the markdown epic's render command exists, reuse it with a raw-text-to-HTML call
- During streaming, raw text is fine — avoids re-rendering partial markdown on every chunk
- Use `inner_html` on the message div for rendered content

## Dependencies

- This task depends on the markdown rendering epic (z69) being at least partially complete (the pulldown-cmark pipeline needs to exist)
- If the markdown epic isn't done yet, this task can use a minimal backend command that just runs pulldown-cmark on a string

## Testing

- Manual: send a message asking for markdown-formatted content, verify rendering