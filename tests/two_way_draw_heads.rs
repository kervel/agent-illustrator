//! A two-way connector (`<->`) revealed by `draw` keeps BOTH heads.
//!
//! A drawn line is masked by a stroke along its path, about 1.6 stroke widths
//! wide. The end head is its own node, but the start head is a marker on the
//! masked path: it sits behind the line's first point and is wider than the
//! mask, so the mask cut it away for good, in the JS player and the pure-CSS
//! loop alike. The mask now opens a patch over the start.

use agent_illustrator::{render_with_config, RenderConfig};

const SRC: &str = r#"
row r [gap: 120] {
    rect a [label: "A"]
    rect b [label: "B"]
}
a <-> b as link [appears: later, stroke_width: 2]
keyframe "one" { show link [enter: draw] }
"#;

fn render(css: bool) -> String {
    let mut c = RenderConfig::new();
    if css {
        c.animate_css = true;
    } else {
        c.animate = true;
    }
    render_with_config(SRC, c).expect("should render")
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

fn first_point(d: &str) -> (f64, f64) {
    let mut it = d.trim_start_matches('M').split([' ', 'L', 'C']).filter(|s| !s.is_empty());
    (it.next().unwrap().parse().unwrap(), it.next().unwrap().parse().unwrap())
}

#[test]
fn a_drawn_two_way_line_keeps_its_start_head() {
    for css in [false, true] {
        let svg = render(css);
        // The line itself: masked, with the start marker on it.
        let line = svg
            .split('<')
            .find(|t| t.starts_with("path") && t.contains("ai-connection") && t.contains("mask=\"url(#aim-"))
            .unwrap_or_else(|| panic!("css={css}: no masked connection:\n{svg}"));
        assert!(line.contains("marker-start="), "css={css}: start marker gone: {line}");
        let start = first_point(attr(line, "d").unwrap());

        let mask = &svg[svg.find("<mask id=\"aim-").expect("draw mask")..];
        let mask = &mask[..mask.find("</mask>").unwrap()];
        // Some stroke in the mask, starting at the line's first point, wide
        // enough to uncover the start head (an arrow: 4 stroke widths across,
        // 3.6 behind the point; other end shapes: 6.5).
        let covers = mask.split('<').filter(|t| t.starts_with("path")).any(|p| {
            let w: f64 = attr(p, "stroke-width").unwrap().parse().unwrap();
            let (x, y) = first_point(attr(p, "d").unwrap());
            (x - start.0).abs() < 0.01 && (y - start.1).abs() < 0.01 && w >= 2.0 * 6.5 * 2.0
        });
        assert!(covers, "css={css}: the mask clips the start head away:\n{mask}");
        // ...and it is driven by the same draw channel as the line's mask,
        // but is not a route (`aid-`), so a morph never stretches it.
        let patch = mask.split('<').filter(|t| t.starts_with("path")).nth(1).expect("start patch");
        assert!(patch.contains("class=\"aimask-"), "css={css}: {patch}");
        assert!(!patch.contains("aid-"), "css={css}: {patch}");
    }
}

#[test]
fn a_one_way_drawn_line_has_no_start_patch() {
    let svg = render_with_config(&SRC.replace("<->", "->"), {
        let mut c = RenderConfig::new();
        c.animate = true;
        c
    })
    .unwrap();
    let mask = &svg[svg.find("<mask id=\"aim-").unwrap()..];
    let mask = &mask[..mask.find("</mask>").unwrap()];
    assert_eq!(mask.matches("<path").count(), 1, "{mask}");
}
