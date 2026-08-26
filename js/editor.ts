// Register Cally web component custom elements (<calendar-date>, <calendar-month>)
import "cally";

import {
    EditorView,
    keymap,
    drawSelection,
    highlightActiveLine,
    scrollPastEnd,
    ViewUpdate,
    ViewPlugin,
    Decoration,
    DecorationSet,
} from "@codemirror/view";
import {
    EditorState,
    EditorSelection,
    Compartment,
    StateField,
    RangeSetBuilder,
} from "@codemirror/state";
import {
    defaultKeymap,
    indentWithTab,
    history,
    historyKeymap,
} from "@codemirror/commands";
import { markdown, markdownKeymap } from "@codemirror/lang-markdown";
import {
    search,
    searchKeymap,
    highlightSelectionMatches,
    SearchQuery,
    getSearchQuery,
    setSearchQuery,
    openSearchPanel,
    closeSearchPanel,
    findNext,
    findPrevious,
    replaceNext,
    replaceAll,
} from "@codemirror/search";
import { runScopeHandlers } from "@codemirror/view";
import type { Panel } from "@codemirror/view";
import {
    autocompletion,
    closeBrackets,
    closeBracketsKeymap,
    startCompletion,
    CompletionContext,
    CompletionResult,
} from "@codemirror/autocomplete";
import {
    indentOnInput,
    bracketMatching,
    syntaxHighlighting,
    syntaxTree,
    HighlightStyle,
} from "@codemirror/language";
import { tags } from "@lezer/highlight";
import type { SyntaxNode } from "@lezer/common";

// ── Theme ──────────────────────────────────────────────────────────

const granitHighlightStyle = HighlightStyle.define([
    { tag: tags.heading1, fontWeight: "800", fontSize: "1.6em" },
    { tag: tags.heading2, fontWeight: "700", fontSize: "1.4em" },
    { tag: tags.heading3, fontWeight: "600", fontSize: "1.2em" },
    { tag: tags.heading4, fontWeight: "600", fontSize: "1.1em" },
    { tag: tags.heading5, fontWeight: "600" },
    { tag: tags.heading6, fontWeight: "600" },
    { tag: tags.strong, fontWeight: "bold" },
    { tag: tags.emphasis, fontStyle: "italic" },
    { tag: tags.strikethrough, textDecoration: "line-through" },
    { tag: tags.link, color: "var(--color-primary)", textDecoration: "underline" },
    { tag: tags.url, color: "var(--color-primary)", opacity: "0.7" },
    { tag: tags.monospace, fontFamily: "monospace", color: "var(--color-accent)" },
    { tag: tags.quote, color: "var(--color-base-content)", opacity: "0.7", fontStyle: "italic" },
    { tag: tags.meta, color: "var(--color-base-content)", opacity: "0.5" },
    { tag: tags.processingInstruction, color: "var(--color-base-content)", opacity: "0.4" },
]);

