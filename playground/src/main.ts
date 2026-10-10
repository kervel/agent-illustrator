import "./style.css";
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection } from "@codemirror/view";
import { EditorState, type Extension } from "@codemirror/state";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { bracketMatching, indentOnInput } from "@codemirror/language";
import { setDiagnostics, type Diagnostic as CmDiagnostic } from "@codemirror/lint";
import { ail, ailHighlight } from "./ail-lang";
import { init, Illustrator, version, type Diagnostic, type RenderOptions, type RenderResult } from "../../wasm/ts/index";

// ---------- Types and state ----------

type Mode = "none" | "js" | "css";

interface Example {
  id: string;
  title: string;
  description: string;
  path: string;
  css: string | null;
  animate: Mode;
  group: string;
}

interface Manifest {
  examples: Example[];
  stylesheets: string[];
}

/** A text file open in a tab. */
interface Doc {
  path: string;
  role: "main" | "dep" | "style";
  state: EditorState;
  edited: boolean;
}

const BLANK: Example = {
  id: "blank",
  title: "Blank page",
  description: "Start from a few boxes and arrows.",
  path: "untitled.ail",
  css: null,
  animate: "none",
  group: "",
};

const BLANK_SOURCE = `// Say what is there and how it relates. The renderer decides where.
row [gap: 90] {
  rect browser [label: "Browser"]
  rect api [label: "API", fill: accent-1]
  col [gap: 50] {
    rect db [label: "Database"]
    rect cache [label: "Cache"]
  }
}
browser.right -> api.left [label: "HTTPS"]
api.right -> db.left
api.right -> cache.left
constrain api.center_y = db.center_y
constrain browser.center_y = api.center_y
`;

const TEXT_FILE = /\.(ail|css|py|txt|js|ts|rs|json|md|toml|ya?ml)$/i;

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;

// Created once the module is loaded (see main).
let ailRenderer: Illustrator;
let manifest: Manifest = { examples: [], stylesheets: [] };
let example: Example = BLANK;
let docs = new Map<string, Doc>();
let active = "";
let mainPath = "";
let stylePath: string | null = null;
let mode: Mode = "none";
let frame: string | null = null;
let atPct = 100;
let lint = true;
let zoom: number | "fit" = "fit";
let lastSvg = "";
let natural = { w: 0, h: 0 };
let renderSeq = 0;

const dirname = (p: string) => (p.includes("/") ? p.slice(0, p.lastIndexOf("/")) : "");
const basename = (p: string) => p.slice(p.lastIndexOf("/") + 1);
/** A path as the main document would write it. */
const relToBase = (p: string) => {
  const base = dirname(mainPath);
  return base && p.startsWith(base + "/") ? p.slice(base.length + 1) : p;
};

// ---------- Fetching (the host side of sans-io) ----------

const fetchCache = new Map<string, Uint8Array | null>();

/** Example files are served next to the page. Unknown paths are null. */
async function fetchFile(path: string): Promise<Uint8Array | null> {
  if (fetchCache.has(path)) return fetchCache.get(path)!;
  let bytes: Uint8Array | null = null;
  try {
    const res = await fetch(`./${path}`);
    // A dev server answers unknown paths with the app page; that is not the file.
    const html = (res.headers.get("content-type") ?? "").includes("text/html");
    if (res.ok && !html) bytes = new Uint8Array(await res.arrayBuffer());
  } catch {
    bytes = null;
  }
  fetchCache.set(path, bytes);
  return bytes;
}

/** The renderer's fetcher: a file open in a tab wins over the served copy. */
async function supply(path: string): Promise<Uint8Array | null> {
  const open = docs.get(path);
  if (open) return new TextEncoder().encode(open.state.doc.toString());
  const bytes = await fetchFile(path);
  if (bytes && TEXT_FILE.test(path) && !path.endsWith(".css")) {
    addDoc(path, "dep", new TextDecoder().decode(bytes), false);
  }
  return bytes;
}

// ---------- Editor ----------

const view = new EditorView({ parent: $("editor") });

