//! A `{name}` that no template parameter or `each` variable binds is reported.

use agent_illustrator::{render_with_lint, RenderConfig};

fn lint(source: &str) -> Result<Vec<String>, String> {
    let (_svg, w) = render_with_lint(source, RenderConfig::new().with_lint(true)).map_err(|e| format!("{e:?}"))?;
    Ok(w.iter().map(|w| w.message.clone()).collect())
}

#[test]
fn unbound_each_variable_in_text_is_reported() {
    let src = "template \"t\" (items: []) {\n    col c { each a in items { text zz } }\n}\nt x [items: [\"p\"]]";
    let w = lint(src).expect("lints");
    assert!(w.iter().any(|m| m.contains("{zz}")), "{w:?}");
}

#[test]
fn bound_variables_and_plain_braces_are_not_reported() {
    let src = "template \"t\" (items: []) {\n    col c { each a in items { text a } }\n}\nt x [items: [\"p\"]]\nrect r [label: \"{unique (a, b)}\"]";
    let w = lint(src).expect("lints");
    assert!(!w.iter().any(|m| m.contains("literal text")), "{w:?}");
}
