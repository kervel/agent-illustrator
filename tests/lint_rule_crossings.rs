//! A rule crossing what it rules over is not a collision.
//!
//! An 800x2 axis, a 2x180 now-marker, a 560x56 version bar: a 2px vertical
//! rule crossing a bar is what a rule is FOR, and a 16px pin centred on a 2px
//! axis is a marker on a line. On a correct, shipped 8-diagram deck this
//! produced 25 findings, all noise, which is not a signal-to-noise problem but
//! a category that cannot be left switched on.
//!
//! The cost is documented rather than hypothetical: that deck excluded the
//! category, and while excluded it hid a rule drawn through a bar's name and a
//! caption grazing a card — and, because the near-miss rule reports under
//! `overlap`, excluding it also discarded the best check in the tool.

use agent_illustrator::{render_with_lint, RenderConfig};

fn overlaps(source: &str) -> Vec<String> {
    let (_svg, w) = render_with_lint(source, RenderConfig::new().with_lint(true)).expect("renders");
    w.iter()
        .filter(|x| x.message.contains("overlap"))
        .map(|x| x.message.clone())
        .collect()
}

/// A timeline: long thin axis, thin vertical now-marker, a wide version bar.
const TIMELINE: &str = r#"
rect axis [width: 800, height: 2, fill: foreground-2]
constrain axis.left = 20
constrain axis.center_y = 200

rect v1 [width: 560, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.left = 20
constrain v1.center_y = 200

rect nowline [width: 2, height: 180, fill: foreground-1]
constrain nowline.center_x = 400
constrain nowline.center_y = 200
"#;

#[test]
fn a_rule_crossing_a_bar_is_not_reported() {
    let msgs = overlaps(TIMELINE);
    assert!(
        msgs.is_empty(),
        "a 2px rule crossing a 560px bar is what a rule is for: {msgs:?}"
    );
}

#[test]
fn a_pin_on_an_axis_is_not_reported() {
    let msgs = overlaps(
        r#"
rect axis [width: 800, height: 2, fill: foreground-2]
constrain axis.left = 20
constrain axis.center_y = 200
circle ev [size: 16, fill: accent-2, stroke: accent-dark]
constrain ev.center_x = 300
constrain ev.center_y = 200
"#,
    );
    assert!(msgs.is_empty(), "a marker on a line: {msgs:?}");
}

#[test]
fn two_ordinary_boxes_still_report() {
    // The exemption is for line-like elements, not an amnesty.
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
    assert_eq!(msgs.len(), 1, "got: {msgs:?}");
}

#[test]
fn two_rules_crossing_each_other_are_not_reported() {
    // An axis and a now-marker crossing is the same structural case.
    let msgs = overlaps(
        r#"
rect axis [width: 800, height: 2, fill: foreground-2]
constrain axis.left = 20
constrain axis.center_y = 200
rect nowline [width: 2, height: 180, fill: foreground-1]
constrain nowline.center_x = 400
constrain nowline.center_y = 200
"#,
    );
    assert!(msgs.is_empty(), "got: {msgs:?}");
}

#[test]
fn a_merely_oblong_box_is_not_treated_as_a_rule() {
    // A 3:1 card is not a rule. The threshold has to be well clear of
    // ordinary shapes or this becomes the blanket exemption it must not be.
    let msgs = overlaps(
        r#"
rect card [width: 180, height: 60, fill: accent-light, stroke: accent-dark]
constrain card.center_x = 100
constrain card.center_y = 100
rect other [width: 180, height: 60, fill: secondary-light, stroke: secondary-dark]
constrain other.center_x = 200
constrain other.center_y = 100
"#,
    );
    assert_eq!(msgs.len(), 1, "a 3:1 box is an ordinary shape: {msgs:?}");
}

#[test]
fn the_near_miss_rule_still_fires_around_rules() {
    // The reason this matters: text grazing a rule is the defect, and it
    // reports under the same category that was being switched off.
    let (_svg, w) = render_with_lint(
        r#"
rect axis [width: 400, height: 2, fill: foreground-1]
constrain axis.center_x = 200
constrain axis.center_y = 200
text "confirmed_from" note [font_size: 12]
constrain note.center_x = 200
constrain note.bottom = axis.top
"#,
        RenderConfig::new().with_lint(true),
    )
    .expect("renders");
    assert!(
        w.iter().any(|x| x.message.contains("grazes")),
        "the near-miss rule must survive the crossing exemption: {:?}",
        w.iter().map(|x| &x.message).collect::<Vec<_>>()
    );
}

#[test]
fn text_actually_under_a_rule_is_still_reported() {
    // A rule drawn THROUGH a bar's name is the defect the deck lost while the
    // category was excluded. Line-like or not, text must not be exempt.
    let (_svg, w) = render_with_lint(
        r#"
rect bar [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain bar.center_x = 200
constrain bar.center_y = 100
text "V2" name [font_size: 16]
constrain name.center_x = 200
constrain name.center_y = 100
rect rule [width: 2, height: 180, fill: foreground-1]
constrain rule.center_x = 200
constrain rule.center_y = 100
"#,
        RenderConfig::new().with_lint(true),
    )
    .expect("renders");
    assert!(
        w.iter()
            .any(|x| x.message.contains("name") && x.message.contains("rule")),
        "a rule through a label must still be reported: {:?}",
        w.iter().map(|x| &x.message).collect::<Vec<_>>()
    );
}

// ── the same geometry, seen by the connection rule ────────────────

fn connection_crossings(source: &str) -> Vec<String> {
    let (_svg, w) = render_with_lint(source, RenderConfig::new().with_lint(true)).expect("renders");
    w.iter()
        .filter(|x| x.message.contains("crosses element") || x.message.contains("overlaps element"))
        .map(|x| x.message.clone())
        .collect()
}

#[test]
fn a_rule_crossing_a_connection_is_not_reported() {
    // The overlap rule stopped reporting a rule crossing a bar; the connection
    // rule still reported the same geometry for the same reason. A guide rule
    // passing over a connector is as ordinary as one passing over a bar — more
    // so, because connectors take up more of the canvas.
    let msgs = connection_crossings(
        r#"
rect a [width: 80, height: 40, fill: accent-light, stroke: accent-dark]
constrain a.center_x = 100
constrain a.center_y = 100
rect b [width: 80, height: 40, fill: accent-light, stroke: accent-dark]
constrain b.center_x = 500
constrain b.center_y = 100
a -> b [routing: direct]
rect nowline [width: 2, height: 200, fill: foreground-1]
constrain nowline.center_x = 300
constrain nowline.center_y = 100
"#,
    );
    assert!(
        msgs.is_empty(),
        "a 2px guide rule crossing a connector is not a defect: {msgs:?}"
    );
}

#[test]
fn a_connection_crossing_an_ordinary_box_is_still_reported() {
    let msgs = connection_crossings(
        r#"
rect a [width: 80, height: 40, fill: accent-light, stroke: accent-dark]
constrain a.center_x = 100
constrain a.center_y = 100
rect b [width: 80, height: 40, fill: accent-light, stroke: accent-dark]
constrain b.center_x = 500
constrain b.center_y = 100
a -> b [routing: direct]
rect blocker [width: 90, height: 90, fill: secondary-light, stroke: secondary-dark]
constrain blocker.center_x = 300
constrain blocker.center_y = 100
"#,
    );
    assert!(
        !msgs.is_empty(),
        "a connector routed straight through a box is still a defect"
    );
}