const granitTheme = EditorView.theme({
    "&": {
        backgroundColor: "transparent",
        color: "var(--color-base-content)",
        height: "100%",
    },
    "&.cm-focused": {
        outline: "none",
    },
    ".cm-content": {
        caretColor: "var(--color-primary)",
        padding: "0",
        lineHeight: "1.5",
    },
    ".cm-cursor, .cm-dropCursor": {
        borderLeftColor: "var(--color-primary)",
        borderLeftWidth: "2px",
    },
    ".cm-selectionBackground": {
        backgroundColor: "color-mix(in oklch, var(--color-primary) 20%, transparent) !important",
    },
    "&.cm-focused .cm-selectionBackground": {
        backgroundColor: "color-mix(in oklch, var(--color-primary) 25%, transparent) !important",
    },
    ".cm-activeLine": {
        backgroundColor: "color-mix(in oklch, var(--color-base-content) 4%, transparent)",
    },
    ".cm-gutters": {
        display: "none",
    },
    ".cm-scroller": {
        overflow: "auto",
    },
    ".cm-line": {
        padding: "0.175em 0",
    },
    ".cm-md-heading1": {
        paddingTop: "0.5em",
        paddingBottom: "0.25em",
    },
    ".cm-md-heading2": {
        paddingTop: "0.9em",
        paddingBottom: "0.25em",
    },
    ".cm-md-heading3": {
        paddingTop: "0.7em",
        paddingBottom: "0.2em",
    },
    ".cm-md-heading4": {
        paddingTop: "0.7em",
        paddingBottom: "0.2em",
    },
    ".cm-md-heading5": {
        paddingTop: "0.6em",
        paddingBottom: "0.15em",
    },
    ".cm-md-heading6": {
        paddingTop: "0.6em",
        paddingBottom: "0.15em",
    },
    ".cm-wiki-link": {
        color: "var(--color-primary)",
        textDecoration: "underline",
    },
    // Unresolved targets: muted like the reader's `.broken-link` style
    ".cm-wiki-link-broken": {
        color: "color-mix(in oklch, var(--color-base-content) 40%, transparent)",
        textDecoration: "none",
    },
    // Pointer cursor while the follow-link modifier (Cmd/Ctrl) is held
    "&.cm-mod-down .cm-wiki-link, &.cm-mod-down .cm-md-link": {
        cursor: "pointer",
    },
    // ── Tera template blocks ──────────────────────────────────────
    ".cm-tera": {
        fontFamily: "monospace",
        fontSize: "0.925em",
        borderRadius: "0.25em",
        backgroundColor: "color-mix(in oklch, var(--color-base-content) 5%, transparent)",
    },
    ".cm-tera-expr": {
        color: "var(--color-accent)",
    },
    ".cm-tera-stmt": {
        color: "var(--color-secondary)",
    },
    ".cm-tera-comment": {
        color: "color-mix(in oklch, var(--color-base-content) 45%, transparent)",
        fontStyle: "italic",
    },
    // ── Search popover (Cmd/Ctrl+F) ───────────────────────────────
    // The panel container floats over the editor's top-right corner
    // instead of rendering as a full-width bar.
    ".cm-panels.cm-panels-top": {
        position: "absolute",
        top: "0.25rem",
        right: "1rem",
        left: "auto",
        width: "auto",
        zIndex: "20",
        backgroundColor: "transparent",
        border: "none",
    },
    ".cm-panel.cm-search": {
        display: "flex",
        flexDirection: "column",
        gap: "0.25rem",
        padding: "0.5rem",
        fontSize: "0.8125rem",
        color: "var(--color-base-content)",
        backgroundColor: "var(--color-base-200)",
        border: "1px solid color-mix(in oklch, var(--color-base-content) 12%, transparent)",
        borderRadius: "0.5rem",
        boxShadow: "0 4px 12px color-mix(in oklch, var(--color-neutral) 30%, transparent)",
    },
    ".cm-search-row": {
        display: "flex",
        alignItems: "center",
        gap: "0.25rem",
    },
    ".cm-panel.cm-search .cm-textfield": {
        backgroundColor: "var(--color-base-100)",
        border: "1px solid color-mix(in oklch, var(--color-base-content) 15%, transparent)",
        borderRadius: "0.375rem",
        padding: "0.2rem 0.5rem",
        width: "11rem",
        color: "var(--color-base-content)",
        outline: "none",
    },
    ".cm-panel.cm-search .cm-textfield:focus": {
        borderColor: "var(--color-primary)",
    },
    ".cm-panel.cm-search .cm-button": {
        background: "var(--color-base-300)",
        backgroundImage: "none",
        border: "1px solid color-mix(in oklch, var(--color-base-content) 15%, transparent)",
        borderRadius: "0.375rem",
        padding: "0.2rem 0.5rem",
        color: "var(--color-base-content)",
        cursor: "pointer",
        display: "inline-flex",
        alignItems: "center",
    },
    ".cm-panel.cm-search .cm-button:hover": {
        background: "color-mix(in oklch, var(--color-base-content) 8%, var(--color-base-300))",
    },
    ".cm-panel.cm-search .cm-search-toggle": {
        fontSize: "0.6875rem",
        fontFamily: "monospace",
        color: "color-mix(in oklch, var(--color-base-content) 60%, transparent)",
    },
    ".cm-panel.cm-search .cm-search-toggle-active": {
        color: "var(--color-primary)",
        borderColor: "var(--color-primary)",
    },
    ".cm-searchMatch": {
        backgroundColor: "color-mix(in oklch, var(--color-warning) 30%, transparent)",
    },
    ".cm-searchMatch.cm-searchMatch-selected": {
        backgroundColor: "color-mix(in oklch, var(--color-warning) 55%, transparent)",
    },
    // Other occurrences of the currently selected text
    ".cm-selectionMatch": {
        backgroundColor: "color-mix(in oklch, var(--color-primary) 12%, transparent)",
    },
});

// Tooltip styles must use baseTheme because CM6 renders tooltips at the
// document body level, outside the editor's scoped DOM subtree.
const granitTooltipTheme = EditorView.baseTheme({
    ".cm-tooltip.cm-tooltip-autocomplete": {
        backgroundColor: "var(--color-base-200)",
        border: "1px solid color-mix(in oklch, var(--color-base-content) 12%, transparent)",
        borderRadius: "0.5rem",
        boxShadow: "0 4px 12px color-mix(in oklch, var(--color-neutral) 30%, transparent)",
        overflow: "hidden",
    },
    ".cm-tooltip-autocomplete > ul": {
        fontFamily: "inherit",
        fontSize: "0.875rem",
    },
    ".cm-tooltip-autocomplete > ul > li": {
        padding: "0.25rem 0.75rem",
        color: "var(--color-base-content)",
    },
    ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
        backgroundColor: "color-mix(in oklch, var(--color-primary) 20%, transparent)",
        color: "var(--color-base-content)",
    },
    ".cm-completionMatchedText": {
        textDecoration: "none",
        fontWeight: "600",
        color: "var(--color-primary)",
    },
    // Broken-link targets (no note yet) render in italics.
    ".cm-tooltip-autocomplete > ul > li.cm-completion-broken .cm-completionLabel": {
        fontStyle: "italic",
    },
});

// ── Markdown block spacing ─────────────────────────────────────────

const headingLineDecos: Record<string, Decoration> = {
    ATXHeading1: Decoration.line({ class: "cm-md-heading1" }),
    ATXHeading2: Decoration.line({ class: "cm-md-heading2" }),
    ATXHeading3: Decoration.line({ class: "cm-md-heading3" }),
    ATXHeading4: Decoration.line({ class: "cm-md-heading4" }),
    ATXHeading5: Decoration.line({ class: "cm-md-heading5" }),
    ATXHeading6: Decoration.line({ class: "cm-md-heading6" }),
};

