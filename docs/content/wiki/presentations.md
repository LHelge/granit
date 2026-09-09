---
title: Presentations
category: Notes & Writing
tags: [presentations, slides, templates, css]
---

Any note can be presented as slides in a separate window while the main Granit window stays free for your notes. Slides are cut from the note's Markdown at horizontal rules, rendered by Granit, and styled by a **presentation template**: a plain CSS file stored in the cave. Two templates, a dark and a light one, come with every cave; you can edit them or add your own.

# Presenting a note

A note is presentable when its frontmatter names a template:

```markdown
---
presentation: default-dark
---
# My talk

First slide.

---

Second slide.
```

The value is the file stem of a CSS file in `.granit/presentations/` (see [[#presentation-templates]] below). You rarely type it: in the editor, the **Presentation** dropdown in the frontmatter row next to the tags lists the cave's templates plus "No presentation", and picking one writes the field. A note created from a note [[templates|template]] inherits the template's `presentation` field, so a "slides" template gives you presentable notes directly.

The [[ai-agent|AI agent]] can do the work for you in Agent mode: ask it to turn a note into a presentation, or to draft one on a topic, and it writes the slides and sets the template through its note tools. The default [[system-prompt]] explains the slide format and lists the cave's templates to the model.

Once the field is set, a **Present** button appears in the action bar at the top right of the editor, in both the reader and the editor. It opens the presentation window on the first slide. If the named template no longer exists, the button shows an error naming the template instead of falling back to a default.

## Splitting into slides

Slides are separated by a thematic break: `---`, `***` or `___` on its own line. Every separator starts a new slide, empty slides are dropped, and a note without any separator is a one-slide presentation. Granit adds no title slide; the first slide is whatever comes before the first separator.

Because separators are detected on the parsed Markdown, not the raw text, a `---` inside a fenced code block or a table's header separator row never splits a slide.

> [!TIP]
> Leave a blank line before a `---`. CommonMark reads a `---` directly under a line of text as a setext heading underline, which turns the line above into a heading instead of starting a new slide.

## What ends up on a slide

Slides use the same rendering as the reader, with a few presentation-specific rules:

- [[wiki-links|Wiki-links]] flatten to their label text; regular links are kept but inert.
- Task checkboxes render, but cannot be toggled.
- `{#id}` heading anchors are stripped; headings render plain. The `{.class}` attributes of a slide's first heading become layout classes on the slide instead, see [[#slide-layouts]].
- Relative image paths resolve against the note's folder, as in the reader.
- HTML comments are dropped, so `<!-- like this -->` works as speaker notes. Any other raw HTML is escaped, as in the reader.

Content that does not fit a slide is clipped, never scrolled. The window has no scrollbars anywhere.

# The presentation window

The window opens as a normal window so you can drag it to a projector or second screen, then go fullscreen. Only one presentation window exists: starting a presentation while it is open replaces its content and brings it to the front.

| Key or gesture | Action |
|----------------|--------|
| `→`, `↓`, `Space`, `Page Down`, click on the right half | Next slide |
| `←`, `↑`, `Page Up`, click on the left half | Previous slide |
| `Home` / `End` | First / last slide |
| `F` or `F11` | Toggle fullscreen |
| `Esc` | Leave fullscreen; pressed again, close the window |

The current slide is kept in the window's URL, so a reload keeps its position:

- **Saving the presented note** reloads the window in place, so you can fix a typo in the main window while the slides stay up.
- **Saving the template** the presented note uses reloads the window too, which makes editing a template a live preview.
- **Renaming or deleting the presented note**, or opening another cave, closes the window. Navigating to another note in the main window does not.

# Presentation templates

Templates live in the cave's `.granit/presentations/` directory, next to the note templates in `.granit/templates/`. The directory is flat: every `.css` file in it is a template, named by its file stem, and any other file there (a logo, a font) is an asset that templates reference by relative path, for example `url("logo.png")`. Assets are shared between all templates in the cave.

The **Templates** tab of the [[explorer]] shows the cave's presentation templates below its note templates. A template opens in the editor as CSS with syntax highlighting and property completion; it has no reader view, no tags and no icon. Rename it through the title, delete it with the trash button, and create a new one with the section's plus button, which seeds it from the dark default.

When a cave is opened for the first time, Granit seeds two templates, `default-dark` and `default-light`. Both start the content at a fixed distance from the top so headings sit at the same height on every slide, treat the first slide as a title slide centred in both directions, support the `section` and `two-column` layout classes, and show a row of progress dots along the bottom edge with the current slide as a wider accent-coloured pill. They are ordinary files: edit them, rename them, or delete them, and Granit never puts them back as long as the directory exists.

> [!WARNING]
> Renaming or deleting a template does not touch the notes that reference it. Such a note keeps its `presentation` value, the dropdown shows it as "*name* (missing)", and the Present button reports the missing template until you pick another one.

## How a template is applied

Granit renders the note into a standalone page and loads two stylesheets in order: its own **base stylesheet**, then your template. The base stylesheet handles mechanics only: which slide is shown, the canvas scaling described below, clipping, and hiding the mouse cursor after a few seconds of inactivity. Everything visual (colours, fonts, spacing, backgrounds, decorations) is the template's job, and since it loads last it can override anything the base stylesheet sets. Inactive slides are hidden by an `!important` rule, so a template is free to set `display` on `.slide` or on a layout class, `display: flex` to centre content or `display: grid` for columns, without ever revealing a hidden slide.

## The DOM contract

The page contains one section per slide, holding the same HTML the reader renders for that Markdown:

```html
<body style="--slide-count: 2; --deck-title: 'My talk'; --deck-created: '2026-09-01'; --deck-modified: '2026-09-09'">
  <section class="slide active" data-index="0" data-number="1"
           style="--slide-index: 0; --slide-number: 1; --slide-heading: 'My talk'; --slide-chapter: 'My talk'">
    <h1>My talk</h1>
    <p>First slide.</p>
  </section>
  <section class="slide" data-index="1" data-number="2"
           style="--slide-index: 1; --slide-number: 2; --slide-heading: 'Agenda'; --slide-chapter: 'My talk'">
    <h2>Agenda</h2>
    <p>Second slide.</p>
  </section>
</body>
```

- Exactly one `section.slide` carries the `active` class at any time; the others are hidden.
- `data-index` is the slide's 0-based position and `data-number` the 1-based one, handy for a slide counter: `.slide::after { content: attr(data-number); }`.
- Custom properties carry everything else, usable in `calc()` and `content:`. On the body: `--slide-count` (a number) and the note's `--deck-title`, `--deck-created`, and `--deck-modified` (strings; the dates come from the frontmatter as `YYYY-MM-DD` and are empty when absent). On each section: `--slide-index` and `--slide-number` (numbers), `--slide-heading` (the slide's first heading as plain text) and `--slide-chapter` (the latest level-1 heading up to and including the slide). Strings are empty, never missing, when there is nothing to show.
- The numbers are how the default templates draw their progress dots: a repeating `radial-gradient` sized `calc(var(--slide-count) * 18px)`, with the current pill placed at `calc(50% - var(--slide-count) * 9px + var(--slide-index) * 18px)`.
- Fenced code blocks render as `<pre><code class="language-rust">`, so a template can style languages the same way a reader stylesheet would.
- Fenced `mermaid` blocks render as `<div class="mermaid">` holding the diagram source. Granit's bundled Mermaid replaces it with an inline `<svg>` when the slide is shown, using Mermaid's dark theme when the slide background is dark; a template can force the choice with `color-scheme: dark` or `color-scheme: light` on `.slide`. The diagram has no background of its own, so it sits directly on the slide. A block Mermaid cannot parse shows its source as `<pre><code>` inside the container, which gets a `mermaid-error` class. The default templates centre diagrams and cap their height.
- Task checkboxes are `<input type="checkbox" disabled>`, tables are plain `<table>` markup, and alerts (`> [!NOTE]`) render as `<blockquote class="markdown-alert-note">` (and `-tip`, `-important`, `-warning`, `-caution`).

### Slide layouts

Pandoc-style attributes on a slide's first heading name the layout the slide should get: `# Part two {.section}` renders as `<section class="slide section">`, and a template styles `.slide.section` however it likes. Several classes can be combined, `{.two-column .dark}`, and any name is allowed except the reserved `slide` and `active`. The heading itself renders plain; only the section carries the classes. Attributes on later headings in the same slide are ignored.

The default templates define two layouts:

- `{.section}` centres the slide in both directions, like the title slide, for chapter dividers.
- `{.two-column}` keeps the heading across the full width and flows everything after it into two columns, so a paragraph followed by an image gives text on the left and the image on the right.

A template can add its own, for example a full-bleed image slide:

```css
.slide.cover {
  padding: 0;
}

.slide.cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
```

### A chapter header

`--slide-chapter` lets a template show the current chapter while the slide itself carries a level-2 heading: write each chapter's opening slide with `#` and the slides within it with `##`, then put the chapter in a header line:

```css
.slide:not(:first-child)::before {
  content: var(--deck-title) " · " var(--slide-chapter);
  position: absolute;
  top: 28px;
  left: 80px;
  font-size: 20px;
  opacity: 0.6;
}
```

### The canvas

Every slide is laid out on a fixed logical canvas of **1280 × 720** CSS pixels. Granit scales that canvas to fit the window, centred and letterboxed, so a template looks identical on a laptop screen and a projector; design against those pixel sizes and use absolute units freely. The size is exposed as the custom properties `--slide-width` and `--slide-height`. The letterbox area outside the canvas is painted by the `html` and `body` background, which is yours to set.

### A title-slide style

The first slide needs no layout class: it is `:first-child` and can be styled on its own. This is how the default templates keep headings at a fixed height while centring the title slide in both directions:

```css
.slide {
  display: flex;
  flex-direction: column;
}

.slide:first-child {
  justify-content: center;
  align-items: center;
  text-align: center;
}

.slide:first-child h1 {
  font-size: 96px;
}
```

# Related pages

- [[templates]] — note templates, which can carry a `presentation` field.
- [[notes-and-markdown]] — frontmatter fields and the reader's Markdown support.
- [[explorer]] — the Templates tab.
- [[cave-rules]] — what lives in `.granit/`.
