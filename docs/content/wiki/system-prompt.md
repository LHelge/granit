---
title: System Prompt
category: AI Agent
tags: [agent, system-prompt, templates, customization]
---

The agent's system prompt — the standing instructions sent with every conversation — is an editable file in your cave: `.granit/agent/system.md`. It is a [Tera](https://keats.github.io/tera/) template rendered when the agent starts, with access to variables describing the current configuration, so one prompt can adapt itself to the active mode, the registered tools, and your [[skills-and-tasks|skills]]. This page covers where the file lives, the available variables, and how to edit it.

# Where it lives

Granit seeds `.granit/agent/system.md` with the default prompt the first time a cave is opened, and never overwrites an existing file. If an older cave had a custom prompt in the `system_prompt` config setting, that value is migrated into the file automatically — see [[configuration#agent]].

The file sits under `.granit/`, outside the note tree, so it never appears in the [[explorer]] file tree and is not a [[wiki-links]] target. See [[cave-rules]] for the role of the `.granit/` directory.

# Editing the prompt

There are two ways in:

- The **Agent tab** in the [[explorer]] sidebar has a *System prompt* entry that opens the file in the editor.
- **Settings → Agent → Edit in editor** opens the same file (and *Reset to default* restores the built-in template).

The editor treats the prompt like any other document: edit mode shows the raw template with [[skills-and-tasks#tera-in-the-editor|Tera syntax highlighting and variable completion]], while reading mode shows the *rendered* prompt — the template evaluated with the live context — so you see exactly what the model will receive. Saving applies immediately: the next message you send uses the new prompt, no restart needed.

> [!NOTE]
> Edits made outside the app (in another editor) take effect the next time the agent is rebuilt — for example after changing any agent setting or reopening the cave.

# Template variables

The template is rendered with the following context:

| Variable | Contents |
|----------|----------|
| `mode` | `"agent"` or `"ask"` — the active operating mode |
| `tools` | Names of the tools actually registered for this agent (disabled and unavailable tools are excluded) |
| `icons` | The note icon IDs the agent may assign |
| `skills` | The cave's [[skills-and-tasks|skills]], each with `name` and `description` |
| `rag` | `true` when note context is injected automatically (Ask mode with embeddings enabled) |
| `today` | Today's date as `YYYY-MM-DD` |
| `year`, `month`, `day` | Today's date components |
| `weekday`, `weekday_short` | Weekday name, full and abbreviated |

Full Tera syntax is available — conditionals, loops, and filters. The default template is a working example of the common patterns:

```jinja2
{% if mode == "agent" -%}
You may create, edit, and organize notes with your tools.

When creating notes you can set an icon using one of these IDs:
{{ icons | join(sep=", ") }}
{%- else -%}
You are in read-only Ask mode: do not offer to modify notes.
{%- endif %}

{% if skills -%}
The following skills are available:
{% for skill in skills -%}
- {{ skill.name }}: {{ skill.description }}
{% endfor -%}
{%- endif %}

Today's date is {{ today }}.
```

Because `tools` reflects what is actually registered, a template can state things conditionally and never mislead the model — for example `{% if "semantic_search" in tools %}` only renders its content when the [[agent-tools-and-rag#retrieval-rag|vector index]] is available.

> [!TIP]
> A template error never breaks the agent: if the file fails to render, Granit falls back to using it as plain text and logs a warning. Reading mode in the editor is an easy way to check that everything renders as intended.

# Related pages

- [[ai-agent]] — provider setup and the two operating modes.
- [[skills-and-tasks]] — skills listed in the prompt, and slash-command tasks.
- [[agent-tools-and-rag]] — the tool names usable in `{{ tools }}` conditions.
- [[configuration]] — the agent settings block and the legacy `system_prompt` migration.
