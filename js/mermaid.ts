// Mermaid diagram rendering shared by the note reader and the presentation
// window. Bundled by esbuild to build/mermaid.js and exposed as
// `window.GranitMermaid`; the backend embeds the same bundle for the
// presentation page.
//
// The backend renders a fenced ```mermaid block as
// `<div class="mermaid">` holding the escaped diagram source. `run(root)`
// turns every such block under `root` into an inline SVG, picking mermaid's
// dark theme when the block sits on a dark background. The source is kept
// on the element so a block can be re-rendered when the theme changes. A
// block that fails to parse shows its source as a code block
// (`<pre><code>`) with the `mermaid-error` class on the container and the
// parser message as its tooltip, instead of mermaid's own error graphic.
import mermaid from "mermaid";

type Theme = "dark" | "default";

mermaid.initialize({ startOnLoad: false, suppressErrorRendering: true });

/** Relative luminance of a computed CSS colour, or `null` if unparseable. */
function luminance(color: string): number | null {
  const rgb = color.match(/^rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+%?))?\s*\)$/);
  if (rgb) {
    if (rgb[4] !== undefined && parseFloat(rgb[4]) === 0) return null;
    const channel = (v: string) => {
      const c = parseFloat(v) / 255;
      return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
    };
    return 0.2126 * channel(rgb[1]) + 0.7152 * channel(rgb[2]) + 0.0722 * channel(rgb[3]);
  }
  // oklch()/oklab(): the first component is perceptual lightness in 0..1
  // (or a percentage); a transparent alpha still means "look further up".
  const ok = color.match(/^okl(?:ch|ab)\(\s*([\d.]+%?)[^/)]*(?:\/\s*([\d.]+%?))?\s*\)$/);
  if (ok) {
    if (ok[2] !== undefined && parseFloat(ok[2]) === 0) return null;
    const l = ok[1].endsWith("%") ? parseFloat(ok[1]) / 100 : parseFloat(ok[1]);
    return l * l;
  }
  return color === "transparent" ? null : 0.5;
}

/** Whether `element` is drawn on a dark background. */
function isDark(element: Element): boolean {
  // DaisyUI themes declare `color-scheme`, which is the most reliable hint
  // in the reader; presentation templates usually do not, so fall back to
  // the luminance of the nearest opaque background.
  const scheme = getComputedStyle(element).colorScheme || "";
  if (scheme.includes("dark") && !scheme.includes("light")) return true;
  if (scheme.includes("light") && !scheme.includes("dark")) return false;
  for (let el: Element | null = element; el; el = el.parentElement) {
    const lum = luminance(getComputedStyle(el).backgroundColor);
    if (lum !== null) return lum < 0.5 * 0.5;
  }
  return false;
}

/** Mermaid's parser throws plain `{ str, hash }` objects, not `Error`s. */
function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object" && "str" in error) return String(error.str);
  return String(error);
}

/** Render every `.mermaid` block under `root` that is not already rendered
 * for the current theme. Resolves once all blocks have been processed. */
export async function run(root: ParentNode = document): Promise<void> {
  const nodes = Array.from(root.querySelectorAll<HTMLElement>(".mermaid"));
  for (const node of nodes) {
    if (node.dataset.mermaidSrc === undefined) {
      node.dataset.mermaidSrc = node.textContent ?? "";
    }
    const theme: Theme = isDark(node) ? "dark" : "default";
    if (node.dataset.mermaidTheme === theme && node.getAttribute("data-processed")) {
      continue;
    }
    node.textContent = node.dataset.mermaidSrc;
    node.removeAttribute("data-processed");
    node.classList.remove("mermaid-error");
    node.removeAttribute("title");
    mermaid.initialize({ startOnLoad: false, suppressErrorRendering: true, theme });
    try {
      await mermaid.run({ nodes: [node] });
      node.dataset.mermaidTheme = theme;
    } catch (error) {
      const code = document.createElement("code");
      code.textContent = node.dataset.mermaidSrc;
      const pre = document.createElement("pre");
      pre.appendChild(code);
      node.replaceChildren(pre);
      node.classList.add("mermaid-error");
      node.title = errorMessage(error);
    }
  }
}