function buildBlockDecorations(view: EditorView): DecorationSet {
    const builder = new RangeSetBuilder<Decoration>();
    const decorated = new Set<number>();
    syntaxTree(view.state).iterate({
        enter(node) {
            const deco = headingLineDecos[node.name];
            if (deco) {
                const line = view.state.doc.lineAt(node.from);
                if (!decorated.has(line.number)) {
                    decorated.add(line.number);
                    builder.add(line.from, line.from, deco);
                }
            }
        },
    });
    return builder.finish();
}

const markdownBlockSpacing = ViewPlugin.fromClass(
    class {
        decorations: DecorationSet;
        constructor(view: EditorView) {
            this.decorations = buildBlockDecorations(view);
        }
        update(update: ViewUpdate) {
            if (update.docChanged || update.viewportChanged || update.startState.tree !== syntaxTree(update.state)) {
                this.decorations = buildBlockDecorations(update.view);
            }
        }
    },
    { decorations: (v) => v.decorations }
);

// ── Search panel ───────────────────────────────────────────────────

// Search options persist across panel opens for the session; they render
// as compact toggle chips next to the find field.
const searchOptions = { caseSensitive: false, regexp: false, wholeWord: false };

// The panel renders as a floating popover in the editor's top-right corner
// (see the `.cm-panels` styling) rather than a full-width bar.
function createSearchPanel(view: EditorView): Panel {
    const dom = document.createElement("div");
    dom.className = "cm-search";

    const field = (name: string, placeholder: string) => {
        const input = document.createElement("input");
        input.className = "cm-textfield";
        input.name = name;
        input.placeholder = placeholder;
        input.setAttribute("aria-label", placeholder);
        return input;
    };
    const button = (label: string, title: string, onClick: () => void) => {
        const b = document.createElement("button");
        b.className = "cm-button";
        b.type = "button";
        b.textContent = label;
        b.title = title;
        b.onclick = onClick;
        return b;
    };

    const searchInput = field("search", "Find");
    searchInput.setAttribute("main-field", "true");
    const replaceInput = field("replace", "Replace");

    // openSearchPanel seeds the query state (from the selection, falling
    // back to the previous query) before the panel is created — read it.
    const initial = getSearchQuery(view.state);
    searchInput.value = initial.search;
    replaceInput.value = initial.replace;

    const commit = () =>
        view.dispatch({
            effects: setSearchQuery.of(
                new SearchQuery({
                    search: searchInput.value,
                    replace: replaceInput.value,
                    ...searchOptions,
                })
            ),
        });
    searchInput.oninput = commit;
    replaceInput.oninput = commit;

    // Option toggle chips: match case / regexp / whole word
    const toggleChip = (label: string, title: string, key: keyof typeof searchOptions) => {
        const chip = document.createElement("button");
        chip.className = "cm-button cm-search-toggle";
        chip.type = "button";
        chip.textContent = label;
        chip.title = title;
        const refresh = () => {
            chip.classList.toggle("cm-search-toggle-active", searchOptions[key]);
            chip.setAttribute("aria-pressed", String(searchOptions[key]));
        };
        chip.onclick = () => {
            searchOptions[key] = !searchOptions[key];
            refresh();
            commit();
        };
        refresh();
        return chip;
    };

    dom.onkeydown = (event: KeyboardEvent) => {
        // Give panel-scoped bindings (Escape close, Mod-f, F3/Mod-g) first go.
        if (runScopeHandlers(view, event, "search-panel")) {
            event.preventDefault();
            return;
        }
        if (event.key === "Enter" && event.target === searchInput) {
            event.preventDefault();
            (event.shiftKey ? findPrevious : findNext)(view);
        } else if (event.key === "Enter" && event.target === replaceInput) {
            event.preventDefault();
            replaceNext(view);
        }
    };

    const row = (...children: HTMLElement[]) => {
        const div = document.createElement("div");
        div.className = "cm-search-row";
        div.append(...children);
        return div;
    };
    dom.append(
        row(
            searchInput,
            toggleChip("Aa", "Match case", "caseSensitive"),
            toggleChip(".*", "Regular expression", "regexp"),
            toggleChip("W", "Whole word", "wholeWord"),
            button("↑", "Previous match (Shift+Enter)", () => findPrevious(view)),
            button("↓", "Next match (Enter)", () => findNext(view)),
            button("✕", "Close (Esc)", () => closeSearchPanel(view))
        ),
        row(
            replaceInput,
            button("Replace", "Replace current match", () => replaceNext(view)),
            button("All", "Replace all matches", () => replaceAll(view))
        )
    );

    return {
        dom,
        top: true,
        mount() {
            searchInput.focus();
            searchInput.select();
            // Apply the session's search options to the seeded query.
            // Deferred: a panel mounts mid-update, where dispatching a
            // transaction is not allowed.
            setTimeout(commit, 0);
        },
        update(update: ViewUpdate) {
            // Keep the fields in sync with query changes made outside the
            // panel (e.g. re-seeding from the selection on reopen).
            for (const tr of update.transactions) {
                for (const effect of tr.effects) {
                    if (effect.is(setSearchQuery)) {
                        const query = effect.value as SearchQuery;
                        searchInput.value = query.search;
                        replaceInput.value = query.replace;
                    }
                }
            }
        },
    };
}

// ── Editing commands (formatting, links, tasks) ────────────────────

