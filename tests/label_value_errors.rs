//! A `label:` that is not a string used to vanish without a trace.

use agent_illustrator::render;

#[test]
fn a_list_label_is_an_error_that_points_to_each() {
    let err = render(r#"rect a [label: ["x", "y"]]"#).unwrap_err().to_string();
    assert!(err.contains("not a list"), "{err}");
    assert!(err.contains("<br>"), "{err}");
}

#[test]
fn a_number_label_asks_for_quotes() {
    let err = render("rect a [label: 42]").unwrap_err().to_string();
    assert!(err.contains("quote"), "{err}");
}

#[test]
fn a_string_label_still_renders() {
    assert!(render(r#"rect a [label: "x<br>y"]"#).is_ok());
}
