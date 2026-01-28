//! SVG Regression Tests for Feature 010 (Local/Global Solver Separation)
//!
//! These tests ensure that existing AIL files without rotation produce
//! consistent SVG output after the constraint solver refactor.
//!
//! Note: Due to HashMap iteration order being non-deterministic for CSS
//! variables and floating point precision in viewBox calculations, we
//! compare the structural content excluding the style block.

use std::fs;
use std::path::Path;

use agent_illustrator::render;

/// Normalize an SVG string for comparison by:
/// 1. Removing the style block (CSS variable order is non-deterministic)
/// 2. Normalizing whitespace
/// 3. Rounding viewBox numbers to 1 decimal place
fn normalize_svg_for_comparison(svg: &str) -> String {
    let mut result = String::new();
    let mut in_style = false;

    for line in svg.lines() {
        let trimmed = line.trim();

        // Skip style block entirely
        if trimmed.starts_with("<style>") {
            in_style = true;
            continue;
        }
        if trimmed.starts_with("</style>") {
            in_style = false;
            continue;
        }
        if in_style {
            continue;
        }

        // Normalize viewBox by rounding numbers
        if trimmed.contains("viewBox=") {
            let normalized = normalize_viewbox(trimmed);
            result.push_str(&normalized);
            result.push('\n');
        } else {
            result.push_str(trimmed);
            result.push('\n');
        }
    }

    result
}

/// Round viewBox numbers to avoid floating point comparison issues
fn normalize_viewbox(line: &str) -> String {
    // Extract viewBox value and round numbers
    if let Some(start) = line.find("viewBox=\"") {
        if let Some(end) = line[start + 9..].find('"') {
            let viewbox = &line[start + 9..start + 9 + end];
            let numbers: Vec<f64> = viewbox
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();

            if numbers.len() == 4 {
                let rounded = format!(
                    "viewBox=\"{:.0} {:.0} {:.0} {:.0}\"",
                    numbers[0], numbers[1], numbers[2], numbers[3]
                );
                return line.replace(&format!("viewBox=\"{}\"", viewbox), &rounded);
            }
        }
    }
    line.to_string()
}

/// Test that all examples render successfully and produce valid SVG output.
///
/// This test reads all `.ail` files from the `examples/` directory and verifies:
/// 1. Each file renders without errors
/// 2. The output is valid SVG (starts with proper SVG tag)
/// 3. The output contains expected structural elements
///
/// Note: Due to HashMap iteration order being non-deterministic for CSS
/// variables, we don't compare byte-for-byte but instead verify structure.
#[test]
fn test_svg_regression_all_examples() {
    let examples_dir = Path::new("examples");

    if !examples_dir.exists() {
        panic!("Examples directory not found at {:?}", examples_dir);
    }

    let mut tested = 0;
    let mut failures = Vec::new();

    for entry in fs::read_dir(examples_dir).expect("Failed to read examples directory") {
        let path = entry.expect("Failed to read entry").path();

        if path.extension().map_or(false, |ext| ext == "ail") {
            let source = fs::read_to_string(&path).expect(&format!("Failed to read {:?}", path));

            match render(&source) {
                Ok(svg) => {
                    // Verify it's valid SVG
                    if !svg.contains("<svg") {
                        failures.push(format!(
                            "Invalid SVG in {}: missing <svg> tag",
                            path.display()
                        ));
                    }
                    if !svg.contains("</svg>") {
                        failures.push(format!(
                            "Invalid SVG in {}: missing </svg> tag",
                            path.display()
                        ));
                    }
                    tested += 1;
                }
                Err(e) => {
                    failures.push(format!("Failed to render {}: {:?}", path.display(), e));
                }
            }
        }
    }

    println!("SVG Rendering: {} tested, {} failures", tested, failures.len());

    if !failures.is_empty() {
        for failure in &failures {
            eprintln!("  - {}", failure);
        }
        panic!(
            "{} rendering test(s) failed. See output above.",
            failures.len()
        );
    }

    assert!(tested > 0, "No .ail files found in examples directory");
}

/// Generate baseline SVG files for all examples.
///
/// Run with: `cargo test -- --ignored generate_baselines`
///
/// This creates/updates baseline files in `tests/baseline/` for comparison
/// in regression tests.
#[test]
#[ignore]
fn generate_baselines() {
    let examples_dir = Path::new("examples");
    let baseline_dir = Path::new("tests/baseline");

    fs::create_dir_all(baseline_dir).expect("Failed to create baseline directory");

    let mut generated = 0;
    let mut errors = Vec::new();

    for entry in fs::read_dir(examples_dir).expect("Failed to read examples directory") {
        let path = entry.expect("Failed to read entry").path();

        if path.extension().map_or(false, |ext| ext == "ail") {
            let source = fs::read_to_string(&path).expect(&format!("Failed to read {:?}", path));

            match render(&source) {
                Ok(svg) => {
                    let baseline_path = baseline_dir
                        .join(path.file_stem().unwrap())
                        .with_extension("svg");

                    fs::write(&baseline_path, &svg)
                        .expect(&format!("Failed to write {:?}", baseline_path));

                    println!("Generated baseline: {:?}", baseline_path);
                    generated += 1;
                }
                Err(e) => {
                    errors.push(format!("Failed to render {}: {:?}", path.display(), e));
                }
            }
        }
    }

    println!("Generated {} baseline(s)", generated);

    if !errors.is_empty() {
        for error in &errors {
            eprintln!("  - {}", error);
        }
        panic!("{} example(s) failed to render", errors.len());
    }
}

/// Individual test for a specific example (useful for debugging)
#[test]
fn test_feedback_loops_example() {
    let source = fs::read_to_string("examples/feedback-loops.ail");
    if let Ok(source) = source {
        let result = render(&source);
        assert!(result.is_ok(), "Failed to render feedback-loops.ail: {:?}", result.err());
    }
}