// Toggle an inline mark (e.g. `**` for bold) around each selection range.
// An empty range expands to the word under the cursor; a range already
// wrapped (or selected including the marks) is unwrapped.
function toggleInlineMark(view: EditorView, mark: string): boolean {
    if (view.state.readOnly) return false;
    const changes = view.state.changeByRange((range) => {
        let { from, to } = range;
        if (from === to) {
            const word = view.state.wordAt(from);
            if (word) ({ from, to } = word);
        }
        const before = view.state.sliceDoc(Math.max(0, from - mark.length), from);
        const after = view.state.sliceDoc(to, Math.min(view.state.doc.length, to + mark.length));
        if (before === mark && after === mark) {
            return {
                changes: [
                    { from: from - mark.length, to: from },
                    { from: to, to: to + mark.length },
                ],
                range: EditorSelection.range(from - mark.length, to - mark.length),
            };
        }
        const inner = view.state.sliceDoc(from, to);
        if (inner.length >= 2 * mark.length && inner.startsWith(mark) && inner.endsWith(mark)) {
            return {
                changes: [
                    { from, to: from + mark.length },
                    { from: to - mark.length, to },
                ],
                range: EditorSelection.range(from, to - 2 * mark.length),
            };
        }
        return {
            changes: [
                { from, insert: mark },
                { from: to, insert: mark },
            ],
            range: EditorSelection.range(from + mark.length, to + mark.length),
        };
    });
    view.dispatch(changes, { scrollIntoView: true, userEvent: "input" });
    return true;
}

// Wrap the selection as a link. Plain text becomes a wiki-link (nearly all
// links in a cave are internal references); a selected URL becomes a
// markdown link with the cursor in the empty label. An empty selection
// inserts `[[]]` and opens slug completion.
function insertLink(view: EditorView): boolean {
    if (view.state.readOnly) return false;
    let completeSlug = false;
    const changes = view.state.changeByRange((range) => {
        const text = view.state.sliceDoc(range.from, range.to);
        if (isUrl(text)) {
            return {
                changes: { from: range.from, to: range.to, insert: `[](${text})` },
                range: EditorSelection.cursor(range.from + 1),
            };
        }
        if (text.length === 0) {
            completeSlug = true;
            return {
                changes: { from: range.from, insert: "[[]]" },
                range: EditorSelection.cursor(range.from + 2),
            };
        }
        return {
            changes: { from: range.from, to: range.to, insert: `[[${text}]]` },
            range: EditorSelection.cursor(range.from + text.length + 4),
        };
    });
    view.dispatch(changes, { scrollIntoView: true, userEvent: "input" });
    if (completeSlug) startCompletion(view);
    return true;
}

// Matches the line prefix: indent, optional list marker, optional checkbox.
const taskLineRegex = /^(\s*)(?:([-*+]|\d+[.)])\s+(\[[ xX]\]\s+)?)?/;

// Toggle the task checkbox on every selected line: `[ ]` ⇄ `[x]`, adding
// `[ ]` to plain list items and `- [ ] ` to non-list lines.
function toggleTask(view: EditorView): boolean {
    if (view.state.readOnly) return false;
    const changes: { from: number; to?: number; insert?: string }[] = [];
    const seen = new Set<number>();
    for (const range of view.state.selection.ranges) {
        const first = view.state.doc.lineAt(range.from).number;
        const last = view.state.doc.lineAt(range.to).number;
        for (let n = first; n <= last; n++) {
            if (seen.has(n)) continue;
            seen.add(n);
            const line = view.state.doc.line(n);
            const m = line.text.match(taskLineRegex)!;
            const [full, indent, marker, box] = m;
            if (box) {
                const stateIdx = full.length - box.length + 1;
                const checked = line.text[stateIdx] !== " ";
                changes.push({
                    from: line.from + stateIdx,
                    to: line.from + stateIdx + 1,
                    insert: checked ? " " : "x",
                });
            } else if (marker) {
                changes.push({ from: line.from + full.length, insert: "[ ] " });
            } else {
                changes.push({ from: line.from + indent.length, insert: "- [ ] " });
            }
        }
    }
    view.dispatch({ changes, userEvent: "input" });
    return true;
}

const editingKeymap = [
    { key: "Mod-b", run: (view: EditorView) => toggleInlineMark(view, "**") },
    { key: "Mod-i", run: (view: EditorView) => toggleInlineMark(view, "*") },
    { key: "Mod-k", run: insertLink },
    { key: "Mod-l", run: toggleTask },
];

// ── Clickable links (Cmd/Ctrl+click to follow) ─────────────────────

// Matches `[[target]]` / `[[target|label]]` on a single line.
const wikiLinkRegex = /\[\[([^[\]\n]+)\]\]/g;

// Wiki-links inside code spans/blocks are literal text, not links.
function inCodeContext(state: EditorState, pos: number): boolean {
    for (
        let node: SyntaxNode | null = syntaxTree(state).resolveInner(pos, 1);
        node;
        node = node.parent
    ) {
        if (node.name.includes("Code")) return true;
    }
    return false;
}

interface WikiLinkMatch {
    from: number;
    to: number;
    target: string;
}

// All wiki-links intersecting [from, to), with their raw targets
// (the part before an optional `|label`).
function wikiLinksIn(state: EditorState, from: number, to: number): WikiLinkMatch[] {
    const matches: WikiLinkMatch[] = [];
    const text = state.sliceDoc(from, to);
    wikiLinkRegex.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = wikiLinkRegex.exec(text)) !== null) {
        const start = from + m.index;
        if (inCodeContext(state, start + 2)) continue;
        const target = m[1].split("|")[0].trim();
        if (target.length === 0) continue;
        matches.push({ from: start, to: start + m[0].length, target });
    }
    return matches;
}

