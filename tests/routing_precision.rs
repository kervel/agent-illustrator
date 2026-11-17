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

// ── via: under the default (orthogonal) routing ────────────────────────

fn on_path(pts: &[(f64, f64)], p: (f64, f64)) -> bool {
    pts.windows(2).any(|w| {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        let within = |v: f64, a: f64, b: f64| v >= a.min(b) - 0.01 && v <= a.max(b) + 0.01;
        ((x0 - x1).abs() < 0.01 && (p.0 - x0).abs() < 0.01 && within(p.1, y0, y1))
            || ((y0 - y1).abs() < 0.01 && (p.1 - y0).abs() < 0.01 && within(p.0, x0, x1))
    })
}

fn orthogonal(pts: &[(f64, f64)]) -> bool {
    pts.windows(2).all(|w| (w[0].0 - w[1].0).abs() < 0.01 || (w[0].1 - w[1].1).abs() < 0.01)
}

const SELF_VIA: &str = r#"
rect a [width: 60, height: 40]
point p
point q
constrain a.left = 0
constrain a.top = 0
constrain p.center_x = 30
constrain p.center_y = -40
constrain q.center_x = 120
constrain q.center_y = p.center_y
a.top -> a.right as loop [via: [p, q]]
"#;

#[test]
fn self_connection_follows_via_under_default_routing() {
    let pts = points(&path_d(&render(SELF_VIA).unwrap(), Some("loop")));
    assert!(orthogonal(&pts), "right angles only: {pts:?}");
    assert_eq!(pts[0], (30.0, 0.0), "leaves a.top: {pts:?}");
    assert!(on_path(&pts, (30.0, -40.0)) && on_path(&pts, (120.0, -40.0)), "through p and q: {pts:?}");
    // Arrives at a.right from the right (the arrowhead pulls the end back).
    let (last, prev) = (pts[pts.len() - 1], pts[pts.len() - 2]);
    assert!((last.1 - 20.0).abs() < 0.01 && (prev.1 - 20.0).abs() < 0.01 && last.0 > 60.0 && prev.0 > last.0, "{pts:?}");
}

#[test]
fn self_connection_on_one_anchor_follows_via() {
    let src = SELF_VIA.replace("a.top -> a.right", "a.top -> a.top");
    let pts = points(&path_d(&render(&src).unwrap(), Some("loop")));
    assert!(pts.len() >= 4, "a loop out and back, not a zero-length line: {pts:?}");
    assert!(on_path(&pts, (30.0, -40.0)) && on_path(&pts, (120.0, -40.0)), "{pts:?}");
}

#[test]
fn via_steers_an_orthogonal_connection_between_two_elements() {
    let src = r#"
rect a [width: 40, height: 40]
rect b [width: 40, height: 40]
point w
constrain a.left = 0
constrain a.top = 0
constrain b.left = 200
constrain b.top = 0
constrain w.center_x = 100
constrain w.center_y = 120
a -> b as c [via: [w]]
"#;
    let pts = points(&path_d(&render(src).unwrap(), Some("c")));
    assert!(orthogonal(&pts), "{pts:?}");
    assert!(on_path(&pts, (100.0, 120.0)), "detours through w below: {pts:?}");
    assert_eq!(pts[0], (20.0, 40.0), "leaves a toward w (its bottom): {pts:?}");
}

#[test]
fn zero_length_connection_is_reported() {
    let src = r#"
rect a [width: 60, height: 40]
a.top -> a.top as c
"#;
    let msgs = lint(src);
    assert!(msgs.iter().any(|m| m.contains("a→a") && m.contains("via")), "{msgs:?}");
}

// ── labels sit on the drawn polyline ──────────────────────────────────

/// (x, text-anchor) of the label text whose content is `text`.
fn label_x(svg: &str, text: &str) -> (f64, String) {
    let line = svg
        .lines()
        .find(|l| l.contains("ai-label") && l.contains(&format!(">{text}<")))
        .unwrap_or_else(|| panic!("label {text} in {svg}"));
    let attr = |name: &str| {
        let s = line.find(&format!(" {name}=\"")).unwrap() + name.len() + 3;
        line[s..s + line[s..].find('"').unwrap()].to_string()
    };
    (attr("x").parse().unwrap(), attr("text-anchor"))
}

const SELF_MSG: &str = r#"
rect a [width: 20, height: 100]
point out
point back
constrain a.left = 0
constrain a.top = 0
constrain out.center_x = a.right + 40
constrain out.center_y = a.top
constrain back.center_x = out.center_x
constrain back.center_y = a.bottom
a.top -> a.bottom [routing: direct, via: [out, back], label: "validate"OFFSET]
"#;

#[test]
fn label_on_direct_via_polyline_is_offset_from_the_drawn_line() {
    // The polyline's middle is on its vertical run at x = 60.
    let (x, anchor) = label_x(&render(&SELF_MSG.replace("OFFSET", "")).unwrap(), "validate");
    assert_eq!((x, anchor.as_str()), (70.0, "start"), "default offset 10 from the line at x=60");
    let (x, _) = label_x(&render(&SELF_MSG.replace("OFFSET", ", label_offset: 6")).unwrap(), "validate");
    assert_eq!(x, 66.0, "label_offset counts from the drawn line");
}

#[test]
fn label_on_orthogonal_z_route_is_offset_from_its_middle_run() {
    let src = r#"
rect a [width: 40, height: 40]
rect b [width: 40, height: 40]
constrain a.left = 0
constrain a.top = 0
constrain b.left = 200
constrain b.top = 100
a.right -> b.left [label: "go"]
"#;
    // Z route: (40,20) (120,20) (120,120) (200,120): the middle is on x = 120.
    let (x, anchor) = label_x(&render(src).unwrap(), "go");
    assert_eq!(anchor, "start");
    assert_eq!(x, 130.0);
}