function extensions(path: string): Extension[] {
  const isAil = path.endsWith(".ail");
  return [
    lineNumbers(),
    highlightActiveLineGutter(),
    highlightActiveLine(),
    drawSelection(),
    history(),
    indentOnInput(),
    bracketMatching(),
    keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
    EditorState.tabSize.of(2),
    ...(isAil ? [ail, ailHighlight] : []),
    EditorView.updateListener.of((u) => {
      if (!u.docChanged) return;
      const doc = docs.get(path);
      if (!doc) return;
      doc.state = u.state;
      if (!doc.edited) {
        doc.edited = true;
        renderTabs();
      }
      if (doc.role !== "style") ailRenderer.setFile(path, u.state.doc.toString());
      scheduleRender();
    }),
  ];
}

function addDoc(path: string, role: Doc["role"], text: string, edited: boolean) {
  if (docs.has(path)) return;
  docs.set(path, { path, role, edited, state: EditorState.create({ doc: text, extensions: extensions(path) }) });
  renderTabs();
}

function openTab(path: string) {
  const doc = docs.get(path);
  if (!doc) return;
  // Keep the outgoing tab's state (undo history, selection).
  const current = docs.get(active);
  if (current) current.state = view.state;
  active = path;
  view.setState(doc.state);
  if (path === mainPath) applyDiagnostics(lastDiagnostics);
  renderTabs();
}

function renderTabs() {
  const order = [...docs.values()].sort((a, b) => rank(a) - rank(b) || a.path.localeCompare(b.path));
  const tabs = $("tabs");
  tabs.replaceChildren(
    ...order.map((d) => {
      const b = document.createElement("button");
      b.className = "tab";
      b.setAttribute("role", "tab");
      b.setAttribute("aria-selected", String(d.path === active));
      b.title = d.path;
      b.append(d.role === "style" ? basename(d.path) : relToBase(d.path));
      if (d.role !== "main") {
        const note = document.createElement("span");
        note.className = "tab-note";
        note.textContent = d.role === "style" ? "style" : "import";
        b.append(note);
      }
      if (d.edited && d.role !== "main") {
        const dot = document.createElement("span");
        dot.className = "dot";
        dot.title = "Edited";
        b.append(dot);
      }
      b.onclick = () => openTab(d.path);
      return b;
    }),
  );
}

const rank = (d: Doc) => (d.role === "main" ? 0 : d.role === "dep" ? 1 : 2);

let lastDiagnostics: Diagnostic[] = [];

/** Error underlines in the main document. */
function applyDiagnostics(diags: Diagnostic[]) {
  if (active !== mainPath) return;
  const doc = view.state.doc;
  const marks: CmDiagnostic[] = [];
  for (const d of diags) {
    if (d.line == null || d.line > doc.lines) continue;
    const line = doc.line(d.line);
    const from = Math.min(line.from + (d.column ?? 1) - 1, line.to);
    let to = from + 1;
    if (d.endLine != null && d.endColumn != null && d.endLine <= doc.lines) {
      to = Math.min(doc.line(d.endLine).from + d.endColumn - 1, doc.length);
    }
    marks.push({
      from,
      to: Math.max(to, from + 1) > doc.length ? doc.length : Math.max(to, from + 1),
      severity: d.severity,
      message: d.hint ? `${d.message}\n${d.hint}` : d.message,
    });
  }
  view.dispatch(setDiagnostics(view.state, marks));
}

// ---------- Rendering ----------

let timer: number | undefined;
function scheduleRender(delay = 160) {
  clearTimeout(timer);
  timer = window.setTimeout(render, delay);
}

function options(): RenderOptions {
  const style = stylePath ? docs.get(stylePath)?.state.doc.toString() : undefined;
  const o: RenderOptions = { animate: mode, lint, stylesheetCss: style };
  if (mode === "none" && frame !== null) {
    o.frame = frame;
    if (atPct < 100) o.at = `${atPct}%`;
  }
  return o;
}

async function render() {
  const seq = ++renderSeq;
  const doc = docs.get(mainPath);
  if (!doc) return;
  const source = doc.state.doc.toString();
  setStatus("busy", "Rendering…");
  const t0 = performance.now();
  ailRenderer.basePath = dirname(mainPath);
  const result = await ailRenderer.renderResolved(source, supply, options());
  if (seq !== renderSeq) return; // a newer edit already started
  const ms = Math.round(performance.now() - t0);
  showResult(result, ms);
  updateFrames(source);
}

