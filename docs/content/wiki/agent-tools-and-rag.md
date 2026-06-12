---
title: Agent Tools and RAG
category: AI Agent
tags: [agent, tools, rag, configuration]
---

In Agent mode the AI agent works through a set of tools that read and modify your cave, and in Ask mode it draws on a local retrieval index built from your notes. This page lists the available tools, explains how to disable individual ones, and describes how retrieval (RAG) is built and kept up to date. For provider setup and the difference between the modes, see [[ai-agent]].

# Tool catalog

The agent has access to the following tools. Read-only tools are available in both modes; the tools that change your cave are available only in Agent mode.

| Tool | What it does |
|------|--------------|
| `read_note` | Read a note's content and backlinks by slug, or the currently active note |
| `list_notes` | List all notes in the cave with their slugs |
| `create_note` | Create a new markdown note in the cave |
| `update_note` | Replace the entire body of a note |
| `edit_note` | Find and replace text within a note's body |
| `delete_note` | Delete a note from the cave |
| `move_note` | Move a note to a different folder |
| `rename_note` | Rename a note in-place |
| `create_folder` | Create a new folder in the cave |
| `rename_folder` | Rename a folder in-place |
| `move_folder` | Move a folder under a new parent |
| `delete_folder` | Delete a folder and all its notes |
| `open_daily_note` | Open or create today's daily note |
| `list_folders` | List all folders in the cave |
| `search_notes` | Search notes by slug (case-insensitive) |
| `search_content` | Search inside note bodies (full-text) |
| `list_todos` | List todo checkboxes from notes, with optional filtering |
| `toggle_todo` | Toggle the completion status of a todo checkbox in a note |
| `web_fetch` | Fetch a webpage and return its content as markdown |
| `web_search` | Search the web using Brave Search |

`web_fetch` is always available and needs no API key. `web_search` is registered only when a Brave Search API key is configured.

> [!WARNING]
> In Agent mode the agent can create, edit, move, and delete notes and folders. These actions write to your files. Review what the agent proposes, and keep your cave under version control or backed up if you let the agent make changes unattended.

# Disabling tools

You can prevent specific tools from being registered by listing their names in the `disabled_tools` setting in the cave's agent configuration. A disabled tool is never offered to the model, regardless of mode. For example, you might disable `delete_note` and `delete_folder` to keep the agent from removing anything. See [[configuration]] for where this setting lives.

Note that switching to Ask mode already removes every cave-modifying tool, so `disabled_tools` is mainly for restricting Agent mode further.

# Retrieval (RAG)

In Ask mode the agent answers from your notes using retrieval-augmented generation. Your notes are embedded into vectors locally, on the CPU, using [fastembed](https://github.com/qdrant/fastembed) — no external service is involved and your note content does not leave your machine for embedding. The resulting vectors are cached in `.granit/embeddings.bin` inside the cave so they do not have to be recomputed on every launch.

When you ask a question, the most similar notes are retrieved and injected into the conversation as context. The `rag_top_n` setting controls how many notes are injected per query. A higher value gives the model more context at the cost of a larger prompt.

The index is built or rebuilt in the background when a cave is opened or when the RAG configuration changes. After that it is updated incrementally as you work: creating, saving, renaming, or deleting a note updates only the affected entries rather than rebuilding the whole index.

> [!NOTE]
> Retrieval context is injected only in Ask mode. In Agent mode the model gathers information through the tools above instead. See [[ai-agent]] for the mode distinction and [[configuration]] for the RAG settings.
