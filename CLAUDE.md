# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What Granit is

Granit is a minimal desktop note-taking app. It opens a local directory (a "cave") of markdown notes and provides a rendered reader, an explorer sidebar, and an integrated streaming AI agent. Built for personal use — no plugins, no sync, no speculative abstractions.

## Workspace layout

A Cargo workspace with three crates plus a JS build step:

- `granit-ui` (root crate, `src/`) — Leptos 0.8 CSR frontend, compiled to WASM. Thin IPC view layer.
- `granit` (`src-tauri/`) — Tauri 2 Rust backend. **Single source of truth** for state, file I/O, markdown rendering, cave operations, and agent behavior.
- `granit-types/` — shared types (config, document, agent, IPC payloads) used by both frontend and backend.
- `js/editor.ts` — CodeMirror 6 editor, bundled by esbuild to `build/codemirror.js` and exposed as `window.GranitEditor`.
- `js/mermaid.ts` — Mermaid wrapper, bundled to `build/mermaid.js` as `window.GranitMermaid`. The backend build script stages the same bundle into `OUT_DIR` (empty, with a warning, when `npm run build` has not run) and serves it at the `granit://` `asset/mermaid.js` route for the presentation window.

## Build, test, run

```sh
npm ci                               # install JS deps (first time)
npm run build                        # build CSS (Tailwind) + CodeMirror bundle into build/
cd src-tauri && cargo tauri dev      # run the full app (launches Trunk + Tauri)
trunk serve                          # frontend only, port 1420

cargo test -p granit                 # backend tests
cargo test -p granit-types           # shared-types tests
cargo test -p granit <name>          # single backend test by name
wasm-pack test --headless --firefox  # frontend WASM tests

cargo fmt --all
cargo clippy -p granit --all-targets -- -D warnings              # backend lints
cargo clippy -p granit-ui --target wasm32-unknown-unknown -- -D warnings  # frontend lints
```

The frontend is a `wasm32-unknown-unknown` crate, so it must be clippy-checked against that target — a plain `cargo clippy --workspace` fails on it. CI ([.github/workflows/ci.yml](.github/workflows/ci.yml)) runs fmt, both clippy invocations, `cargo test -p granit`, and the wasm-pack tests.

`npm run build` must run before the frontend can load — Trunk does not produce `build/styles.css` or `build/codemirror.js`. Use `npm run watch:css` / `npm run watch:js` during frontend iteration.

### Git hooks

Version-controlled hooks live in `.githooks/`. Activate them once per clone:

```sh
git config core.hooksPath .githooks
```

`pre-commit` runs `cargo fmt --all -- --check`; `pre-push` runs both clippy invocations, `cargo test -p granit`, and the wasm-pack frontend tests (in Chrome locally; set `GRANIT_WASM_TEST_BROWSER=firefox` to match CI). They mirror CI so failures are caught before pushing.

## Architecture: backend-owned state

The backend is authoritative. The frontend holds reactive copies in `AppCtx` (see [src/app/context.rs](src/app/context.rs)) but never owns logic. **If the frontend needs derived data, add a backend command rather than duplicating logic in WASM.**

- **`AppState`** ([src-tauri/src/commands/state.rs](src-tauri/src/commands/state.rs)) is the `tauri::manage`d singleton. It holds the `AppConfig`, the open `Cave` (behind `SharedCave = Arc<Mutex<Option<Cave>>>`), the lazily-built `Agent`, an agent generation counter, and the RAG `CaveVectorIndex` — each behind its own mutex.
- **IPC** is one `#[tauri::command]` per operation, all registered in `tauri::generate_handler!` in [src-tauri/src/lib.rs](src-tauri/src/lib.rs). Handlers are thin and delegate into `cave/`, `agent/`, or `markdown/` modules. The frontend calls them through the typed `invoke_cmd` wrappers in [src/app/ipc.rs](src/app/ipc.rs).
- **Cave mutations** (including those triggered by agent tools) are picked up by the frontend via an event listener in [src/app/mod.rs](src/app/mod.rs) that refreshes notes/folders/active note. The active note slug is pushed back to the backend (`set_active_note`) so agent tools can see it.
- **Errors** use `thiserror` with one error enum per module (`CaveError`, `AgentError`, `ConfigError`). Do **not** use `anyhow`.

