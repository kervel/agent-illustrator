//! Rendering every keyframe should be one command, not a shell loop over
//! hand-copied frame names.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_agent-illustrator");

const SOURCE: &str = r#"
rect a [width: 60, height: 40]
rect b [width: 60, height: 40]
constrain b.x = a.right + 20
constrain b.y = a.y

keyframe "intro" {
  hide b
}
// Hyphenated, not spaced: a keyframe name becomes a CSS class, and
// `.frame-both shown` is a descendant selector that matches nothing. The
// expected filename below was already slugging it.
keyframe "both-shown" {
  show b
}
"#;

fn write_source(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("input.ail");
    std::fs::write(&path, SOURCE).expect("write source");
    path
}

#[test]
fn frame_names_are_listed_in_declaration_order() {
    assert_eq!(
        agent_illustrator::frame_names(SOURCE).expect("parse"),
        vec!["intro".to_string(), "both-shown".to_string()]
    );
}

#[test]
fn a_document_without_keyframes_has_no_frame_names() {
    assert!(agent_illustrator::frame_names("rect a")
        .expect("parse")
        .is_empty());
}

#[test]
fn frames_to_dir_writes_one_svg_per_frame() {
    let tmp = std::env::temp_dir().join(format!("ail-frames-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("create tmp");
    let source_path = write_source(&tmp);
    let out = tmp.join("out");

    let status = Command::new(BIN)
        .arg("--frames-to-dir")
        .arg(&out)
        .arg(&source_path)
        .status()
        .expect("run binary");
    assert!(status.success(), "expected success, got {:?}", status);

    // Numbered in frame order, with the name sanitised for the filesystem.
    let first = out.join("00-intro.svg");
    let second = out.join("01-both-shown.svg");
    assert!(first.exists(), "missing {}", first.display());
    assert!(second.exists(), "missing {}", second.display());

    // Frame 0 hides `b`; frame 1 shows it. Static frames drop hidden
    // elements entirely, so the ids tell the frames apart.
    let frame0 = std::fs::read_to_string(&first).expect("read frame 0");
    let frame1 = std::fs::read_to_string(&second).expect("read frame 1");
    assert!(frame0.contains("id=\"a\""));
    assert!(!frame0.contains("id=\"b\""), "b should be hidden in frame 0");
    assert!(frame1.contains("id=\"b\""), "b should be shown in frame 1");

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn frames_to_dir_refuses_input_without_keyframes() {
    let tmp = std::env::temp_dir().join(format!("ail-frames-none-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("create tmp");
    let source_path = tmp.join("plain.ail");
    std::fs::write(&source_path, "rect a").expect("write source");

    let output = Command::new(BIN)
        .arg("--frames-to-dir")
        .arg(tmp.join("out"))
        .arg(&source_path)
        .output()
        .expect("run binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("requires keyframes"),
        "unexpected stderr: {}",
        stderr
    );

    let _ = std::fs::remove_dir_all(&tmp);
}
