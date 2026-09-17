//! Text that stops just short of a visible edge reads as struck through.
//!
//! Both `overlap` and `label` ask "do these bounding boxes intersect". Text
//! whose glyph box abuts a 2px rule at zero overlap answers no, so nothing
//! fires — and on screen the rule runs through the words. It is the one
//! collision class that survived every other rule in the friction report:
//! it caught a caption resting on an axis, and a vertical rule drawn through
//! a bar's name that reached a screenshot instead of the linter.
//!
//! The threshold is deliberately small. A rule that fires on anything merely
//! *near* a line would swamp the category the way overlap was swamped, and an
//! unusable category is worse than a missing one.

use agent_illustrator::{render_with_lint, RenderConfig};

fn warnings(source: &str) -> Vec<String> {
    let (_svg, w) = render_with_lint(source, RenderConfig::new().with_lint(true)).expect("renders");
    w.iter().map(|x| x.message.clone()).collect()
}

fn near_misses(source: &str) -> Vec<String> {
    warnings(source)
        .into_iter()
        .filter(|m| m.contains("grazes") || m.contains("struck through"))
        .collect()
}

/// A caption whose box stops `gap` px above a 2px rule.
fn caption_above_rule(gap: f64) -> String {
    format!(
        r#"
rect axis [width: 400, height: 2, fill: foreground-1]
constrain axis.center_x = 200
constrain axis.center_y = 200

text "confirmed_from" note [font_size: 12]
constrain note.center_x = 200
constrain note.bottom = axis.top - {gap}
"#
    )
}

#[test]
fn text_resting_on_a_rule_is_reported() {
    // Zero gap: the glyph box touches the rule. This is the reported defect —
    // it renders as a strikethrough and no existing rule sees it.
    let msgs = near_misses(&caption_above_rule(0.0));
    assert!(
        !msgs.is_empty(),
        "text abutting a visible rule must be reported: {:?}",
        warnings(&caption_above_rule(0.0))
    );
}

#[test]
fn text_a_whisker_from_a_rule_is_reported() {
    let msgs = near_misses(&caption_above_rule(1.0));
    assert!(!msgs.is_empty(), "got: {msgs:?}");
}

#[test]
fn text_clearly_clear_of_a_rule_is_not_reported() {
    // The threshold must stay small: a rule that fires on anything merely
    // near a line is how a category becomes unusable.
    let msgs = near_misses(&caption_above_rule(12.0));
    assert!(msgs.is_empty(), "got: {msgs:?}");
}

#[test]
fn text_that_genuinely_overlaps_is_left_to_the_overlap_rule() {
    // Real intersection is already reported. Reporting it twice under two
    // names is the duplicate-reporting habit that taught authors to stop
    // reading lint output.
    let source = r#"
rect axis [width: 400, height: 2, fill: foreground-1]
constrain axis.center_x = 200
constrain axis.center_y = 200
text "confirmed_from" note [font_size: 12]
constrain note.center_x = 200
constrain note.center_y = 200
"#;
    assert!(
        near_misses(source).is_empty(),
        "an actual overlap belongs to the overlap rule alone"
    );
}

#[test]
fn text_near_an_invisible_element_is_not_reported() {
    // Consistent with the paints-nothing exemption: an invisible tick cannot
    // strike anything through.
    let source = r#"
rect tick [width: 400, height: 2, fill: none, stroke: none]
constrain tick.center_x = 200
constrain tick.center_y = 200
text "confirmed_from" note [font_size: 12]
constrain note.center_x = 200
constrain note.bottom = tick.top
"#;
    assert!(near_misses(source).is_empty(), "got: {:?}", warnings(source));
}

#[test]
fn a_box_near_a_rule_is_not_reported() {
    // The defect is about TEXT being struck through. Two shapes resting
    // against each other is ordinary layout — a bar on an axis, a chip in a
    // track — and firing on those would swamp the category.
    let source = r#"
rect axis [width: 400, height: 2, fill: foreground-1]
constrain axis.center_x = 200
constrain axis.center_y = 200
rect bar [width: 80, height: 30, fill: accent-light, stroke: accent-dark]
constrain bar.center_x = 200
constrain bar.bottom = axis.top
"#;
    assert!(near_misses(source).is_empty(), "got: {:?}", warnings(source));
}

#[test]
fn the_message_names_both_parties_and_says_what_it_looks_like() {
    let msgs = near_misses(&caption_above_rule(0.0));
    let m = msgs.first().expect("reported");
    assert!(m.contains("note"), "names the text: {m}");
    assert!(m.contains("axis"), "names the edge: {m}");
}
