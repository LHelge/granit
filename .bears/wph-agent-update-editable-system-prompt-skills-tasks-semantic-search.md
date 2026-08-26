---
id: wph
title: "Agent update: editable system prompt, skills, tasks, semantic search"
type: epic
status: done
priority: P2
created: "2026-08-26T08:44:33.316165Z"
updated: "2026-08-26T09:15:11.329793Z"
---

Open up the agent with user-authored files in `.granit/agent/` and on-demand vector search. Plan: /Users/linusb/.claude/plans/i-would-like-to-vectorized-oasis.md

Four feature commits:
1. System prompt as editable, Tera-templated cave file (.granit/agent/system.md; context: mode, tools, icons, skills, date vars; seeded on cave open; legacy config field migrated)
2. Skills per the Agent Skills spec (.granit/agent/skills/<name>/SKILL.md) + use_skill tool
3. Custom tasks via slash commands in chat (.granit/agent/tasks/*.md, Tera-rendered)
4. semantic_search tool backed by CaveVectorIndex::search (auto-RAG stays Ask-only)