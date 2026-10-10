// Smoke test of the TypeScript API against the real examples.
// Run after `npm run build`: node --test test/
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { init, Illustrator, version, docs } from "../dist/index.js";

const examples = fileURLToPath(new URL("../../examples/", import.meta.url));
const fetchExample = (path) => readFile(examples + path).catch(() => null);

await init(await readFile(new URL("../pkg/agent_illustrator_wasm_bg.wasm", import.meta.url)));

test("renders a plain document", () => {
  const r = new Illustrator().render("row { rect a  rect b }  a -> b");
  assert.ok(r.ok, JSON.stringify(r.diagnostics));
  assert.match(r.output, /<svg/);
  assert.ok(version().length > 0);
  assert.ok(docs("grammar").length > 1000);
});

test("reports the location of a syntax error", () => {
  const r = new Illustrator().render("row { rect a ");
  assert.equal(r.ok, false);
  const [d] = r.diagnostics;
  assert.equal(d.code, "syntax");
  assert.equal(d.severity, "error");
  assert.equal(d.line, 1);
  assert.match(d.text, /line 1/);
});

test("sans-io: lists unresolved imports, then renders once they are supplied", async () => {
  const src = (await readFile(examples + "motion/git-copies.ail")).toString();
  const ail = new Illustrator({ basePath: "motion" });
  assert.deepEqual(ail.unresolved(src).sort(), ["motion/assets/code.svg", "motion/assets/docx.svg", "motion/git-deck.ail"]);
  const failed = ail.render(src, { animate: "js" });
  assert.equal(failed.ok, false);
  assert.ok(failed.missingFiles.length > 0);
  const r = await ail.renderResolved(src, fetchExample, { animate: "js" });
  assert.ok(r.ok, JSON.stringify(r.diagnostics));
  assert.deepEqual(ail.unresolved(src), []);
  assert.ok(ail.frameNames(src).length > 1);
});

test("unavailable files are reported, not looped on", async () => {
  const ail = new Illustrator();
  const r = await ail.renderResolved('import "nope.ail"\nrect a', async () => null);
  assert.equal(r.ok, false);
  assert.deepEqual(r.missingFiles, ["nope.ail"]);
  assert.equal(r.diagnostics[0].code, "missing-file");
});

test("svg templates are inlined from supplied files", async () => {
  const src = (await readFile(examples + "agentic-loop-story.ail")).toString();
  const r = await new Illustrator().renderResolved(src, fetchExample, { animate: "css" });
  assert.ok(r.ok, JSON.stringify(r.diagnostics));
});

test("bad options throw nothing, they fail the render", () => {
  const r = new Illustrator().render("rect a", { animate: "sideways" });
  assert.equal(r.ok, false);
  assert.match(r.diagnostics[0].message, /animate/);
});

test("lint findings are structured diagnostics", () => {
  const r = new Illustrator().render("rect a [width: 100]\nrect b [width: 100]\nconstrain b.left = a.left\nconstrain b.top = a.top", { lint: true });
  assert.ok(r.output);
  assert.ok(r.diagnostics.some((d) => d.code === "overlap"), JSON.stringify(r.diagnostics));
});
