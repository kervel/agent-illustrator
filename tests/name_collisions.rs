//! A part `b` of instance `a` is named `a_b` internally. Two declarations
//! of one name used to collapse silently into one element (the other was
//! gone from the render, and `hide a.b` hit whichever survived). Every
//! repeated name is now an error that locates both declarations.

use agent_illustrator::{render_with_config, RenderConfig};

fn err(src: &str) -> String {
    match render_with_config(src, RenderConfig::new()) {
        Ok(_) => panic!("expected a name-collision error"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn a_user_name_equal_to_a_one_part_templates_path_is_an_error() {
    // The reviewer's repro: the template's only part is folded into the
    // instance, yet `a.b` still names `a_b`.
    let e = err(include_str!("names/coll.ail"));
    assert!(e.contains("both named `a_b`"), "{e}");
    assert!(e.contains("part `a.b` of template instance `a`"), "{e}");
    assert!(e.contains("line 5"), "names the user's rect: {e}");
    assert!(e.contains("line 4"), "names the instance: {e}");
}

#[test]
fn a_user_name_equal_to_a_part_of_a_many_part_template_is_an_error() {
    let e = err("template \"t\" () {\n  rect b\n  rect c\n}\nt a\nrect a_b\n");
    assert!(e.contains("both named `a_b`"), "{e}");
}

#[test]
fn a_user_name_equal_to_a_nested_part_is_an_error() {
    let e = err(
        "template \"in\" () {\n  rect x\n  rect y\n}\ntemplate \"out\" () {\n  in p\n  rect q\n}\nout o\nrect o_p_x\n",
    );
    assert!(e.contains("both named `o_p_x`"), "{e}");
}

#[test]
fn two_plain_declarations_of_one_name_are_an_error() {
    let e = err("rect x\nrect x\n");
    assert!(e.contains("line 1") && e.contains("line 2"), "{e}");
}

#[test]
fn a_connection_named_like_an_element_is_an_error() {
    let e = err("rect a\nrect b\nrect c\na -> b as c\n");
    assert!(e.contains("both named `c`"), "{e}");
}

#[test]
fn two_keyframes_with_one_name_are_an_error() {
    let e = err("rect a\nkeyframe \"k\" { hide a }\nkeyframe \"k\" { show a }\n");
    assert!(e.contains("two keyframes are both named \"k\""), "{e}");
}

#[test]
fn an_underscore_name_that_is_not_a_part_is_fine() {
    let svg = render_with_config(
        "template \"t\" () {\n  rect b\n  rect c\n}\nt a\nrect a_z\n",
        RenderConfig::new(),
    )
    .expect("a_z is not a part of a");
    assert!(svg.contains(r#"id="a_z""#) && svg.contains(r#"id="a_b""#));
}
