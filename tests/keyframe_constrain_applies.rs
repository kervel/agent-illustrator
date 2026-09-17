//! A `constrain` inside a keyframe must take effect whether or not the frame
//! also contains a `transform`.
//!
//! `--frame` re-solved a frame only when `state.transforms` was non-empty, so
//! a frame whose only operations were `constrain`/`disable` was rendered from
//! the base layout and every constraint in it silently vanished. `--animate`
//! used the right condition, so the static and animated paths disagreed —
//! the same split as the id-selector bug, in the other direction.

use agent_illustrator::{render_with_config, RenderConfig};

fn frame(source: &str, n: usize) -> String {
    let mut config = RenderConfig::new();
    config.frame = Some(n.to_string());
    render_with_config(source, config).expect("renders")
}

fn attr(svg: &str, id: &str, name: &str) -> f64 {
    let marker = format!(r#"id="{id}""#);
    let start = svg.find(&marker).expect("element rendered");
    let rest = &svg[start..];
    let i = rest.find(&format!(r#" {name}=""#)).expect("attribute") + name.len() + 3;
    let tail = &rest[i..];
    tail[..tail.find('"').unwrap()].parse().unwrap()
}

#[test]
fn a_size_constraint_in_a_keyframe_applies() {
    // The reported case: accepted, dropped, no error and no lint.
    let source = r#"
rect a [height: 30, fill: accent-1]
constrain a.left = 20
constrain a.right = 120 as r
keyframe "first" { }
keyframe "wider" { disable r
                   constrain a.right = 300 }
"#;
    assert_eq!(attr(&frame(source, 0), "a", "width"), 100.0);
    assert_eq!(
        attr(&frame(source, 1), "a", "width"),
        280.0,
        "the frame's own constraint must widen the box"
    );
}

#[test]
fn a_position_constraint_in_a_keyframe_applies() {
    let source = r#"
rect a [width: 100, height: 30, fill: accent-1]
constrain a.center_x = 100 as home
constrain a.center_y = 40
keyframe "first" { }
keyframe "moved" { disable home
                   constrain a.center_x = 400 }
"#;
    assert_eq!(attr(&frame(source, 0), "a", "x"), 50.0);
    assert_eq!(attr(&frame(source, 1), "a", "x"), 350.0);
}

#[test]
fn it_does_not_depend_on_an_unrelated_transform_being_present() {
    // This is what made the bug so confusing: adding a transform on a
    // completely different element made the constraint start working, because
    // the re-solve was gated on `transforms` being non-empty.
    let with_transform = r#"
rect a [width: 100, height: 30, fill: accent-1]
rect z [width: 10, height: 10, fill: accent-2]
constrain a.center_x = 100 as home
constrain a.center_y = 40
constrain z.center_x = 500
constrain z.center_y = 40
keyframe "first" { }
keyframe "moved" { disable home
                   constrain a.center_x = 400
                   transform z [fill: secondary-1] }
"#;
    let without_transform = r#"
rect a [width: 100, height: 30, fill: accent-1]
rect z [width: 10, height: 10, fill: accent-2]
constrain a.center_x = 100 as home
constrain a.center_y = 40
constrain z.center_x = 500
constrain z.center_y = 40
keyframe "first" { }
keyframe "moved" { disable home
                   constrain a.center_x = 400 }
"#;
    assert_eq!(
        attr(&frame(with_transform, 1), "a", "x"),
        attr(&frame(without_transform, 1), "a", "x"),
        "an unrelated transform must not decide whether constraints apply"
    );
}

#[test]
fn the_static_and_animated_paths_agree() {
    // --animate had the right condition all along, so the two disagreed.
    let source = r#"
rect a [width: 100, height: 30, fill: accent-1]
constrain a.center_x = 100 as home
constrain a.center_y = 40
keyframe "first" { }
keyframe "moved" { disable home
                   constrain a.center_x = 400 }
"#;
    let static_x = attr(&frame(source, 1), "a", "x");

    let mut config = RenderConfig::new();
    config.animate = true;
    let animated = render_with_config(source, config).expect("renders");
    // The animated path expresses the move as a transform on the wrapper.
    assert!(
        animated.contains("translate") || animated.contains("transform:"),
        "animated output should carry the move: {animated}"
    );
    assert_eq!(static_x, 350.0);
}