## Cave model

- A cave is any user-selected directory containing markdown notes plus a `.granit/` directory (`config.yml` + `templates/`).
- **Filenames are globally unique across the whole cave** — two notes in different subfolders cannot share a name. The filename stem is the note's identity, slug, and displayed title. Frontmatter does **not** override the title.
- Wiki-links `[[note]]` / `[[note|label]]` resolve by filename across the whole cave, not by relative path. Broken links are styled separately.
- **Heading anchors**: a heading marked with a pandoc attribute — `# Volvo {#volvo}` — becomes a wiki-link target in the *same global namespace as note filenames*, linkable with plain `[[Volvo]]` (resolves to `note#anchor`, scrolling the reader to the heading). Plain headings without `{#id}` are not targets. Anchor ids must be globally unique against both note slugs and other anchors; a duplicate refuses to open the cave (`CaveError::DuplicateAnchor`). The anchor index lives on `Cave` alongside `backlinks`; `Cave::resolve_link` is the resolver passed to markdown rendering.
- `Cave` keeps in-memory indexes (slug→path, backlinks, templates) populated lazily via `ensure_scanned()`. Use `AppState::with_cave` / `with_shared_cave`, which lock, ensure-scan, then run a closure.
- Config is **cave-local**: each cave stores its own sidebar/theme/font/daily-note/agent settings in `.granit/config.yml`. The last-open cave path is persisted separately via `tauri-plugin-store`. `active_cave` is runtime-only in IPC responses and is **never** serialized into cave YAML.
- Hidden directories and `.granit/` are excluded from note scans. Templates live in `.granit/templates/` with their own flat slug namespace.
- **Presentation templates** are CSS files in the flat `.granit/presentations/` directory (index `Cave::presentations`, CRUD in [src-tauri/src/cave/presentations.rs](src-tauri/src/cave/presentations.rs)); other files there are shared assets. Two defaults are seeded in the open-cave flow when the directory is missing, never inside `ensure_scanned`. A note binds to one through the `presentation` frontmatter field.
- **`granit://` scheme** ([src-tauri/src/scheme.rs](src-tauri/src/scheme.rs)): path-based routes `cave/<path>` (any file under the cave root, used for note images) and `presentation/<path>` (template files, or the rendered presentation page for a bare note slug). Paths are canonicalized and scoped; `scheme::base_url` hides the Windows `http://granit.localhost/` spelling.

## Markdown

