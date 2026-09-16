# Friction Report Remediation — Design

**Source:** `friction.txt`, filed by an agent (session `mr944-b6`) that built an
8-diagram reveal.js deck against agent-illustrator 0.1.24.

**Scope:** all seven reported items. Three of the seven needed re-diagnosis
before they could be fixed; the corrections are recorded here because two of
them invalidate the fix the reporter proposed.

---

## Verification pass

Every claim was probed against `d648813` (main) before any design work. The
probes are in this document so a later reader can re-run them.

### The report is right about

| Item | Status |
|---|---|
| P0-2 no include/import | Confirmed. No `include` token in the lexer, no statement in the AST. |
| P1-3 `stroke_dasharray` not animatable | Confirmed. `ElementDiff` carries fill, stroke, opacity, label_fill, label and the geometry keys. No dash, no stroke width. |
| P1-5 overlap category unusable | Confirmed. `is_bare_container` exempts `Layout`/`Group` with no fill; a plain `rect [fill: none, stroke: none]` gets no exemption and collides with everything it encloses. |
| P2-7 palette incomplete | Confirmed. `docs/skill.md:389` lists foreground/accent/secondary/text and omits the `background-*` family entirely. |

### The report is wrong about

**P0-1(a) — "centre-constrained text renders off-centre, and `align: center`
does not change it."**

Half right. The default anchor for a `text` shape is `TextAnchor::Start`
(`src/renderer/svg.rs:1259`), so the glyphs do sit at the left edge of a box
sized for the longest wording. But `align` works exactly as documented:

    text "THIS DECLARED WORDING IS NEVER DISPLAYED" b [font_size: 14, align: center]
    constrain b.center_x = 300
    keyframe "idle" { transform b [label: "short one"] }

    -> <text id="b" x="300" text-anchor="middle" ...>short one</text>

Dead on the requested centre. The reporter concluded align was broken because
they tested it on a *rect's* `label:`, which fails for an unrelated reason (see
below). So this is a **bad default plus a discoverability failure**, not broken
machinery — and the fix is a default and a doc line, not new code.

**P0-1(3) — "the same left-anchoring applies to a `label:` on a shape sized by
left+right constraints."**

Wrong diagnosis, and the reporter's proposed fix would not have touched it.
The label is not mis-anchored; its position is **stale**.

    rect v1 [height: 56, label: "V1", align: center]
    constrain v1.left  = t_start.center_x     // 100
    constrain v1.right = t_now.center_x + 70  // 470

    -> <rect id="v1" x="100" width="370" .../>
       <text class="ai-label" x="140" text-anchor="middle" ...>

`text-anchor="middle"` — the anchor is correct. The box spans 100..470 (centre
285) and the label sits at 140, which is the centre of the box *before* the
solver resized it. Isolating the two solver operations:

- **Shift** — `constrain a.center_x = 500` on a fixed-size box: label follows.
  `shift_element_by_name` does `label.position.x += delta` (8 such sites across
  `engine.rs`).
- **Resize** — `constrain a.left = 100; constrain a.right = 400`: label does
  not follow. `resize_element_by_name` and `resize_element_recursive` write
  `elem.bounds.width` and nothing else. No label, no anchors.

The reporter's fix #1 was per-frame glyph anchoring. That addresses P0-1(a) and
leaves this untouched.

**P1-4 — "`constrain` override rules differ between top level and keyframes."**

Real, and the report's own repro was misquoted in a way that hides the trigger.
The line shown in `friction.txt` is a cross-reference; the line that actually
errored was a literal, from before the reporter refactored that shared part
onto tick anchors. Confirmed with the reporter (`mr944-b6`).

The trigger is **two literals on the same element+property**. Minimal case:

    rect a [width: 20, height: 20]
    constrain a.center_x = 100
    constrain a.center_x = 300
    -> Cannot satisfy a.CenterX = 300: conflicts with existing constraints

Replace the first with a cross-reference and it renders, last-wins.

Root cause, in `src/layout/solver.rs`:

- `LayoutConstraint::Fixed` — a literal — is added at `Strength::REQUIRED`
  (line 461).
- `LayoutConstraint::Equal` — a cross-reference — is added at
  `Strength::STRONG` (line 493).

Two REQUIRED equalities on the same expression with different values are
mathematically unsatisfiable, so kasuari refuses. A STRONG one is breakable, so
it silently loses to whatever comes later. Same authored intent, two opposite
behaviours, decided by whether the right-hand side is a number.

