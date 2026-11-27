# CLAUDE.md

## What Granit is

Granit is a minimal desktop note-taking app. It opens a local directory (a "cave") of markdown notes and provides a rendered reader, an explorer sidebar, an integrated streaming AI agent, slide presentations, and encrypted backups to a self-hosted server. Built for personal use — no plugins, no sync, no speculative abstractions.

## Workspace layout

A Cargo workspace with seven crates plus a JS build step. The frontend is being ported from Tauri + Leptos to GPUI (Bears epic `zem`); until the cutover both apps live side by side.

- `granit-ui` (root crate, `src/`) — Leptos 0.8 CSR frontend, compiled to WASM. Thin IPC view layer.
- `granit-core/` — host-agnostic backend logic with **no Tauri dependency** (keep it that way: `cargo tree -p granit-core -i tauri` must find nothing): `cave/`, `markdown/`, `agent/`, `backup/`, the shared `CoreState` (`state.rs`), the open-cave flow and RAG index build (`open.rs`), and the `granit://` URL builders (`urls.rs`). **Single source of truth** for state, file I/O, markdown rendering, cave operations, backups, and agent behavior. The GPUI app links it directly.
- `granit-app/` — the native GPUI app replacing `granit` + `granit-ui`: `gpui-pre` 0.3.12 (aliased as `gpui`) with GPUI Kit 0.10, one `gpui-pre` in the tree. Binary `granit-app` until the cutover (the Tauri package owns `granit`); product name `Granit`, bundle id `se.lhelge.granit`. See [GPUI app](#gpui-app-granit-app).
- `granit` (`src-tauri/`) — Tauri 2 shell around `granit-core`: IPC commands (`commands/`), events, plugins (store, dialog, opener, clipboard, log, updater), the presentation window, and the `granit://` request handler (`scheme.rs`). `lib.rs` re-imports the core modules at the crate root, so `crate::cave::…` paths still work there.
- `granit-types/` — shared types (config, document, agent, IPC payloads) used by both frontend and backend.
- `granit-api/` — wire types of the backup API, shared by the desktop client and the server.
- `granit-server` (`backend/`) — self-hosted backup server: axum, Postgres via sqlx (committed `.sqlx/` offline data, reversible migrations), S3-compatible storage (RustFS), Caddy. See [backend/README.md](backend/README.md).
- `js/editor.ts` / `js/mermaid.ts` — CodeMirror 6 and Mermaid, bundled by esbuild to `build/codemirror.js` / `build/mermaid.js` (`window.GranitEditor` / `window.GranitMermaid`). The Mermaid bundle is also served to the presentation window at `granit://asset/mermaid.js`.

## Build, test, run

```sh
npm ci                               # install JS deps (first time)
npm run build                        # Tailwind CSS + CodeMirror/Mermaid bundles into build/
cd src-tauri && cargo tauri dev      # run the full app (launches Trunk + Tauri)
trunk serve                          # frontend only, port 1420
cargo run -p granit-app -- <cave>    # run the GPUI app on a cave (not remembered as the last-open cave)

cargo fmt --all
cargo clippy -p granit -p granit-core -p granit-app --all-targets -- -D warnings  # backend + core + GPUI app
cargo clippy -p granit-ui --target wasm32-unknown-unknown -- -D warnings  # frontend
SQLX_OFFLINE=true cargo clippy -p granit-server -p granit-api --all-targets -- -D warnings

cargo test -p granit-core            # core tests: cave, markdown, agent, backup (cargo test -p granit-core <name> for one)
cargo test -p granit                 # Tauri shell tests: commands, scheme
cargo test -p granit-app             # GPUI app: headless gpui::test windows, no display needed
cargo test -p granit-types
cargo test -p granit-api
cargo test -p granit-server          # needs DATABASE_URL to a Postgres (backend/README.md)
wasm-pack test --headless --chrome   # frontend WASM tests (CI uses --firefox)

cd backend && cargo sqlx prepare -- --all-targets   # after changing any sqlx query
```

The frontend is a `wasm32-unknown-unknown` crate, so a plain `cargo clippy --workspace` fails on it. On Linux, `granit-app` needs the GPUI system libraries (xkbcommon, Wayland, Vulkan, fontconfig; the list is in CI's apt step) and a Vulkan driver to run. CI ([.github/workflows/ci.yml](.github/workflows/ci.yml)) and the git hooks run the commands above. Trunk does not produce `build/`, so `npm run build` must run before the frontend loads; use `npm run watch:css` / `npm run watch:js` while iterating. Tailwind scans `src/**/*.rs`, so new utility classes need a CSS rebuild.

Activate the version-controlled hooks once per clone with `git config core.hooksPath .githooks`: `pre-commit` checks formatting, `pre-push` runs the clippy and test commands (`GRANIT_WASM_TEST_BROWSER=firefox` to match CI).

## Architecture: backend-owned state

The backend is authoritative. The frontend holds reactive copies in `AppCtx` ([src/app/context.rs](src/app/context.rs)) but never owns logic. **If the frontend needs derived data, add a backend command rather than duplicating logic in WASM.**

- **`CoreState`** ([granit-core/src/state.rs](granit-core/src/state.rs)) is the host-agnostic state: `AppConfig`, the open `Cave` (`SharedCave = Arc<Mutex<Option<Cave>>>`, which also holds the active note), the lazily-built `Agent` and its generation counter, and the RAG `CaveVectorIndex` — each behind its own mutex — with the helpers `with_cave`, `ensure_agent` and `reset_agent`.
- **`AppState`** ([src-tauri/src/commands/state.rs](src-tauri/src/commands/state.rs)) is the `tauri::manage`d singleton: it wraps a `CoreState` (reached through `Deref`) and adds the Tauri-only parts, the shown presentation and the IPC config response. Event emitting stays in the command handlers; core code returns values instead.
- **IPC** is one `#[tauri::command]` per operation, all registered in `tauri::generate_handler!` in [src-tauri/src/lib.rs](src-tauri/src/lib.rs). Handlers are thin and delegate into `granit-core`'s `cave/`, `agent/`, `backup/`, or `markdown/`. The frontend calls them through the typed wrappers in [src/app/ipc.rs](src/app/ipc.rs).
- **Cave mutations** (including agent tool calls) emit `cave:notes-changed`; the listener in [src/app/mod.rs](src/app/mod.rs) refreshes notes/folders/active note. The active note slug is pushed back with `set_active_note` so agent tools can see it.
- **Errors** use `thiserror`, one enum per module (`CaveError`, `AgentError`, `BackupError`, …). Do **not** use `anyhow`.
- **Updates**: [src-tauri/src/commands/updater.rs](src-tauri/src/commands/updater.rs) checks GitHub releases on startup (not in debug builds) and keeps pending release notes in the store.

## Cave model

- A cave is any user-selected directory of markdown notes plus a `.granit/` directory: `config.yml`, `templates/`, `presentations/`, `agent/`, and the caches `embeddings.bin` and `backup.key`.
- **Filenames are globally unique across the whole cave**; a duplicate refuses to open the cave (`CaveError::DuplicateSlug`). The filename stem is the note's identity, slug, and displayed title; frontmatter does **not** override it. Slug matching is case-insensitive.
- Wiki-links `[[note]]` / `[[note|label]]` resolve by filename across the cave, not by relative path. Broken links are styled separately. Renaming a note rewrites the links to it.
- **Heading anchors**: `# Volvo {#volvo}` makes a link target in the same namespace as note slugs (`[[Volvo]]` resolves to `note#volvo`). Plain headings are not targets; a collision with a slug or another anchor refuses to open the cave (`CaveError::DuplicateAnchor`). `Cave::resolve_link` is the resolver passed to markdown rendering.
- `Cave` keeps in-memory indexes (notes, backlinks, anchors, templates, skills, tasks, presentations) populated lazily by `ensure_scanned()`. Use `CoreState::with_cave` / `with_shared_cave`, which lock, ensure-scan, then run a closure. Anything a mutation changes must be kept in sync in these indexes.
- **Opening a cave** is `granit_core::open`: `prepare_cave` (open, write a default config, load it, seed `system.md` with the legacy-prompt migration, seed presentation templates), then `CoreState::install_cave`, then `sync_vector_index`. Both hosts call it; the Tauri app also closes the presentation window and persists the path in between.
- Config is **cave-local** in `.granit/config.yml`. The last-open cave path lives separately: in `tauri-plugin-store` for the Tauri app, in `<config dir>/se.lhelge.granit/last-cave.json` for the GPUI app. `active_cave` is runtime-only in IPC responses and is **never** serialized into cave YAML. New config fields need serde defaults so existing caves still load.
- Hidden directories and `.granit/` are excluded from note scans. Templates in `.granit/templates/` have their own flat slug namespace.
- **Presentation templates** are CSS files in the flat `.granit/presentations/` ([cave/presentations.rs](granit-core/src/cave/presentations.rs)); other files there are shared assets. Defaults are seeded in the open-cave flow when the directory is missing, never inside `ensure_scanned`. A note binds to one with the `presentation` frontmatter field.
- **`granit://` scheme** (served by [src-tauri/src/scheme.rs](src-tauri/src/scheme.rs), URLs built by [granit-core/src/urls.rs](granit-core/src/urls.rs)): `cave/<path>` (files under the cave root, e.g. note images) and `presentation/<path>` (template files, or the rendered page for a bare note slug). Paths are canonicalized and scoped to their root.

## Markdown

Rendered in the backend with `pulldown-cmark` ([granit-core/src/markdown/](granit-core/src/markdown/)). Frontmatter (`tags`, `created_at`/`modified_at`, `icon`, `favorite`, `presentation`) is parsed separately and never shown in the editor; the note header edits it. `read_note` returns the body (the file minus its frontmatter block, byte for byte) and `save_note`/`update_note` re-attach the existing block via `Markdown::rebuild`, which never re-serializes it: an unchanged save writes nothing, and a change splices only the touched owned keys (`tags`, `icon`, `favorite`, `presentation`, plus `modified_at`, stamped only when the body or an owned field changed) while unknown keys, order, comments, quoting and CRLF survive and `created_at` is never touched. What counts as frontmatter is decided in `locate_frontmatter` ([markdown/frontmatter.rs](granit-core/src/markdown/frontmatter.rs)): a leading `---` … `---`/`...` block that is blank, a YAML mapping, or invalid YAML whose first line looks like `key:` (kept verbatim and never stamped); anything else is body, and only the first block of a file is considered, so a body's own leading `---` block is never stripped. Raw HTML is sanitized before reaching the webview. Task-list checkboxes are interactive in the reader and disabled in agent-rendered markdown. Fenced `mermaid` blocks become diagrams only in the reader and on slides. Relative image sources are rewritten to the `granit://` cave route (`Markdown::with_image_base`).

**Presentations** ([markdown/presentation.rs](granit-core/src/markdown/presentation.rs), window in [commands/presentation.rs](src-tauri/src/commands/presentation.rs)): the body splits into slides on thematic breaks and renders as a standalone page in a single `presentation` window, reloaded on note/template save and closed on rename/delete/cave switch. The slide DOM and CSS custom-property contract is public — documented in [docs/content/wiki/presentations.md](docs/content/wiki/presentations.md) — and templates rely on it, so change it only deliberately and update that page.

## AI agent

All agent logic is backend-side ([granit-core/src/agent/](granit-core/src/agent/)), built on `rig-core` + `rig-agent`.

- **Providers**: Ollama, Anthropic, Mistral, and any OpenAI-compatible endpoint. Add one with match arms in `Agent::from_config` and `list_models` and a `ProviderConfig` variant.
- The `Agent` is built lazily by `CoreState::ensure_agent` and torn down (`reset_agent`, bumping the generation counter) whenever config, provider, model, or mode changes.
- **Tools** ([agent/tools/](granit-core/src/agent/tools/), catalogue in `mod.rs`) cover notes, folders, daily notes, todos, tags, templates (list only), text and semantic search, skills and agent docs, web fetch, and web search. `build_toolset` drops `disabled_tools`, `semantic_search` without a vector index, and `web_search` without a Brave key.
- **Modes** (`AgentMode`): `Ask` gets the read-only tools (everything outside `MUTATING_TOOLS`) plus injected RAG context; `Agent` gets all tools and no RAG injection.
- **Agent documents** live in `.granit/agent/` ([cave/agent_docs.rs](granit-core/src/cave/agent_docs.rs)): `system.md` is a Tera template rendered with `PromptContext` ([agent/prompt.rs](granit-core/src/agent/prompt.rs)), seeded from `DEFAULT_SYSTEM_PROMPT_TEMPLATE` when missing; `skills/<name>/SKILL.md` (Agent Skills format) are listed in the prompt and loaded with `use_skill`; `tasks/<name>.md` are user-invoked `/task` prompts.
- **RAG** ([agent/vectordb.rs](granit-core/src/agent/vectordb.rs)): `CaveVectorIndex` embeds notes with local CPU `fastembed` and caches them in `.granit/embeddings.bin` (`rkyv`). It must be built inside a tokio runtime, whose handle it keeps for its debounced updates. It is rebuilt in a background task (`granit_core::open::build_vector_index`, given the model directories and a tokio handle by the host) on cave open or RAG config change ([commands/config.rs](src-tauri/src/commands/config.rs)) and updated incrementally on UI note create/save/rename/delete ([commands/cave.rs](src-tauri/src/commands/cave.rs)).
- Streaming: `Agent::stream_with_history` returns an `AgentStream` of `AgentStreamItem`s (text / tool call / tool result / done).

## Backups

Client side in [granit-core/src/backup/](granit-core/src/backup/), commands in [commands/backup.rs](src-tauri/src/commands/backup.rs), settings in `AppConfig.backup`. A cave is archived (tar + zstd, `.git` excluded), encrypted locally with a key derived from a passphrase (Argon2id; the key is cached in `.granit/backup.key`, the passphrase never stored), and uploaded through presigned URLs from `granit-server`. Restore goes to a new folder or replaces the cave in place (the old one is kept as `<cave>.pre-restore-*`); a disaster restore works with no cave open. Progress is reported with `backup:*` / `restore:*` events, and a single `OperationGuard` keeps backups and restores from overlapping. The server's API must change only additively, and merged migrations are never edited.

## GPUI app (`granit-app`)

`granit-app [<cave>]` opens the cave given on the command line (never remembered, so testing never touches the live cave pointer), else the last-open cave, else the folder picker (GPUI's `cx.prompt_for_paths`). Modules in [granit-app/src/](granit-app/src/):

- **`host.rs`**: the `Host` GPUI global holding `Arc<CoreState>` (`Host::core(cx)`), the last-cave store and the embedding-model dirs (`HostOptions`; `HostOptions::default()` in tests persists nothing and builds no RAG index).
- **`cave_ops.rs`**: the UI's cave operations (open, list, read, create/rename/move/delete notes and folders, set the active note). Each runs on the core, keeps the RAG index current like the Tauri commands, and emits a `CaveEvent`. Synchronous on the foreground except opening, which prepares the cave on the background executor.
- **`events.rs`**: `CaveEvents`, the app-wide emitter of `CaveEvent` (`Opened`, `NotesChanged`, `NoteRenamed`, `NotesRemoved`). Views `cx.subscribe(&CaveEvents::entity(cx), …)` and refresh; code that changes the cave outside `cave_ops` (agent tools) calls `CaveEvents::emit(CaveEvent::NotesChanged, cx)` from the foreground.
- **`runtime.rs`**: one process-wide multi-threaded tokio runtime (`runtime::handle()`, `runtime::spawn`). Await its `JoinHandle` inside `cx.spawn` to bring results back to the foreground.
- **`workspace.rs`**: the root view (inside GPUI Kit's `Root`, opened by `open_main_window`): sidebar (cave name, open-cave button, explorer) and main area. It owns the open note: `Workspace::open_note` updates the core's active slug, the main view and the explorer selection, and follows `NoteRenamed`/`NotesRemoved`.
- **`explorer/`**: the note tree on GPUI Kit's `Tree` (`tree_model.rs` builds it, `row.rs` renders rows with note icons and favorite stars, inline rename input, drag and drop). Emits `ExplorerEvent::OpenNote`.
- **`note_view.rs`**: read-only placeholder for the open note until the editor lands.
- **`theme.rs`** / `themes.json`: Catppuccin Latte and Frappé, following the window appearance; read colors with `cx.theme()`. **`assets.rs`**: GPUI Kit's icons plus the note-icon catalog (`note-icons/<Id>.svg`, built from `granit_types::NOTE_ICONS`) and a few `AppIcon`s.

Tests are `#[gpui::test]`s on GPUI's headless test platform ([granit-app/src/tests.rs](granit-app/src/tests.rs) has the `App` harness); render paths are exercised with `window.draw(cx)`.

## Frontend conventions

- Leptos signals + the `AppCtx` context (`expect_context::<AppCtx>()`) rather than threading props. IPC goes through the [src/app/ipc.rs](src/app/ipc.rs) wrappers.
- The editor autosaves after a 2 s holdoff; saves go through the snapshots in `editor/save_queue.rs`, never live signals, so a quick note switch cannot write to the wrong slug.
- Styling is Tailwind CSS 4 + DaisyUI 5 utility classes written directly in `view!` macros. Prefer DaisyUI component classes before hand-rolling.
- Icons use `leptos_icons` + `icondata_lu`. `Icon` has **no** `class` prop — put sizing/color/spacing/rotation on a wrapper element. Provider brand assets live in `ProviderIcon` ([src/app/components/icons.rs](src/app/components/icons.rs)).

## Conventions

- All code, comments, and docs in English.
- Build only what is needed now. No plugin system, sync layer, or speculative abstractions (deferred: file watching / external reload, live-preview editor, sync).
- A change to features, configuration, or behavior updates this file where it describes that part and the user docs in `docs/content/wiki/` (built with aphid; see the `aphid-content` skill) in the same change.
- **Release notes are generated from commit messages**, so each feature lands as one focused commit with a Conventional Commits subject in the user's terms. Don't commit after a first implementation pass — present the result, iterate on feedback, and commit only once the feature is agreed done. Follow-up tweaks in the same session are amended or squashed into the feature commit rather than added as `fix`/`refactor` commits (never amend commits that are already merged or that others may have pulled).
  - **Exception:** under the `implement-epic` skill the user has delegated review to the `task-reviewer` agent, so each Bears task lands as one commit once its review passes, without waiting for the user.
- The repo owner is strong on Rust backend work and less experienced with Leptos/Tailwind/DaisyUI — be more explicit and cautious for frontend changes.
- Before committing, run `cargo fmt` and the clippy and test commands above for the crates you touched.

## Planning

Tasks are tracked with [Bears](https://github.com/LHelge/bea-rs) (`.bears/`), through its MCP tools in the main checkout; agents working in a git worktree use the `bea` CLI instead, because the MCP server writes to the main checkout. Use the `bears-planning` skill to plan epics, and `/implement-epic <id>` to implement one unattended.
