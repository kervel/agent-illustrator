//! `opacity:` fades the whole element: the shape AND the words on it. It used
//! to land on the `<rect>` only, so a "dimmed" box kept a fully opaque label,
//! and a `text` element ignored it entirely.

use agent_illustrator::{render_with_config, RenderConfig};

fn render(source: &str, configure: impl FnOnce(RenderConfig) -> RenderConfig) -> String {
    render_with_config(source, configure(RenderConfig::new())).expect("should render")
}

/// The opacity in force at the line holding `needle`: the product of every
/// `opacity="..."` on that line and on the `<g>` elements still open around it.
fn effective_opacity(svg: &str, needle: &str) -> f64 {
    let mut stack: Vec<f64> = Vec::new();
    for line in svg.lines() {
        let t = line.trim();
        let own = attr(t, "opacity");
        if t.contains(needle) {
            return stack.iter().product::<f64>() * own.unwrap_or(1.0);
        }
        if t.starts_with("<g") && !t.ends_with("/>") {
            stack.push(own.unwrap_or(1.0));
            // A group opened and closed on one line contributes nothing after it.
            if t.ends_with("</g>") {
                stack.pop();
            }
        } else if t.starts_with("</g>") {
            stack.pop();
        }
    }
    panic!("{needle:?} not found in:\n{svg}");
}

fn attr(tag: &str, name: &str) -> Option<f64> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = tag[start..].find('"')? + start;
    tag[start..end].parse().ok()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn a_shapes_label_fades_with_it() {
    let svg = render(r#"rect a [label: "Hello", opacity: 0.3]"#, |c| c);
    assert!(close(effective_opacity(&svg, "<rect id=\"a\""), 0.3), "{svg}");
    assert!(close(effective_opacity(&svg, ">Hello<"), 0.3), "label must fade too:\n{svg}");
}

#[test]
fn a_text_element_takes_opacity() {
    let svg = render(r#"text "Words" t [opacity: 0.4]"#, |c| c);
    assert!(close(effective_opacity(&svg, ">Words<"), 0.4), "{svg}");
}

#[test]
fn a_single_shape_template_instance_fades_with_its_label() {
    let svg = render(
        r#"
template "tag" (says: "x") {
    rect body [label: says]
}
tag t [says: "Tagged", opacity: 0.5]
"#,
        |c| c,
    );
    assert!(close(effective_opacity(&svg, "<rect id=\"t_body\""), 0.5), "{svg}");
    assert!(close(effective_opacity(&svg, ">Tagged<"), 0.5), "{svg}");
}

#[test]
fn a_multi_part_template_instance_fades_everything() {
    let svg = render(
        r#"
template "card" {
    rect body [label: "Card"]
    text "inner" tt
}
card c [opacity: 0.5]
"#,
        |c| c,
    );
    assert!(close(effective_opacity(&svg, ">Card<"), 0.5), "{svg}");
    assert!(close(effective_opacity(&svg, ">inner<"), 0.5), "{svg}");
}

#[test]
fn opacity_is_applied_once_in_animated_output() {
    // A named shape (the motion wrapper owns its opacity) and an unnamed one
    // (wrapped by the renderer), both with labels; neither may be faded twice.
    let src = r#"
col {
    rect a [label: "Named", opacity: 0.3]
    rect [label: "Anon", opacity: 0.6]
    rect b [label: "Later", appears: later]
}
keyframe "one" { show b [enter: fade] }
"#;
    for css in [false, true] {
        let svg = render(src, |mut c| {
            if css {
                c.animate_css = true;
            } else {
                c.animate = true;
            }
            c
        });
        assert!(close(effective_opacity(&svg, ">Named<"), 0.3), "css={css}:\n{svg}");
        assert!(close(effective_opacity(&svg, "<rect id=\"a\""), 0.3), "css={css}:\n{svg}");
        assert!(close(effective_opacity(&svg, ">Anon<"), 0.6), "css={css}:\n{svg}");
        assert!(
            close(effective_opacity(&svg, "class=\"ai-shape ai-rect\""), 0.6),
            "css={css}:\n{svg}"
        );
    }
}
