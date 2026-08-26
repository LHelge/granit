---
id: avh
title: System prompt as editable Tera-templated cave file
status: in_progress
priority: P2
created: "2026-08-26T08:44:39.611794Z"
updated: "2026-08-26T08:44:57.878761Z"
parent: wph
---

.granit/agent/system.md, rendered with Tera (mode, tools, icons, skills, today/date vars). Default prompt rewritten as Tera template (drop <|think|>). Seed on cave open, never overwrite; migrate legacy AgentConfig.system_prompt. IPC read/update + reset_agent. Frontend: DocumentKind::SystemPrompt raw editing, settings textarea replaced with Edit-in-editor + Reset.