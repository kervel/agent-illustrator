//! Animated output must survive having its ids rewritten.
//!
//! The documented way to use `--animate` is to inline the SVG in a page, so
//! something can flip a class on the root. Anything that inlines several SVGs
//! into one document has to namespace their ids, or two diagrams that both
//! name an element "cs" collide.
//!
//! Show/hide already used class selectors (`.kf-cs`) and kept working. Per-frame
//! property changes used `#cs`, so an id rewrite silently detached them: the
//! diagram still stepped through its frames and merely stopped changing colour.
//! Nothing errored, and `--frame` rendering was correct because it writes
//! attributes directly — so the static path and the animated path disagreed,
//! and only the animated one is what anyone looks at.

use agent_illustrator::{render_with_config, RenderConfig};

const SOURCE: &str = r#"
rect cs [width: 100, height: 40, fill: accent-light, stroke: accent-dark, label: "cs"]
constrain cs.center_x = 100
constrain cs.center_y = 60
keyframe "draft" { }
keyframe "published" {
    transform cs [fill: secondary-light, stroke: secondary-dark,
                  stroke_dasharray: "0,0", stroke_width: 3]
}
"#;

fn animated(source: &str) -> String {
    let mut config = RenderConfig::new();
    config.animate = true;
    render_with_config(source, config).expect("renders")
}

#[test]
fn per_frame_property_rules_do_not_use_id_selectors() {
    let svg = animated(SOURCE);
    let style = svg
        .split("<style")
        .nth(1)
        .and_then(|s| s.split("</style>").next())
        .expect("a style block");
    assert!(
        !style.contains("#cs"),
        "an id selector does not survive id namespacing: {style}"
    );
}

#[test]
fn per_frame_property_rules_key_off_a_class_on_the_shape() {
    let svg = animated(SOURCE);
    assert!(svg.contains(".kfp-cs {"), "expected a class rule: {svg}");
    assert!(
        svg.contains("kfp-cs\"") || svg.contains(" kfp-cs"),
        "the class must be on the shape itself: {svg}"
    );
}

