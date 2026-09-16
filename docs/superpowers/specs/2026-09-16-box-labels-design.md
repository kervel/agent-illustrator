# Box Labels: Making "Text In / Above / Below a Box" Trivial for an Agent

Date: 2026-09-16
Status: Approved for implementation

## Problem

The single most common thing an agent draws is a box with words attached to it:
centered inside, as a title above, as a caption below. Today the language has
exactly one facility for this — `label:` — and it is weak enough that agents
routinely abandon it and hand-place `text` elements with absolute constraints.
That hand-placement is where illustrations break.

Two real failures motivating this work (user-supplied screenshots, reproduced
locally):

1. A card whose body text runs straight out through the left and right borders,
   with a second text block overlapping the text above it.
2. A card whose third line straddles the bottom border, and a caption below the
   card that is neither aligned to an edge nor clear of the box.

Both were written as `rect` + several `text` elements + `constrain
t3.center_y = 145`, where `145` was a guess.

## Evidence

### The label facility is too narrow to use

- `label:` renders **one line**, always centered inside the shape. `"a\nb"`
  emits a literal newline into `<text>`, which SVG collapses — no tspans, no
  second line. A three-line card therefore *must* become three elements plus
  three constraint pairs.
- An explicit `width:` **silently disables label auto-fit**
  (`engine.rs:1082`: the label minimum is only applied when `width.is_none()`).
  `rect card [width: 300, label: "…"]` overflows with no recourse. This is
  failure 1 above.
- There is no way to place a label outside the shape. "Under it, aligned
  right" requires the agent to pick an edge pairing (`left = left` vs
  `right = right` vs `center_x = center_x`) *and* invent `top = bottom + gap`.
  This is failure 2 above.

The correct existing idiom — a wrapper `col` plus `constrain card contains
lines [padding: 14]` — works, but costs six statements and requires knowing
that declaration order determines z-order. Too expensive to reach for, so
agents don't.

### The linter cries wolf, so agents ignore it

Reproducing failure 2 locally produces:

```
lint: overlap: elements "card" and "t2" overlap by 300x12px
lint: overlap: elements "card" and "t3" overlap by 288x11px
lint: label: label on "t2" straddles the edge of element "card"; overlaps by 300x12px
lint: label: label on "t3" straddles the edge of element "card"; overlaps by 288x11px
```

Two facts, reported four times, and the `overlap` phrasing describes text
*deliberately* placed inside a box. Worse, the **correct** idiom also warns:

```
lint: containment: element "lines" extends 0px past left edge of container "card"
```

A 0px violation, caused by float rounding. An agent that sees warnings on
correct code learns that warnings are noise.

### Text measurement is inconsistent, which makes lint untrustworthy

Eight call sites, three different constants, all measuring **bytes**:

| site | formula | effective px/char at 14px |
|---|---|---|
| `engine.rs:1034` (sizes the box) | `len * 8.0 + 20` | 8.0 |
| `lint.rs:979` (checks whether it fits) | `len * font_size * 0.6` | 8.4 |
| `types.rs:1056` (bounds for collision) | `len * 7.0` | 7.0 |
| `engine.rs:1055`, `keyframe.rs:515`, `engine.rs:1966/2105/2166`, `routing.rs:1097` | assorted | 7.0–8.4 |

Boxes are sized with 8.0 and lint-checked with 8.4, so **lint reports overflow
on text that actually fits** — a ~5% systematic false-positive bias, on top of
the 20% spread between sites.

All eight use `str::len()`, which counts UTF-8 bytes. The motivating
screenshots are full of `≤`, `—`, `·`, each 3 bytes:

```
"T ≤ now confirmed_from ≤ T < confirmed_until"   48 bytes, 44 chars
"plannedDate 2027-03-14 · confirmedDate —"      43 bytes, 40 chars
```

Any feature that leans harder on measurement (multi-line, wrapping) compounds
this. Measurement must be fixed first.

### The documentation teaches the fragile style

