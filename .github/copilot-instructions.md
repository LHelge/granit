# Granit — Copilot Instructions

Granit is a minimal desktop note-taking app built around a local markdown "cave" and a backend-owned state model.

## Architecture

- Backend: Tauri 2 + Rust. It is the single source of truth for state, file I/O, markdown rendering, cave operations, and agent behavior.
- Frontend: Leptos 0.8 CSR in `src/`. Treat it as a thin IPC view layer.
- Editor: CodeMirror 6 in `js/editor.ts`, bundled to `build/codemirror.js` and exposed as `window.GranitEditor`.
- Styling: Tailwind CSS 4 + DaisyUI 5. Use utility classes and DaisyUI classes directly in `view!` macros.
- Shared types: `granit-types/` contains config, document, agent, and IPC payload types.

Key areas:
- `src/app/agent/` — chat UI and streaming
- `src/app/editor/` — writer, reader, frontmatter, CodeMirror bindings
- `src/app/explorer/` — calendar, tree, search, todo, tags, favorites, templates
- `src-tauri/src/commands/` — Tauri command handlers and app state
- `src-tauri/src/cave/` — cave operations
- `src-tauri/src/markdown/` — frontmatter parsing and HTML rendering

## Core Rules

- Keep logic in the backend when possible. If the frontend needs derived data, add a backend command instead of duplicating logic.
- One `#[tauri::command]` per operation. Keep handlers thin and delegate into modules.
- Use typed errors with `thiserror`. Do not use `anyhow`.
- All code, comments, and documentation should be in English.
- Build only what is needed now. No plugin system, sync layer, or speculative abstractions.

## Cave Model

- A cave is a user-selected directory containing markdown notes plus a `.granit/` directory.
- Notes are identified by filename stem only. Do not assume a frontmatter `title` field exists.
- Wiki-links resolve by filename across the whole cave, not by relative path.
- Note filenames are globally unique across a cave.
- `.granit/config.yml` stores cave-local settings.
- `.granit/templates/` stores cave-local note templates.
- `active_cave` is runtime-only and must not be serialized into cave YAML.
- The last open cave path is persisted separately via `tauri-plugin-store`.

## Markdown and Editor

- Markdown is rendered in the backend with `pulldown-cmark`.
- Frontmatter is parsed separately from the markdown body.
- Raw HTML must be sanitized before reaching the webview.
- Reader mode renders backend HTML; edit mode uses CodeMirror.
- Task list checkboxes are interactive in the note reader and disabled in agent-rendered markdown.

## AI Agent

- Agent logic lives entirely in the backend.
- Supported providers are Ollama, Anthropic, Mistral, and Prisma.
- Tools cover notes, folders, templates, daily notes, todos, search, web fetch, and web search.

## Frontend Notes

- Use Leptos signals and the existing IPC wrapper layer in `src/app/ipc.rs`.
- Prefer DaisyUI component classes before hand-rolled equivalents.
- The user is strong on Rust backend work and less experienced with Leptos/Tailwind/DaisyUI. Be more explicit for frontend changes.

## Icons

- Icons use `leptos_icons` + `icondata_lu`.
- `Icon` has no `class` prop. Put sizing, color, spacing, and rotation on a wrapper element.
- `ProviderIcon` remains in `src/app/components/icons.rs` for provider brand assets.

## Development

Common commands:

```sh
npm ci
npm run build
cd src-tauri && cargo tauri dev
cargo test -p granit
cargo test -p granit-types
wasm-pack test --headless --firefox
```

Before committing, run formatting, clippy, and the relevant tests.

## Not Yet

- File watching / external change reload
- Obsidian-style live preview editor
- Backlinks panel
- Sync
