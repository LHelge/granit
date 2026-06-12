import * as esbuild from "esbuild";

const watch = process.argv.includes("--watch");

/** @type {esbuild.BuildOptions} */
const opts = {
  entryPoints: ["js/editor.ts"],
  bundle: true,
  format: "iife",
  globalName: "GranitEditor",
  outfile: "build/codemirror.js",
  minify: !watch,
  sourcemap: watch ? "inline" : false,
  target: "es2020",
  logLevel: "info",
};

if (watch) {
  const ctx = await esbuild.context(opts);
  await ctx.watch();
  console.log("esbuild: watching for changes...");
} else {
  await esbuild.build(opts);
}