`docs/examples.md`'s architecture example is built from `constrain
prod_label.center_y = 80`. `examples/architecture.ail` has
`constrain postgis_label.top = 748` sitting next to `constrain
postgres.center_y = 715` — literally "label under the box, offset guessed by
hand." `examples/gallic-wars-timeline.ail` spends 12 `text` elements on four
events that are each one three-line label. Agents copy the examples far more
readily than they read the docs.

## Rejected: HTML in labels via `<foreignObject>`

Considered and rejected for two independent reasons.

**Portability.** `<foreignObject>` is the only native HTML-in-SVG mechanism,
and it renders nothing in `resvg`, `librsvg`/`rsvg-convert`, Inkscape, Figma
import, Google Slides import, and most SVG→PDF paths. The project's own docs
already tell agents that `rsvg-convert` lags on CSS variables; foreignObject
would escalate that from "use Chrome to verify" to "the consumer must be
Chrome."

**Measurement.** `<foreignObject>` requires `width`/`height` up front; the HTML
lays out *inside* the box you already sized, and the only way to learn the
resulting height is to run a browser. The layout engine is a Rust constraint
solver that needs sizes *before* placing anything, so this would mean invoking
headless Chrome mid-layout, per label, per keyframe. The thing HTML was
supposed to buy — a box that grows to fit its text — is precisely what
foreignObject cannot report.

A `<tspan>`-based inline markup subset (below) gives the same authoring feel,
renders everywhere, and keeps measurement in our hands.

## Design

Six parts. **Zero new grammar keywords** — every style key involved already
parses today.

### 0. Unify text measurement (prerequisite)

A single measurement module, used by all eight current call sites.

```rust
pub struct TextRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub scale: f64,            // 1.0 normal, 0.8 for <small>
    pub fill: Option<Color>,
}

pub struct TextMetrics { pub width: f64, pub height: f64, pub line_count: usize }

