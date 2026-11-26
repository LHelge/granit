---
name: bears-planning
description: "Plan work in the Bears task tracker. Use when: breaking down features into epics and sub-tasks, creating implementation plans, organizing work with dependencies and priorities, scoping new functionality, decomposing user stories into trackable tasks."
---

# Bears Planning

Plan work as Bears epics with ordered, dependency-linked sub-tasks, stored as Markdown files
in `.bears/`. A planned epic is meant to be implementable unattended with
`/implement-epic <id>`, so every task must be clear enough that an agent can build and a
reviewer can check it without asking.

## Tools

Use the Bears MCP tools (`mcp__bears__*`). Without them, use the `bea` CLI with `--json`
(`bea <command> --help` lists the flags):

| Action | MCP tool | CLI |
|--------|----------|-----|
| Create epic | `create_task(title, type="epic", priority)` | `bea create "Title" --epic --priority P1` |
| Create task | `create_task(title, priority, parent, tags, depends_on)` | `bea create "Title" --priority P1 --parent <epic> --tag backend --depends-on <id>` |
| Add / remove dependency | `add_dependency` / `remove_dependency` | `bea dep add <id> <dep>` / `bea dep remove <id> <dep>` |
| Move under another epic | `update_task(id, parent)` | `bea update <id> --parent <epic>` |
| List, ready, epics | `list_all_tasks`, `list_ready`, `list_epics` | `bea list --epic <id>`, `bea ready --epic <id>`, `bea epics` |
| Show, search | `get_task`, `search_tasks` | `bea show <id>`, `bea search "query"` |

IDs are hierarchical: a task under epic `zem` gets an id like `zem.4q`, so the parent is part
of the id and the file name. Moving a task renames it and its subtree; use the new ids the
call returns.

## Procedure

1. **Understand the request.** Clarify scope, must-haves and constraints with the user. Read
   `CLAUDE.md` and the code where the work will land. Check existing epics and tasks
   (`list_epics`, `search_tasks`) for duplicates and related work.
2. **Design the breakdown** before creating anything:
   - **One task = one commit = one release-note line.** Release notes are generated from
     commit messages, so a task is a user-visible change (or a clearly named internal one),
     and its title reads like the changelog entry it will become.
   - **Layer order** for a feature: `granit-types` → backend module (`cave/`, `agent/`,
     `backup/`, `markdown/`) → `#[tauri::command]` in `generate_handler!` → `src/app/ipc.rs`
     wrapper → Leptos component. Split along that order only where a layer is substantial
     on its own; a small feature is one task through all layers.
   - **Tests and docs belong to the task** that changes the behaviour: its `CLAUDE.md` and
     `docs/content/wiki/` updates are acceptance criteria, not a separate task.
   - **Tasks that touch the same shared seam** (the IPC surface, `Cargo.toml`/`Cargo.lock`,
     `package.json`, the JS bundles, migrations) are serialized with dependencies, since
     only one can run at a time.
   - **Tag** each task by area: `backend`, `frontend`, `agent`, `editor`, `docs`, `ci`,
     `theme`. Tag work only the user can do (accounts, secrets, manual testing on a device)
     `manual`: `implement-epic` never hands it to an agent. Name visual-design tasks plainly
     (a theme, a panel's look): the orchestrator does those itself.
3. **Create the epic**, then its sub-tasks in dependency order with `parent` and
   `depends_on`. The epic's body states its scope and its own acceptance criteria (what
   the whole branch must deliver).
4. **Write each task body** — edit the `.bears/<id>-<slug>.md` file directly for long text:

   ```markdown
   ## Summary
   What this task delivers and why, in one paragraph.

   ## Acceptance Criteria
   - [ ] Specific, testable condition (each is implemented and tested, or the task is not done)
   - [ ] Docs: the wiki page / CLAUDE.md section that changes

   ## Implementation Notes
   - Files to read or modify, patterns to follow (with paths)
   - Key types, commands, events, config fields involved

   ## Edge Cases
   - Error scenarios, validation rules, effect on existing caves
   ```

   The checks are the ones `CLAUDE.md` lists; do not restate them per task.
5. **Verify and present.** `list_ready(epic)` should show only the first unblocked tasks.
   Present a table to the user and ask them to confirm or adjust:

   ```
   Epic: <title> (<id>)

   | Task | ID | Priority | Depends on |
   |------|----|----------|------------|
   ```

## Priorities

| Priority | When |
|----------|------|
| **P0** | Blocking other work or breaking existing functionality |
| **P1** | Core functionality on the critical path |
| **P2** | Standard work in the epic, not blocking |
| **P3** | Polish, nice-to-have, follow-up |

## Notes

- Epics close by themselves when their last child is done.
- Bears rejects dependency edges that would form a cycle.
- Frontmatter (`status`, `priority`, `tags`, `depends_on`) can be edited in the file; the
  parent cannot, because it is part of the id.