function wikiLinkTargetAt(state: EditorState, pos: number): string | null {
    const line = state.doc.lineAt(pos);
    const hit = wikiLinksIn(state, line.from, line.to).find(
        (m) => m.from <= pos && pos < m.to
    );
    return hit ? hit.target : null;
}

// The http(s) URL of the markdown link, autolink, or bare URL at `pos`,
// or null when there is none.
function urlTargetAt(state: EditorState, pos: number): string | null {
    for (
        let node: SyntaxNode | null = syntaxTree(state).resolveInner(pos, 1);
        node;
        node = node.parent
    ) {
        let urlNode: SyntaxNode | null = null;
        if (node.name === "URL") {
            urlNode = node;
        } else if (node.name === "Link" || node.name === "Autolink" || node.name === "Image") {
            urlNode = node.getChild("URL");
        } else {
            continue;
        }
        if (!urlNode) return null;
        const url = state.sliceDoc(urlNode.from, urlNode.to);
        return /^https?:\/\//i.test(url) ? url : null;
    }
    return null;
}

// Decorates wiki-links (styling; the markdown parser treats them as plain
// text) and tags markdown links/autolinks with `cm-md-link` so the pointer
// cursor can target them while the follow-link modifier is held.
const mdLinkDeco = Decoration.mark({ class: "cm-md-link" });
const wikiLinkDeco = Decoration.mark({ class: "cm-wiki-link" });
const wikiLinkBrokenDeco = Decoration.mark({ class: "cm-wiki-link cm-wiki-link-broken" });

function buildLinkDecorations(view: EditorView): DecorationSet {
    // Targets that resolve to a note or heading anchor (case-insensitive,
    // matching the backend resolver); everything else styles as broken.
    const resolved = new Set(
        view.state.field(slugsField).slugs.map((s) => s.toLowerCase())
    );
    const ranges: { from: number; to: number; deco: Decoration }[] = [];
    for (const { from, to } of view.visibleRanges) {
        for (const m of wikiLinksIn(view.state, from, to)) {
            const deco = resolved.has(m.target.toLowerCase()) ? wikiLinkDeco : wikiLinkBrokenDeco;
            ranges.push({ from: m.from, to: m.to, deco });
        }
        syntaxTree(view.state).iterate({
            from,
            to,
            enter(node) {
                if (node.name === "Link" || node.name === "Autolink") {
                    ranges.push({ from: node.from, to: node.to, deco: mdLinkDeco });
                } else if (node.name === "URL" && !node.node.parent?.name.match(/^(Link|Autolink|Image)$/)) {
                    // Bare URL autolinked by GFM
                    ranges.push({ from: node.from, to: node.to, deco: mdLinkDeco });
                }
            },
        });
    }
    ranges.sort((a, b) => a.from - b.from || a.to - b.to);
    const builder = new RangeSetBuilder<Decoration>();
    for (const r of ranges) builder.add(r.from, r.to, r.deco);
    return builder.finish();
}

const linkDecorations = ViewPlugin.fromClass(
    class {
        decorations: DecorationSet;
        constructor(view: EditorView) {
            this.decorations = buildLinkDecorations(view);
        }
        update(update: ViewUpdate) {
            if (
                update.docChanged ||
                update.viewportChanged ||
                update.startState.tree !== syntaxTree(update.state) ||
                // Slug lists changed (setSlugs): broken-ness may have changed
                update.startState.field(slugsField, false) !== update.state.field(slugsField, false)
            ) {
                this.decorations = buildLinkDecorations(update.view);
            }
        }
    },
    { decorations: (v) => v.decorations }
);

// Toggles `cm-mod-down` on the editor root while Cmd/Ctrl is held, so links
// show a pointer cursor exactly when a click would follow them.
const modKeyWatcher = ViewPlugin.fromClass(
    class {
        private onKey = (e: KeyboardEvent) => this.set(e.metaKey || e.ctrlKey);
        private onClear = () => this.set(false);
        constructor(private view: EditorView) {
            window.addEventListener("keydown", this.onKey);
            window.addEventListener("keyup", this.onKey);
            window.addEventListener("blur", this.onClear);
        }
        private set(on: boolean) {
            this.view.dom.classList.toggle("cm-mod-down", on);
        }
        destroy() {
            window.removeEventListener("keydown", this.onKey);
            window.removeEventListener("keyup", this.onKey);
            window.removeEventListener("blur", this.onClear);
        }
    }
);

// Cmd/Ctrl+click follows the link under the cursor; a plain click still
// just places the cursor. Reports the raw target to the host app, which
// resolves wiki targets and opens URLs.
function linkClickExtension(onLinkClick: (kind: "wiki" | "url", target: string) => void) {
    return EditorView.domEventHandlers({
        mousedown(event: MouseEvent, view: EditorView) {
            if (event.button !== 0 || !(event.metaKey || event.ctrlKey)) return false;
            const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
            if (pos === null) return false;

            const wiki = wikiLinkTargetAt(view.state, pos);
            if (wiki !== null) {
                event.preventDefault();
                onLinkClick("wiki", wiki);
                return true;
            }
            const url = urlTargetAt(view.state, pos);
            if (url !== null) {
                event.preventDefault();
                onLinkClick("url", url);
                return true;
            }
            return false;
        },
    });
}

// ── URL paste extension ────────────────────────────────────────────

function isUrl(text: string): boolean {
    return /^https?:\/\/\S+$/i.test(text.trim());
}