function showResult(r: RenderResult, ms: number) {
  lastDiagnostics = r.diagnostics;
  applyDiagnostics(r.diagnostics);
  if (r.output) {
    lastSvg = r.output;
    paint(r.output);
    $("sheet").classList.remove("is-stale");
  } else {
    $("sheet").classList.toggle("is-stale", !!lastSvg);
  }
  $("empty").hidden = !!lastSvg;
  if (!lastSvg) {
    $("empty").innerHTML = "<strong>Nothing drawn yet</strong>Fix the error on the left and the drawing appears here.";
  }
  renderReport(r, ms);
}

/** The SVG in a sandboxed frame: its styles stay out of the page, and the player script runs. */
function paint(svg: string) {
  natural = svgSize(svg);
  const page = `<!doctype html><meta charset="utf-8"><style>
html,body{margin:0;height:100%;overflow:hidden;background:transparent}
body>svg{display:block;width:100%;height:100%}
</style>${svg}`;
  const frame = $<HTMLIFrameElement>("out");
  frame.srcdoc = page;
  layoutSheet();
}

function svgSize(svg: string): { w: number; h: number } {
  const tag = /<svg\b[^>]*>/.exec(svg)?.[0] ?? "";
  const vb = /viewBox="([^"]+)"/.exec(tag)?.[1]?.split(/[\s,]+/).map(Number);
  const w = parseFloat(/\swidth="([\d.]+)/.exec(tag)?.[1] ?? "") || vb?.[2] || 400;
  const h = parseFloat(/\sheight="([\d.]+)/.exec(tag)?.[1] ?? "") || vb?.[3] || 300;
  return { w, h };
}

function layoutSheet() {
  if (!natural.w) return;
  const stage = $("stage");
  const pad = window.innerWidth < 860 ? 32 : 96;
  const fit = Math.min((stage.clientWidth - pad) / natural.w, (stage.clientHeight - pad) / natural.h, 2);
  const scale = zoom === "fit" ? Math.max(fit, 0.05) : zoom;
  const frame = $<HTMLIFrameElement>("out");
  frame.style.width = `${Math.round(natural.w * scale)}px`;
  frame.style.height = `${Math.round(natural.h * scale)}px`;
  $("dims").textContent = `${Math.round(natural.w)} × ${Math.round(natural.h)}${scale !== 1 ? ` · ${Math.round(scale * 100)}%` : ""}`;
  $("zoom-fit").textContent = zoom === "fit" ? "Fit" : `${Math.round(scale * 100)}%`;
}

function setZoom(z: number | "fit") {
  if (z !== "fit") z = Math.min(Math.max(z, 0.1), 8);
  zoom = z;
  layoutSheet();
}

function currentScale(): number {
  const w = parseFloat($<HTMLIFrameElement>("out").style.width) || natural.w;
  return w / (natural.w || 1);
}

// ---------- Report ----------

function setStatus(kind: "ok" | "error" | "warn" | "busy", text: string) {
  const report = $("report");
  let s = report.querySelector<HTMLElement>(".status");
  if (!s) {
    s = document.createElement("div");
    report.prepend(s);
  }
  s.className = `status is-${kind}`;
  s.textContent = text;
}

function renderReport(r: RenderResult, ms: number) {
  const report = $("report");
  report.replaceChildren();
  const errors = r.diagnostics.filter((d) => d.severity === "error").length;
  const warnings = r.diagnostics.length - errors;
  if (!r.output) setStatus("error", "Not rendered: the source has an error.");
  else if (errors) setStatus("error", `Rendered in ${ms} ms, with ${plural(errors, "layout error")}.`);
  else if (warnings) setStatus("warn", `Rendered in ${ms} ms, with ${plural(warnings, "lint warning")}.`);
  else setStatus("ok", `Rendered in ${ms} ms${lint ? ", lint clean" : ""}.`);

  for (const path of r.missingFiles) {
    const row = document.createElement("div");
    row.className = "missing";
    const code = document.createElement("code");
    code.textContent = relToBase(path);
    const make = document.createElement("button");
    make.className = "btn";
    make.textContent = "Create file";
    make.onclick = () => {
      addDoc(path, "dep", path.endsWith(".ail") ? "// Templates and motion macros here are available to the main file.\n" : "", true);
      ailRenderer.setFile(path, docs.get(path)!.state.doc.toString());
      openTab(path);
      scheduleRender(0);
    };
    row.append("Missing file ", code, make);
    report.append(row);
  }

  for (const d of r.diagnostics) {
    if (d.code === "missing-file" && r.missingFiles.length) continue;
    const row = document.createElement("button");
    row.className = "diag";
    const sev = document.createElement("span");
    sev.className = `diag-sev ${d.severity}`;
    sev.textContent = d.severity === "error" ? "Error" : "Warning";
    const msg = document.createElement("span");
    msg.className = "diag-msg";
    msg.textContent = d.message;
    row.append(sev, msg);
    const meta = [d.code, d.line != null ? `line ${d.line}` : "", d.frames.length ? `in ${d.frames.join(", ")}` : ""].filter(Boolean);
    const m = document.createElement("span");
    m.className = "diag-meta";
    m.textContent = meta.join("  ·  ");
    row.append(m);
    if (d.hint) {
      const h = document.createElement("span");
      h.className = "diag-hint";
      h.textContent = d.hint;
      row.append(h);
    }
    if (d.line != null) {
      row.dataset.line = String(d.line);
      row.onclick = () => jumpTo(d.line!, d.column ?? 1);
    } else {
      row.tabIndex = -1;
    }
    report.append(row);
  }
}

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

function jumpTo(line: number, column: number) {
  if (active !== mainPath) openTab(mainPath);
  const l = view.state.doc.line(Math.min(line, view.state.doc.lines));
  const pos = Math.min(l.from + column - 1, l.to);
  view.dispatch({ selection: { anchor: pos }, scrollIntoView: true });
  view.focus();
}

// ---------- Keyframes ----------

let lastFrames: string[] = [];

function updateFrames(source: string) {
  const names = ailRenderer.frameNames(source);
  const changed = names.join("\n") !== lastFrames.join("\n");
  lastFrames = names;
  if (frame !== null && !names.includes(frame)) frame = null;
  $("frames").hidden = names.length === 0 || mode !== "none";
  if (!changed && $("chips").childElementCount) {
    syncChips();
    return;
  }
  const chips = names.map((n) => {
    const b = document.createElement("button");
    b.className = "chip";
    b.textContent = n;
    b.dataset.frame = n;
    b.onclick = () => {
      frame = frame === n ? null : n;
      atPct = 100;
      ($("at") as HTMLInputElement).value = "100";
      syncChips();
      scheduleRender(0);
    };
    return b;
  });
  const all = document.createElement("button");
  all.className = "chip";
  all.textContent = "start";
  all.title = "The picture before any keyframe";
  all.dataset.frame = "";
  all.onclick = () => {
    frame = null;
    syncChips();
    scheduleRender(0);
  };
  $("chips").replaceChildren(all, ...chips);
  syncChips();
}

function syncChips() {
  for (const c of $("chips").querySelectorAll<HTMLElement>(".chip")) {
    c.setAttribute("aria-pressed", String((c.dataset.frame || null) === frame));
  }
  $("scrub").hidden = frame === null;
  $("at-out").textContent = atPct === 100 ? "end" : `${atPct}%`;
}

// ---------- Examples ----------

async function loadExample(ex: Example, overrides: Partial<SharedState> = {}) {
  example = ex;
  docs = new Map();
  active = "";
  ailRenderer.clearFiles();
  lastSvg = "";
  frame = overrides.frame ?? null;
  atPct = 100;
  lastFrames = [];
  $("chips").replaceChildren();
  mainPath = ex.path;
  mode = overrides.mode ?? ex.animate;
  $("example-title").textContent = ex.title;

  let source = overrides.source;
  if (source === undefined) {
    if (ex === BLANK) source = BLANK_SOURCE;
    else {
      const bytes = await fetchFile(ex.path);
      source = bytes ? new TextDecoder().decode(bytes) : `// Could not load ${ex.path}\n`;
    }
  }
  addDoc(mainPath, "main", source, overrides.source !== undefined);
  ailRenderer.setFile(mainPath, source);
  await setStyle(overrides.stylePath !== undefined ? overrides.stylePath : ex.css, overrides.styleText);
  openTab(mainPath);
  syncControls();
  render();
}

async function setStyle(path: string | null, text?: string) {
  if (stylePath && docs.get(stylePath)?.role === "style") docs.delete(stylePath);
  stylePath = path;
  if (path) {
    const content = text ?? new TextDecoder().decode((await fetchFile(path)) ?? new Uint8Array());
    addDoc(path, "style", content, text !== undefined);
  }
  if (!docs.has(active)) openTab(mainPath);
  renderTabs();
  syncControls();
}

function buildExamplePanel() {
  const panel = $("examples-panel");
  const groups = new Map<string, Example[]>();
  for (const e of manifest.examples) groups.set(e.group, [...(groups.get(e.group) ?? []), e]);
  const item = (e: Example) => {
    const b = document.createElement("button");
    b.className = "ex";
    b.dataset.id = e.id;
    const title = document.createElement("span");
    title.className = "ex-title";
    title.textContent = e.title;
    const kind = document.createElement("span");
    kind.className = "ex-kind";
    kind.textContent = e.animate === "js" ? "Steps" : e.animate === "css" ? "Loops" : "";
    const desc = document.createElement("span");
    desc.className = "ex-desc";
    desc.textContent = e.description;
    b.append(title, kind, desc);
    b.onclick = () => {
      closePanel();
      if (e.id !== example.id || [...docs.values()].some((d) => d.edited)) {
        location.hash = "";
        loadExample(e);
      }
    };
    return b;
  };
  panel.replaceChildren(item(BLANK));
  for (const [name, list] of groups) {
    const h = document.createElement("h2");
    h.textContent = name;
    panel.append(h, ...list.map(item));
  }
}

function openPanel() {
  const panel = $("examples-panel");
  panel.hidden = false;
  $("examples-btn").setAttribute("aria-expanded", "true");
  for (const b of panel.querySelectorAll<HTMLElement>(".ex")) b.setAttribute("aria-current", String(b.dataset.id === example.id));
  panel.querySelector<HTMLElement>('[aria-current="true"]')?.focus();
}

function closePanel() {
  $("examples-panel").hidden = true;
  $("examples-btn").setAttribute("aria-expanded", "false");
}

// ---------- Controls ----------

function syncControls() {
  for (const b of document.querySelectorAll<HTMLElement>(".seg button")) {
    b.setAttribute("aria-checked", String(b.dataset.mode === mode));
  }
  const sel = $<HTMLSelectElement>("style-select");
  sel.value = stylePath ?? "";
  $<HTMLInputElement>("lint-toggle").checked = lint;
}

function buildStyleSelect() {
  const sel = $<HTMLSelectElement>("style-select");
  const none = new Option("Default palette", "");
  sel.replaceChildren(none, ...manifest.stylesheets.map((s) => new Option(basename(s).replace(/\.css$/, ""), s)));
  sel.onchange = async () => {
    await setStyle(sel.value || null);
    scheduleRender(0);
  };
}

function wireControls() {
  for (const b of document.querySelectorAll<HTMLElement>(".seg button")) {
    b.onclick = () => {
      mode = b.dataset.mode as Mode;
      syncControls();
      scheduleRender(0);
    };
  }
  $<HTMLInputElement>("lint-toggle").onchange = (e) => {
    lint = (e.target as HTMLInputElement).checked;
    scheduleRender(0);
  };
  const at = $<HTMLInputElement>("at");
  at.oninput = () => {
    atPct = Number(at.value);
    syncChips();
    scheduleRender(30);
  };

  $("examples-btn").onclick = () => ($("examples-panel").hidden ? openPanel() : closePanel());
  document.addEventListener("click", (e) => {
    if (!(e.target as HTMLElement).closest(".picker")) closePanel();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !$("examples-panel").hidden) {
      closePanel();
      $("examples-btn").focus();
    }
  });

  $("zoom-in").onclick = () => setZoom(currentScale() * 1.25);
  $("zoom-out").onclick = () => setZoom(currentScale() / 1.25);
  $("zoom-fit").onclick = () => setZoom("fit");
  new ResizeObserver(() => zoom === "fit" && layoutSheet()).observe($("stage"));

  $("svg-btn").onclick = () => lastSvg && download(new Blob([lastSvg], { type: "image/svg+xml" }), "svg");
  $("png-btn").onclick = () => lastSvg && downloadPng();
  $("share-btn").onclick = share;

  wireSplit();
}

