# Find Clipart Skill

Find and integrate open-source SVG clipart into Agent Illustrator diagrams.

## When to Use

Use this sub-skill when a diagram needs **visual richness beyond basic shapes** —
icons, illustrations, logos, or pictograms that make the diagram look professional
rather than a collection of labeled rectangles.

---

## Part 1: Search Strategy

### Where to Search

Search for SVG clipart on these open-source repositories:

1. **OpenClipart** — `openclipart.org` (public domain, CC0)
2. **SVG Repo** — `svgrepo.com` (various open licenses, check per-icon)
3. **Wikimedia Commons** — `commons.wikimedia.org` (various licenses)
4. **Heroicons** — `heroicons.com` (MIT, good for UI icons)
5. **Lucide** — `lucide.dev` (ISC license, clean line icons)

### Search Process

1. Identify what visual elements the diagram needs (e.g., "server", "person", "terminal")
2. For each element, search the web for: `site:openclipart.org [element] SVG`
   or `site:svgrepo.com [element] SVG`
3. Download the SVG file
4. Check the license (prefer CC0/public domain/MIT)
5. Simplify if needed (remove unnecessary metadata, comments)

### Spawn a Subagent for Search

For diagrams needing multiple clipart items, spawn a search subagent to find
them in parallel. The subagent should:

1. Search the web for each needed clipart item
2. Download each SVG
3. Save files to a `clipart/` directory next to the .ail file
4. Report what was found and what licenses apply

Subagent prompt template:

> Find SVG clipart for these items: [list]
> Search openclipart.org and svgrepo.com.
> For each item:
> 1. Find a clean, simple SVG (not photorealistic)
> 2. Download it to [directory]/clipart/[name].svg
> 3. Verify the file is valid SVG
> 4. Report: filename, source URL, license
>
> Prefer: simple line art or flat design. Avoid: gradients, filters, photorealism.
> The SVGs will be embedded in a technical diagram, so simpler is better.

---

## Part 2: Process Each Found SVG (REQUIRED before integration)

Raw downloads are almost never usable as-is. Found clipart comes from different
authors, at different scales, in different styles. Process every SVG before importing
it — skipping this is the main reason clipart diagrams look wrong.

### 2a. Normalize the SVG

- Ensure a `viewBox` exists — the embedder reads it to size the clipart. If only
  `width`/`height` are present, convert them to `viewBox="0 0 W H"`.
- Strip the `<?xml … ?>` declaration, `<!DOCTYPE …>`, editor metadata (`<metadata>`,
  Inkscape/Sodipodi namespaces and attributes), and comments.
- Remove the root `width`/`height` attributes so the AIL instance `[width:/height:]`
  controls the rendered size (a leftover fixed `width` fights the template sizing).

### 2b. Tighten the viewBox to the artwork (scale prep)

Many SVGs have large empty padding around the subject. If the `viewBox` is mostly
whitespace, `[width: 60]` makes the *subject* render tiny. Crop the `viewBox` to the
artwork's tight bounding box so the subject fills the box. (Render and eyeball, or use
a tool that reports the content bbox, e.g. `inkscape --query-*` or `usvg`.)

### 2c. Reconcile scale across ALL clipart (do this holistically, not per-item)

Clipart from different sources has no shared scale — a "person" and a "car" found
separately can differ by 5–10× intrinsically. After tightening each viewBox, fix the
sizes in one pass that compares all items together (it cannot be judged item-by-item).

Sizing must be **intentional** — pick one of two regimes, never the middle:

- **Icon set (flat / line art, usually square-ish):** make the icons *exactly* the same
  box size — a uniform grid (e.g. all `[width: 60, height: 60]`). Two icons at "almost
  but not quite" the same size (58 vs 62) look broken — the eye reads the mismatch as a
  mistake. Snap them equal.
- **Skeuomorphic / photorealistic scene art:** size by true real-world proportion
  instead (person ~1.7 m tall, car ~4 m long → the car ~2.3× the person). Here genuine
  size *differences* read naturally, so deliberate, obvious differences are correct.

The trap to avoid in both cases is the near-match: icons that are close-but-unequal.
Either identical, or clearly different — nothing in between.

### 2d. Simplify

If a clipart is too detailed or too colorful for a technical diagram, reduce it: drop
filters/gradients/clip-paths, merge or remove fine detail, and cut the palette to a few
flat colors. Tools like `svgo` / `scour` help, but also prune by hand. Simpler reads
better at diagram scale.

### 2e. Match the design's stroke/fill

Edit the artwork to match the diagram's visual language: set strokes/fills to
`currentColor`, symbolic palette tokens (`fill="var(--accent-1)"`), or a shared CSS
class, and match stroke width/style to the surrounding shapes. All clipart should look
like it belongs to one set, not a ransom note of styles.

### 2f. Make ids/defs unique

Embedding puts multiple clipart (and repeated instances) into one SVG document. Rename
every `id=`, gradient/clipPath/filter/mask id, and the matching `url(#…)` reference so
they don't collide — id collisions make one clipart's fill/clip silently apply to
another.

### 2g. Record license + attribution

Save the source URL and license for each item (e.g. a `clipart/CREDITS.md`). Required
for CC-BY / Wikimedia; good practice everywhere.

> A search subagent can do 2a, 2d–2g per item in parallel, but **2b/2c (scale)** need a
> final single pass that compares all the processed clipart together.

---

## Part 3: Integration with AIL

### File-Based SVG Templates

Import downloaded clipart as file-based templates:

```
template "server_icon" from "clipart/server.svg"
template "person_icon" from "clipart/person.svg"
template "terminal_icon" from "clipart/terminal.svg"
```

### Sizing and Positioning

Clipart SVGs come in various sizes. Always specify explicit dimensions:

```
server_icon my_server [width: 80, height: 80]
person_icon user1 [width: 50, height: 70]
```

### Wrapping with Anchors

For clipart that needs connections, wrap in an inline template with anchors:

```
template "server" (name: "Server") {
    template "server_svg" from "clipart/server.svg"
    server_svg icon [width: 70, height: 70]
    text name label [font_size: 11]
    constrain label.center_x = icon.center_x
    constrain label.top = icon.bottom + 6

    anchor top_conn [position: icon.top - 4, direction: up]
    anchor bottom_conn [position: label.bottom + 4, direction: down]
    anchor left_conn [position: icon.left - 4, direction: left]
    anchor right_conn [position: icon.right + 4, direction: right]
}
```

### Self-Contained Output

When sharing SVGs, use `--image-href base64` to embed all raster/file references
directly in the SVG. For SVG-only clipart, the content is embedded by default.

---

## Part 4: Quality Checks

1. **Render test** — After importing each clipart, render the diagram and verify
   the icon appears at the right size and position
2. **Style consistency** — All clipart should use a similar visual style
   (all line art, or all flat design — don't mix)
3. **Color override** — If clipart has hardcoded colors that clash, consider
   adding `fill:` or using CSS class overrides
4. **Size harmony** — Icons should be proportional to other diagram elements.
   A 200px icon next to 50px text boxes looks wrong.
