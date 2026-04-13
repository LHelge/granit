// Register Cally web component custom elements (<calendar-date>, <calendar-month>)
import "cally";

import {
    EditorView,
    keymap,
    drawSelection,
    highlightActiveLine,
    ViewUpdate,
} from "@codemirror/view";
import { EditorState, Compartment, StateField } from "@codemirror/state";
import {
    defaultKeymap,
    indentWithTab,
    history,
    historyKeymap,
} from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";
import {
    autocompletion,
    closeBrackets,
    closeBracketsKeymap,
    CompletionContext,
    CompletionResult,
} from "@codemirror/autocomplete";
import {
    indentOnInput,
    bracketMatching,
    syntaxHighlighting,
    HighlightStyle,
} from "@codemirror/language";
import { tags } from "@lezer/highlight";

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
        lineHeight: "1.625",
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
        fontFamily: "inherit",
        fontSize: "inherit",
    },
    ".cm-line": {
        padding: "0",
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
});

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

// A StateField that holds the current list of slugs available for completion.
// Reconfigured via a Compartment whenever the slug list changes.
const slugsField = StateField.define<string[]>({
    create: () => [],
    update: (value) => value,
});

function slugsExtension(slugs: string[]) {
    return slugsField.init(() => slugs);
}

function wikiLinkCompletionSource(context: CompletionContext): CompletionResult | null {
    // Match `[[` followed by any non-`]` characters up to the cursor
    const match = context.matchBefore(/\[\[[^\]]*$/);
    if (!match) return null;

    const typed = match.text.slice(2);
    const slugs = context.state.field(slugsField);

    return {
        from: match.from + 2, // complete only the slug part (after `[[`)
        options: slugs.map((slug) => ({
            label: slug,
            apply: (view, completion, from, to) => {
                // closeBrackets may have already inserted `]]` after the cursor.
                // Extend `to` to overwrite them so we don't end up with `]]]]`.
                const docTo = view.state.doc.sliceString(to, to + 2) === "]]" ? to + 2 : to;
                view.dispatch({
                    changes: { from, to: docTo, insert: `${completion.label}]]` },
                    selection: { anchor: from + completion.label.length + 2 },
                });
            },
        })),
        filter: typed.length > 0, // use CM6 built-in fuzzy filter once typing starts
    };
}

// ── Editor instances ───────────────────────────────────────────────

interface EditorInstance {
    view: EditorView;
    fontCompartment: Compartment;
    readOnlyCompartment: Compartment;
    slugsCompartment: Compartment;
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
    onChange?: (content: string) => void;
    onSelectionChange?: (selectedText: string) => void;
}

export function create(
    element: HTMLElement,
    config: CreateConfig = {}
): number {
    const fontCompartment = new Compartment();
    const readOnlyCompartment = new Compartment();
    const slugsCompartment = new Compartment();

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
            slugsCompartment.of(slugsExtension(config.slugs ?? [])),
            autocompletion({ override: [wikiLinkCompletionSource] }),
            markdown(),
            closeBrackets(),
            bracketMatching(),
            indentOnInput(),
            history(),
            drawSelection(),
            highlightActiveLine(),
            EditorView.lineWrapping,
            urlPasteExtension,
            keymap.of([
                ...closeBracketsKeymap,
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

export function setSlugs(handle: number, slugs: string[]): void {
    const inst = instances.get(handle);
    if (!inst) return;
    inst.view.dispatch({
        effects: inst.slugsCompartment.reconfigure(slugsExtension(slugs)),
    });
}

export function destroy(handle: number): void {
    const inst = instances.get(handle);
    if (!inst) return;
    inst.view.destroy();
    instances.delete(handle);
}
