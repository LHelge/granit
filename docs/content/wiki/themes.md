---
title: Themes & Fonts
category: Reference
tags: [themes, fonts, appearance]
---

Granit ships a curated set of UI themes plus per-surface font settings, all stored per cave in
`.granit/config.yml`. This page lists the available themes, how to switch them, and how to set fonts
for the editor, reader, and agent chat. For the full configuration reference, see [[configuration]].

# Available themes

Each theme has an id (the value written to the `theme` key) and a light or dark color scheme. The
built-in `dark` theme is the default.

| Theme | Id | Variant |
| --- | --- | --- |
| Default Dark | `dark` | Dark |
| Catppuccin Latte | `catppuccin-latte` | Light |
| Catppuccin Frappé | `catppuccin-frappe` | Dark |
| Catppuccin Macchiato | `catppuccin-macchiato` | Dark |
| Catppuccin Mocha | `catppuccin-mocha` | Dark |
| Gruvbox Light | `gruvbox-light` | Light |
| Gruvbox Dark | `gruvbox-dark` | Dark |
| Tokyo Night | `tokyo-night` | Dark |
| Rosé Pine Dawn | `rose-pine-dawn` | Light |
| Rosé Pine Moon | `rose-pine-moon` | Dark |
| One Dark | `one-dark` | Dark |

> [!TIP]
> Catppuccin Mocha is the designated dark theme for systems that prefer a dark color scheme.

# Switching themes

The easiest way to change theme is from the in-app settings dialog, which lists every theme above.
Selecting one updates the `theme` key in the current cave's `.granit/config.yml`.

You can also set it directly in the config file:

```yml
theme: catppuccin-mocha
```

The value is one of the ids from the table. An unknown id falls back to the default. Because the
theme is stored per cave, different caves can use different themes.

# Fonts

Fonts are configured independently for three surfaces. Each surface has its own `font_family` and
`font_size` (in pixels), so you can pair, for example, a monospace editor with a serif reader.

| Config key | Surface | Default family | Default size |
| --- | --- | --- | --- |
| `markdown_font` | Markdown editor (edit mode) | `monospace` | `14` |
| `reading_font` | Rendered reader (read mode) | `sans-serif` | `16` |
| `agent_font` | Agent chat panel | `sans-serif` | `14` |

A font block looks like this:

```yml
markdown_font:
  font_family: "JetBrains Mono"
  font_size: 14
reading_font:
  font_family: "Georgia"
  font_size: 16
agent_font:
  font_family: "Inter"
  font_size: 14
```

`font_family` accepts any CSS font family available on your system (including generic families such
as `monospace`, `sans-serif`, or `serif`). Like themes, font settings are cave-local. For the
complete set of configuration keys, see [[configuration]].
