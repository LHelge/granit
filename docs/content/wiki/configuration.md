---
title: Configuration
category: Reference
tags: [config, yaml, reference]
---

Every cave stores its own settings in a `config.yml` file inside its `.granit/` directory. The
configuration is **cave-local**: opening a different cave loads a different set of settings, and
nothing is shared between caves. The only piece of state kept outside the cave is the path of the
last-open cave, which the app persists separately (via `tauri-plugin-store`) so it can reopen your
most recent cave on startup — that path is never written into any cave's `config.yml`.

Most settings have sensible defaults and can be changed from the in-app settings dialog, so you
rarely need to edit `config.yml` by hand. This page documents every user-facing key for the times
you do. Any key you omit falls back to its default, so a minimal config file is valid.

# Example config

The following is a complete `.granit/config.yml` with realistic values. It exercises every section
of the configuration.

```yml
# Theme and fonts
theme: catppuccin-mocha

markdown_font:
  font_family: "JetBrains Mono"
  font_size: 14
reading_font:
  font_family: "Georgia"
  font_size: 16
agent_font:
  font_family: "Inter"
  font_size: 14

# Sidebar panels
sidebar:
  visible: true
  width: 256
agent_panel:
  visible: false
  width: 320

# Daily notes
daily_note_folder: "Daily"
daily_note_template_slug: "daily-template"

# AI agent
agent:
  providers:
    - name: "Local Ollama"
      provider: ollama
      base_url: "http://localhost:11434"
    - name: "Claude"
      provider: anthropic
      api_key: "sk-ant-..."
  selected_provider: 1
  selected_model: "claude-sonnet-4-6"
  mode: agent
  max_history: 100
  max_turns: 10
  system_prompt: null
  disabled_tools:
    - delete_note
  tool_config:
    web_search:
      api_key: "BSA-..."
      max_results: 5
    web_fetch:
      max_output_chars: 100000
  rag:
    enabled: true
    top_n: 5
    embedding_model: null
```

> [!NOTE]
> API keys are stored in plain text inside the cave's `.granit/config.yml`. Keep that file out of
> any directory you sync or share publicly.

# Theme

`theme` (string, default `"dark"`) selects the active UI theme by id. See [[themes]] for the full
list of available theme ids and their light/dark variants.

# Fonts

Each editing surface has its own font configuration. A font block has two keys:

- `font_family` (string) — any CSS font family, e.g. `"JetBrains Mono"` or `"sans-serif"`.
- `font_size` (number) — size in pixels.

| Key | Surface | Default family | Default size |
| --- | --- | --- | --- |
| `markdown_font` | Markdown editor (edit mode) | `monospace` | `14` |
| `reading_font` | Rendered reader (read mode) | `sans-serif` | `16` |
| `agent_font` | Agent chat panel | `sans-serif` | `14` |

Per-surface fonts are also covered on the [[themes]] page.

# Sidebar panels

Two collapsible panels each store their own visibility and width. See [[explorer]] for what the
left sidebar contains.

| Key | Panel | Default `visible` | Default `width` |
| --- | --- | --- | --- |
| `sidebar` | Left explorer sidebar | `true` | `256` |
| `agent_panel` | Right agent chat panel | `false` | `320` |

Each block has:

- `visible` (boolean) — whether the panel is shown on load.
- `width` (number) — panel width in pixels.

# Daily notes

These keys control where daily notes are created and whether they start from a template. See
[[daily-notes]] for the feature itself.

- `daily_note_folder` (string, default `"Daily"`) — folder, relative to the cave root, where daily
  notes are stored.
- `daily_note_template_slug` (string, optional) — slug of a template used to seed a new daily note.
  Omit the key (or set it to `null`) to create blank daily notes.

# Agent

The `agent` block configures the integrated AI assistant. See [[ai-agent]] for an overview and
[[agent-tools-and-rag]] for the tools and retrieval behavior.

## Providers

`providers` is a list of provider entries. Each entry has an optional `name` (a display label for
disambiguation) plus a `provider` discriminator that selects the type and its fields:

| `provider` | Fields | Notes |
| --- | --- | --- |
| `ollama` | `base_url` (optional) | Local Ollama. Defaults to `http://localhost:11434` when omitted. |
| `anthropic` | `api_key` (required) | Anthropic Claude models. |
| `mistral` | `api_key` (required) | Mistral models. |
| `openai` | `api_key` (required), `base_url` (required) | Any OpenAI/ChatGPT-compatible endpoint at a custom base URL. |

```yml
providers:
  - provider: ollama
    base_url: "http://localhost:11434"
  - name: "Claude"
    provider: anthropic
    api_key: "sk-ant-..."
  - name: "Local LLM"
    provider: openai
    api_key: "sk-..."
    base_url: "http://localhost:1234/v1"
```

At least one provider must be configured.

## Provider and model selection

- `selected_provider` (number, default `0`) — zero-based index into the `providers` list selecting
  the active provider.
- `selected_model` (string, optional) — last-used model id. When omitted, the active provider's
  built-in default model is used (for example `claude-sonnet-4-6` for Anthropic, or
  `mistral-small-latest` for Mistral).

## Mode and limits

- `mode` (`agent` | `ask`, default `agent`) — operating mode. `agent` gives the assistant full tool
  access; `ask` is a read-only Q&A mode that injects RAG context. See [[ai-agent]] for the
  difference.
- `max_history` (number, default `100`) — maximum number of chat messages retained. The oldest
  messages are dropped once the limit is exceeded. Must be greater than 0.
- `max_turns` (number, default `10`) — maximum number of multi-turn tool-call rounds per prompt.
  Must be greater than 0.
- `system_prompt` (string, optional) — overrides the built-in system prompt. When omitted (or
  `null`), Granit uses its default prompt.

## Tools

- `disabled_tools` (list of strings, optional) — names of agent tools that should not be registered.
  Omit the key to enable every tool. See [[agent-tools-and-rag]] for the available tool names.
- `tool_config` — per-tool settings:
  - `web_search`:
    - `api_key` (string, optional) — Brave Search API key. Required for the `web_search` tool to
      work.
    - `max_results` (number, default `5`) — maximum search results returned.
  - `web_fetch`:
    - `max_output_chars` (number, default `100000`) — maximum characters returned by a single
      `web_fetch` call.

## RAG

The `rag` block configures retrieval-augmented generation over your cave. Retrieved context is only
injected in `ask` mode. See [[agent-tools-and-rag]] for how the vector index is built and updated.

- `enabled` (boolean, default `true`) — whether RAG is active.
- `top_n` (number, default `5`) — number of similar notes retrieved per query.
- `embedding_model` (string, optional) — `fastembed` model identifier (for example
  `AllMiniLML6V2`). When omitted (or `null`), the built-in default embedding model is used.

# Keys not written by hand

Some runtime-only fields may appear in IPC responses but are never serialized into a cave's
`config.yml`:

- `active_cave` — the path of the currently open cave. This is runtime state only; the last-open
  cave path is persisted separately by the app, outside any cave.
