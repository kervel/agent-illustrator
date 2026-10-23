//! A label wider than the gap has to cross a line. `label_mask: N` cuts every
//! connection-layer line away within N of the label's box, so the text reads
//! clean without painting a background colour over the drawing.

use agent_illustrator::{render_with_config, render_with_lint, RenderConfig};

const BASE: &str = r#"
rect a [size: 10]
rect b [size: 10]
rect rule_hi [size: 4]
rect rule_lo [size: 4]
constrain b.center_x = a.center_x + 200
constrain rule_hi.center_x = a.center_x + 100
constrain rule_lo.center_x = a.center_x + 100
constrain rule_hi.center_y = a.center_y - 60
constrain rule_lo.center_y = a.center_y + 60
rule_hi -- rule_lo
"#;

fn render(extra: &str) -> String {
    render_with_config(&format!("{BASE}{extra}"), RenderConfig::new()).expect("should render")
}

#[test]
fn a_masked_label_cuts_the_lines_behind_it() {
    let svg = render(r#"a -> b [label: "crosses the rule", label_offset: 0, label_mask: 4]"#);
    assert!(svg.contains("<mask id=\"aimlabel-"), "no mask emitted:\n{svg}");
    assert!(svg.contains("mask=\"url(#aimlabel-"), "connections not masked:\n{svg}");
}

#[test]
fn without_label_mask_nothing_is_cut() {
    let svg = render(r#"a -> b [label: "crosses the rule", label_offset: 0]"#);
    assert!(!svg.contains("aimlabel-"), "unexpected mask:\n{svg}");
}

#[test]
fn a_label_mask_silences_the_label_over_connection_warning() {
    let lint = |extra: &str| {
        let (_svg, warnings) =
            render_with_lint(&format!("{BASE}{extra}"), RenderConfig::new().with_lint(true)).expect("renders");
        warnings.iter().any(|w| w.message.contains("overlaps connection"))
    };
    assert!(lint(r#"a -> b [label: "crosses the rule", label_offset: 0]"#));
    assert!(!lint(r#"a -> b [label: "crosses the rule", label_offset: 0, label_mask: 4]"#));
}
