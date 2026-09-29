//! Golden frames of the motion example scenes.
//!
//! Every settled frame of each scene (rendered natively, with the scene's
//! stylesheet) is compared with a committed SVG that a person has looked at.
//! The seek tests cannot catch a frame that is wrong the same way in the
//! player and the native renderer (a timeline that starts fully drawn, text
//! that turns white): only a reviewed picture can. When a change is intended,
//! regenerate with `AIL_UPDATE_GOLDEN=1 cargo test --test motion_golden`,
//! look at every changed frame, and commit them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SCENES: &[&str] = &["git-copies", "git-snapshots", "git-branches", "git-merge"];

/// The build stamp differs between builds; nothing else may.
fn normalise(svg: &str) -> String {
    svg.lines()
        .filter(|l| !l.trim_start().starts_with("<!-- agent-illustrator"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_frames(scene: &str, out: &Path) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/motion");
    let status = Command::new(env!("CARGO_BIN_EXE_agent-illustrator"))
        .current_dir(&dir)
        .args(["--stylesheet-css", "git-deck.css", "--frames-to-dir"])
        .arg(out)
        .arg(format!("{scene}.ail"))
        .output()
        .expect("runs");
    assert!(status.status.success(), "{scene}: {}", String::from_utf8_lossy(&status.stderr));
}

fn svgs(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    v.retain(|p| p.extension().is_some_and(|e| e == "svg"));
    v.sort();
    v
}

#[test]
fn every_frame_of_the_motion_scenes_matches_its_golden() {
    let update = std::env::var("AIL_UPDATE_GOLDEN").is_ok();
    let mut failures = Vec::new();
    for scene in SCENES {
        let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/motion").join(scene);
        let out = std::env::temp_dir().join(format!("ail-golden-{}-{scene}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        render_frames(scene, &out);
        let fresh = svgs(&out);
        if update {
            let _ = fs::remove_dir_all(&golden);
            fs::create_dir_all(&golden).unwrap();
            for f in &fresh {
                fs::copy(f, golden.join(f.file_name().unwrap())).unwrap();
            }
            continue;
        }
        let names = |v: &[PathBuf]| v.iter().map(|p| p.file_name().unwrap().to_owned()).collect::<Vec<_>>();
        let want = svgs(&golden);
        if names(&fresh) != names(&want) {
            failures.push(format!("{scene}: frames {:?}, golden has {:?}", names(&fresh), names(&want)));
            continue;
        }
        for (f, g) in fresh.iter().zip(&want) {
            let (a, b) = (fs::read_to_string(f).unwrap(), fs::read_to_string(g).unwrap());
            if normalise(&a) != normalise(&b) {
                failures.push(format!("{scene}/{}", g.file_name().unwrap().to_string_lossy()));
            }
        }
        let _ = fs::remove_dir_all(&out);
    }
    assert!(
        failures.is_empty(),
        "frames differ from their goldens (look at them; if intended, regenerate with \
         AIL_UPDATE_GOLDEN=1 and commit): {failures:?}"
    );
}
