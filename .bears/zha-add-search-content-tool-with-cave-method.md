---
id: zha
title: Add search_content tool with cave method
status: done
priority: P1
created: 2026-04-05T11:04:19.718920417Z
updated: 2026-04-05T11:18:09.633837988Z
tags:
- agent
- backend
depends_on:
- fmh
parent: fm9
---

## Summary
Add full-text search across note bodies. Requires a new `cave.search_content()` method plus a `SearchContentTool`.

## Implementation

### Cave method (`cave/mod.rs`)
```rust
pub fn search_content(&self, query: &str, max_results: Option<usize>) -> Result<Vec<SearchHit>, CaveError>
```
- Iterate all notes, read each file body (strip frontmatter)
- Case-insensitive substring match on body text
- Return matching slug, relative path, and a context snippet (line containing the match)
- Default max_results: 20

### Agent tool (`navigation.rs`)
- `SearchContentArgs { query: String }`
- Output: list of matches with slug, path, snippet
- Register in `build_toolset`, `TOOL_CATALOGUE`
- Add to `build_tool_call_info` (extract `query` param)

## Acceptance Criteria
- [ ] Cave has `search_content()` method with tests
- [ ] Agent can search note bodies by content
- [ ] Returns useful snippets for context