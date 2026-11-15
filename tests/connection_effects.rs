//! Emphasis effects aimed at a connection act on the line.
//!
//! `highlight`, `flash`, `pulse`, `ping`, `shake`, `nudge` and `loop x [pulse]`
//! on a named arrow used to compile to hooks (`.aihl-<id>`, `.kf-<id>`, ...)
//! that no node carried: both renderers played them on nothing and lint stayed
//! silent. Every effect now has a node along the line, in the JS player's
//! output and in the pure-CSS film alike.

use agent_illustrator::{render_with_config, RenderConfig};

const SOURCE: &str = r#"
rect a [label: "A"]
rect b [label: "B"]
constrain b.left = a.right + 120
constrain b.center_y = a.center_y
a -> b as link [label: "req"]
keyframe "one" { highlight link }
keyframe "two" { flash link   pulse link }
keyframe "three" { ping link   shake link }
keyframe "four" { nudge link [direction: up]   loop link [pulse, period: 1.2] }
"#;

fn render(source: &str, css: bool) -> String {
    let mut config = RenderConfig::new();
    if css {
        config.animate_css = true;
    } else {
        config.animate = true;
    }
    render_with_config(source, config).unwrap_or_else(|e| panic!("{e}"))
}

/// The node carrying class `<prefix>-<scope>link`, if any.
fn node_with<'a>(svg: &'a str, prefix: &str) -> Option<&'a str> {
    svg.split('<').find(|tag| {
        tag.split("class=\"")
            .nth(1)
            .and_then(|c| c.split('"').next())
            .is_some_and(|c| c.split(' ').any(|k| k.starts_with(prefix) && k.ends_with("-link")))
    })
}

#[test]
fn every_effect_on_a_connection_has_a_node_in_both_renderers() {
    for css in [false, true] {
        let svg = render(SOURCE, css);
        let which = if css { "--animate-css" } else { "--animate" };
        for prefix in ["aihl-", "aiflash-", "aipulse-", "aiping-"] {
            let tag = node_with(&svg, prefix).unwrap_or_else(|| panic!("{which}: no {prefix} node for the connection"));
            assert!(tag.contains("opacity:0"), "{which}: {prefix} must start hidden: {tag}");
        }
        // shake / nudge move the line: it (and its label) sit in a `kf-` wrapper.
        let wraps = svg
            .split("<g class=\"")
            .skip(1)
            .filter(|t| t.split('"').next().is_some_and(|c| c.starts_with("kf-") && c.ends_with("-link")))
            .count();
        assert_eq!(wraps, 2, "{which}: one wrapper for the line, one for its label");
    }
}

#[test]
fn the_css_film_animates_the_connection_nodes() {
    let svg = render(SOURCE, true);
    for prefix in [".aihl-", ".aiflash-", ".aipulse-", ".aiping-", ".kf-"] {
        assert!(
            svg.lines().any(|l| l.starts_with(prefix) && l.contains("-link {") && l.contains("animation:")),
            "no animation rule for {prefix}…link"
        );
    }
}

#[test]
fn the_player_manifest_targets_the_connection_nodes() {
    let svg = render(SOURCE, false);
    for prefix in [".aihl-", ".aiflash-", ".aipulse-", ".aiping-"] {
        assert!(svg.contains(prefix), "manifest has no {prefix} channel");
    }
}

#[test]
fn the_line_effect_strokes_follow_the_line() {
    let svg = render(SOURCE, true);
    let line = node_with(&svg, "aid-").expect("the connection path");
    let d = line.split("d=\"").nth(1).unwrap().split('"').next().unwrap();
    for prefix in ["aihl-", "aiflash-", "aipulse-"] {
        let tag = node_with(&svg, prefix).unwrap();
        assert!(tag.contains(&format!("d=\"{d}\"")), "{prefix} stroke is not along the line: {tag}");
    }
}

#[test]
fn accent_on_a_connection_says_what_to_use_instead() {
    let src = r#"
rect a
rect b
constrain b.left = a.right + 120
a -> b as link
keyframe "one" { accent link }
"#;
    let mut config = RenderConfig::new();
    config.animate = true;
    let err = render_with_config(src, config).expect_err("accent on a line is an error").to_string();
    assert!(err.contains("link"), "{err}");
    assert!(err.contains("highlight") || err.contains("not an element"), "{err}");
}
