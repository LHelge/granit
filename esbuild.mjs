import * as esbuild from "esbuild";

const watch = process.argv.includes("--watch");

/** @type {esbuild.BuildOptions[]} */
const bundles = [
  {
    entryPoints: ["js/editor.ts"],
    globalName: "GranitEditor",
    outfile: "build/codemirror.js",
  },
  {
    entryPoints: ["js/mermaid.ts"],
    globalName: "GranitMermaid",
    outfile: "build/mermaid.js",
  },
].map((bundle) => ({
  ...bundle,
  bundle: true,
  format: "iife",
  minify: !watch,
  sourcemap: watch ? "inline" : false,
  target: "es2020",
  logLevel: "info",
}));

if (watch) {
  for (const opts of bundles) {
    const ctx = await esbuild.context(opts);
    await ctx.watch();
  }
  console.log("esbuild: watching for changes...");
} else {
  await Promise.all(bundles.map((opts) => esbuild.build(opts)));
}
