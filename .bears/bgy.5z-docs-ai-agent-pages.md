---
id: bgy.5z
title: "docs: AI agent pages"
status: open
priority: P2
created: "2026-11-26T23:42:04.930389696Z"
updated: "2026-11-26T23:42:04.930389696Z"
tags:
  - docs
  - agent
---

## Summary

Fix `docs/content/wiki/ai-agent.md`, `agent-tools-and-rag.md`, `system-prompt.md` and `skills-and-tasks.md`.

## Acceptance Criteria

- [ ] **ai-agent, new "Chat panel" section:** attaching the current note, and the selected text, with the "Attach current note" chip (`src/app/agent/mod.rs`; how the prompt wraps it is in `agent/prompt.rs`), and the "Clear chat" button.
- [ ] **ai-agent, modes:**
  - Tool calls show in Ask mode too, since it runs the read-only tools.
  - Ask mode injects RAG context only when RAG is enabled.
  - This page holds the full mode description; agent-tools-and-rag links to it instead of repeating it.
- [ ] **agent-tools-and-rag, settings:**
  - The setting is `rag.top_n` (default 5), not `rag_top_n`.
  - Mention `rag.enabled`, `rag.embedding_model`, and `semantic_search`'s optional result count, linking to configuration.
- [ ] **agent-tools-and-rag, indexing:** the incremental re-index happens only for edits made in the UI (`commands/cave.rs`), and it is debounced. Notes the agent creates or edits are re-embedded at the next rebuild, which happens on cave open or a RAG config change.
- [ ] **agent-tools-and-rag, cleanup:** drop the closing NOTE that repeats the section above it, and fold the duplicated mode sentence into one.
- [ ] **system-prompt:**
  - The default file is re-seeded whenever it is missing when a cave opens, so deleting it restores the default.
  - The snippet is labelled a simplified excerpt of `DEFAULT_SYSTEM_PROMPT_TEMPLATE`.
- [ ] **skills-and-tasks:** the app rewrites a skill's frontmatter without its comments (`cave/agent_docs.rs`), and deleting a skill removes its whole folder, `scripts/` included.

## Implementation Notes

Load the `aphid-content` skill. Tool names and arguments are in `src-tauri/src/agent/tools/mod.rs`.