The second half of the item is independent and also real: `disable` is
keyframe-only. At top level, `disable home` is
`Parse error: Unexpected Disable`. A file composing a shared part cannot
release a pin it did not write.

**P2-6 — "`--lint` is not discoverable from `--skill`."**

Half fixed already. `--lint` is step 1 of the Self-Assessment Checklist
(`docs/skill.md:147`). What remains is that checklist items 2.1 ("No
overlapping elements or labels") and 2.4 ("All labels readable at rendered
size") still ask the agent to check by eye the two things `--lint` answers
mechanically and better.

---

## Part 1 — A label is placed from its box, not remembered

**The defect:** `LabelLayout` stores an absolute `position: Point`. That point
is computed once in `layout_shape` from the shape's pre-solver bounds. Every
later geometry change has to remember to patch it, and the resize path does
not. The eight `label.position.x += delta` sites are the shape of the bug, not
the fix.

**The change:** store the *intent* and derive the point.

```rust
/// Where a label sits relative to its element's box, and how it lines up.
/// A label's point is always derived from this and the element's current
/// bounds, never remembered across a geometry change.
#[derive(Debug, Clone)]
pub struct LabelPlacement {
    pub position: ShapeLabelPosition,   // Inside | Above | Below | Left | Right
    pub align: Option<TextAnchor>,
    pub offset: f64,
    /// Shapes whose label is not centred on the box: a Line's label rides
    /// above the stroke, a Callout's clears its pointer.
    pub nudge: (f64, f64),
}
```

`LabelLayout` gains `pub placement: LabelPlacement`, keeping `position` and
`anchor` as the derived cache the renderer already reads.

The 5-way match currently inline in `layout_shape` moves out whole:

```rust
/// The one place a label's point is decided. Pure in (bounds, placement,
/// metrics), so it can be re-run after any geometry change.
pub fn place_label(
    bounds: &BoundingBox,
    placement: &LabelPlacement,
    metrics: &TextMetrics,
) -> (Point, TextAnchor)
```

`layout_shape` calls it to seed the cache. A new `refresh_label(elem)` calls it
again whenever bounds change, and `resize_element_by_name` /
`resize_element_recursive` call `refresh_label`. The shift sites keep their
`+= delta` — it is correct, and equivalent to a refresh for a pure
translation — but the invariant is now stated in one place and testable.

`ShapeType::Line` becomes `Inside` with `nudge: (0.0, -12.0)`; `Callout`
becomes `Inside` with the pointer nudge it already computes. Both reproduce
current output exactly.

**Why not recompute every label after solving, unconditionally?** Because
connection labels and grid gutter labels are not placed from a box, and a
blanket pass would have to special-case them back out. Refreshing at the two
sites that change a box's size is the smaller claim.

**Tests:** a label centred in a solver-resized box lands on the box's centre; a
`label_position: below` label on a resized box stays under it and keeps its
`align` edge; a label on a shifted box still follows (regression guard on the
existing path); Line and Callout labels are byte-identical to before.

## Part 2 — Lint does not report collisions with things that paint nothing

**The defect:** the reporter's 8 files produced 143 overlap warnings, 2 of
which were real — a 1.4% hit rate. The category was excluded wholesale, and
that is how the second real defect (a vertical rule through a bar's name)
reached a screenshot instead of the linter. Roughly 103 of the 143 come from
two idioms where one party is invisible: an invisible full-size rect fixing
the canvas extent (~62) and invisible tick anchors on an axis (~41).

**The change:** generalise the existing exemption. `is_bare_container` already
encodes "a thing that paints nothing cannot collide"; it just restricts itself
to `Layout`/`Group`. Extend it to any element with no fill and no visible
stroke:

```rust
/// An element with no fill and no visible stroke paints nothing. It marks out
/// a region — a canvas extent, a tick anchor, a spacer — and a region cannot
/// collide with what sits in it. Two *visible* things overlapping is still a
/// finding.
fn paints_nothing(elem: &ElementLayout) -> bool {
    !has_visible_fill(elem) && !has_visible_border(elem)
}
```

`has_visible_border` already exists and handles `stroke: none`, zero width and
zero opacity. `has_visible_fill` is its counterpart: `fill: none`, absent fill
on a shape whose default is none, or `fill_opacity: 0`.

Deliberately **not** doing per-element `lint_overlap: allow` suppression, which
the report also suggested. It is a grammar addition that makes every future
false positive the author's problem to annotate; fixing the rule fixes it for
everyone, and no new syntax survives into the docs. If real cases remain after
this, suppression can be revisited with evidence.

**Tests:** the reporter's canvas idiom produces no warning; two visible
overlapping rects still do; a rule crossing a *visible* bar still does; a
`fill_opacity: 0` rect is exempt but a `fill_opacity: 0.2` zone is not.

## Part 3 — `stroke_dasharray` and `stroke_width` animate

**The defect:** "a planned thing becomes a confirmed thing" is dashed-outline →
solid-fill. It cannot be expressed, and the workaround (a hidden solid twin,
which then paints over the original's name, which then needs a duplicate name
element and a second hide list) is three elements and two hide lists for one
property.

**The change:** mechanical, five touch points each, following `stroke` exactly:

1. `ElementDiff` gains `stroke_dasharray: Option<String>` and
   `stroke_width: Option<f64>`.
2. `ElementDiff::is_empty` tests both.
3. `apply_modifiers_ordered` handles `StyleKey::StrokeDasharray` and
   `StyleKey::StrokeWidth` in pass 1 (visual).
4. `compute_frame_diffs` emits them when they differ from base.
5. The per-frame CSS emits `stroke-dasharray` / `stroke-width`.

Both are CSS-animatable properties, so the existing `.ai-shape` transition
covers them. Note `stroke-dasharray` interpolates only between patterns with
the same dash count — dashed→solid is expressed as `stroke_dasharray: "0,0"`
or by transitioning to a zero-length pattern; the docs must say so rather than
promise a smooth morph that the renderer cannot deliver.

**Tests:** a keyframe that changes dasharray emits it in that frame's CSS;
stroke_width likewise; neither appears in the diff when unchanged.

## Part 4 — `include "parts/base.ail"`

**The defect:** six scenario diagrams share one base picture. With no include,
the deck needs a shell script concatenating `parts/*.ail` plus a hand-rolled
`// base: a,b` comment directive to record which parts each scenario sits on.
A reader of a generated file cannot tell which lines are shared and which are
the scenario's own.

This is the only part of this spec that touches the grammar.

**Syntax:** one statement, top level only.

    include "parts/base.ail"

**Resolution:** relative to the directory of the *including* file, not the
process working directory, so a part can include a sibling part and a deck can
move as a unit.

**Semantics:** textual-equivalent splice at the AST level. The included file is
parsed on its own and its statements are inserted in place of the `include`.
Ordering is preserved, which matters — z-order and last-wins constraint
override both depend on statement order, and an author reasons about the
composed file top to bottom.

**Machinery:**

- Lexer: `Token::Include`.
- AST: `Statement::Include(Spanned<String>)`.
- `RenderConfig` gains `base_dir: Option<PathBuf>`. The CLI sets it from the
  input file's parent. `render(source)` with no base_dir and an `include` in
  the source is a hard error naming the reason — the library takes a string
  and genuinely cannot know what the path is relative to.
- A resolution pass between parse and `validate_colors`, walking the statement
  list and splicing depth-first.

**Failure modes, all hard errors with the offending path and span:**

- File not found, or not readable.
- Cycle — track the canonicalised path stack; report the full chain, because
  "a includes b includes a" is unreadable as "cycle detected".
- Depth over 10, as a backstop against a cycle that canonicalisation misses
  (symlinks).
- A parse error inside an included file reports *that file's* path and span,
  not the includer's. An agent that cannot tell which file failed will edit the
  wrong one.

**Not doing:** include with parameters, conditional include, glob include,
include inside a group or template body. Each is a language feature wearing an
include's clothes; the reported need is "these six files share this picture".

**Tests:** a file including a sibling renders identically to the concatenation;
relative paths resolve from the includer's directory, not the cwd; a nested
include works; a cycle names every file in the chain; a missing file names the
path and the line; a parse error inside an include names the included file; a
library call with an include and no base_dir errors clearly.

## Part 5 — Defaults and docs

**5a. Text-element alignment default.** A `text` element whose `center_x` is
constrained almost certainly wants its glyphs centred. Changing the global
default from `start` would move every existing unaligned text element, so the
rule is narrower: when a text shape has no explicit `align` **and** its box was
sized for a wording wider than the one being drawn (i.e. keyframes rewrote it,
or the box was constraint-sized), the glyphs centre in the box.

This makes the common case right and leaves plain `text "x"` with no
constraints exactly where it has always been drawn. The `--lint` `label`
category also gains a note when a text element's drawn wording is much
narrower than its box and no `align` was given — that is the P0-1(b) symptom,
and it is cheaper to report than to guess at.

**5b. Size from displayed wordings.** The reporter's P0-1(b): the declared
wording sizes the box even when every keyframe overrides it, so the box
reserves space for a string that is never drawn. Size from the union of
wordings actually displayed in some frame. Strictly shrinks boxes; no new
failure mode.

**5c. Palette.** Add the `background-*` family to the `docs/skill.md` palette
list — `background-light`, `background-dark`, `background-1`, `background-2`,
`background-3`. The error message already suggests them, which made this a
30-second fix rather than a hunt; the table should simply not have lied.

**5d. Checklist.** Replace checklist items 2.1 and 2.4 — the two eyeball checks
`--lint` performs better — with a statement that the screenshot pass is for
meaning, not collisions. Keep the rest of the visual pass; it catches things
no linter can.

## Part 6 — A later constraint wins, whatever is on its right

**The defect:** whether restating a constraint is an error or an override
depends on whether the right-hand side is a number. That is invisible from the
source, and it is why a shared part could not be re-pinned by the file
composing it.

**Rejected fix — demote `Fixed` to STRONG.** It makes literal-then-literal stop
erroring, but two STRONG equalities at equal strength leave Cassowary free to
satisfy either. This codebase has been bitten by exactly that before (see the
"Target vs Reference" and HashMap-ordering notes in the project memory): equal
strengths plus kasuari's internal hashing is how nondeterministic output got
in last time. Not doing it again.

**The change:** make last-wins explicit, before the solver sees anything.

When resolving `constrain` statements, index user-defined constraints by
`(element_id, property)`. If a later statement targets a pair an earlier one
already claimed, drop the earlier one and keep the later. The surviving
constraint keeps its current strength, so `Fixed` stays REQUIRED and the solver
keeps its ability to report genuine over-constraint.

Two properties this deliberately preserves:

- **Deterministic.** One constraint per pair reaches the solver; there is no
  tie for Cassowary to break.
- **Still catches real conflicts.** The rule matches exact `(element,
  property)` pairs only. `constrain a.center_x = 100` together with
  `constrain a.left = 0; constrain a.right = 50` still errors, because those
  are different properties that happen to conflict through derived
  expressions — which is a genuine over-constraint and worth reporting.

Overriding is silent in the solver and reported by lint: a new
`overridden-constraint` warning naming both statements and their lines, so an
author who did it by accident finds out. The report asked for exactly this
("allow last-wins at top level, with a lint warning").

**Top-level `disable`.** Accept `disable <name>` and `enable <name>` as
top-level statements, sharing the keyframe implementation. This is the escape
hatch for the case last-wins does not cover: releasing a pin without replacing
it. Grammar cost is two statement forms over tokens that already exist.

**Tests:** literal-then-literal renders, later value wins; cross-reference
followed by literal still renders, later wins (unchanged); literal followed by
cross-reference renders and the cross-reference wins (this is the case that
currently works by accident and must keep working); `center_x` against
`left`+`right` still errors; the override emits one lint warning naming both
lines; top-level `disable` releases a pin and the element holds its laid-out
position; `disable` of an unknown name is a hard error.

---

## Ordering

1, 2, 3, 5 and 6 are independent and can land in any order. 4 (`include`) is
last: it is the only grammar change of real size, the only one that touches
the CLI's file handling, and the only one whose tests need a fixture tree on
disk. 6 pairs naturally with 4 — re-pinning a shared part is only useful once
`include` exists — so 6 immediately before 4 gives the best story, but they do
not depend on each other mechanically.

## Global constraints

- No `git add -A`; stage explicit paths.
- Push to both `github` and `origin`.
- Examples re-render byte-identically unless a change is explained — renders
  are deterministic as of `e7fe8c9`, so any diff is a real diff.
- Part 1 must not change any existing example's output. Parts 2 and 5b will
  change lint output and some box sizes; those diffs get explained, not
  absorbed.