pub fn measure_runs(lines: &[Vec<TextRun>], font_size: f64) -> TextMetrics;
pub fn measure_str(text: &str, font_size: f64) -> f64;   // convenience
```

- Advance table: a static `&[(char, f64)]` of per-em advances for U+0020–U+00FF
  plus a curated set of symbols agents actually emit (`≤ ≥ ≠ — – · × ÷ → ← ↔ ∈
  … ° ± « » " " ' '`). Values from Helvetica AFM advances / 1000. Fallback
  `0.55` em for anything unlisted. No font loading, no new dependencies.
- Iterate `chars()`, never `len()`.
- Bold approximated as `× 1.05`; italic as `× 1.0`. Documented as an
  approximation.
- Line height `1.25 × font_size`.

This changes the auto-computed size of every existing auto-sized box, so all
example SVGs must be re-rendered in the same commit (the pre-commit hook
already does this). The SVG regression test checks structure, not bytes, so it
should continue to pass.

### A. `label_position:` on shapes

The key already parses (`grammar.rs:160`) and is currently honoured only for
connection labels. Extend it to shapes:

```
label_position: inside | above | below | left | right     (default: inside)
```

Cross-axis alignment comes from the existing `align: start | center | end`
(spellings `left`/`center`/`right` already accepted):

- `above` / `below` — `align` selects horizontal alignment against the shape's
  left edge / centre / right edge. Default `center`.
- `left` / `right` — `align` selects vertical alignment against the shape's top
  edge / centre / bottom edge. Default `center`.
- `inside` — unchanged from today (`align` insets from the border by
  `LABEL_INSET`).

Distance from the shape's edge comes from the existing `label_offset:` key,
defaulting to `6`.

```
rect b [label: "Cache"]                                        // centred inside
rect b [label: "Cache", align: end]                            // inside, right-aligned
rect b [label: "based_on", label_position: below, align: end]  // under it, right-aligned
rect b [label: "Zone A", label_position: above, align: start]  // title above, left-aligned
```

An outside label is included in the element's layout bounds, so:

- `row`/`col` reserve space for it and `gap:` measures from the label, not the
  border;
- the linter sees it as part of the element;
- `constrain bg contains a` surrounds `a`'s label too.

`expand_bounds_for_label` (`types.rs:1060`) already performs this union for
anchored labels; it needs extending for multi-line height and for the
`left`/`right` positions.

### B. Inline markup in `label:`

`label:` accepts a small markup subset, compiled to `<tspan>`s:

| tag | effect |
|---|---|
| `<br>`, `<br/>` | line break |
| `<b>…</b>` | bold |
| `<i>…</i>` | italic |
| `<small>…</small>` | `0.8 ×` font size |
| `<span fill=NAME>…</span>` | run colour; `NAME` is any colour the language accepts, quoted or bare |

```
rect card [label: "<b>changeset from INES</b><br>
                   temporal_mode = correction<br>
                   <small>V2 inherits its label from V1</small>"]
```

Rules:

- A literal `\n` in the source string is a synonym for `<br>`.
- Tags nest. `<b><i>x</i></b>` is valid.
- **A `<` that does not begin a recognised tag is literal text.** Labels like
  `"T < confirmed_until"` must keep working, and they appear in the motivating
  screenshots. Likewise a `</` with no matching recognised opener.
- A *recognised* tag that is malformed or unclosed (`<b>x`, `<span>x</span>`
  with no `fill=`) is a hard parse error, with the span pointing into the label
  string.
- `&lt;` `&gt;` `&amp;` are accepted and unescaped.
- Applies everywhere a label is read: shape `label:`, connection `label:`, and
  `transform … [label: …]` inside keyframes.

Rendering: one `<text>` element; one `<tspan x=… dy=…>` per line; one nested
`<tspan>` per run carrying `font-weight` / `font-style` / `font-size` / `fill`.
For vertical centring of *n* lines, the first line's offset is
`-(n-1)/2 × line_height`.

### C. An explicit `width:` wraps instead of overflowing

When a shape has both an explicit `width:` and a label:

- The label word-wraps greedily to `width − 2 × LABEL_INSET`, measured with the
  module from part 0.
- `<br>` forces a break regardless of remaining space.
- A single word longer than the line is **not** broken; it overflows and lint
  reports it. (Avoids hyphenation rules; matches existing behaviour for one
  long token.)
- If `height:` is not explicit, the shape's height grows to fit the wrapped
  lines plus padding. If `height:` is explicit, it is honoured and lint reports
  any overflow.

Failure 1 becomes unrepresentable.

### D. Linter changes

1. **Fix the 0px containment false positive** — apply a 0.5px epsilon before
   reporting a containment violation.
2. **Merge duplicate reports** — when the same element pair produces both an
   `overlap` and a `label`/straddle warning describing the same geometry, emit
   one warning.
3. **New rule, naming the fix**: a `text` element positioned inside a filled
   shape, where the text is not a `contains` target of that shape, reports
   `text "t2" sits inside rect "card" — write card [label: "…"] instead of
   positioning it`.
4. **New rule**: a label containing `<word>` where `word` is not in the
   supported subset reports `label on "x" contains unsupported markup <bold>;
   supported: <br> <b> <i> <small> <span fill=…>`.
5. Keep the overflow-at-explicit-size warning — it becomes trustworthy once
   part 0 lands, because sizing and checking use the same measurer.

The target is *fewer* warnings, each carrying the replacement syntax.

### E. Documentation

- `docs/grammar.md` — `label_position` / `label_offset` under STYLE MODIFIERS
  with the shape values; a short markup-subset block.
- `docs/skill.md` — a four-row labels table near the top, in the existing
  "do this / not that" style:

  | want | write | not |
  |---|---|---|
  | text centred in a box | `rect b [label: "x"]` | a separate `text` + constraints |
  | several lines in a box | `label: "a<br>b<br>c"` | one `text` per line |
  | a caption under a box | `[label: "x", label_position: below]` | `constrain cap.top = b.bottom + 20` |
  | a title above, left-aligned | `[label: "x", label_position: above, align: start]` | absolute `center_y` |

- `docs/examples.md` — rewrite the architecture example off absolute label
  coordinates.

### F. Adapt the shipped examples

Agents copy `examples/*.ail` more readily than they read docs, so the examples
must model the new idiom.

- **`examples/architecture.ail`** — `postgis_label` becomes
  `label_position: below` on `postgres`; the three zone labels (`app_label`,
  `async_label`, `data_label`) become `label_position: above, align: start` on
  their background rects. Removes ~6 absolute constraints and 4 `text`
  elements.
- **`examples/gallic-wars-timeline.ail`** — each event's three `text` elements
  collapse to one `label:` using `<b>`/`<br>`/`<small>`. Removes 12 `text`
  elements and their constraints.
- **`examples/card-labels.ail`** (new) — the canonical reference for this
  feature: one figure showing a label inside, above, below, left and right of a
  box, a multi-line card with mixed markup, and a wrapped fixed-width card.
  Registered in `examples/render-all.sh`.

Other examples are left alone; this is a representative subset, not a sweep.

## Backward compatibility

- A label string containing a literal `<b>` renders as bold after this change
  rather than as the characters `<b>`. Breaking, but the subset was chosen so
  that the common literal case (`<` as less-than) stays literal.
- Auto-sized boxes change size slightly everywhere because part 0 replaces the
  estimator. Example SVGs are re-rendered in the same commit.
- Every existing document without `label_position:` on a shape renders as
  before, modulo the size change above.

## Testing

- **Unit** — advance table (ASCII, Latin-1, curated symbols, unlisted
  fallback); `chars()` vs bytes on the screenshot strings; markup parser
  (nesting, `<` as literal, unclosed recognised tag errors, entity
  unescaping); greedy wrap including the unbreakable-long-word case.
- **Layout** — `label_position` in all five values × three `align` values;
  outside labels included in bounds; `contains` surrounding a label; `row`/`col`
  gap measured from the label.
- **Lint** — the reproduction of failure 2 from this session becomes a fixture
  that must produce exactly one actionable warning naming the fix; the
  `col` + `contains` idiom must produce zero warnings.
- **Regression** — all examples re-rendered; the structural SVG regression test
  must pass; `examples/card-labels.ail` gets a golden.

## Success criterion

Both motivating screenshots become unrepresentable: the first because an
explicit width wraps rather than overflows, the second because a caption below
a box is one modifier rather than two guessed constraints. No agent computes a
label offset.