const urlPasteExtension = EditorView.domEventHandlers({
    paste(event: ClipboardEvent, view: EditorView) {
        const clipText = event.clipboardData?.getData("text/plain");
        if (!clipText || !isUrl(clipText.trim())) return false;

        const url = clipText.trim();
        const { from, to } = view.state.selection.main;
        const selected = view.state.sliceDoc(from, to);

        if (selected.length > 0) {
            // Wrap selection as markdown link
            event.preventDefault();
            const linkText = `[${selected}](${url})`;
            view.dispatch({
                changes: { from, to, insert: linkText },
                selection: { anchor: from + linkText.length },
            });
            return true;
        }
        // No selection: let default paste happen
        return false;
    },
});

// ── Wiki-link autocompletion ──────────────────────────────────────

// A StateField that holds the current lists of wiki-link targets available
// for completion: slugs that resolve (notes + heading anchors) and broken
// targets already used somewhere in the cave but without a note yet.
// Reconfigured via a Compartment whenever the lists change.
interface SlugLists {
    slugs: string[];
    broken: string[];
}

const slugsField = StateField.define<SlugLists>({
    create: () => ({ slugs: [], broken: [] }),
    update: (value) => value,
});

function slugsExtension(slugs: string[], broken: string[]) {
    return slugsField.init(() => ({ slugs, broken }));
}

// Insert `<label>]]`, overwriting any `]]` that closeBrackets already inserted
// (so we don't end up with `]]]]`), and place the cursor after the closing `]]`.
function applyWikiLink(
    view: EditorView,
    completion: { label: string },
    from: number,
    to: number,
) {
    const docTo = view.state.doc.sliceString(to, to + 2) === "]]" ? to + 2 : to;
    view.dispatch({
        changes: { from, to: docTo, insert: `${completion.label}]]` },
        selection: { anchor: from + completion.label.length + 2 },
    });
}

