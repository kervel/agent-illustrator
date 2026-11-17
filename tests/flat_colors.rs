//! `--flat-colors`: an SVG whose colours do not depend on CSS custom
//! properties or `context-stroke`. Hosts that evaluate neither (image
//! viewers, converters) paint `fill="var(--accent-1)"` and context-stroke
//! arrowheads black.

use agent_illustrator::{render_with_config, RenderConfig};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_agent-illustrator");

const SRC: &str = r#"
col {
    rect a [label: "Primary", fill: role-primary]
    rect b [label: "A <span fill=accent-dark>marked</span> word", fill: role-surface, stroke: role-ink]
    rect h [fill: hatch(accent-1), width: 80, height: 30]
    rect g [fill: gradient(accent-light, accent-dark), width: 80, height: 30]
    text "plain words" t [fill: role-muted]
}
a -> b [stroke: role-primary]
b <-> h [stroke: status-error]
h -> g [from_end: diamond, to_end: circle]
g -> a [routing: direct]
"#;

const THEME: &str = ":root { --role-primary: #aa0066; }";

fn render(flat: bool, configure: impl Fn(&mut RenderConfig)) -> String {
    let mut c = RenderConfig::new().with_custom_css(THEME.to_string());
    c.svg.flat_colors = flat;
    configure(&mut c);
    render_with_config(SRC, c).expect("should render")
}

fn count(svg: &str, needle: &str) -> usize {
    svg.matches(needle).count()
}

fn check(flat: &str, normal: &str, what: &str) {
    assert!(normal.contains("var("), "{what}: the normal output was expected to use var()");
    assert!(!flat.contains("var("), "{what}: var() left in flat output:\n{flat}");
    assert!(!flat.contains("context-stroke"), "{what}: context-stroke left:\n{flat}");
    // Same picture: every shape, text and line still there, every line end still referenced.
    // (Markers are compared by use, not definition: flat output defines one
    // per line colour.)
    let after_defs = |s: &str| s[s.find("</defs>").map(|i| i + 7).unwrap_or(0)..].to_string();
    for tag in ["<pattern", "<linearGradient"] {
        assert_eq!(count(flat, tag), count(normal, tag), "{what}: {tag} count differs");
    }
    for tag in ["<rect", "<path", "<text", "<tspan", "<circle", "marker-start=", "marker-end="] {
        assert_eq!(count(&after_defs(flat), tag), count(&after_defs(normal), tag), "{what}: {tag} count differs");
    }
    // The theme's override wins over the default palette value.
    assert!(flat.contains("#aa0066"), "{what}: theme colour missing:\n{flat}");
    // Each line colour gets its own arrowhead, painted in that colour.
    assert!(flat.contains(r##"<marker id="ai-arrow-aa0066""##), "{what}:\n{flat}");
    assert!(flat.contains(r##"<marker id="ai-arrow-f44336""##), "{what}:\n{flat}");
    let red = &flat[flat.find(r##"<marker id="ai-arrow-f44336""##).unwrap()..];
    let red = &red[..red.find("</marker>").unwrap()];
    assert!(red.contains(r##"fill="#f44336""##), "{what}: {red}");
    // Every marker a line points at exists.
    for part in flat.split("url(#").skip(1) {
        let id = &part[..part.find(')').unwrap()];
        assert!(flat.contains(&format!("id=\"{id}\"")), "{what}: dangling url(#{id})");
    }
}

#[test]
fn flat_static_output_has_literal_colours_only() {
    check(&render(true, |_| {}), &render(false, |_| {}), "static");
}

#[test]
fn flat_animated_output_has_literal_colours_only() {
    let anim = |c: &mut RenderConfig| c.animate = true;
    check(&render(true, anim), &render(false, anim), "--animate");
    let css = |c: &mut RenderConfig| c.animate_css = true;
    check(&render(true, css), &render(false, css), "--animate-css");
}

#[test]
fn the_cli_flag_is_wired_and_documented() {
    let input = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("flat_colors_in.ail");
    std::fs::write(&input, SRC).unwrap();
    let out = Command::new(BIN).arg("--flat-colors").arg(&input).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let svg = String::from_utf8(out.stdout).unwrap();
    assert!(svg.contains("<svg") && !svg.contains("var(") && !svg.contains("context-stroke"));

    let help = Command::new(BIN).arg("--help").output().unwrap();
    assert!(String::from_utf8_lossy(&help.stdout).contains("--flat-colors"));
    let doc = Command::new(BIN).args(["--doc", "flat-colors"]).output().unwrap();
    assert!(doc.status.success(), "`--doc flat-colors` should be a topic");
    assert!(String::from_utf8_lossy(&doc.stdout).contains("--flat-colors"));
}
