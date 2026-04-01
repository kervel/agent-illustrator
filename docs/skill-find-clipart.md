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

## Part 2: Integration with AIL

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

## Part 3: Quality Checks

1. **Render test** — After importing each clipart, render the diagram and verify
   the icon appears at the right size and position
2. **Style consistency** — All clipart should use a similar visual style
   (all line art, or all flat design — don't mix)
3. **Color override** — If clipart has hardcoded colors that clash, consider
   adding `fill:` or using CSS class overrides
4. **Size harmony** — Icons should be proportional to other diagram elements.
   A 200px icon next to 50px text boxes looks wrong.
