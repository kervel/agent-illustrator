//! The alignment warning is about connections that bend for a few pixels. A
//! connection that is drawn straight has nothing to straighten, even when the
//! boxes it joins have different centres.

use agent_illustrator::{render_with_lint, RenderConfig};

fn lint(source: &str) -> Vec<String> {
    let (_svg, warnings) =
        render_with_lint(source, RenderConfig::new().with_lint(true)).expect("should render");
    warnings.iter().map(|w| w.message.clone()).collect()
}

const CARD: &str = r#"
template "card" (h: 40) {
    col box [gap: 0, padding: 0] {
        rect hd [width: 100, height: 30]
        rect body [width: 100, height: h]
    }
    anchor l_hd [position: hd.left, direction: left]
    anchor r_hd [position: hd.right, direction: right]
}
"#;

#[test]
fn a_straight_line_between_header_anchors_is_not_nearly_horizontal() {
    let source = format!(
        "{CARD}\nrow r [gap: 120, align: top] {{\n  card a [h: 40]\n  card b [h: 60]\n}}\na.r_hd -- b.l_hd\n"
    );
    let warnings = lint(&source);
    assert!(
        !warnings.iter().any(|w| w.contains("nearly horizontal")),
        "got: {warnings:?}"
    );
}
