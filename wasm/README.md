# agent-illustrator-wasm

Agent Illustrator compiled to WebAssembly, with a TypeScript API. It is the
same renderer as the CLI. It runs in the browser, in Node, or in any other
WebAssembly host. The [playground](https://kervel.github.io/agent-illustrator/)
is built on it.

```bash
npm install https://github.com/kervel/agent-illustrator/releases/download/vX.Y.Z/agent-illustrator-wasm.tgz
```

```ts
import { init, Illustrator } from "agent-illustrator-wasm";

await init(); // in Node: await init(await readFile(".../agent_illustrator_wasm_bg.wasm"))
const ail = new Illustrator();
const r = ail.render(`row { rect a  rect b }  a -> b`, { lint: true });
if (r.output) document.body.innerHTML = r.output;
for (const d of r.diagnostics) console.log(d.severity, d.code, d.line, d.message);
```

## Files: you supply them (sans-io)

The library never reads a file or makes a request. A document can reference
other files: `import "deck.ail"`, `template "logo" from "logo.svg"`, or
`code c [file: "main.py"]`. You hand those files over yourself, keyed by their
path relative to the root:

```ts
const ail = new Illustrator({ basePath: "slides" }); // the document's directory
ail.unresolved(source);              // ["slides/deck.ail", "slides/logo.svg"]
ail.setFile("slides/deck.ail", deckText);
ail.unresolved(source);              // ["slides/logo.svg", "slides/parts.ail"]: deck.ail imports parts.ail
```

`unresolved` scans the document, and every file already supplied, without
rendering. Loop until it returns nothing. You can also let `renderResolved` run
that loop with your own fetcher:

```ts
const r = await ail.renderResolved(source, async (path) => {
  const res = await fetch(`/content/${path}`);
  return res.ok ? new Uint8Array(await res.arrayBuffer()) : null; // null: does not exist
});
r.missingFiles; // what could still not be found
```

Paths are normalized (`a/./b/../c.svg` → `a/c.svg`). Images in supplied files are
inlined as base64 data URIs, so the SVG stands alone. Built-in libraries
(`import "ail:motion/git"`) are inside the module and never show up as
unresolved.

## API

| | |
|---|---|
| `init(module?)` | Load the WebAssembly module (once). |
| `new Illustrator({ basePath?, files? })` | A renderer and the files it can see. |
| `setFile(path, string \| Uint8Array)`, `setFiles({...})`, `removeFile`, `clearFiles`, `files()` | Manage supplied files. |
| `dependencies(source)` → `{ needed, missing }` | Every referenced file, and the missing ones. `unresolved(source)` is `missing`. |
| `render(source, options)` → `RenderResult` | Render with what has been supplied. A bad document never makes it throw. |
| `resolve(source, fetcher)`, `renderResolved(source, fetcher, options)` | The fetch-and-supply loop. |
| `frameNames(source)` | Keyframe names, in order. |
| `version()`, `docs(topic)`, `docTopics()`, `playerJs()` | Version; the language reference (`"skill"`, `"brief"`, `"grammar"`, `"animation"`, `"styling"`, or a topic); the standalone player script. |

**Options**: `animate` (`"none"`, `"js"` for the click-to-step player, or `"css"` for a self-playing loop), `stylesheetCss`, `frame` and `at` (a still, possibly mid-motion), `lint`, `debug`, `cropToContent`, `noFrameCss`, `imageHref` (`"base64"` or `"verbatim"`), and `timeline` or `states` (text output).

**Result**: `ok` (rendered, with no error-severity diagnostic), `output` (the
SVG, absent only when the render failed; a lint error keeps it),
`diagnostics`, `missingFiles`. Each diagnostic has:

| field | |
|---|---|
| `severity` | `"error"` or `"warning"` |
| `code` | Stable, kebab-case. `syntax`, `undefined-identifier`, `missing-file`, `unknown-template`, and so on, or a lint category (`overlap`, `label`, `canvas-overflow`, …). |
| `message` | One line, without the location. |
| `line`, `column`, `endLine`, `endColumn` | 1-based. Present when the diagnostic points at the source. |
| `hint` | What would fix it, when known (`did you mean: server?`). |
| `frames` | The keyframes a lint finding occurs in. |
| `text` | The full report as the CLI prints it: location, source line, caret. |

## Not in the WebAssembly build

- **PNG output.** The CLI rasterizes with bundled fonts. In a browser, draw the SVG onto a canvas instead; the playground's PNG button does that.
- **`--serve`, `--frames-to-dir`, `--film`.** These need a filesystem or a server. Render each frame with `frame` instead.
- **TOML stylesheets** (`--stylesheet`, deprecated). Use `stylesheetCss`.

## Building

```bash
cd wasm
npm ci
npm run build   # wasm-pack (target web) + tsc
npm test        # API tests against the repo's examples
```
