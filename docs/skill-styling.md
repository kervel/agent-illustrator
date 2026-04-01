# Styling Sub-Skill

Style AIL diagrams with CSS and inline modifiers. Read `--skill` first for general usage.

## When to Use

Use this sub-skill when a diagram needs visual polish beyond the defaults — custom
colors, drop shadows, rounded corners, font choices, transitions, or dark mode support.

---

## Part 1: Two Styling Layers

AIL has two complementary styling mechanisms:

### Layer 1: Inline Modifiers (in the .ail file)

Set per-element properties directly in square brackets:

```
rect card [fill: accent-light, stroke: none, corner_radius: 12, opacity: 0.9]
text "Title" heading [font_size: 16, fill: foreground-1]
```

**Use inline modifiers for:** structural styling that is part of the diagram's
meaning — which elements are emphasized, which are subtle, which are grouped.

### Layer 2: CSS Stylesheet (separate .css file)

Pass with `--stylesheet-css styles.css`. Injected into the SVG `<style>` block
after the default color palette variables.

```css
/* Target elements by ID */
#card { filter: drop-shadow(0 2px 6px rgba(0,0,0,0.1)); }

/* Target by class */
.ai-shape { transition: opacity 0.3s ease; }
```

**Use CSS for:** visual flourishes that don't change the diagram's structure —
shadows, transitions, font families, hover effects, animation polish.

---

## Part 2: Color System

AIL provides semantic color tokens as CSS custom properties. Always use these
instead of hardcoded hex values:

| Token family | Light variant | Base | Dark variant | Use for |
|---|---|---|---|---|
| accent | accent-light | accent-1 | accent-dark | Primary elements, key arrows |
| secondary | secondary-light | secondary-1 | secondary-dark | Supporting elements, responses |
| foreground | foreground-light | foreground-1 | foreground-dark | Text, borders |
| background | background-light | background-1 | background-dark | Canvas, zones |
| status | — | status-success/warning/error | — | State indicators |

### Contrast Rules

- Text on `accent-light` or `secondary-light` backgrounds: use `foreground-1` or darker
- Text on dark backgrounds: use `text-light` or `foreground-light`
- Never use `foreground-3` text on any colored background — it's too faint
- Labels inside shapes should contrast with the shape's fill

---

## Part 3: Modern Styling Patterns

### Borderless Cards (recommended default)

Fat borders look dated. Use fill + corner radius + optional shadow instead:

```
rect card [fill: accent-light, stroke: none, corner_radius: 10]
```

With CSS shadow:
```css
#card { filter: drop-shadow(0 2px 6px rgba(0,0,0,0.12)); }
```

### Subtle Separators

When you need to distinguish elements, prefer:
1. Different fill colors from the same family (accent-light vs secondary-light)
2. Slight opacity differences (0.85 vs 0.95)
3. Corner radius variation

Over: thick borders, outlines, or heavy strokes.

### Typography

```css
/* Clean system font stack for labels */
text {
    font-family: -apple-system, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
}

/* Monospace for code content */
.code-text {
    font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace;
}
```

To apply the monospace class to specific elements, use `css_class`:
```
text "find . -name '*.py'" cmd [font_size: 11, fill: accent-dark, css_class: code-text]
```

### Transitions (for animations)

```css
/* Smooth show/hide for keyframe animations */
svg * {
    transition: opacity 0.4s ease-in-out;
}
```

---

## Part 4: Gotchas

1. **CSS opacity overrides break keyframe hide/show.** Never set `opacity` in CSS
   on elements that are toggled by keyframes. The CSS rule overrides the inline
   `opacity="0"` that hides them. Use `transition` on opacity instead.

2. **`--stylesheet-css` adds to the default palette, not replaces it.** The default
   color tokens (accent-light, foreground-1, etc.) are always available. Your CSS
   file adds rules on top.

3. **rsvg-convert ignores CSS variables.** If you render to PNG with rsvg-convert,
   all `var(--token)` colors render as black. Use headless Chrome for accurate
   previews: `google-chrome --headless --screenshot=out.png --window-size=2400,1800 file.svg`

4. **Drop shadows on hidden elements.** `filter: drop-shadow(...)` can cause
   faint artifacts on elements that should be invisible. Apply shadows only to
   specific element IDs, not broadly to all shapes.

5. **Font rendering varies.** SVG text uses whatever fonts the viewer has installed.
   Stick to system font stacks. Don't rely on specific font metrics for layout.

6. **Inline `fill:` uses token names, CSS uses `var()`.** In AIL: `fill: accent-light`.
   In CSS: `fill: var(--accent-light)`. Don't mix them up.
