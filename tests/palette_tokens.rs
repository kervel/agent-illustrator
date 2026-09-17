//! Anything the stylesheet defines must be nameable in the DSL.
//!
//! The parser enumerated five colour categories while the stylesheet is an
//! open map of tokens, so the default palette shipped three colours the
//! language could not spell: `fill: status-success` was a parse error reading
//! "Unexpected Minus", which looks like a syntax mistake in the author's file
//! rather than a gap in the grammar. The workaround is hardcoding a hex, which
//! defeats theming.

use agent_illustrator::render;

#[test]
fn a_status_token_can_be_named() {
    let svg = render(r#"rect a [width: 40, height: 20, fill: status-success]"#)
        .expect("status-success is in the default palette");
    assert!(
        svg.contains("var(--status-success)"),
        "should resolve as a palette token: {svg}"
    );
}

#[test]
fn every_status_token_can_be_named() {
    for token in ["status-success", "status-warning", "status-error"] {
        let src = format!(r#"rect a [width: 40, height: 20, fill: {token}]"#);
        let svg = render(&src).unwrap_or_else(|e| panic!("{token} should parse: {e:?}"));
        assert!(svg.contains(&format!("var(--{token})")), "{token}: {svg}");
    }
}

#[test]
fn the_established_spellings_still_work() {
    for token in [
        "foreground-1",
        "accent-2",
        "accent-dark",
        "secondary-light",
        "text-3",
        "background-light",
    ] {
        let src = format!(r#"rect a [width: 40, height: 20, fill: {token}]"#);
        let svg = render(&src).unwrap_or_else(|e| panic!("{token} should parse: {e:?}"));
        assert!(svg.contains(&format!("var(--{token})")), "{token}: {svg}");
    }
}

#[test]
fn a_plain_css_colour_is_still_passed_through() {
    let svg = render(r#"rect a [width: 40, height: 20, fill: red]"#).expect("renders");
    assert!(svg.contains(r#"fill="red""#), "got: {svg}");
}

#[test]
fn an_unknown_token_is_still_an_error_that_names_alternatives() {
    // Accepting more spellings must not turn a typo into silence.
    let err = render(r#"rect a [width: 40, height: 20, fill: status-succes]"#)
        .expect_err("a misspelled token must not be accepted");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("status-succes"),
        "the error should name what was written: {msg}"
    );
}
