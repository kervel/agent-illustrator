//! `--png`: a picture without a browser, from the binary alone.

use std::path::Path;
use std::process::Command;

fn run(args: &[&str]) -> (Vec<u8>, String) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/motion");
    let out = Command::new(env!("CARGO_BIN_EXE_agent-illustrator")).current_dir(&dir).args(args).output().expect("runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    (out.stdout, String::from_utf8_lossy(&out.stderr).to_string())
}

#[test]
fn a_mid_motion_still_is_a_png_and_every_font_is_there() {
    let (png, notes) = run(&["--stylesheet-css", "git-deck.css", "--frame", "push", "--at", "50%", "--png", "git-history.ail"]);
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert!(!notes.contains("png:"), "the bundled fonts cover the git deck: {notes}");
}

#[test]
fn scale_zooms() {
    let (a, _) = run(&["--frame", "pull", "--png", "git-history.ail"]);
    let (b, _) = run(&["--frame", "pull", "--png", "--scale", "2", "git-history.ail"]);
    let width = |p: &[u8]| u32::from_be_bytes([p[16], p[17], p[18], p[19]]);
    assert_eq!(width(&b), 2 * width(&a));
}

#[test]
fn a_character_no_font_has_is_reported() {
    let dir = std::env::temp_dir().join(format!("ail-png-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("s.ail");
    std::fs::write(&f, "rect a [width: 80, height: 40, label: \"✕ ✓ 漢\"]").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_agent-illustrator")).arg(&f).arg("--png").output().expect("runs");
    let notes = String::from_utf8_lossy(&out.stderr);
    assert!(notes.contains("U+6F22") && !notes.contains("U+2715"), "{notes}");
}
