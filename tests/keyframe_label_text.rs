//! `transform x [label: "..."]` rewrites an element's words for a frame, so
//! one caption can follow a walkthrough instead of ten stacked text elements.

use agent_illustrator::{render_with_config, RenderConfig};

const CAPTION: &str = r#"
rect stage [width: 200, height: 60]
text "the first step" cap
constrain cap.y = stage.bottom + 20

keyframe "one" {
}
keyframe "two" {
  transform cap [label: "the second step"]
}
keyframe "three" {
}
keyframe "four" {
  transform cap [label: "the last step"]
}
"#;

fn render(source: &str, configure: impl FnOnce(RenderConfig) -> RenderConfig) -> String {
    render_with_config(source, configure(RenderConfig::new())).expect("should render")
}

#[test]
fn a_static_frame_shows_the_text_that_frame_set() {
    let frame = |name: &str| {
        let name = name.to_string();
        render(CAPTION, move |mut c| {
            c.frame = Some(name);
            c
        })
    };

    assert!(frame("one").contains(">the first step<"));
    assert!(frame("two").contains(">the second step<"));
    // Transforms are cumulative: frame three keeps frame two's wording.
    assert!(frame("three").contains(">the second step<"));
    assert!(frame("four").contains(">the last step<"));

    // Only the wording for that frame is present, not the others.
    let two = frame("two");
    assert!(!two.contains(">the first step<"), "got: {}", two);
}

#[test]
fn an_animation_renders_one_node_per_distinct_wording() {
    let svg = render(CAPTION, |mut c| {
        c.animate = true;
        c
    });

    // Three wordings → the base node plus two variants, no duplicates for
    // the frames that merely inherit a wording.
    assert_eq!(svg.matches(">the first step<").count(), 1);
    assert_eq!(svg.matches(">the second step<").count(), 1);
    assert_eq!(svg.matches(">the last step<").count(), 1);

    assert!(svg.contains("aitxt-cap-base"), "base node should be classed");
    assert!(svg.contains("aitxt-cap-v0"));
    assert!(svg.contains("aitxt-cap-v1"));

    // The frame that inherits frame two's wording keeps showing that variant.
    assert!(svg.contains(".aitxt-cap-v0 { opacity: 1; }"));
    assert!(svg.contains(".aitxt-cap-base { opacity: 0; }"));
}

#[test]
fn css_only_animation_drives_the_variants_too() {
    let svg = render(CAPTION, |mut c| {
        c.animate_css = true;
        c
    });
    assert!(
        svg.contains("@keyframes kf-txt-aitxt-cap-base"),
        "base wording needs its own timeline"
    );
    assert!(svg.contains("@keyframes kf-txt-aitxt-cap-v0"));
    assert!(svg.contains("@keyframes kf-txt-aitxt-cap-v1"));
}

#[test]
fn a_shape_label_can_be_rewritten_too() {
    let source = r#"
rect box [width: 200, height: 60, label: "before"]
keyframe "a" {
}
keyframe "b" {
  transform box [label: "after"]
}
"#;
    let frame_b = render(source, |mut c| {
        c.frame = Some("b".to_string());
        c
    });
    assert!(frame_b.contains(">after<"), "got: {}", frame_b);
    assert!(!frame_b.contains(">before<"));
}

fn caption_x(svg: &str) -> f64 {
    let line = svg
        .lines()
        .find(|l| l.contains("id=\"cap\""))
        .expect("caption");
    let start = line.find(" x=\"").expect("x attribute") + 4;
    let rest = &line[start..];
    rest[..rest.find('"').unwrap()].parse::<f64>().unwrap()
}

#[test]
fn an_auto_sized_caption_is_laid_out_for_its_longest_wording() {
    // Sized up front, so it never has to resize — and therefore never
    // shifts — when a later frame gives it more words. The author does not
    // have to guess how wide the longest wording renders.
    let source = r#"
rect stage [width: 400, height: 60]
text "hi" cap [align: start]
constrain cap.center_x = stage.center_x
constrain cap.y = stage.bottom + 20
keyframe "a" {
}
keyframe "b" {
  transform cap [label: "a considerably longer caption than before"]
}
"#;
    let frame = |name: &str| {
        let name = name.to_string();
        render(source, move |mut c| {
            c.frame = Some(name);
            c
        })
    };
    let (a, b) = (frame("a"), frame("b"));
    assert!(b.contains("a considerably longer caption than before"));
    assert!(
        (caption_x(&a) - caption_x(&b)).abs() < 0.01,
        "a centred caption must not shift: {} vs {}",
        caption_x(&a),
        caption_x(&b)
    );
}

#[test]
fn an_explicit_width_is_never_changed_and_is_linted_when_too_small() {
    let source = r#"
rect stage [width: 400, height: 60]
text "hi" cap [width: 200, align: start]
constrain cap.center_x = stage.center_x
constrain cap.y = stage.bottom + 20
keyframe "a" {
}
keyframe "b" {
  transform cap [label: "a wording that is far wider than two hundred pixels"]
}
"#;
    let frame = |name: &str| {
        let name = name.to_string();
        render(source, move |mut c| {
            c.frame = Some(name);
            c
        })
    };
    assert!(
        (caption_x(&frame("a")) - caption_x(&frame("b"))).abs() < 0.01,
        "an explicit box must not resize behind the author's back"
    );

    // ...but silence would hide the problem, so the linter names the width
    // the wording actually needs.
    let (_svg, warnings) =
        agent_illustrator::render_with_lint(source, RenderConfig::new().with_lint(true))
            .expect("should render");
    let overflow = warnings
        .iter()
        .find(|w| w.category.to_string() == "label-overflow")
        .unwrap_or_else(|| {
            panic!(
                "expected a label-overflow warning, got: {:?}",
                warnings.iter().map(|w| &w.message).collect::<Vec<_>>()
            )
        });
    assert!(overflow.message.contains("needs about"), "{}", overflow.message);
    assert!(overflow.message.contains("200px"), "{}", overflow.message);
    assert_eq!(overflow.frames, vec!["b".to_string()]);
}
