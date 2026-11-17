//! Routing precision: sub-pixel offsets are drawn straight, lint never claims
//! "0px", labels sit on the drawn polyline, `via:` works under the default
//! (orthogonal) routing, including for an element connected to itself, and a
//! connection to a bare `point` ends exactly on it.

use agent_illustrator::{render, render_with_lint, RenderConfig};

/// The `d` attribute of the connection path named `name` (or the first one).
fn path_d(svg: &str, name: Option<&str>) -> String {
    let line = svg
        .lines()
        .find(|l| {
            l.contains("ai-connection") && l.contains(" d=\"") && name.is_none_or(|n| l.contains(&format!("-{n}\"")))
        })
        .unwrap_or_else(|| panic!("connection path in {svg}"));
    let start = line.find(" d=\"").unwrap() + 4;
    let end = line[start..].find('"').unwrap() + start;
    line[start..end].to_string()
}

/// The points of an `M x y L x y ...` path.
fn points(d: &str) -> Vec<(f64, f64)> {
    let nums: Vec<f64> = d
        .split(|c: char| c == 'M' || c == 'L' || c == ' ' || c == ',')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().unwrap_or_else(|_| panic!("number in {d}")))
        .collect();
    nums.chunks(2).map(|c| (c[0], c[1])).collect()
}

fn lint(src: &str) -> Vec<String> {
    render_with_lint(src, RenderConfig::new().with_lint(true))
        .unwrap()
        .1
        .into_iter()
        .map(|w| w.message)
        .collect()
}

const SUBPIXEL: &str = r#"
rect a [width: 129, height: 40, fill: #eeeeee]
rect b [width: 129.25, height: 40, fill: #eeeeee]
constrain a.left = 0
constrain a.top = 0
constrain b.left = 0
constrain b.top = a.bottom + 80
a.bottom -> b.top as c
"#;

#[test]
fn subpixel_offset_between_anchors_is_drawn_straight() {
    let svg = render(SUBPIXEL).unwrap();
    let pts = points(&path_d(&svg, Some("c")));
    assert_eq!(pts.len(), 2, "one straight segment, not a zig-zag: {pts:?}");
    assert_eq!(pts[0].0, pts[1].0, "exactly vertical: {pts:?}");
    assert!(lint(SUBPIXEL).iter().all(|m| !m.contains("jogs")), "{:?}", lint(SUBPIXEL));
}

#[test]
fn offset_beyond_the_epsilon_still_jogs() {
    let src = SUBPIXEL.replace("129.25", "133");
    let pts = points(&path_d(&render(&src).unwrap(), Some("c")));
    assert_eq!(pts.len(), 4, "a real 2px offset keeps its jog: {pts:?}");
}

#[test]
fn half_pixel_epsilon_for_thin_lines() {
    // stroke_width 1: epsilon 0.5px, so a 0.75px offset is a (tiny) jog,
    // and lint names its size with decimals, never "0px".
    let src = SUBPIXEL
        .replace("129.25", "130.5")
        .replace("a.bottom -> b.top as c", "a.bottom -> b.top as c [stroke_width: 1]");
    let pts = points(&path_d(&render(&src).unwrap(), Some("c")));
    assert_eq!(pts.len(), 4, "{pts:?}");
    let msgs = lint(&src);
    let jog = msgs.iter().find(|m| m.contains("jogs")).expect("reducible-bend reported");
    assert!(jog.contains("0.75px"), "real size with decimals: {jog}");
    assert!(!jog.contains(" 0px"), "{jog}");
    // ... with stroke_width 2 the same offset is within the epsilon (1px).
    let thick = src.replace("stroke_width: 1", "stroke_width: 2");
    assert_eq!(points(&path_d(&render(&thick).unwrap(), Some("c"))).len(), 2);
}

#[test]
fn alignment_lint_prints_subpixel_offsets_with_decimals() {
    let src = r#"
rect a [width: 10, height: 10]
rect b [width: 10, height: 10]
constrain a.left = 0
constrain a.top = 0
constrain b.left = 0.4
constrain b.top = 200
a -> b [routing: direct]
"#;
    let msgs = lint(src);
    for m in &msgs {
        assert!(!m.contains("off by 0px"), "{m}");
    }
    let src = src.replace("0.4", "3.25");
    let msgs = lint(&src);
    let m = msgs.iter().find(|m| m.contains("nearly vertical")).expect("alignment reported");
    assert!(m.contains("off by 3.25px"), "{m}");
}
