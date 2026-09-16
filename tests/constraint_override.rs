//! Restating a constraint must mean the same thing whatever is on its right.
//!
//! Before this, a literal RHS was added at REQUIRED strength and a
//! cross-reference at STRONG. Two REQUIRED equalities with different values
//! are unsatisfiable, so kasuari refused; a STRONG one is breakable, so it
//! quietly lost. Same authored intent, two opposite behaviours, decided by
//! whether the right-hand side happened to be a number.

use agent_illustrator::{render, render_with_lint, RenderConfig};

fn x_of(svg: &str, id: &str) -> f64 {
    let marker = format!(r#"<rect id="{id}""#);
    let start = svg.find(&marker).expect("element rendered");
    let rest = &svg[start..];
    let xi = rest.find(r#" x=""#).expect("x attr") + 4;
    let tail = &rest[xi..];
    let end = tail.find('"').unwrap();
    tail[..end].parse().unwrap()
}

#[test]
fn a_literal_can_override_an_earlier_literal() {
    // The reported failure, reduced. Was: Unsatisfiable constraints.
    let svg = render(
        r#"
rect a [width: 20, height: 20]
constrain a.center_x = 100
constrain a.center_x = 300
"#,
    )
    .expect("later constraint wins instead of conflicting");
    assert_eq!(x_of(&svg, "a"), 290.0, "centre 300 minus half of 20");
}

#[test]
fn a_literal_can_override_a_cross_reference() {
    // This already worked; it must keep working.
    let svg = render(
        r#"
rect ev [width: 20, height: 20]
constrain ev.center_x = 200
rect lab [width: 60, height: 20]
constrain lab.center_x = ev.center_x
constrain lab.center_x = 545
"#,
    )
    .expect("renders");
    assert_eq!(x_of(&svg, "lab"), 515.0);
}

#[test]
fn a_cross_reference_can_override_a_literal() {
    // The mirror case, which used to work only by accident of strength.
    let svg = render(
        r#"
rect ev [width: 20, height: 20]
constrain ev.center_x = 200
rect lab [width: 60, height: 20]
constrain lab.center_x = 545
constrain lab.center_x = ev.center_x
"#,
    )
    .expect("renders");
    assert_eq!(x_of(&svg, "lab"), 170.0, "should follow ev to 200");
}

#[test]
fn the_reported_case_renders() {
    // friction.txt P1-4, the reporter's corrected repro verbatim.
    render(
        r#"
circle ev [size: 16]
constrain ev.center_x = 620
constrain ev.center_y = 250
text "E — go-live" ev_label [font_size: 13]
constrain ev_label.center_x = 620
constrain ev_label.center_y = 300
constrain ev_label.center_x = 545
"#,
    )
    .expect("should render, not conflict");
}

#[test]
fn an_override_is_reported_by_lint() {
    // Silent last-wins would hide a mistake as effectively as the hard error
    // did. The warning is the point, not decoration on the fix.
    let (_svg, warnings) = render_with_lint(
        r#"
rect a [width: 20, height: 20]
constrain a.center_x = 100
constrain a.center_x = 300
"#,
        RenderConfig::new().with_lint(true),
    )
    .expect("renders");
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("center_x") && w.message.contains("overrid")),
        "expected an override warning, got: {:?}",
        warnings.iter().map(|w| &w.message).collect::<Vec<_>>()
    );
}

#[test]
fn a_genuine_over_constraint_is_still_an_error() {
    // center_x against left+right are different properties that conflict
    // through derived expressions. That is a real defect and must still fail.
    let result = render(
        r#"
rect a [height: 20]
constrain a.left = 0
constrain a.right = 50
constrain a.center_x = 500
"#,
    );
    assert!(
        result.is_err(),
        "conflicting *different* properties must still be reported"
    );
}

#[test]
fn constraints_on_different_elements_do_not_override_each_other() {
    let svg = render(
        r#"
rect a [width: 20, height: 20]
rect b [width: 20, height: 20]
constrain a.center_x = 100
constrain b.center_x = 300
"#,
    )
    .expect("renders");
    assert_eq!(x_of(&svg, "a"), 90.0);
    assert_eq!(x_of(&svg, "b"), 290.0);
}

#[test]
fn different_properties_on_one_element_do_not_override_each_other() {
    let svg = render(
        r#"
rect a [width: 20, height: 20]
constrain a.center_x = 100
constrain a.center_y = 300
"#,
    )
    .expect("renders");
    assert_eq!(x_of(&svg, "a"), 90.0);
}

// ── top-level disable ─────────────────────────────────────────────

#[test]
fn a_top_level_disable_releases_a_named_constraint() {
    // The escape hatch last-wins does not cover: releasing a pin without
    // replacing it. A file composing a shared part could not do this, because
    // `disable` was keyframe-only.
    let svg = render(
        r#"
rect a [width: 20, height: 20]
constrain a.center_x = 100 as a_home
constrain a.center_y = 50
disable a_home
"#,
    )
    .expect("renders");
    assert_ne!(
        x_of(&svg, "a"),
        90.0,
        "the disabled pin should no longer hold a at 100"
    );
}

#[test]
fn a_released_pin_can_be_replaced_by_a_later_one() {
    let svg = render(
        r#"
rect a [width: 20, height: 20]
constrain a.center_x = 100 as a_home
constrain a.center_y = 50
disable a_home
constrain a.center_x = 400
"#,
    )
    .expect("renders");
    assert_eq!(x_of(&svg, "a"), 390.0);
}

#[test]
fn disabling_an_unknown_constraint_is_an_error() {
    let result = render(
        r#"
rect a [width: 20, height: 20]
constrain a.center_x = 100 as a_home
disable no_such_pin
"#,
    );
    assert!(result.is_err(), "a typo must not silently do nothing");
}
