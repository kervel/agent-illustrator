//! The "first scene" in --skill-animation is a real scene: it renders,
//! lints clean, and has the steps the text describes.

use agent_illustrator::{render_with_config, render_with_lint, RenderConfig};

fn first_scene() -> String {
    let doc = include_str!("../docs/skill-animation.md");
    let start = doc.find("```ail\n").expect("an ```ail block") + "```ail\n".len();
    let end = start + doc[start..].find("```").unwrap();
    doc[start..end].to_string()
}

#[test]
fn the_first_scene_lints_clean() {
    let (_, w) = render_with_lint(&first_scene(), RenderConfig::new().with_lint(true)).expect("renders");
    let w: Vec<String> = w.into_iter().map(|w| w.message).collect();
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn the_first_scene_has_its_four_steps_and_no_stage_note() {
    let mut c = RenderConfig::new();
    c.states = true;
    let s = render_with_config(&first_scene(), c).expect("renders");
    for step in ["start", "clone", "push", "pull"] {
        assert!(s.contains(step), "{s}");
    }
    assert!(!s.contains("note:"), "the example sits centred on its stage: {s}");
}
