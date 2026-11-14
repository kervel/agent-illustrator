//! The class-box template quoted in the docs must be the one in
//! examples/class-diagram.ail, character for character: an older copy in
//! `--skill` / `--examples` is what agents copy, and they then draw old boxes.

fn template(src: &str) -> &str {
    let a = src.find("template \"cls\"").expect("template \"cls\" present");
    let b = src[a..].find("\n}\n").expect("template end") + a + 3;
    &src[a..b]
}

#[test]
fn docs_quote_the_current_class_template() {
    let example = std::fs::read_to_string("examples/class-diagram.ail").unwrap();
    let want = template(&example);
    for doc in ["docs/skill.md", "docs/examples.md"] {
        let text = std::fs::read_to_string(doc).unwrap();
        assert_eq!(template(&text), want, "{doc} quotes an outdated class template; copy it from examples/class-diagram.ail");
    }
}

#[test]
fn examples_doc_quotes_the_current_sequence_example() {
    let ex = std::fs::read_to_string("examples/sequence-diagram.ail").unwrap();
    let doc = std::fs::read_to_string("docs/examples.md").unwrap();
    assert!(doc.contains(ex.trim_end()), "docs/examples.md EXAMPLE 12 is out of date with examples/sequence-diagram.ail");
}
