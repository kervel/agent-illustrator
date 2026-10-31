//! `from_end` / `to_end`: line-end styles on any connection.

use agent_illustrator::render;

fn src(conn: &str) -> String {
    format!("row r [gap: 120] {{\n  rect a [width: 60, height: 30]\n  rect b [width: 60, height: 30]\n}}\n{conn}\n")
}

#[test]
fn every_end_style_renders_its_marker() {
    for (word, id) in [
        ("arrow", "ai-arrow"),
        ("open", "ai-end-open"),
        ("triangle", "ai-end-triangle"),
        ("diamond", "ai-end-diamond"),
        ("diamond-filled", "ai-end-diamond-filled"),
        ("crow", "ai-end-crow"),
        ("circle", "ai-end-circle"),
    ] {
        let svg = render(&src(&format!("a -- b [to_end: {word}]"))).unwrap_or_else(|e| panic!("{word}: {e}"));
        assert!(svg.contains(&format!(r#"<marker id="{id}""#)), "{word}: marker defined\n{svg}");
        assert!(svg.contains(&format!(r#"marker-end="url(#{id})""#)), "{word}: marker-end used");
        assert!(!svg.contains("marker-start"), "{word}: only the to end is decorated");
    }
}

#[test]
fn none_removes_the_operators_head() {
    let svg = render(&src("a -> b [to_end: none]")).unwrap();
    assert!(!svg.contains("marker-end"));
    assert!(!svg.contains("<marker"));
}

#[test]
fn from_end_decorates_the_start_and_operators_still_imply_defaults() {
    let svg = render(&src("a -- b [from_end: diamond-filled]")).unwrap();
    assert!(svg.contains(r#"marker-start="url(#ai-end-diamond-filled)""#));
    assert!(!svg.contains("marker-end"));
    // `<-` puts its arrow on the from end; `<->` on both.
    let back = render(&src("a <- b")).unwrap();
    assert!(back.contains(r#"marker-start="url(#ai-arrow)""#) && !back.contains("marker-end"));
    let both = render(&src("a <-> b")).unwrap();
    assert!(both.contains("marker-start") && both.contains("marker-end"));
}

#[test]
fn ends_override_the_operator_and_combine_with_dashes() {
    let svg = render(&src("a -> b [to_end: triangle, from_end: circle, stroke_dasharray: dashed]")).unwrap();
    assert!(svg.contains(r#"marker-end="url(#ai-end-triangle)""#));
    assert!(svg.contains(r#"marker-start="url(#ai-end-circle)""#));
    assert!(svg.contains(r#"stroke-dasharray="8,4""#));
    assert!(!svg.contains("url(#ai-arrow)"), "the operator's arrow is replaced");
}

#[test]
fn ends_follow_the_line_in_every_routing_mode() {
    for routing in ["direct", "orthogonal", "curved"] {
        let svg = render(&src(&format!("a -> b [to_end: triangle, routing: {routing}]"))).unwrap();
        assert!(svg.contains(r#"orient="auto-start-reverse""#), "{routing}");
        assert!(svg.contains(r#"marker-end="url(#ai-end-triangle)""#), "{routing}");
    }
}

#[test]
fn hollow_ends_take_the_surface_colour_and_the_lines_stroke() {
    let svg = render(&src("a -- b [to_end: triangle]")).unwrap();
    assert!(svg.contains("var(--role-surface"), "hollow fill follows the stylesheet's surface");
    assert!(svg.contains(r#"stroke="context-stroke""#), "outline takes the connection's stroke");
}

#[test]
fn the_line_stops_short_so_the_shape_lands_on_the_anchor() {
    let plain = render(&src("a -- b")).unwrap();
    let tri = render(&src("a -- b [to_end: triangle]")).unwrap();
    let d = |svg: &str| {
        let i = svg.find(r#"class="ai-connection"#).expect("connection path");
        let rest = &svg[i..];
        let j = rest.find(" d=\"").unwrap() + 4;
        rest[j..j + rest[j..].find('"').unwrap()].to_string()
    };
    let last_x = |d: &str| d.split_whitespace().rev().nth(1).unwrap().trim_start_matches('L').parse::<f64>().unwrap();
    assert!(last_x(&d(&tri)) < last_x(&d(&plain)), "{} vs {}", d(&tri), d(&plain));
}

#[test]
fn unknown_end_lists_the_valid_ones() {
    let err = render(&src("a -> b [to_end: zigzag]")).unwrap_err().to_string();
    assert!(err.contains("zigzag") && err.contains("to_end"), "{err}");
    for w in ["arrow", "open", "triangle", "diamond", "diamond-filled", "crow", "circle", "none"] {
        assert!(err.contains(w), "{w} listed in: {err}");
    }
}

#[test]
fn ends_survive_keyframes_and_the_css_animation() {
    let anim = "row r [gap: 120] {\n  rect a [width: 60, height: 30]\n  rect b [width: 60, height: 30]\n}\na -- b as link [to_end: diamond-filled, from_end: crow]\nkeyframe \"one\" { }\nkeyframe \"two\" { hide link }\n";
    for (animate, animate_css) in [(false, false), (true, false), (false, true)] {
        let mut config = agent_illustrator::RenderConfig::new();
        config.animate = animate;
        config.animate_css = animate_css;
        let svg = agent_illustrator::render_with_config(anim, config).unwrap_or_else(|e| panic!("{e}"));
        assert!(svg.contains("ai-end-diamond-filled") && svg.contains("ai-end-crow"), "animate={animate} css={animate_css}");
    }
}
