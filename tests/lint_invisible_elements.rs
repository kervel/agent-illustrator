//! An element with no fill and no visible stroke paints nothing. It marks out
//! a region — a canvas extent, a tick anchor, a spacer — and a region cannot
//! collide with what sits in it.
//!
//! The friction report's 8 files produced 143 overlap warnings, 2 of which
//! were real. ~103 came from two idioms where one party is invisible, which is
//! how the whole category got excluded and how the second real defect reached
//! a screenshot instead of the linter.

use agent_illustrator::{render_with_lint, RenderConfig};

fn warnings(source: &str) -> Vec<String> {
    let (_svg, w) = render_with_lint(source, RenderConfig::new().with_lint(true)).expect("renders");
    w.iter().map(|x| x.message.clone()).collect()
}

fn overlaps(source: &str) -> Vec<String> {
    warnings(source)
        .into_iter()
        .filter(|m| m.contains("overlap"))
        .collect()
}

#[test]
fn an_invisible_canvas_rect_does_not_collide_with_its_contents() {
    let msgs = overlaps(
        r#"
rect canvas [width: 400, height: 200, fill: none, stroke: none]
constrain canvas.left = 0
constrain canvas.top = 0
rect a [width: 60, height: 40, label: "A"]
constrain a.center_x = 100
constrain a.center_y = 100
"#,
    );
    assert!(msgs.is_empty(), "got: {msgs:?}");
}

#[test]
fn invisible_tick_anchors_do_not_collide_with_the_rule_they_position() {
    // The real idiom: the ticks ARE the coordinate system and the visible rule
    // takes its whole geometry from them.
    let msgs = overlaps(
        r#"
rect t_start [width: 1, height: 1, fill: none, stroke: none]
constrain t_start.center_x = 60
constrain t_start.center_y = 250
rect t_end [width: 1, height: 1, fill: none, stroke: none]
constrain t_end.center_x = 860
constrain t_end.center_y = t_start.center_y
rect rule [height: 2, fill: foreground-2]
constrain rule.left = t_start.center_x
constrain rule.right = t_end.center_x
constrain rule.center_y = t_start.center_y
"#,
    );
    assert!(msgs.is_empty(), "got: {msgs:?}");
}

#[test]
fn two_visible_elements_still_report_an_overlap() {
    // The exemption must not be a blanket amnesty.
    let msgs = overlaps(
        r#"
rect a [width: 100, height: 60, fill: accent-light, stroke: accent-dark]
constrain a.center_x = 100
constrain a.center_y = 100
rect b [width: 100, height: 60, fill: secondary-light, stroke: secondary-dark]
constrain b.center_x = 150
constrain b.center_y = 100
"#,
    );
    assert_eq!(msgs.len(), 1, "expected exactly one overlap, got: {msgs:?}");
}

#[test]
fn a_visible_pin_on_a_visible_rule_still_reports() {
    // From the reporter's fixture: 4 of 5 overlaps involve ticks and go quiet;
    // rule-vs-pin is the deliberate survivor.
    let msgs = overlaps(
        r#"
rect t_start [width: 1, height: 1, fill: none, stroke: none]
constrain t_start.center_x = 60
constrain t_start.center_y = 250
rect t_end [width: 1, height: 1, fill: none, stroke: none]
constrain t_end.center_x = 860
constrain t_end.center_y = t_start.center_y
rect rule [height: 2, fill: foreground-2]
constrain rule.left = t_start.center_x
constrain rule.right = t_end.center_x
constrain rule.center_y = t_start.center_y
circle pin [size: 16, fill: accent-2, stroke: accent-dark, stroke_width: 2]
constrain pin.center_x = 400
constrain pin.center_y = rule.center_y
"#,
    );
    assert_eq!(
        msgs.len(),
        1,
        "only rule-vs-pin should survive, got: {msgs:?}"
    );
    assert!(msgs[0].contains("rule") && msgs[0].contains("pin"), "{msgs:?}");
}

#[test]
fn a_zero_opacity_fill_counts_as_painting_nothing() {
    let msgs = overlaps(
        r#"
rect zone [width: 400, height: 200, fill: accent-light, fill_opacity: 0, stroke: none]
constrain zone.left = 0
constrain zone.top = 0
rect a [width: 60, height: 40, label: "A"]
constrain a.center_x = 100
constrain a.center_y = 100
"#,
    );
    assert!(msgs.is_empty(), "got: {msgs:?}");
}

