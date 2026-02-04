//! Integration tests for the --lint feature

use agent_illustrator::{render_with_config, render_with_lint, RenderConfig};

#[test]
fn test_true_positives_all_categories() {
    let source = include_str!("lint-fixtures/true-positives.ail");
    let config = RenderConfig::new().with_lint(true);
    let (svg, warnings) = render_with_lint(source, config).expect("Should render");

    // SVG is still produced
    assert!(svg.contains("<svg"));

    // Should have warnings in multiple categories
    assert!(!warnings.is_empty(), "Expected lint warnings for true-positives");

    let categories: Vec<String> = warnings.iter().map(|w| w.category.to_string()).collect();
    assert!(
        categories.contains(&"overlap".to_string()),
        "Expected overlap warning, got: {:?}",
        categories
    );
    assert!(
        categories.contains(&"containment".to_string()),
        "Expected containment warning, got: {:?}",
        categories
    );
    assert!(
        categories.contains(&"connection".to_string()),
        "Expected connection warning, got: {:?}",
        categories
    );
}

#[test]
fn test_true_negatives_clean() {
    let source = include_str!("lint-fixtures/true-negatives.ail");
    let config = RenderConfig::new().with_lint(true);
    let (svg, warnings) = render_with_lint(source, config).expect("Should render");

    // SVG is still produced
    assert!(svg.contains("<svg"));

    // Should have zero warnings
    assert!(
        warnings.is_empty(),
        "Expected no warnings for true-negatives, got: {:?}",
        warnings.iter().map(|w| format!("{}: {}", w.category, w.message)).collect::<Vec<_>>()
    );
}

#[test]
fn test_lint_disabled_no_warnings() {
    let source = include_str!("lint-fixtures/true-positives.ail");
    // lint: false (default)
    let config = RenderConfig::new();
    let (_, warnings) = render_with_lint(source, config).expect("Should render");

    // When lint is disabled, no warnings should be computed
    assert!(warnings.is_empty(), "Lint disabled should produce no warnings");
}

#[test]
fn test_render_with_config_still_works() {
    // render_with_config returns just String, no warnings
    let source = include_str!("lint-fixtures/true-positives.ail");
    let config = RenderConfig::new();
    let svg = render_with_config(source, config).expect("Should render");
    assert!(svg.contains("<svg"));
}

#[test]
fn test_lint_warning_format() {
    let source = include_str!("lint-fixtures/true-positives.ail");
    let config = RenderConfig::new().with_lint(true);
    let (_, warnings) = render_with_lint(source, config).expect("Should render");

    for w in &warnings {
        let cat = w.category.to_string();
        assert!(
            ["overlap", "containment", "label", "connection", "alignment"].contains(&cat.as_str()),
            "Unexpected category: {}",
            cat
        );
        assert!(!w.message.is_empty(), "Warning message should not be empty");
    }
}