function wikiLinkCompletionSource(context: CompletionContext): CompletionResult | null {
    // Match `[[` followed by any non-`]` characters up to the cursor
    const match = context.matchBefore(/\[\[[^\]]*$/);
    if (!match) return null;

    const typed = match.text.slice(2);
    const { slugs, broken } = context.state.field(slugsField);

    const options = slugs.map((slug) => ({
        label: slug,
        apply: applyWikiLink,
    }));

    // Broken-link targets already used elsewhere in the cave: linking one
    // renders as a broken link until its note is created. Shown in italics
    // (via optionClass) and ranked just below resolving slugs on equal matches.
    for (const target of broken) {
        options.push({
            label: target,
            type: "broken",
            boost: -1,
            apply: applyWikiLink,
        } as (typeof options)[number]);
    }

    // Obsidian-style: when the typed text matches no existing slug, offer to
    // link a not-yet-existing note. It sorts last (negative boost) and inserts
    // the typed text verbatim — the resulting link renders as a broken link
    // until the note is created.
    const trimmed = typed.trim();
    const known = (s: string) => s.toLowerCase() === trimmed.toLowerCase();
    if (trimmed.length > 0 && !slugs.some(known) && !broken.some(known)) {
        options.push({
            label: trimmed,
            detail: "Create new note",
            boost: -99,
            apply: applyWikiLink,
        } as (typeof options)[number]);
    }

    return {
        from: match.from + 2, // complete only the slug part (after `[[`)
        options,
        filter: typed.length > 0, // use CM6 built-in fuzzy filter once typing starts
    };
}

// ── Blockquote alert autocompletion ───────────────────────────────

const alertTypes = [
    { label: "[!NOTE]", detail: "Useful information that users should know" },
    { label: "[!TIP]", detail: "Helpful advice for doing things better or more easily" },
    { label: "[!IMPORTANT]", detail: "Key information users need to know to achieve their goal" },
    { label: "[!WARNING]", detail: "Urgent info that needs immediate user attention" },
    { label: "[!CAUTION]", detail: "Advises about risks or negative outcomes of certain actions" },
];

function alertCompletionSource(context: CompletionContext): CompletionResult | null {
    // Match `> [!` (with optional leading whitespace) up to the cursor on the current line
    const line = context.state.doc.lineAt(context.pos);
    const lineTextBefore = line.text.slice(0, context.pos - line.from);
    const m = lineTextBefore.match(/^>\s*\[!(\w*)$/);
    if (!m) return null;

    const from = context.pos - m[1].length - 2; // start from `[!`

    return {
        from,
        options: alertTypes.map((t) => ({
            label: t.label,
            detail: t.detail,
            apply: (view, completion, from, to) => {
                // Insert the tag and add a newline with `> ` prefix for content
                const insert = `${completion.label}\n> `;
                view.dispatch({
                    changes: { from, to, insert },
                    selection: { anchor: from + insert.length },
                });
            },
        })),
        filter: m[1].length > 0,
    };
}

// ── Tera template support ─────────────────────────────────────────
//
// Templates, tasks, and the agent system prompt are rendered with Tera on
// the backend. When the host app enables Tera mode for a document (via
// `setTeraMode`), `{{ … }}` / `{% … %}` / `{# … #}` blocks are highlighted
// and the document's context variables, Tera keywords, and common filters
// are offered as completions. Notes (and skills, which are loaded verbatim)
// keep Tera mode off: the field is `null` and both the decorator and the
// completion source turn themselves off.

interface TeraVariable {
    label: string;
    detail?: string;
}

const teraField = StateField.define<TeraVariable[] | null>({
    create: () => null,
    update: (value) => value,
});

function teraExtension(variables: TeraVariable[] | null) {
    return teraField.init(() => variables);
}

// One Tera block of any kind, non-greedy, possibly spanning lines.
const teraBlockRegex = /\{\{[\s\S]*?\}\}|\{%[\s\S]*?%\}|\{#[\s\S]*?#\}/g;

const teraExprDeco = Decoration.mark({ class: "cm-tera cm-tera-expr" });
const teraStmtDeco = Decoration.mark({ class: "cm-tera cm-tera-stmt" });
const teraCommentDeco = Decoration.mark({ class: "cm-tera cm-tera-comment" });

function buildTeraDecorations(view: EditorView): DecorationSet {
    if (view.state.field(teraField) === null) return Decoration.none;
    const builder = new RangeSetBuilder<Decoration>();
    for (const { from, to } of view.visibleRanges) {
        const text = view.state.sliceDoc(from, to);
        teraBlockRegex.lastIndex = 0;
        let m: RegExpExecArray | null;
        while ((m = teraBlockRegex.exec(text)) !== null) {
            const deco = m[0].startsWith("{{")
                ? teraExprDeco
                : m[0].startsWith("{%")
                  ? teraStmtDeco
                  : teraCommentDeco;
            builder.add(from + m.index, from + m.index + m[0].length, deco);
        }
    }
    return builder.finish();
}

const teraDecorations = ViewPlugin.fromClass(
    class {
        decorations: DecorationSet;
        constructor(view: EditorView) {
            this.decorations = buildTeraDecorations(view);
        }
        update(update: ViewUpdate) {
            if (
                update.docChanged ||
                update.viewportChanged ||
                // Tera mode toggled (setTeraMode)
                update.startState.field(teraField, false) !== update.state.field(teraField, false)
            ) {
                this.decorations = buildTeraDecorations(update.view);
            }
        }
    },
    { decorations: (v) => v.decorations }
);

// Statement keywords supported by Tera one-off rendering (no template
// inheritance/imports, which need a full template registry).
const teraKeywords = [
    "if",
    "elif",
    "else",
    "endif",
    "for",
    "in",
    "endfor",
    "set",
    "set_global",
    "raw",
    "endraw",
    "filter",
    "endfilter",
    "break",
    "continue",
    "and",
    "or",
    "not",
    "is",
    "as",
];

// Common Tera built-in filters.
const teraFilters = [
    "abs",
    "capitalize",
    "concat",
    "date",
    "default",
    "escape",
    "first",
    "float",
    "get",
    "group_by",
    "int",
    "join",
    "last",
    "length",
    "lower",
    "map",
    "nth",
    "replace",
    "reverse",
    "round",
    "safe",
    "slice",
    "slugify",
    "sort",
    "split",
    "title",
    "trim",
    "truncate",
    "unique",
    "upper",
    "urlencode",
    "wordcount",
];

// The `{{` / `{%` opener of the unclosed Tera block the cursor is inside,
// or null when the cursor is not inside one. Scans a bounded window back
// from `pos` so huge documents stay cheap.
function teraOpenerAt(state: EditorState, pos: number): "{{" | "{%" | null {
    const windowFrom = Math.max(0, pos - 500);
    const before = state.sliceDoc(windowFrom, pos);
    const open = Math.max(before.lastIndexOf("{{"), before.lastIndexOf("{%"));
    if (open === -1) return null;
    const opener = before.slice(open, open + 2) as "{{" | "{%";
    const closer = opener === "{{" ? "}}" : "%}";
    return before.slice(open + 2).includes(closer) ? null : opener;
}

function teraCompletionSource(context: CompletionContext): CompletionResult | null {
    const variables = context.state.field(teraField);
    if (variables === null) return null;

    const opener = teraOpenerAt(context.state, context.pos);
    if (opener === null) return null;

    // After a `|`: complete filter names.
    const filterMatch = context.matchBefore(/\|\s*\w*$/);
    if (filterMatch) {
        const typed = filterMatch.text.replace(/^\|\s*/, "");
        return {
            from: context.pos - typed.length,
            options: teraFilters.map((f) => ({ label: f, type: "function", detail: "filter" })),
            validFor: /^\w*$/,
            filter: typed.length > 0,
        };
    }

    const word = context.matchBefore(/[\w.]*$/);
    if (!word) return null;
    // Only pop up unprompted right after the opener or while typing a word.
    if (word.from === word.to && !context.explicit) {
        const opened = context.matchBefore(/(\{\{|\{%)\s*$/);
        if (!opened) return null;
    }

    const options = [
        ...variables.map((v) => ({ label: v.label, detail: v.detail, type: "variable" })),
        ...(opener === "{%"
            ? teraKeywords.map((k) => ({ label: k, type: "keyword" }))
            : []),
    ];
    return {
        from: word.from,
        options,
        validFor: /^[\w.]*$/,
        filter: word.text.length > 0,
    };
}

// ── Editor instances ───────────────────────────────────────────────

interface EditorInstance {
    view: EditorView;
    fontCompartment: Compartment;
    readOnlyCompartment: Compartment;
    slugsCompartment: Compartment;
    teraCompartment: Compartment;
    onChange: ((content: string) => void) | null;
    onSelectionChange: ((selectedText: string) => void) | null;
}

let nextHandle = 1;
const instances = new Map<number, EditorInstance>();

function fontExtension(family: string, size: string) {
    return EditorView.theme({
        ".cm-scroller": {
            fontFamily: family || "inherit",
            fontSize: size ? `${size}px` : "inherit",
        },
    });
}

// ── Public API (exposed as window.GranitEditor) ────────────────────

export interface CreateConfig {
    content?: string;
    fontFamily?: string;
    fontSize?: string;
    slugs?: string[];
    brokenSlugs?: string[];
    onChange?: (content: string) => void;
    onSelectionChange?: (selectedText: string) => void;
    onLinkClick?: (kind: "wiki" | "url", target: string) => void;
}

export function create(
    element: HTMLElement,
    config: CreateConfig = {}
): number {
    const fontCompartment = new Compartment();
    const readOnlyCompartment = new Compartment();
    const slugsCompartment = new Compartment();
    const teraCompartment = new Compartment();

    const updateListener = EditorView.updateListener.of((update: ViewUpdate) => {
        const inst = instances.get(handle);
        if (!inst) return;

        if (update.docChanged && inst.onChange) {
            inst.onChange(update.state.doc.toString());
        }

        if (update.selectionSet && inst.onSelectionChange) {
            const { from, to } = update.state.selection.main;
            const selected = from !== to ? update.state.sliceDoc(from, to) : "";
            inst.onSelectionChange(selected);
        }
    });

    const state = EditorState.create({
        doc: config.content ?? "",
        extensions: [
            granitTheme,
            granitTooltipTheme,
            syntaxHighlighting(granitHighlightStyle),
            fontCompartment.of(
                fontExtension(config.fontFamily ?? "", config.fontSize ?? "")
            ),
            readOnlyCompartment.of(EditorState.readOnly.of(false)),
            slugsCompartment.of(slugsExtension(config.slugs ?? [], config.brokenSlugs ?? [])),
            teraCompartment.of(teraExtension(null)),
            teraDecorations,
            autocompletion({
                override: [wikiLinkCompletionSource, teraCompletionSource, alertCompletionSource],
                optionClass: (completion) =>
                    completion.type === "broken" ? "cm-completion-broken" : "",
            }),
            markdown(),
            closeBrackets(),
            bracketMatching(),
            indentOnInput(),
            history(),
            drawSelection(),
            highlightActiveLine(),
            highlightSelectionMatches(),
            search({ top: true, createPanel: createSearchPanel }),
            scrollPastEnd(),
            EditorView.lineWrapping,
            markdownBlockSpacing,
            linkDecorations,
            modKeyWatcher,
            ...(config.onLinkClick ? [linkClickExtension(config.onLinkClick)] : []),
            urlPasteExtension,
            keymap.of([
                ...editingKeymap,
                ...closeBracketsKeymap,
                // Before defaultKeymap: Enter continues lists/quotes and
                // Backspace dissolves an empty list marker.
                ...markdownKeymap,
                ...searchKeymap,
                ...defaultKeymap,
                ...historyKeymap,
                indentWithTab,
            ]),
            updateListener,
        ],
    });

    const view = new EditorView({ state, parent: element });
    const handle = nextHandle++;

    instances.set(handle, {
        view,
        fontCompartment,
        readOnlyCompartment,
        slugsCompartment,
        teraCompartment,
        onChange: config.onChange ?? null,
        onSelectionChange: config.onSelectionChange ?? null,
    });

    return handle;
}

export function setContent(handle: number, content: string): void {
    const inst = instances.get(handle);
    if (!inst) return;

    const current = inst.view.state.doc.toString();
    if (current === content) return;

    // Suppress onChange callback during programmatic replacement
    const savedCb = inst.onChange;
    inst.onChange = null;

    inst.view.dispatch({
        changes: {
            from: 0,
            to: inst.view.state.doc.length,
            insert: content,
        },
    });

    inst.onChange = savedCb;
}

export function getContent(handle: number): string {
    const inst = instances.get(handle);
    return inst ? inst.view.state.doc.toString() : "";
}

export function focus(handle: number): void {
    const inst = instances.get(handle);
    if (inst) inst.view.focus();
}

export function openSearch(handle: number): void {
    const inst = instances.get(handle);
    if (inst) openSearchPanel(inst.view);
}

export function setFont(
    handle: number,
    family: string,
    size: string
): void {
    const inst = instances.get(handle);
    if (!inst) return;
    inst.view.dispatch({
        effects: inst.fontCompartment.reconfigure(fontExtension(family, size)),
    });
}

export function setReadOnly(handle: number, readOnly: boolean): void {
    const inst = instances.get(handle);
    if (!inst) return;
    inst.view.dispatch({
        effects: inst.readOnlyCompartment.reconfigure(
            EditorState.readOnly.of(readOnly)
        ),
    });
}

// Enable Tera template support (highlighting + completion) with the given
// context variables, or disable it with `null` (plain notes).
export function setTeraMode(handle: number, variables: TeraVariable[] | null): void {
    const inst = instances.get(handle);
    if (!inst) return;
    inst.view.dispatch({
        effects: inst.teraCompartment.reconfigure(teraExtension(variables)),
    });
}

export function setSlugs(handle: number, slugs: string[], brokenSlugs: string[] = []): void {
    const inst = instances.get(handle);
    if (!inst) return;
    inst.view.dispatch({
        effects: inst.slugsCompartment.reconfigure(slugsExtension(slugs, brokenSlugs)),
    });
}

export function destroy(handle: number): void {
    const inst = instances.get(handle);
    if (!inst) return;
    inst.view.destroy();
    instances.delete(handle);
}
