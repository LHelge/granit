---
description: daisyUI 5
alwaysApply: true
applyTo: "**"
---

# DaisyUI 5

Local rules for DaisyUI usage in this repo.

## Stack

- DaisyUI 5 requires Tailwind CSS 4.
- Do not add `tailwind.config.js`. Tailwind 4 in this repo is configured from CSS.
- The normal CSS setup is:

```css
@import "tailwindcss";
@plugin "daisyui";
```

## Usage

- Prefer DaisyUI component classes such as `btn`, `input`, `card`, `menu`, `tabs`, `badge`, `alert`, and `modal`.
- Use Tailwind utilities for layout, spacing, sizing, and small overrides.
- If DaisyUI does not provide the exact component, build it with Tailwind utilities instead of adding custom CSS.
- Use responsive utility prefixes for layouts.
- Avoid custom CSS unless a utility or DaisyUI class cannot solve the problem cleanly.

## Colors and Themes

- Prefer DaisyUI semantic colors: `primary`, `secondary`, `accent`, `neutral`, `base-100`, `base-200`, `base-300`, `info`, `success`, `warning`, `error` and their `*-content` variants.
- Prefer semantic colors over fixed Tailwind palette colors so themes keep working.
- Do not add `bg-base-100 text-base-content` to `body` unless there is a specific need.

## Design Guidance

- Preserve the existing visual language unless the task explicitly asks for a redesign.
- Prefer clear, intentional layouts over generic utility piles.
- Do not add custom fonts unless there is a strong reason.
- When placeholders are needed, image placeholders such as `https://picsum.photos/...` are acceptable.

## In `view!` Macros

- Use only valid DaisyUI class names and Tailwind utility classes.
- Prefer inline class composition in the Rust `view!` markup instead of introducing extra wrapper CSS.