#[test]
fn a_rotated_element_is_still_selected() {
    // Rotation wraps the shape in an extra <g>, so it sits one level deeper
    // inside the keyframe group. A child selector silently misses it — which
    // is the same failure mode as the id selector, one layer in.
    let svg = animated(
        r#"
rect r [width: 100, height: 40, fill: accent-light, stroke: accent-dark, rotation: 20]
constrain r.center_x = 100
constrain r.center_y = 60
keyframe "a" { }
keyframe "b" { transform r [fill: secondary-light] }
"#,
    );
    let shape = svg
        .lines()
        .find(|l| l.contains(r#"id="r""#))
        .expect("rendered");
    assert!(shape.contains("kfp-r"), "got: {shape}");
    assert!(svg.contains(".kfp-r {"), "and a rule that selects it");
}

#[test]
fn a_group_colour_change_does_not_cascade_to_its_children() {
    // A descendant selector under the keyframe group would restyle every
    // shape inside a group whose own fill changed.
    let svg = animated(
        r#"
group box [fill: accent-light] {
    rect inner [width: 40, height: 20, fill: secondary-light, stroke: secondary-dark]
}
constrain box.center_x = 100
constrain box.center_y = 60
keyframe "a" { }
keyframe "b" { transform box [fill: accent-dark] }
"#,
    );
    let inner = svg.lines().find(|l| l.contains(r#"id="inner""#));
    if let Some(inner) = inner {
        assert!(
            !inner.contains("kfp-box"),
            "the child must not carry the group's frame class: {inner}"
        );
    }
}

#[test]
fn the_class_hook_is_actually_on_a_node() {
    // A rule only binds if some element carries the class. This is the
    // invariant that makes the whole mechanism work.
    let svg = animated(SOURCE);
    assert!(
        svg.contains(r#"class="kf-cs"#),
        "no node carries kf-cs, so the rules bind to nothing: {svg}"
    );
}

#[test]
fn every_animated_property_is_still_emitted() {
    let svg = animated(SOURCE);
    for prop in [
        "fill:",
        "stroke:",
        "stroke-dasharray:",
        "stroke-width:",
    ] {
        assert!(
            svg.contains(prop),
            "{prop} should appear in the frame CSS: {svg}"
        );
    }
}

#[test]
fn namespacing_the_ids_leaves_the_rules_attached() {
    // Simulate what an inlining filter does: rewrite every id and every
    // reference to one. The frame rules must still select the element.
    let svg = animated(SOURCE);
    let rewritten = svg
        .replace(r#"id="cs""#, r#"id="ns-cs""#)
        .replace(r##"href="#cs""##, r##"href="#ns-cs""##)
        .replace("url(#cs)", "url(#ns-cs)");

    let style = rewritten
        .split("<style")
        .nth(1)
        .and_then(|s| s.split("</style>").next())
        .expect("a style block");

    // Whatever the frame rules select must still exist in the document.
    for selector in style
        .lines()
        .filter(|l| l.contains("fill:") || l.contains("stroke:"))
    {
        if let Some(class) = selector.split('.').nth(1).and_then(|s| s.split_whitespace().next()) {
            let class = class.trim_end_matches('{').trim();
            if class.starts_with("kf-") || class.starts_with("kfp-") {
                assert!(
                    rewritten.contains(&format!("class=\"{class}"))
                        || rewritten.contains(&format!("{class} ")),
                    "rule selects .{class} but nothing carries it after namespacing"
                );
            }
        }
    }
    assert!(
        !style.contains("#cs"),
        "no id selector should remain to be broken"
    );
}

#[test]
fn a_static_frame_still_renders_the_property_directly() {
    // The static path writes attributes rather than CSS, and must keep doing
    // so — it is the path that was always correct.
    let mut config = RenderConfig::new();
    config.frame = Some("published".to_string());
    let svg = render_with_config(SOURCE, config).expect("renders");
    let line = svg
        .lines()
        .find(|l| l.contains(r#"id="cs""#))
        .expect("cs rendered");
    assert!(line.contains("secondary-light"), "got: {line}");
}

#[test]
fn a_hidden_then_shown_element_still_carries_its_property_class() {
    // Hiding is how a diagram tells a story, so most elements that change
    // colour are also revealed at some point. The class was keyed off a set
    // that deliberately excludes frame-0-hidden elements — they get their
    // wrapper from the visibility path instead — so the rule was emitted and
    // the class was not, and the selector bound to nothing.
    let svg = animated(
        r#"
rect box [width: 120, height: 40, fill: accent-1, label: "box"]
constrain box.left = 20
constrain box.center_y = 40
keyframe "start"    { hide box }
keyframe "appear"   { show box }
keyframe "recolour" { transform box [fill: secondary-1] }
"#,
    );
    let shape = svg
        .lines()
        .find(|l| l.contains(r#"id="box""#))
        .expect("box rendered");
    assert!(
        shape.contains("kfp-box"),
        "hidden-then-shown element must still carry the class: {shape}"
    );
    assert!(svg.contains(".kfp-box {"), "and the rule that selects it");
}

#[test]
fn every_property_rule_binds_to_something() {
    // The general invariant behind both this and the id-selector bug: a rule
    // that selects a class nothing carries is silent and useless. Checked
    // across the shapes a diagram is actually built from.
    let svg = animated(
        r#"
rect a [width: 60, height: 30, fill: accent-1, label: "a"]
circle b [size: 30, fill: accent-2]
rect c [width: 60, height: 30, fill: accent-1, rotation: 15]
constrain a.left = 10
constrain b.left = 100
constrain c.left = 200
constrain a.center_y = 40
constrain b.center_y = 40
constrain c.center_y = 40
keyframe "one" { hide a, b, c }
keyframe "two" { show a, b, c }
keyframe "three" {
    transform a [fill: secondary-1]
    transform b [fill: secondary-2]
    transform c [fill: secondary-1]
}
"#,
    );
    for id in ["a", "b", "c"] {
        if svg.contains(&format!(".kfp-{id} {{")) {
            assert!(
                svg.contains(&format!("kfp-{id}\"")) || svg.contains(&format!("kfp-{id} ")),
                "rule .kfp-{id} is emitted but no node carries the class:\n{svg}"
            );
        }
    }
}
