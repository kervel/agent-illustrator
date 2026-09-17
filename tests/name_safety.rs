//! A name the author chose becomes part of a CSS selector. Nothing checked
//! that it survived the trip.
//!
//! `keyframe "date passes"` emitted `.frame-date passes { ... }`, which CSS
//! reads as a descendant selector, so the rule matched nothing and the frame
//! silently did nothing. No parse error, no lint, `data-frames` looked right,
//! and `--frame "date passes"` rendered correctly — so every static check
//! passed while the animated output was dead.

use agent_illustrator::{render, render_with_config, RenderConfig};

#[test]
fn a_keyframe_name_with_a_space_is_rejected() {
    let err = render(
        r#"
rect a [width: 100, height: 40, fill: accent-1]
keyframe "idle" { }
keyframe "date passes" { transform a [fill: secondary-1] }
"#,
    )
    .expect_err("a name that cannot be a CSS class must not be accepted");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("date passes"),
        "the error must name the offending keyframe: {msg}"
    );
}

#[test]
fn the_error_says_what_is_allowed() {
    let err = render(
        r#"
rect a [width: 100, height: 40, fill: accent-1]
keyframe "idle" { }
keyframe "date passes" { transform a [fill: secondary-1] }
"#,
    )
    .expect_err("rejected");
    let msg = format!("{err:?}").to_lowercase();
    assert!(
        msg.contains("letter") || msg.contains("a-z") || msg.contains("class"),
        "an author needs to know what to write instead: {msg}"
    );
}

#[test]
fn ordinary_keyframe_names_still_work() {
    for name in ["idle", "date-passes", "date_passes", "step2", "Confirmed"] {
        let src = format!(
            r#"
rect a [width: 100, height: 40, fill: accent-1]
keyframe "start" {{ }}
keyframe "{name}" {{ transform a [fill: secondary-1] }}
"#
        );
        render(&src).unwrap_or_else(|e| panic!("{name} should be accepted: {e:?}"));
    }
}

#[test]
fn other_css_breaking_characters_are_rejected() {
    for name in ["a.b", "a#b", "a>b", "a,b", "a b"] {
        let src = format!(
            r#"
rect a [width: 100, height: 40, fill: accent-1]
keyframe "start" {{ }}
keyframe "{name}" {{ transform a [fill: secondary-1] }}
"#
        );
        assert!(
            render(&src).is_err(),
            "{name:?} cannot be a CSS class and must be rejected"
        );
    }
}

// ── scoping ───────────────────────────────────────────────────────

fn animated(source: &str) -> String {
    let mut config = RenderConfig::new();
    config.animate = true;
    render_with_config(source, config).expect("renders")
}

const SLIDE: &str = r#"
rect v1 [height: 40, fill: accent-1]
constrain v1.left = 20
constrain v1.right = 580 as w
keyframe "idle" { disable w
                  constrain v1.right = 840 }
"#;

const OTHER_SLIDE: &str = r#"
rect v1 [height: 40, fill: secondary-1]
constrain v1.left = 20
constrain v1.right = 300 as w
keyframe "idle" { disable w
                  constrain v1.right = 320 }
"#;

#[test]
fn two_diagrams_with_the_same_names_do_not_share_element_classes() {
    // Both name a keyframe "idle" and an element "v1". Inlined into one page
    // — the documented way to use animated output — each one's rules matched
    // the other's elements, so one slide rendered at the other's width.
    let a = animated(SLIDE);
    let b = animated(OTHER_SLIDE);

    let classes = |svg: &str| -> Vec<String> {
        svg.match_indices("kfp-")
            .map(|(i, _)| {
                svg[i..]
                    .split(|c: char| c == '"' || c == ' ' || c == '{')
                    .next()
                    .unwrap()
                    .to_string()
            })
            .collect()
    };

    let ca = classes(&a);
    let cb = classes(&b);
    assert!(!ca.is_empty() && !cb.is_empty(), "both should emit classes");
    for x in &ca {
        assert!(
            !cb.contains(x),
            "class {x} appears in both documents; inlining them collides"
        );
    }
}

#[test]
fn a_documents_rules_still_match_its_own_elements() {
    // Scoping must not detach the rules from the thing they select — that is
    // the bug this is next to, not a fix for it.
    let svg = animated(SLIDE);
    for (i, _) in svg.match_indices(".kfp-") {
        let class: String = svg[i + 1..]
            .split(|c: char| c == ' ' || c == '{' || c == '"')
            .next()
            .unwrap()
            .to_string();
        assert!(
            svg.contains(&format!("{class}\"")) || svg.contains(&format!("{class} ")),
            "rule .{class} selects nothing in its own document"
        );
    }
}
