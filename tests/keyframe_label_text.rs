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

#[test]
fn a_longer_wording_grows_the_box_but_a_wider_one_is_kept() {
    // An auto-sized caption grows to fit longer text...
    let grows = r#"
text "hi" cap
keyframe "a" {
}
keyframe "b" {
  transform cap [label: "a considerably longer caption than before"]
}
"#;
    let svg = render(grows, |mut c| {
        c.frame = Some("b".to_string());
        c
    });
    assert!(
        svg.contains("a considerably longer caption than before"),
        "got: {}",
        svg
    );

    // ...but a box given a deliberate width keeps it, so right-aligned text
    // does not wander when the wording changes.
    let fixed = r#"
text "hi" cap [width: 400, align: end]
keyframe "a" {
}
keyframe "b" {
  transform cap [label: "still inside the same box"]
}
"#;
    let frame_a = render(fixed, |mut c| {
        c.frame = Some("a".to_string());
        c
    });
    let frame_b = render(fixed, |mut c| {
        c.frame = Some("b".to_string());
        c
    });
    let x_of = |svg: &str| {
        let line = svg
            .lines()
            .find(|l| l.contains("id=\"cap\""))
            .expect("caption")
            .to_string();
        let start = line.find(" x=\"").expect("x attribute") + 4;
        let rest = &line[start..];
        rest[..rest.find('"').unwrap()].parse::<f64>().unwrap()
    };
    assert!(
        (x_of(&frame_a) - x_of(&frame_b)).abs() < 0.01,
        "anchor should not move: {} vs {}",
        x_of(&frame_a),
        x_of(&frame_b)
    );
}
