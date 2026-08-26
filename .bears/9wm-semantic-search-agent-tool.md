---
id: "9wm"
title: semantic_search agent tool
status: done
priority: P2
created: "2026-08-26T08:44:48.952601Z"
updated: "2026-08-26T09:15:09.516823Z"
parent: wph
---

SemanticSearchTool backed by CaveVectorIndex::search; build_toolset gains Option<&CaveVectorIndex>; registered only when index exists; both modes; auto-RAG dynamic_context stays Ask-only. Independent of the other tasks.