//! A caption that belongs to something follows it.
//!
//! `label_position: below` exists for a shape's own label, but a caption that
//! must change between frames had to be its own text element — and then it was
//! positioned by hand and did not move when its subject did. Almost every
//! collision in a heavily-keyframed deck was a caption re-positioned after its
//! bar moved or a neighbour arrived.
//!
//! `caption_of:` attaches one. Everything else about placement reuses the
//! modifiers a shape's own label already uses.

use agent_illustrator::{render, render_with_config, render_with_lint, RenderConfig};

fn text_x(svg: &str, id: &str) -> f64 {
    let m = format!(r#"id="{id}""#);
    let start = svg.find(&m).unwrap_or_else(|| panic!("{id} not rendered in:\n{svg}"));
    let rest = &svg[start..];
    let i = rest.find(r#" x=""#).expect("x") + 4;
    let tail = &rest[i..];
    tail[..tail.find('"').unwrap()].parse().unwrap()
}

fn text_y(svg: &str, id: &str) -> f64 {
    let m = format!(r#"id="{id}""#);
    let start = svg.find(&m).expect("rendered");
    let rest = &svg[start..];
    let i = rest.find(r#" y=""#).expect("y") + 4;
    let tail = &rest[i..];
    tail[..tail.find('"').unwrap()].parse().unwrap()
}

fn rect_attr(svg: &str, id: &str, name: &str) -> f64 {
    let m = format!(r#"id="{id}""#);
    let start = svg.find(&m).expect("rendered");
    let rest = &svg[start..];
    let i = rest.find(&format!(r#" {name}=""#)).expect("attr") + name.len() + 3;
    let tail = &rest[i..];
    tail[..tail.find('"').unwrap()].parse().unwrap()
}

#[test]
fn a_caption_sits_under_its_subject() {
    let svg = render(
        r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.center_x = 400
constrain v1.center_y = 200
text "confirmed" cap [font_size: 12, caption_of: v1, label_position: below]
"#,
    )
    .expect("renders");
    assert!(
        (text_x(&svg, "cap") - 400.0).abs() < 1.0,
        "centred on its subject, got {}",
        text_x(&svg, "cap")
    );
    assert!(
        text_y(&svg, "cap") > 228.0,
        "below the subject's bottom edge (228), got {}",
        text_y(&svg, "cap")
    );
}

#[test]
fn a_caption_follows_a_subject_the_solver_moves() {
    // The whole point: positioned by hand, it did not move when its subject
    // did, so every layout change meant re-positioning it.
    let near = render(
        r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.center_x = 400
constrain v1.center_y = 200
text "confirmed" cap [font_size: 12, caption_of: v1, label_position: below]
"#,
    )
    .expect("renders");
    let far = render(
        r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.center_x = 700
constrain v1.center_y = 200
text "confirmed" cap [font_size: 12, caption_of: v1, label_position: below]
"#,
    )
    .expect("renders");
    assert!(
        (text_x(&far, "cap") - text_x(&near, "cap") - 300.0).abs() < 1.0,
        "the caption should travel with its subject"
    );
}

#[test]
fn a_caption_follows_a_subject_the_solver_resizes() {
    let svg = render(
        r#"
rect v1 [height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.left = 100
constrain v1.right = 700
text "confirmed" cap [font_size: 12, caption_of: v1, label_position: below]
"#,
    )
    .expect("renders");
    assert!(
        (text_x(&svg, "cap") - 400.0).abs() < 1.0,
        "centred on the SOLVED box (100..700), got {}",
        text_x(&svg, "cap")
    );
}

#[test]
fn align_picks_the_edge() {
    let svg = render(
        r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.left = 100
constrain v1.center_y = 200
text "confirmed" cap [font_size: 12, caption_of: v1, label_position: below, align: end]
"#,
    )
    .expect("renders");
    // End-aligned: the caption's anchor sits on the subject's right edge, 400.
    assert!(
        (text_x(&svg, "cap") - 400.0).abs() < 1.0,
        "end-aligned to the right edge, got {}",
        text_x(&svg, "cap")
    );
}

#[test]
fn every_side_is_available() {
    for side in ["above", "below", "left", "right"] {
        let src = format!(
            r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.center_x = 400
constrain v1.center_y = 200
text "c" cap [font_size: 12, caption_of: v1, label_position: {side}]
"#
        );
        render(&src).unwrap_or_else(|e| panic!("{side} should work: {e:?}"));
    }
}

#[test]
fn label_offset_controls_the_gap() {
    let tight = render(
        r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.center_x = 400
constrain v1.center_y = 200
text "c" cap [font_size: 12, caption_of: v1, label_position: below, label_offset: 4]
"#,
    )
    .expect("renders");
    let loose = render(
        r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.center_x = 400
constrain v1.center_y = 200
text "c" cap [font_size: 12, caption_of: v1, label_position: below, label_offset: 40]
"#,
    )
    .expect("renders");
    assert!(
        text_y(&loose, "cap") - text_y(&tight, "cap") > 30.0,
        "a bigger offset must push it further"
    );
}

#[test]
fn an_unknown_subject_is_an_error() {
    let err = render(
        r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
text "c" cap [font_size: 12, caption_of: no_such_bar, label_position: below]
"#,
    )
    .expect_err("a typo must not silently do nothing");
    assert!(
        format!("{err:?}").contains("no_such_bar"),
        "the error should name it: {err:?}"
    );
}

#[test]
fn a_caption_can_change_between_frames() {
    // The reason it is a text element rather than a shape's label in the first
    // place — and it must still follow its subject in every frame.
    let source = r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.center_x = 400 as home
constrain v1.center_y = 200
text "planned" cap [font_size: 12, caption_of: v1, label_position: below]
keyframe "a" { }
keyframe "b" { disable home
               constrain v1.center_x = 700
               transform cap [label: "confirmed"] }
"#;
    let mut config = RenderConfig::new();
    config.frame = Some("b".to_string());
    let svg = render_with_config(source, config).expect("renders");
    assert!(svg.contains("confirmed"), "the wording changed: {svg}");
    assert!(
        (text_x(&svg, "cap") - 700.0).abs() < 1.0,
        "and it followed the subject to 700, got {}",
        text_x(&svg, "cap")
    );
}

#[test]
fn the_subject_is_not_moved_by_its_caption() {
    // Attaching a caption must not perturb the thing it describes.
    let with = render(
        r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.center_x = 400
constrain v1.center_y = 200
text "c" cap [font_size: 12, caption_of: v1, label_position: below]
"#,
    )
    .expect("renders");
    let without = render(
        r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.center_x = 400
constrain v1.center_y = 200
"#,
    )
    .expect("renders");
    assert_eq!(rect_attr(&with, "v1", "x"), rect_attr(&without, "v1", "x"));
    assert_eq!(rect_attr(&with, "v1", "y"), rect_attr(&without, "v1", "y"));
}

#[test]
fn a_caption_that_is_also_constrained_is_reported() {
    // caption_of wins, so a constraint on the caption's position is silently
    // overridden — which is the exact class of bug this whole effort has been
    // closing. Say so.
    let (_svg, warnings) = render_with_lint(
        r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.center_x = 400
constrain v1.center_y = 200
text "c" cap [font_size: 12, caption_of: v1, label_position: below]
constrain cap.center_x = 50
"#,
        RenderConfig::new().with_lint(true),
    )
    .expect("renders");
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("cap") && w.message.contains("caption_of")),
        "expected a warning that the constraint is overridden: {:?}",
        warnings.iter().map(|w| &w.message).collect::<Vec<_>>()
    );
}

#[test]
fn constraining_to_a_captioned_element_names_the_cause() {
    // A captioned element's position is computed after the solve, so anything
    // constrained to it resolves against stale geometry. That broke loudly —
    // which is right — but as "violated by 594px", which names the symptom.
    // Stacking captions was the natural thing to try, so the diagnosis has to
    // say why it cannot work.
    let (_svg, warnings) = render_with_lint(
        r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.center_x = 400
constrain v1.center_y = 200
text "first" cap [caption_of: v1, label_position: below]
text "second" note [font_size: 12]
constrain note.top = cap.bottom + 8
"#,
        RenderConfig::new().with_lint(true),
    )
    .expect("renders");
    let named: Vec<_> = warnings
        .iter()
        .filter(|w| w.message.contains("cap") && w.message.contains("caption"))
        .collect();
    assert!(
        !named.is_empty(),
        "a warning must say cap is captioned, got: {:?}",
        warnings.iter().map(|w| &w.message).collect::<Vec<_>>()
    );
}

#[test]
fn the_suggested_fix_is_to_caption_the_same_subject() {
    let (_svg, warnings) = render_with_lint(
        r#"
rect v1 [width: 300, height: 56, fill: accent-light, stroke: accent-dark]
constrain v1.center_x = 400
constrain v1.center_y = 200
text "first" cap [caption_of: v1, label_position: below]
text "second" note [font_size: 12]
constrain note.top = cap.bottom + 8
"#,
        RenderConfig::new().with_lint(true),
    )
    .expect("renders");
    let m = warnings
        .iter()
        .find(|w| w.message.contains("caption"))
        .expect("reported");
    assert!(
        m.message.contains("label_offset") || m.message.contains("same subject"),
        "stacking is done by captioning the same subject with a bigger \
         label_offset; say so: {}",
        m.message
    );
}

#[test]
fn constraining_a_normal_element_to_another_is_unaffected() {
    let (_svg, warnings) = render_with_lint(
        r#"
rect a [width: 60, height: 40, fill: accent-light, stroke: accent-dark]
constrain a.center_x = 100
constrain a.center_y = 100
text "t" b [font_size: 12]
constrain b.top = a.bottom + 8
"#,
        RenderConfig::new().with_lint(true),
    )
    .expect("renders");
    assert!(
        !warnings.iter().any(|w| w.message.contains("caption")),
        "got: {:?}",
        warnings.iter().map(|w| &w.message).collect::<Vec<_>>()
    );
}
