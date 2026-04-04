---
id: qmx
title: Set data-theme on HTML element reactively from Leptos
status: done
priority: P1
created: 2026-04-04T20:03:13.212081954Z
updated: 2026-04-04T20:34:02.223919257Z
tags:
- frontend
depends_on:
- een
parent: uqp
---

## Summary

Set `data-theme` on `<html>` to apply daisyUI themes. Add a default theme in `index.html` to prevent FOUC, and create a reactive Leptos function to update it at runtime.

## Implementation Notes

1. **FOUC prevention** — Set a default `data-theme` in `index.html`:
   ```html
   <html data-theme="dark">
   ```
   This ensures the page has a valid theme even before WASM loads. The "dark" theme is closest to the current default stone palette.

2. **Replace `apply_theme()`** in `src/app/mod.rs`:
   - Remove the current `apply_theme(theme: &Theme)` function that sets 18 `--theme-*` CSS variables
   - Replace with `set_daisy_theme(theme_id: &str)` that sets `data-theme` on `<html>`:
     ```rust
     pub fn set_daisy_theme(theme: &str) {
         if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
             if let Some(el) = doc.document_element() {
                 let _ = el.set_attribute("data-theme", theme);
             }
         }
     }
     ```

3. **Update boot sequence** in `App()` component:
   - After loading config via `fetch_config()`, call `set_daisy_theme(&cfg.theme)` instead of fetching full `Theme` object and calling `apply_theme()`
   - Remove the `ipc::get_active_theme().await` call at boot

4. **Update theme change handler** in `src/app/components/settings/theme.rs`:
   - After `set_active_theme()` IPC call, call `set_daisy_theme(&id)` instead of fetching full Theme and calling `apply_theme()`

## Files to Modify
- `index.html` — add `data-theme="dark"` to `<html>`
- `src/app/mod.rs` — replace `apply_theme()` with `set_daisy_theme()`, update boot sequence
- `src/app/components/settings/theme.rs` — update theme change to use `set_daisy_theme()`

## Acceptance Criteria
- [ ] `index.html` has `data-theme` attribute (prevents FOUC)
- [ ] `set_daisy_theme()` sets `data-theme` on document element
- [ ] Boot sequence applies theme from config without fetching Theme object
- [ ] Theme change in settings immediately updates `data-theme`

## Testing
- App loads with correct theme from config
- Switching themes in settings visually changes immediately
- No flash of wrong theme on startup