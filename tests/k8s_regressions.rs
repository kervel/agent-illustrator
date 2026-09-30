//! The Kubernetes scenes written from scratch with v0.2.9 (tests/examples):
//! the false positives and silent failures found in them stay fixed.

use std::path::Path;
use std::process::Command;

fn lint(scene: &str) -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/examples");
    let out = Command::new(env!("CARGO_BIN_EXE_agent-illustrator"))
        .current_dir(&dir)
        .args(["--lint", scene])
        .output()
        .expect("runs");
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

#[test]
fn they_render() {
    for s in ["k8s-container.ail", "k8s-release.ail", "k8s-copies.ail"] {
        assert!(!lint(s).contains("Error"), "{s}: {}", lint(s));
    }
}

#[test]
fn an_empty_grid_collides_with_nothing() {
    let l = lint("k8s-container.ail");
    assert!(!l.contains("\"project\""), "{l}");
}

#[test]
fn a_wire_inside_an_instance_is_not_a_station_of_it() {
    let l = lint("k8s-release.ail");
    assert!(!l.contains("only reaches it"), "{l}");
}

#[test]
fn a_row_of_identical_users_is_not_crowded() {
    let l = lint("k8s-copies.ail");
    assert!(!l.contains("crowded-layout"), "{l}");
}
