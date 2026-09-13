//! Being wrapped by a `contains` box does not take an element out of the
//! diagram. The exemption is for the container sitting over its own contents,
//! not for those contents colliding with everything else.

use agent_illustrator::{render_with_lint, RenderConfig};

fn lint(source: &str) -> Vec<agent_illustrator::LintWarning> {
    let (_svg, warnings) =
        render_with_lint(source, RenderConfig::new().with_lint(true)).expect("should render");
    warnings
}

fn messages(warnings: &[agent_illustrator::LintWarning]) -> Vec<String> {
    warnings.iter().map(|w| w.message.clone()).collect()
}

/// Two identical collisions; only one of the pairs is wrapped by a `contains`.
const ASYMMETRIC: &str = r#"
rect kader [fill: accent-light, stroke: accent-dark, stroke_width: 2]

grid g [cols: 3, rows: 3, gap: 0, cell_width: 46, cell_height: 52] {
    rect boven     [at: [0,0], label: "A", fill: none, stroke: none]
    rect midden_a  [at: [1,0], label: "B", fill: none, stroke: none]
    rect midden_b  [at: [1,1], label: "C", fill: none, stroke: none]
    rect onder     [at: [2,1], label: "D", fill: none, stroke: none]
}
constrain kader contains midden_a, midden_b [padding: 4]

rect streep_boven [height: 3, fill: foreground-1, stroke: none]
constrain streep_boven.left = g.cell(0,0).left
constrain streep_boven.right = g.cell(0,1).right
constrain streep_boven.top = g.cell(0,0).bottom

rect streep_onder [height: 3, fill: foreground-1, stroke: none]
constrain streep_onder.left = g.cell(1,0).left
constrain streep_onder.right = g.cell(1,1).right
constrain streep_onder.top = g.cell(1,0).bottom
"#;

#[test]
fn wrapped_contents_still_collide_with_the_outside_world() {
    let found = messages(&lint(ASYMMETRIC));
    for pair in [
        ("streep_boven", "midden_a"),
        ("streep_boven", "midden_b"),
        ("streep_onder", "onder"),
    ] {
        assert!(
            found
                .iter()
                .any(|m| m.contains(pair.0) && m.contains(pair.1)),
            "identical geometry must be reported whether or not it is wrapped; \
             missing {:?} in {:?}",
            pair,
            found
        );
    }
}

#[test]
fn a_solid_container_crossing_something_outside_it_is_reported() {
    let found = messages(&lint(ASYMMETRIC));
    assert!(
        found
            .iter()
            .any(|m| m.contains("kader") && m.contains("streep")),
        "a solid highlight box cutting through a rule should be reported: {:?}",
        found
    );
}

#[test]
fn a_container_over_its_own_contents_stays_exempt() {
    let found = messages(&lint(ASYMMETRIC));
    assert!(
        !found
            .iter()
            .any(|m| m.contains("kader") && (m.contains("midden_a") || m.contains("midden_b"))),
        "wrapping its contents is what contains is for: {:?}",
        found
    );
}

#[test]
fn the_documented_see_through_zone_stays_quiet() {
    // The idiom the docs give is a background zone at opacity 0.3; things
    // resting on it are not collisions, and that is decided by its opacity.
    let source = r#"
rect a [width: 60, height: 40]
rect b [width: 60, height: 40]
constrain b.x = a.right + 20
constrain b.y = a.y
rect bg [fill: accent-light, opacity: 0.3]
constrain bg contains a, b [padding: 15]
rect extra [width: 30, height: 20]
constrain extra.x = a.x + 10
constrain extra.y = a.y + 60
"#;
    let found = messages(&lint(source));
    assert!(found.is_empty(), "expected no warnings, got: {:?}", found);
}

#[test]
fn a_colour_that_names_nothing_is_reported() {
    let found = messages(&lint(r#"rect b [width: 60, height: 30, fill: geenkleur]"#));
    assert!(
        found.iter().any(|m| m.contains("geenkleur")),
        "got: {:?}",
        found
    );
}

#[test]
fn real_colours_palette_tokens_hex_and_fill_functions_are_left_alone() {
    let source = r#"
rect a [width: 40, height: 40, fill: red, stroke: none]
rect b [width: 40, height: 40, fill: accent-1, label: "x", label_fill: text-1]
rect c [width: 40, height: 40, fill: hatch(accent-1)]
rect d [width: 40, height: 40, fill: #ff0000]
constrain b.x = a.right + 10
constrain c.x = b.right + 10
constrain d.x = c.right + 10
"#;
    let found = messages(&lint(source));
    assert!(found.is_empty(), "false positives: {:?}", found);
}
