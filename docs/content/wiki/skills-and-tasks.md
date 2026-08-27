---
title: Skills and Tasks
category: AI Agent
tags: [agent, skills, tasks, slash-commands, customization]
---

Skills and tasks are the two ways to teach the agent your own workflows, both stored as files in the cave's `.granit/agent/` directory. A **skill** is a set of reusable instructions the agent loads on demand when a request matches its description. A **task** is a reusable prompt you invoke yourself by typing `/task-name` in the chat. Both are managed from the **Agent tab** in the [[explorer]] sidebar and edited like any other document.

# Skills

Skills follow the [Agent Skills specification](https://agentskills.io/specification): each skill is a folder in `.granit/agent/skills/` containing a `SKILL.md` file whose frontmatter declares a `name` and a `description`, followed by the instructions.

```
.granit/agent/skills/
  research-summary/
    SKILL.md
```

The agent sees every skill's name and description in its [[system-prompt]]. When a request matches a description, it calls the `use_skill` tool to load the full instructions — so many skills can exist without bloating every conversation. Write the description to say *what the skill does and when to use it*; that is what triggers it.

Skill names are kebab-case: lowercase letters, digits, and hyphens, up to 64 characters, matching the folder name (Granit keeps the frontmatter `name` in sync when you rename).

> [!TIP]
> Because Granit follows the open specification, a skill folder written for another spec-compliant agent can be dropped straight into `.granit/agent/skills/` and works. Frontmatter fields Granit does not manage — `license`, `compatibility`, `metadata`, `allowed-tools` — are preserved when you edit the skill in the app. Only `SKILL.md` itself is used: bundled `scripts/` and `references/` folders are ignored, so fold any essential reference material into the main file.

# Tasks

Tasks are Markdown files in `.granit/agent/tasks/`, one per task, with a `description` in the frontmatter and a [Tera](https://keats.github.io/tera/) template as the body. Typing `/` in the chat input opens an autocomplete popup over your tasks — arrow keys to navigate, Enter or Tab to complete, Escape to dismiss. Submitting `/task-name some text` renders the task's template and sends the result as the prompt.

The template has access to:

| Variable | Contents |
|----------|----------|
| `input` | The text typed after the task name (may be empty) |
| `active_note` | Slug of the note open in the editor — only set while a note is open |
| `today`, `tomorrow`, `yesterday` | Dates as `YYYY-MM-DD` |
| `year`, `month`, `day`, `weekday`, `weekday_short` | Today's date components |

A minimal example, `.granit/agent/tasks/summarize.md`:

```markdown
---
description: Summarize the given text or the open note.
---

Summarize {% if input %}{{ input }}{% else %}[[{{ active_note }}]]{% endif %}
in three bullet points, in the note's own language.
```

Unlike the [[system-prompt]], a task template that fails to render reports the error in the chat instead of silently sending broken text — guard optional variables like `active_note` with `{% if %}` or the `default` filter.

# Editing and the Agent tab

The Agent tab in the [[explorer]] sidebar lists the system prompt, skills, and tasks, with buttons to create and delete. Opening one puts it in the editor: the title renames it, a description field below the title edits the frontmatter description, and the body is ordinary Markdown — you never touch raw YAML. A collapsible help panel in the tab summarizes the formats and variables.

## Tera in the editor {#tera-in-the-editor}

When editing a task, a note [[templates|template]], or the system prompt, the editor highlights Tera blocks — `{{ expressions }}`, `{% statements %}`, and `{# comments #}` — and completes as you type inside them: the document's own variables, statement keywords after `{%`, and common filters after a `|`. Skills are excluded on purpose: `SKILL.md` is sent to the model verbatim, never rendered as a template.

# The agent can improve its own skills and tasks

Through the `read_agent_doc` and `write_agent_doc` tools, the agent itself can review, refine, and create skills and tasks — ask it in the chat to "create a task that…" or "improve the research-summary skill", and iterate together. Deleting and renaming remain manual operations in the Agent tab. See [[agent-tools-and-rag]] for the tool details.

# Related pages

- [[system-prompt]] — where the skill listing is injected, and its template variables.
- [[ai-agent]] — modes and provider setup.
- [[agent-tools-and-rag]] — `use_skill`, `read_agent_doc`, `write_agent_doc`, and the rest of the toolset.
- [[templates]] — note templates, the third kind of Tera-templated file in a cave.
