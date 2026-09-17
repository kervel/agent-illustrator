//! Stamp the build with a version that is actually true.
//!
//! `Cargo.toml`'s version is not authoritative — CI decides what a release is
//! called, from the tag. So the binary takes its version from, in order:
//!
//! 1. `AI_RELEASE_VERSION`, which the release workflow sets from the tag.
//! 2. `git describe`, so a local build says exactly what it is —
//!    `v0.1.25-12-gc67e445-dirty` rather than a release number it is not.
//! 3. `unknown`, when neither is available (a source tarball with no git).
//!
//! This exists because "which binary rendered this?" was unanswerable, and a
//! stale `--watch` server on an older binary silently rendered markup as
//! literal text for a whole session while the author verified against a newer
//! one by hand.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=AI_RELEASE_VERSION");
    watch_git_head();

    let version = std::env::var("AI_RELEASE_VERSION")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(git_describe)
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=AI_VERSION={version}");

    // The SVG generator stamp is a SEPARATE, coarser string on purpose. The
    // committed example SVGs are rendered by a local build, so stamping them
    // with `git describe` would rewrite all seventeen on every commit and bury
    // real changes in churn. A release build stamps its version; anything else
    // stamps "dev", which is stable and honest. `--version` keeps the precise
    // string, because that is where the question "which binary is this?"
    // actually gets asked.
    let stamp = std::env::var("AI_RELEASE_VERSION")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "dev".to_string());
    println!("cargo:rustc-env=AI_STAMP={stamp}");
}

/// Ask cargo to re-run this script when the checked-out commit changes.
///
/// `.git/HEAD` alone is not enough: on a branch it holds `ref: refs/heads/NAME`
/// and its contents do not change when you commit, so the baked version went
/// stale and `--version` — the flag that exists to tell you which binary you
/// have — reported the previous build. The ref file is what actually moves, so
/// watch that too, and `packed-refs` for a ref that has been packed away.
fn watch_git_head() {
    let git_dir = std::path::Path::new(".git");
    let head = git_dir.join("HEAD");
    println!("cargo:rerun-if-changed={}", head.display());

    if let Ok(contents) = std::fs::read_to_string(&head) {
        if let Some(reference) = contents.strip_prefix("ref: ").map(str::trim) {
            println!(
                "cargo:rerun-if-changed={}",
                git_dir.join(reference).display()
            );
        }
    }
    let packed = git_dir.join("packed-refs");
    if packed.exists() {
        println!("cargo:rerun-if-changed={}", packed.display());
    }
}

fn git_describe() -> Option<String> {
    let out = Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
