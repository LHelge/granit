---
id: qje
title: "Wiki content: AI Agent (ai-agent, agent-tools-and-rag)"
status: open
priority: P1
created: "2026-06-12T11:38:07.737483063Z"
updated: "2026-06-12T11:38:07.737483063Z"
tags:
  - docs
  - content
depends_on:
  - jfv
parent: sm3
---

## Summary

The two `category: AI Agent` wiki pages.

## Acceptance Criteria

- [ ] `wiki/ai-agent.md` — provider setup (Ollama, Anthropic, Mistral, any OpenAI-compatible endpoint with custom base URL + key), model selection, Ask vs Agent mode (RAG context only in Ask), streaming chat.
- [ ] `wiki/agent-tools-and-rag.md` — tool catalog (notes, folders, templates, daily notes, todos, search, web fetch, web search), disabling tools via config, RAG: local CPU fastembed embeddings cached in `.granit/embeddings.bin`, `rag_top_n`, when the index rebuilds (cave open, config change, note CRUD).
- [ ] Cross-linked to each other and to [[configuration]].

## Implementation Notes

- Source: CLAUDE.md "AI agent" section; README agent bullets.
- Keep it user-facing — describe behavior and config, not rig-core internals.
- Use `> [!NOTE]` for the API-key storage location and `> [!TIP]` for Ollama-as-free-local-option.