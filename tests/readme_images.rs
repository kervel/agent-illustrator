//! The committed example images (what the README shows) are what the
//! current code renders. `examples/renders.txt` lists each image with its
//! source and flags; regenerate with `bash examples/render-all.sh`.

use std::path::Path;
use std::process::Command;

/// The build stamp differs between builds; nothing else may.
fn normalise(svg: &str) -> String {
    svg.lines()
        .filter(|l| !l.trim_start().starts_with("<!-- agent-illustrator"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn manifest() -> Vec<(String, String, Vec<String>)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(root.join("examples/renders.txt")).expect("examples/renders.txt");
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let mut w = l.split_whitespace().map(str::to_string);
            let out = w.next().unwrap();
            let src = w.next().expect("a source after the output");
            (out, src, w.collect())
        })
        .collect()
}

#[test]
fn every_committed_example_image_is_current() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut stale = Vec::new();
    for (out, src, flags) in manifest() {
        let run = Command::new(env!("CARGO_BIN_EXE_agent-illustrator"))
            .current_dir(root)
            .arg(&src)
            .args(&flags)
            .output()
            .expect("runs");
        assert!(run.status.success(), "{src}: {}", String::from_utf8_lossy(&run.stderr));
        let fresh = normalise(&String::from_utf8_lossy(&run.stdout));
        let committed = std::fs::read_to_string(root.join(&out)).unwrap_or_default();
        if normalise(&committed) != fresh {
            stale.push(out);
        }
    }
    assert!(
        stale.is_empty(),
        "stale example images (run `bash examples/render-all.sh` and commit): {stale:?}"
    );
}

#[test]
fn every_readme_image_is_rendered_from_the_manifest() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme = std::fs::read_to_string(root.join("README.md")).expect("README.md");
    let listed: Vec<String> = manifest().into_iter().map(|(o, _, _)| o).collect();
    let mut missing = Vec::new();
    for part in readme.split("](").skip(1).chain(readme.split("src=\"").skip(1)) {
        let target: String = part.chars().take_while(|c| *c != ')' && *c != '"').collect();
        if target.ends_with(".svg") && !target.starts_with("http") && !listed.contains(&target) {
            missing.push(target);
        }
    }
    assert!(missing.is_empty(), "README images not in examples/renders.txt: {missing:?}");
}
