//! Lint must ask each frame its own question.
//!
//! `check_collisions` took the base layout plus a scope carrying only
//! VISIBILITY, and never re-solved geometry per frame. So on a diagram where
//! keyframes MOVE elements — `disable` a pin, re-constrain it — every
//! frame-scoped collision warning compared frame-0 coordinates against a later
//! frame's visible set.
//!
//! Two symptoms that look unrelated from the inside and share one cause:
//! confident nonsense about geometry (things reported as overlapping that have
//! moved apart), and confident nonsense about co-existence (a pair reported as
//! colliding that is never on screen together).

use agent_illustrator::{render_with_lint, RenderConfig};

fn warnings(source: &str) -> Vec<String> {
    let (_svg, w) = render_with_lint(source, RenderConfig::new().with_lint(true)).expect("renders");
    w.iter().map(|x| x.message.clone()).collect()
}

/// Warnings as `(message, frames)`, so a test can assert WHICH frames a
/// defect was found in — the thing that was wrong before.
fn warnings_with_frames(source: &str) -> Vec<(String, Vec<String>)> {
    let (_svg, w) = render_with_lint(source, RenderConfig::new().with_lint(true)).expect("renders");
    w.iter()
        .map(|x| (x.message.clone(), x.frames.clone()))
        .collect()
}

/// Two boxes share a home position and a later frame moves one away.
const MOVED_APART: &str = r#"
rect d [width: 50, height: 38, fill: accent-light, stroke: accent-dark, label: "on"]
rect e [width: 50, height: 38, fill: accent-light, stroke: accent-dark, label: "the"]
constrain d.center_x = 400 as d_hx
constrain d.center_y = 300 as d_hy
constrain e.center_x = 400
constrain e.center_y = 300

keyframe "start" { }
keyframe "split" {
    disable d_hx
    disable d_hy
    constrain d.center_x = 150
    constrain d.center_y = 300
}
"#;

#[test]
fn an_overlap_is_scoped_to_the_frames_it_actually_happens_in() {
    // d and e share a home, so they DO overlap in "start". The second frame
    // moves d to x=150 and they do not. Before the per-frame solve the
    // warning could not tell those frames apart, because both were measured
    // against frame-0 coordinates.
    let found = warnings_with_frames(MOVED_APART);
    let overlap = found
        .iter()
        .find(|(m, _)| m.contains("\"d\"") && m.contains("\"e\"") && m.contains("overlap"))
        .expect("they do overlap in the first frame");
    assert_eq!(
        overlap.1,
        vec!["start".to_string()],
        "the overlap belongs to `start` alone; in `split` d has moved away"
    );
}

#[test]
fn elements_that_move_together_are_still_reported() {
    // The fix must not simply silence frame-scoped collisions. Here the
    // keyframe moves d ONTO e, which is invisible from the base layout
    // because they start apart.
    let msgs = warnings(
        r#"
rect d [width: 50, height: 38, fill: accent-light, stroke: accent-dark, label: "on"]
rect e [width: 50, height: 38, fill: accent-light, stroke: accent-dark, label: "the"]
constrain d.center_x = 150 as d_hx
constrain d.center_y = 300
constrain e.center_x = 400
constrain e.center_y = 300

keyframe "apart" { }
keyframe "collide" {
    disable d_hx
    constrain d.center_x = 400
}
"#,
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("\"d\"") && m.contains("\"e\"")),
        "d moves onto e in the second frame and must be reported: {msgs:?}"
    );
}

#[test]
fn a_diagram_without_keyframes_is_unaffected() {
    let msgs = warnings(
        r#"
rect a [width: 100, height: 60, fill: accent-light, stroke: accent-dark]
constrain a.center_x = 100
constrain a.center_y = 100
rect b [width: 100, height: 60, fill: secondary-light, stroke: secondary-dark]
constrain b.center_x = 150
constrain b.center_y = 100
"#,
    );
    assert_eq!(
        msgs.iter().filter(|m| m.contains("overlap")).count(),
        1,
        "got: {msgs:?}"
    );
}

// ── symptom 2: co-existence ───────────────────────────────────────

#[test]
fn text_is_not_told_to_join_a_box_it_is_never_on_screen_with() {
    // A closing note parked in a card's slot after the card is gone. The rule
    // reasons about the base layout and cannot see that the pair is never
    // simultaneously visible.
    let msgs = warnings(
        r#"
rect cs [width: 200, height: 80, fill: background-light, stroke: foreground-2, stroke_width: 2]
constrain cs.center_x = 300
constrain cs.center_y = 200

text "the changeset is gone" now_note [font_size: 12]
constrain now_note.center_x = cs.center_x
constrain now_note.center_y = cs.center_y

keyframe "card" { hide now_note }
keyframe "note" { hide cs
                  show now_note }
"#,
    );
    let sits: Vec<_> = msgs.iter().filter(|m| m.contains("sit inside")).collect();
    assert!(
        sits.is_empty(),
        "cs and now_note are never visible together: {sits:?}"
    );
}

#[test]
fn text_hand_placed_in_a_box_it_shares_a_frame_with_is_still_reported() {
    let msgs = warnings(
        r#"
rect cs [width: 200, height: 80, fill: background-light, stroke: foreground-2, stroke_width: 2]
constrain cs.center_x = 300
constrain cs.center_y = 200

text "a title" note [font_size: 12]
constrain note.center_x = cs.center_x
constrain note.center_y = cs.center_y

keyframe "both" { }
keyframe "still both" { }
"#,
    );
    assert!(
        msgs.iter().any(|m| m.contains("sit inside")),
        "they share every frame and must still be reported: {msgs:?}"
    );
}