Rendered in the backend with `pulldown-cmark` ([src-tauri/src/markdown/](src-tauri/src/markdown/)). Frontmatter (YAML: tags, timestamps, icon, `favorite`, `presentation`) is parsed separately from the body. Raw HTML is sanitized before reaching the webview. Reader mode renders backend HTML; edit mode uses CodeMirror. Task-list checkboxes are interactive in the reader and disabled in agent-rendered markdown. Fenced `mermaid` blocks become `<div class="mermaid">` diagram containers in the reader and on slides only (`render_core`'s `mermaid` flag; agent chat and clipboard export keep them as code blocks); the reader runs the bundle after the HTML is mounted, the presentation page script when a slide is shown. Relative image sources are rewritten to the `granit://` cave route when the document's directory is known (`Markdown::with_image_base`).

**Presentations** ([src-tauri/src/markdown/presentation.rs](src-tauri/src/markdown/presentation.rs)): `render_slides` splits the body on thematic-break events and `render_presentation` assembles a standalone page: base mechanics CSS (`presentation.css`, fixed 1280x720 canvas scaled to the viewport, clipping, cursor hiding) before the linked template CSS, one `<section class="slide" data-index data-number>` per slide with the first `active`, then the page script (`presentation.js`: navigation, fullscreen, hash-kept position). Custom properties expose `--slide-count` and `--deck-title/created/modified` on the body and `--slide-index/number/heading/chapter` on each section, and the `{.class}` attributes of a slide's first heading become classes on the section (`slide` and `active` are reserved). That DOM contract is public and documented in the docs site; templates may rely on it. The window itself is managed in [src-tauri/src/commands/presentation.rs](src-tauri/src/commands/presentation.rs) (single `presentation` window, reload on note/template save, close on rename/delete/cave switch). The editor opens presentation templates in the CodeMirror CSS language mode (`DocumentKind::Presentation`).

## AI agent

All agent logic is backend-side ([src-tauri/src/agent/](src-tauri/src/agent/)), built on `rig-core` + `rig-agent`.

- **Providers**: Ollama, Anthropic, Mistral, and any OpenAI/ChatGPT-compatible endpoint (custom base URL + API key, built on rig's `openai` client). The rig agent runtime erases the provider's model type at construction, so every provider arm produces the same `rig_agent::Agent`. Add a provider by adding match arms in `Agent::from_config` and `list_models`, and extending `ProviderConfig`.
- The `Agent` is built lazily by `AppState::ensure_agent` and torn down (`reset_agent`, bumping the generation counter) whenever config/provider/model/mode changes mid-stream.
- **Tools** ([src-tauri/src/agent/tools/](src-tauri/src/agent/tools/)) cover notes, folders, templates, daily notes, todos, search, web fetch, and web search. `build_toolset` filters out `disabled_tools` from config.
- **Modes**: `Ask` and `Agent` (`AgentMode`). RAG context is injected only in `Ask` mode.
- **RAG / vector index** ([src-tauri/src/agent/vectordb.rs](src-tauri/src/agent/vectordb.rs)): `CaveVectorIndex` embeds notes with local CPU `fastembed`, caches embeddings to `.granit/embeddings.bin` (via `rkyv`), and is wired into the agent builder as `dynamic_context(rag_top_n, index)`. The index is built/rebuilt in a background task when a cave opens or RAG config changes ([src-tauri/src/commands/config.rs](src-tauri/src/commands/config.rs)), and incrementally updated on note create/save/rename/delete ([src-tauri/src/commands/cave.rs](src-tauri/src/commands/cave.rs)).
- Streaming: `Agent::stream_with_history` returns a provider-erased `AgentStream` of `AgentStreamItem`s (text / tool call / tool result / done).

## Frontend conventions

- Leptos signals + the `AppCtx` context (`expect_context::<AppCtx>()`) rather than threading props. IPC goes through [src/app/ipc.rs](src/app/ipc.rs) wrappers.
- Styling is Tailwind CSS 4 + DaisyUI 5 utility classes written directly in `view!` macros. Prefer DaisyUI component classes before hand-rolling.
- Icons use `leptos_icons` + `icondata_lu`. `Icon` has **no** `class` prop — put sizing/color/spacing/rotation on a wrapper element. Provider brand assets live in `ProviderIcon` ([src/app/components/icons.rs](src/app/components/icons.rs)).

## Conventions

- All code, comments, and docs in English.
- Build only what is needed now. No plugin system, sync layer, or speculative abstractions (see deferred: file watching / external reload, live-preview editor, backlinks panel, sync).
- **Release notes are generated from commit messages**, so each feature should land as one focused commit. Don't commit after a first implementation pass — present the result, iterate on feedback, and commit only once the feature is agreed done. If a committed feature needs follow-up tweaks in the same session, amend or squash into the feature commit rather than adding `fix`/`refactor` commits that would clutter the changelog (never amend commits that are already merged or that others may have pulled).
- The repo owner is strong on Rust backend work and less experienced with Leptos/Tailwind/DaisyUI — be more explicit and cautious for frontend changes.
- Before committing, run `cargo fmt`, `cargo clippy`, and the relevant tests.

## Planning 

This project uses [Bears](https://github.com/LHelge/bea-rs) for task tracking.
Bears is registered as an MCP server — use the MCP tools to manage tasks.

### Task workflow

- `list_ready` — show tasks ready to work on (all dependencies done)
- `start_task` — mark a task as in-progress before starting
- `complete_task` — mark a task done when finished
- `create_task` — create new tasks or epics
- `get_graph` — visualize the dependency graph