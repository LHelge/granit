---
id: jqd
title: CSS language mode in the CodeMirror bundle
status: done
priority: P2
created: "2026-09-08T22:02:25.695806571Z"
updated: "2026-09-09T08:18:53.257384Z"
tags:
  - presentation
  - frontend
  - js
parent: h7p
---

Make the CodeMirror bundle (`js/editor.ts`, built by esbuild to `build/codemirror.js`) able to edit CSS documents.

- Add `@codemirror/lang-css` to package.json.
- Add a `language` option to the `GranitEditor.create` config (`"markdown"` default, `"css"`), or an equivalent `setLanguage` call, and a matching wasm-bindgen binding in `src/app/editor/codemirror.rs`.
- When the language is CSS: load the CSS language and highlighting only; keep wiki-link decorations, slug completion, markdown keymap, and Tera mode off. Search, fonts, read-only toggle, and content get/set behave as before.
- Markdown behaviour must be unchanged; existing wasm-pack tests still pass.

Independent of the other presentation tasks. Consumed by the explorer/editor UI task.