function fileStem() {
  return basename(mainPath).replace(/\.ail$/, "") + (mode === "none" && frame ? `-${frame}` : "");
}

function download(blob: Blob, ext: string) {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = `${fileStem()}.${ext}`;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

async function downloadPng() {
  const img = new Image();
  const url = URL.createObjectURL(new Blob([lastSvg], { type: "image/svg+xml" }));
  img.src = url;
  try {
    await img.decode();
    const canvas = document.createElement("canvas");
    canvas.width = Math.round(natural.w * 2);
    canvas.height = Math.round(natural.h * 2);
    const ctx = canvas.getContext("2d")!;
    ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
    canvas.toBlob((b) => b && download(b, "png"), "image/png");
  } catch {
    toast("This drawing could not be turned into a PNG in the browser. Download the SVG instead.");
  } finally {
    URL.revokeObjectURL(url);
  }
}

function wireSplit() {
  const split = $("split");
  const work = $("work");
  const set = (pct: number) => {
    pct = Math.min(Math.max(pct, 20), 75);
    work.style.setProperty("--split", `${pct}%`);
    try {
      localStorage.setItem("ail-split", String(pct));
    } catch {
      /* storage unavailable: the split just isn't remembered */
    }
    if (zoom === "fit") layoutSheet();
  };
  try {
    const saved = Number(localStorage.getItem("ail-split"));
    if (saved) set(saved);
  } catch {
    /* ignore */
  }
  split.onpointerdown = (e) => {
    split.setPointerCapture(e.pointerId);
    // The iframe would swallow the pointer while dragging over it.
    $("out").style.pointerEvents = "none";
    split.onpointermove = (m) => set((m.clientX / work.clientWidth) * 100);
    split.onpointerup = () => {
      split.onpointermove = null;
      $("out").style.pointerEvents = "";
    };
  };
  split.onkeydown = (e) => {
    const now = parseFloat(getComputedStyle(work).getPropertyValue("--split")) || 44;
    if (e.key === "ArrowLeft") set(now - 2);
    if (e.key === "ArrowRight") set(now + 2);
  };
}

function toast(text: string) {
  const t = document.createElement("div");
  t.className = "toast";
  t.setAttribute("role", "status");
  t.textContent = text;
  document.body.append(t);
  setTimeout(() => t.remove(), 2600);
}

// ---------- Sharing: the state in the URL hash ----------

interface SharedState {
  example: string;
  source?: string;
  stylePath?: string | null;
  styleText?: string;
  mode?: Mode;
  frame?: string | null;
}

async function pack(obj: unknown): Promise<string> {
  const stream = new Blob([JSON.stringify(obj)]).stream().pipeThrough(new CompressionStream("deflate-raw"));
  const bytes = new Uint8Array(await new Response(stream).arrayBuffer());
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

async function unpack(s: string): Promise<unknown> {
  const bin = atob(s.replace(/-/g, "+").replace(/_/g, "/"));
  const bytes = Uint8Array.from(bin, (c) => c.charCodeAt(0));
  const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
  return JSON.parse(await new Response(stream).text());
}

async function share() {
  const main = docs.get(mainPath)!;
  const style = stylePath ? docs.get(stylePath) : undefined;
  const state: SharedState = { example: example.id, mode };
  if (main.edited) state.source = main.state.doc.toString();
  if (stylePath !== example.css) state.stylePath = stylePath;
  if (style?.edited) state.styleText = style.state.doc.toString();
  if (frame) state.frame = frame;
  const hash = `#s=${await pack(state)}`;
  window.history.replaceState(null, "", hash);
  try {
    await navigator.clipboard.writeText(location.href);
    toast("Link copied");
  } catch {
    toast("Link is in the address bar");
  }
}

async function fromHash(): Promise<SharedState | null> {
  const m = /^#s=(.+)$/.exec(location.hash);
  if (!m) return null;
  try {
    return (await unpack(m[1])) as SharedState;
  } catch {
    return null;
  }
}

// ---------- Start ----------

async function main() {
  await init();
  ailRenderer = new Illustrator();
  $("version").textContent = `agent-illustrator ${version()}`;
  try {
    manifest = await (await fetch("./examples.json")).json();
  } catch {
    manifest = { examples: [], stylesheets: [] };
  }
  buildExamplePanel();
  buildStyleSelect();
  wireControls();
  window.addEventListener("resize", () => zoom === "fit" && layoutSheet());

  const shared = await fromHash();
  const byId = (id: string) => manifest.examples.find((e) => e.id === id);
  if (shared) {
    await loadExample(byId(shared.example) ?? BLANK, shared);
  } else {
    await loadExample(manifest.examples[0] ?? BLANK);
  }
}

main().catch((e) => {
  setStatus("error", `The renderer failed to start: ${e instanceof Error ? e.message : String(e)}`);
});