#[test]
fn a_translucent_zone_is_not_exempt_from_overlap() {
    // A zone you can see is a zone that can collide. Only *invisible* is exempt.
    let msgs = overlaps(
        r#"
rect zone [width: 400, height: 200, fill: accent-light, fill_opacity: 0.3, stroke: accent-dark]
constrain zone.left = 0
constrain zone.top = 0
rect a [width: 600, height: 40, fill: secondary-light, stroke: secondary-dark]
constrain a.center_x = 100
constrain a.center_y = 100
"#,
    );
    assert!(!msgs.is_empty(), "a visible zone should still report");
}

// ── the hand-placed-label rule has the same hole ──────────────────

#[test]
fn texts_inside_an_invisible_canvas_are_not_told_to_become_its_label() {
    // `is_opaque` exempts a *translucent* zone; a fill:none stroke:none rect
    // sets no opacity at all, so the guard passed and the rule fired.
    let msgs = warnings(
        r#"
rect canvas [width: 400, height: 200, fill: none, stroke: none]
constrain canvas.left = 0
constrain canvas.top = 0
text "alpha" t1 [font_size: 12]
constrain t1.center_x = 100
constrain t1.center_y = 60
text "beta" t2 [font_size: 12]
constrain t2.center_x = 100
constrain t2.center_y = 90
"#,
    );
    let labels: Vec<_> = msgs.iter().filter(|m| m.contains("sit inside")).collect();
    assert!(labels.is_empty(), "got: {labels:?}");
}

#[test]
fn texts_hand_placed_in_a_real_card_are_still_told_to_use_a_label() {
    // The negative case, from the reporter's fixture 3. A filled, stroked card
    // with genuinely hand-placed text. If this goes quiet the exemption is too
    // wide — and over-broadness is invisible from the inside.
    let msgs = warnings(
        r#"
rect card [width: 290, height: 92, fill: background-light, stroke: foreground-2, stroke_width: 2]
constrain card.left = 30
constrain card.top = 20
text "Title" card_name [font_size: 15, fill: text-1]
constrain card_name.center_x = card.center_x
constrain card_name.top = card.top + 10
text "first detail line" card_one [font_size: 11, fill: secondary-dark]
constrain card_one.center_x = card.center_x
constrain card_one.top = card_name.bottom + 8
text "second detail line" card_two [font_size: 11, fill: secondary-dark]
constrain card_two.center_x = card.center_x
constrain card_two.top = card_one.bottom + 6
"#,
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("sit inside") && m.contains("card")),
        "the hand-placed-label rule must still fire, got: {msgs:?}"
    );
}

#[test]
fn children_of_a_layout_are_also_exempt_from_an_invisible_canvas() {
    // Found by mr944-b6 running the corpus: three canvas overlaps survived,
    // all of them elements inside a `row`. The bare-region descent picks one
    // side as "the region" and walks into the other's children, and for an
    // invisible *shape* — not a Layout/Group, so `is_bare_container` is false
    // — it picked the wrong side and reported every child of the row against
    // the canvas. A fixture whose elements are all top-level cannot catch it.
    let msgs = overlaps(
        r#"
rect canvas [width: 400, height: 200, fill: none, stroke: none]
constrain canvas.left = 0
constrain canvas.top = 0
row chips [gap: 8] {
    rect m12 [width: 78, height: 26, fill: accent-light, stroke: accent-dark]
    rect m13 [width: 78, height: 26, fill: accent-light, stroke: accent-dark]
}
constrain chips.center_x = 150
constrain chips.center_y = 100
"#,
    );
    assert!(msgs.is_empty(), "got: {msgs:?}");
}

#[test]
fn two_visible_elements_inside_a_layout_still_report() {
    // The same descent must keep working for things that do paint.
    let msgs = overlaps(
        r#"
rect zone [width: 400, height: 200, fill: accent-light, stroke: accent-dark]
constrain zone.left = 0
constrain zone.top = 0
rect wide [width: 600, height: 30, fill: secondary-light, stroke: secondary-dark]
constrain wide.center_x = 100
constrain wide.center_y = 100
"#,
    );
    assert!(!msgs.is_empty(), "visible-on-visible must still report");
}
