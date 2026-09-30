//! Every SVG the examples produce is well-formed XML: a standalone viewer,
//! an <img>, and --png all need that, even where an inline-in-HTML browser
//! would forgive a missing space between two attributes.

use std::path::{Path, PathBuf};
use std::process::Command;

fn render(dir: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_agent-illustrator")).current_dir(dir).args(args).output().expect("runs");
    assert!(out.status.success(), "{:?} in {}: {}", args, dir.display(), String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn scenes() -> Vec<(PathBuf, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    for (dir, names) in [
        ("examples/motion", vec!["git-copies", "git-snapshots", "git-branches", "git-merge", "git-history"]),
        ("tests/examples", vec!["k8s-container", "k8s-release", "k8s-copies"]),
        ("tests/examples/spa", vec!["spa-app", "spa-classic"]),
    ] {
        for n in names {
            if root.join(dir).join(format!("{n}.ail")).exists() {
                out.push((root.join(dir), format!("{n}.ail")));
            }
        }
    }
    out
}

#[test]
fn every_frame_still_and_animation_is_well_formed_xml() {
    let mut bad = Vec::new();
    for (dir, file) in scenes() {
        let frames = render(&dir, &["--list-frames", &file]);
        let n = frames.lines().filter(|l| !l.trim().is_empty()).count();
        let mut outputs: Vec<(String, String)> = vec![
            ("--animate".into(), render(&dir, &["--animate", &file])),
            ("--animate-css".into(), render(&dir, &["--animate-css", &file])),
        ];
        for i in 0..n {
            let f = i.to_string();
            outputs.push((format!("--frame {i}"), render(&dir, &["--frame", &f, &file])));
            for at in ["50%"] {
                outputs.push((format!("--frame {i} --at {at}"), render(&dir, &["--frame", &f, "--at", at, &file])));
            }
        }
        for (what, svg) in outputs {
            if let Err(e) = roxmltree::Document::parse(&svg) {
                bad.push(format!("{file} {what}: {e}"));
            }
        }
    }
    assert!(bad.is_empty(), "not well-formed:\n{}", bad.join("\n"));
}
