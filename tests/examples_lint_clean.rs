//! The examples must practise what the linter preaches.
//!
//! Agents read examples more than they read docs, so an example that trips a
//! rule teaches the habit that rule exists to prevent. Before this gate the
//! corpus emitted 74 warnings across fourteen files, including a 53px label
//! inside a 10px circle and three elements sharing one id.
//!
//! The six categories gated here are the ones a real 8-file corpus measured at
//! zero false positives. The rest (`overlap`, `alignment`, `contrast`,
//! `over-constrained`) still have a known false-positive rate and are reported
//! for a human to judge, not enforced.

use std::path::Path;
use std::process::Command;

/// Categories with no measured false positives. A warning here is a defect.
const GATED: &[&str] = &[
    "connection",
    "redundant-constant",
    "label",
    "reducible-bend",
    "label-overflow",
    "missing-anchor",
];

/// Warnings that survive review, each with the reason it is not a defect.
///
/// An entry here is a claim that the example is right and the rule is being
/// asked a question it cannot answer. Adding one is a deliberate act — which
/// is the point, because a bare count would let the corpus rot quietly.
const REVIEWED: &[(&str, &str, &str)] = &[
    (
        "person-rotation.ail",
        "p90→p180",
        "This file verifies that anchors rotate with their element, so it \
         always joins hand_right to hand_left. On a figure rotated 180 \
         degrees that anchor is on the far side and the line must reach \
         across the torso — the property being demonstrated.",
    ),
    (
        "person-rotation.ail",
        "p270→p45",
        "Same: the bend is what connecting two differently-rotated figures \
         looks like, and straightening it would stop the file testing \
         anything.",
    ),
    (
        "token-prediction.ail",
        "labels on \"d\" and \"e\" overlap",
        "KNOWN LINT DEFECT, not an example defect. check_collisions takes the \
         BASE layout plus a per-frame scope carrying only visibility — it \
         never re-solves geometry per frame. d, e and f share a home position \
         and keyframes move them apart with disable+constrain, so lint sees \
         frame-0 coordinates with a later frame's visibility. Rendering \
         --frame predict_the puts d at x=262 and e at x=535: they do not \
         overlap on screen. Every frame-scoped collision warning on a diagram \
         that MOVES elements is unreliable for the same reason.",
    ),
    (
        "token-prediction.ail",
        "labels on \"d\" and \"f\" overlap",
        "Same known lint defect.",
    ),
    (
        "token-prediction.ail",
        "labels on \"e\" and \"f\" overlap",
        "Same known lint defect.",
    ),
    (
        "token-prediction.ail",
        "set .center_y to the same constant 300",
        "d, e and f each keep their own literal home row. Keyframes disable \
         those pins individually to move one box without the others, so \
         relating them would couple elements the animation exists to \
         separate.",
    ),
];

/// Run `--lint` the way an agent does.
///
/// The CLI resolves a template's relative path against the .ail file; the
/// library API takes a bare string and cannot. Two examples import clipart
/// that way, so the binary is both the faithful check and the only one that
/// works for the whole corpus.
fn lint_file(path: &Path) -> Result<Vec<(String, String)>, String> {
    let out = Command::new(env!("CARGO_BIN_EXE_agent-illustrator"))
        .arg("--lint")
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if text.contains("Error:") {
        return Err(text.lines().take(2).collect::<Vec<_>>().join(" "));
    }
    let mut found = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("lint: ") else {
            continue;
        };
        // `category: message`, or `category [frames: a, b]: message` — the
        // frame annotation contains ": " itself, so split after its bracket.
        let (head, message) = match rest.split_once("]: ") {
            Some((h, m)) => (h, m),
            None => match rest.split_once(": ") {
                Some((h, m)) => (h, m),
                None => continue,
            },
        };
        let category = head.split(' ').next().unwrap_or(head).trim_end_matches(':');
        if category.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue; // the trailing "N warning(s)" summary
        }
        found.push((category.to_string(), message.to_string()));
    }
    Ok(found)
}

fn is_reviewed(file: &str, message: &str) -> bool {
    REVIEWED
        .iter()
        .any(|(f, needle, _)| *f == file && message.contains(needle))
}

#[test]
fn every_example_is_clean_in_the_high_signal_categories() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut failures = Vec::new();
    let mut checked = 0;

    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("examples/ exists")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "ail"))
        .collect();
    entries.sort();

    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        match lint_file(&path) {
            Err(e) => failures.push(format!("{name}: failed to render: {e}")),
            Ok(warnings) => {
                checked += 1;
                for (category, message) in warnings {
                    if !GATED.contains(&category.as_str()) {
                        continue;
                    }
                    if is_reviewed(&name, &message) {
                        continue;
                    }
                    failures.push(format!("{name}: {category}: {message}"));
                }
            }
        }
    }

    assert!(checked >= 14, "expected the whole corpus, saw {checked}");
    assert!(
        failures.is_empty(),
        "examples must not demonstrate what the linter warns about.\n\
         Fix the example, or add it to REVIEWED with the reason it is right:\n  {}",
        failures.join("\n  ")
    );
}

#[test]
fn every_reviewed_exception_still_fires() {
    // An exception that no longer matches anything is stale: either the
    // example was fixed or the rule changed, and leaving the entry behind
    // silently widens the gate.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    for (file, needle, _reason) in REVIEWED {
        let warnings = lint_file(&dir.join(file)).expect("renders");
        assert!(
            warnings.iter().any(|(_, m)| m.contains(needle)),
            "REVIEWED entry for {file} / {needle:?} no longer matches any warning — \
             remove it rather than leaving the gate wider than it needs to be"
        );
    }
}
