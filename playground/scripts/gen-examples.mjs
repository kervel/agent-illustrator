// Build the playground's example set. examples.txt picks the examples and
// their order; examples/renders.txt (what the repo renders) supplies each
// one's stylesheet and animation mode.
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

/** The first sentence of the file's leading comment, cleaned up. */
function describe(text) {
  // The leading comment's first paragraph, as one line.
  const para = [];
  let inBlock = false;
  for (const raw of text.trimStart().split("\n")) {
    const l = raw.trim();
    if (l.startsWith("/*")) inBlock = true;
    const isComment = inBlock || l.startsWith("//");
    const s = l.replace(/^\/\*+|\*+\/$|^\/\/+/g, "").trim();
    if (!isComment || !s || /^(Render|Usage):/.test(s)) break;
    // A storyboard is notes for the author, not a summary.
    if (/^STORYBOARD\b/.test(s)) return "";
    if (/^INTENT\b/.test(s)) continue;
    para.push(s);
    if (l.endsWith("*/")) break;
  }
  const line = para.join(" ").replace(/^Scene( \d+ of the git deck)?:\s*/, "");
  // `"Title" -> "Next title".` scene headers read as titles.
  if (line.startsWith('"')) return line.replace(/" -> "/g, " → ").replace(/"/g, "").replace(/\.$/, "");
  return line.match(/^.*?\.(?=\s|$)/)?.[0] ?? line;
}

const title = (path) =>
  path
    .split("/")
    .pop()
    .replace(/\.ail$/, "")
    .replace(/-/g, " ")
    .replace(/^./, (c) => c.toUpperCase());

const examples = [];
let group = "";
for (const line of readFileSync(join(here, "..", "examples.txt"), "utf8").split("\n")) {
  const l = line.trim();
  if (l.startsWith("## ")) group = l.slice(3).trim();
  if (!l || l.startsWith("#")) continue;
  const e = bySource.get(l);
  if (!e) throw new Error(`examples.txt: ${l} is not in examples/renders.txt`);
  const text = readFileSync(join(root, e.path), "utf8");
  // Prefer the click-through player for a deck; a CSS loop otherwise.
  const animate = e.modes.has("js") ? "js" : e.modes.has("css") ? "css" : "none";
  examples.push({
    id: e.path.replace(/^examples\//, "").replace(/\.ail$/, ""),
    title: title(e.path),
    description: describe(text),
    path: e.path,
    css: e.css,
    animate,
    group,
  });
}

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
