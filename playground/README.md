# AIL playground

Write Agent Illustrator code in the browser and see the drawing as you type.
It is deployed to GitHub Pages on every release:
<https://kervel.github.io/agent-illustrator/>.

- The examples come from `examples/renders.txt`. `npm run examples` copies their
  sources and stylesheets into `public/` and writes `public/examples.json`, so
  the picker always lists what the repo renders.
- Files an example imports are fetched on demand. This is the host side of the
  [wasm package](../wasm/README.md)'s sans-io loop. Each one opens as an
  editable tab. A missing import shows a "Create file" button.
- Output modes: **Still** (pick a keyframe, scrub through its motion), **Player**
  (`animate: "js"`, click to step), **Loop** (`animate: "css"`).
- **Copy link** puts the edited source, stylesheet and mode into the URL hash.

```bash
(cd ../wasm && npm ci && npm run build:wasm)   # the module the playground imports
npm ci
npm run dev      # http://localhost:5173
npm run build    # dist/, static files with relative URLs
```
