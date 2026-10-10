// Build the playground's example set from examples/renders.txt — the same
// list the repo renders — so the picker never drifts from the repo.
//
// Copies every example source file (and the stylesheets) into public/ at its
// repo path, skipping the rendered outputs, and writes public/examples.json.
import { existsSync, readFileSync, writeFileSync, mkdirSync, cpSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const pub = join(here, "..", "public");

const outputs = new Set();
const bySource = new Map();
for (const line of readFileSync(join(root, "examples/renders.txt"), "utf8").split("\n")) {
  if (!line.trim() || line.startsWith("#")) continue;
  const [out, src, ...flags] = line.trim().split(/\s+/);
  outputs.add(out);
  const e = bySource.get(src) ?? { path: src, css: null, modes: new Set() };
  const i = flags.indexOf("--stylesheet-css");
  if (i >= 0) e.css = flags[i + 1];
  if (flags.includes("--animate")) e.modes.add("js");
  else if (flags.includes("--animate-css")) e.modes.add("css");
  else if (!flags.includes("--frame")) e.modes.add("none");
  bySource.set(src, e);
}

/** First sentence of the file's leading comment, cleaned up. */
function describe(text) {
  const lines = text.split("\n").map((l) => l.trim());
  for (const l of lines) {
    const s = l.replace(/^\/\*+|\*+\/$|^\/\/+/g, "").trim();
    // A storyboard is notes for the author, not a summary.
    if (/^STORYBOARD\b/.test(s)) return "";
    if (!s || /^INTENT\b|^Render:|^Usage:/.test(s)) continue;
    return s
      .replace(/^Scene( \d+ of the git deck)?:\s*/, "")
      .replace(/" -> "/g, " → ")
      .replace(/^"|"\.?$/g, "")
      .replace(/"\s*(\(.*\))?\.?$/, " $1")
      .replace(/^"/, "")
      .trim();
  }
  return "";
}

const title = (path) =>
  path
    .split("/")
    .pop()
    .replace(/\.ail$/, "")
    .replace(/-/g, " ")
    .replace(/^./, (c) => c.toUpperCase());

const examples = [...bySource.values()].map((e) => {
  const text = readFileSync(join(root, e.path), "utf8");
  // Prefer the click-through player for a deck; a CSS loop otherwise.
  const animate = e.modes.has("js") ? "js" : e.modes.has("css") ? "css" : "none";
  return {
    id: e.path.replace(/^examples\//, "").replace(/\.ail$/, ""),
    title: title(e.path),
    description: describe(text),
    path: e.path,
    css: e.css,
    animate,
    group: animate === "none" ? "Diagrams" : "Animations",
  };
});

function copyTree(from, to) {
  for (const name of readdirSync(from)) {
    const src = join(from, name);
    const rel = relative(root, src);
    if (statSync(src).isDirectory()) copyTree(src, join(to, name));
    else if (!outputs.has(rel) && !(name.endsWith(".svg") && existsSync(src.replace(/\.svg$/, ".ail"))) && /\.(ail|css|svg|png|jpe?g|gif|webp|py|txt)$/.test(name) && name !== "renders.txt") {
      mkdirSync(to, { recursive: true });
      cpSync(src, join(to, name));
    }
  }
}

copyTree(join(root, "examples"), join(pub, "examples"));
copyTree(join(root, "stylesheets"), join(pub, "stylesheets"));
const stylesheets = readdirSync(join(root, "stylesheets"))
  .filter((n) => n.endsWith(".css"))
  .map((n) => `stylesheets/${n}`);
for (const e of examples) if (e.css && !stylesheets.includes(e.css)) stylesheets.push(e.css);

mkdirSync(pub, { recursive: true });
writeFileSync(join(pub, "examples.json"), JSON.stringify({ examples, stylesheets }, null, 2) + "\n");
console.log(`examples.json: ${examples.length} examples, ${stylesheets.length} stylesheets`);
