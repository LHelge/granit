---
id: bgy.cf
title: "docs: themes, configuration, installation and download pages"
status: open
priority: P2
created: "2026-11-26T23:42:04.917395741Z"
updated: "2026-11-26T23:42:04.917395741Z"
tags:
  - docs
---

## Summary

Fix the reference pages: `docs/content/wiki/themes.md`, `configuration.md` and `installation.md`, plus `docs/content/pages/download.md` and `docs/content/pages/about.md`.

## Acceptance Criteria

- [ ] **themes, theme list:** rewrite it. Settings → Theme offers 45 themes in three groups (`src/app/settings/theme.rs`): every DaisyUI 5 built-in, 4 Catppuccin and 6 Community. Keep a table only for the custom ones. The label is "Dark", not "Default Dark".
- [ ] **themes, the dialog:** picking a theme previews it, and it is written to `config.yml` only on **Save**. Describe the Light/Dark filter buttons and the groups.
- [ ] **themes, fallback:** describe how an unknown theme id falls back, after checking it in the running CSS (`index.html` hard-codes `data-theme="dark"`; `styles.css` uses `themes: all`). Do not describe the fallback until it has been checked.
- [ ] **Fonts:** keep the font table and YAML in **one** of themes and configuration, and link to it from the other.
- [ ] **configuration:**
  - Add the default models `qwen3.5:9b` (Ollama) and `gpt-4o-mini` (OpenAI), and the default embedding model `AllMiniLML6V2Q` (`granit-types/src/agent.rs`, `agent/vectordb.rs`).
  - Panel widths are set by dragging the panel edge; the dialog has no field for them.
  - Cut the `tauri-plugin-store` internals and the "Keys not written by hand / `active_cave`" section.
- [ ] **installation:**
  - Bundles land in `target/release/bundle/` at the workspace root.
  - List the Linux build packages from `.github/workflows/release.yml`.
  - Run `cargo tauri build` without `TAURI_SIGNING_PRIVATE_KEY`. If it fails because `createUpdaterArtifacts` is on, document `cargo tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}'`.
  - Remove the doubled release links and the license line that about.md already has.
- [ ] **download:** auto-update is explained once, in installation; download keeps one line and a link.
- [ ] **about:** the tech stack also names CodeMirror 6 and `rig-agent`.

## Implementation Notes

Load the `aphid-content` skill. The `cargo tauri build` check is a real build, which is slow; do it once.