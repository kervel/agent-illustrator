//! Lint checks that depend on what is visible at the same time must be
//! evaluated per keyframe: two things that are never on screen together
//! cannot collide, and a defect must be reported once, not once per frame.

use agent_illustrator::{render_with_lint, RenderConfig};

fn lint(source: &str) -> Vec<agent_illustrator::LintWarning> {
    let config = RenderConfig::new().with_lint(true);
    let (_svg, warnings) = render_with_lint(source, config).expect("Should render");
    warnings
}

#[test]
fn captions_sharing_a_spot_across_frames_are_not_collisions() {
    let source = include_str!("lint-fixtures/keyframe-captions.ail");
    let warnings = lint(source);
    assert!(
        warnings.is_empty(),
        "captions shown one per frame must not be reported as overlapping, got: {:?}",
        warnings
            .iter()
            .map(|w| format!("{}: {}", w.category, w.message))
            .collect::<Vec<_>>()
    );
}

#[test]
fn the_same_defect_without_keyframes_is_still_reported() {
    // Identical geometry to the fixture above, minus the keyframes: the
    // overlap checks themselves must still fire.
    let source = r#"
rect stage [width: 300, height: 120]
text "the first step of the walkthrough" cap1
text "the second step of the walkthrough" cap2
constrain cap1.center_x = stage.center_x
constrain cap2.center_x = stage.center_x
constrain cap1.y = stage.bottom + 20
constrain cap2.y = stage.bottom + 20
"#;
    let warnings = lint(source);
    assert!(
        warnings.iter().any(|w| w.category.to_string() == "overlap"),
        "expected an overlap warning without keyframes, got: {:?}",
        warnings
            .iter()
            .map(|w| w.message.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_defect_is_reported_once_and_scoped_to_the_frames_it_occurs_in() {
    let source = include_str!("lint-fixtures/keyframe-overlap-scope.ail");
    let warnings = lint(source);

    let overlaps: Vec<_> = warnings
        .iter()
        .filter(|w| w.category.to_string() == "overlap")
        .collect();
    assert_eq!(
        overlaps.len(),
        3,
        "expected exactly three distinct overlaps, got: {:?}",
        overlaps.iter().map(|w| &w.message).collect::<Vec<_>>()
    );

    // Present in every frame → reported bare, without frame names.
    let always = overlaps
        .iter()
        .find(|w| w.message.contains("\"always_a\" and \"always_b\""))
        .expect("always_a/always_b overlap should be reported");
    assert!(
        always.frames.is_empty(),
        "a defect present in every frame needs no frame annotation, got {:?}",
        always.frames
    );
    assert_eq!(always.frame_suffix(), "");

    // Only once `late` is shown → annotated with that frame.
    let late = overlaps
        .iter()
        .find(|w| w.message.contains("\"always_a\" and \"late\""))
        .expect("always_a/late overlap should be reported");
    assert_eq!(late.frames, vec!["end".to_string()]);
    assert_eq!(late.frame_suffix(), " [frame: end]");
}

#[test]
fn hiding_a_group_hides_everything_inside_it() {
    // `inner_a`/`inner_b` overlap, but their group is hidden from frame 0
    // onwards and never shown, so nothing inside it can collide.
    let source = r#"
group boxed {
  rect inner_a [width: 100, height: 60]
  rect inner_b [width: 100, height: 60]
}
rect other [width: 40, height: 40]
constrain inner_b.x = inner_a.x + 20
constrain inner_b.y = inner_a.y

keyframe "one" {
  hide boxed
}
keyframe "two" {
}
"#;
    let warnings = lint(source);
    assert!(
        !warnings
            .iter()
            .any(|w| w.message.contains("inner_a") || w.message.contains("inner_b")),
        "children of a hidden group must be out of scope, got: {:?}",
        warnings
            .iter()
            .map(|w| w.message.clone())
            .collect::<Vec<_>>()
    );
}
