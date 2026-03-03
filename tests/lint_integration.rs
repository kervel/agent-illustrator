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
    assert!(
        categories.contains(&"reducible-bend".to_string()),
        "Expected reducible-bend warning, got: {:?}",
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
            ["overlap", "containment", "label", "connection", "alignment", "redundant-constant", "reducible-bend"].contains(&cat.as_str()),
            "Unexpected category: {}",
            cat
        );
        assert!(!w.message.is_empty(), "Warning message should not be empty");
    }
}

#[test]
fn test_redundant_constant_true_positive() {
    let source = include_str!("lint-fixtures/true-positives.ail");
    let config = RenderConfig::new().with_lint(true);
    let (_, warnings) = render_with_lint(source, config).expect("Should render");

    let redundant: Vec<_> = warnings
        .iter()
        .filter(|w| w.category.to_string() == "redundant-constant")
        .collect();
    assert!(
        !redundant.is_empty(),
        "Expected redundant-constant warnings for true-positives fixture"
    );
}

#[test]
fn test_shacl_overview_redundant_constants() {
    let source = include_str!("lint-fixtures/shacl-overview.ail");
    let config = RenderConfig::new().with_lint(true);
    let (svg, warnings) = render_with_lint(source, config).expect("Should render");

    assert!(svg.contains("<svg"));

    let redundant: Vec<_> = warnings
        .iter()
        .filter(|w| w.category.to_string() == "redundant-constant")
        .collect();
    assert!(
        !redundant.is_empty(),
        "Expected redundant-constant warnings for SHACL overview"
    );

    // Should have multiple groups (5 center_x columns + center_y groups)
    assert!(
        redundant.len() >= 5,
        "Expected at least 5 redundant-constant warnings, got {}",
        redundant.len()
    );
}

#[test]
fn test_label_element_straddle_true_positive() {
    let source = include_str!("lint-fixtures/true-positives.ail");
    let config = RenderConfig::new().with_lint(true);
    let (_, warnings) = render_with_lint(source, config).expect("Should render");

    let straddle: Vec<_> = warnings
        .iter()
        .filter(|w| w.category.to_string() == "label" && w.message.contains("straddles"))
        .collect();
    assert!(
        !straddle.is_empty(),
        "Expected label-straddle warnings for true-positives fixture"
    );
    // Our test case: straddle_box_a's long label crosses into straddle_box_b
    assert!(
        straddle.iter().any(|w| w.message.contains("straddle_box_a") && w.message.contains("straddle_box_b")),
        "Expected straddle warning for straddle_box_a→straddle_box_b, got: {:?}",
        straddle.iter().map(|w| &w.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_label_element_straddle_true_negative() {
    let source = include_str!("lint-fixtures/true-negatives.ail");
    let config = RenderConfig::new().with_lint(true);
    let (_, warnings) = render_with_lint(source, config).expect("Should render");

    let straddle: Vec<_> = warnings
        .iter()
        .filter(|w| w.message.contains("straddles"))
        .collect();
    assert!(
        straddle.is_empty(),
        "Expected no label-straddle warnings for true-negatives, got: {:?}",
        straddle.iter().map(|w| &w.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_shacl_overview_reducible_bends() {
    let source = include_str!("lint-fixtures/shacl-overview.ail");
    let config = RenderConfig::new().with_lint(true);
    let (svg, warnings) = render_with_lint(source, config).expect("Should render");

    assert!(svg.contains("<svg"));

    let reducible: Vec<_> = warnings
        .iter()
        .filter(|w| w.category.to_string() == "reducible-bend")
        .collect();
    assert!(
        !reducible.is_empty(),
        "Expected reducible-bend warnings for SHACL overview"
    );
}
