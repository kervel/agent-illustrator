//! `each NAME[, i] in LIST { ... }`: one copy of the body per list item.

use agent_illustrator::render;

fn svg(src: &str) -> String {
    render(src).unwrap_or_else(|e| panic!("{e}\n---\n{src}"))
}

fn ids(svg: &str) -> Vec<String> {
    svg.split("id=\"").skip(1).filter_map(|s| s.split('"').next()).map(str::to_string).collect()
}

fn has_id(svg: &str, id: &str) -> bool {
    ids(svg).iter().any(|i| i == id)
}

const CLS: &str = r#"
template "cls" (ttl: "", attrs: [], ops: []) {
    col box [gap: 0, padding: 0] {
        rect hd [width: 220, label: ttl, font_weight: bold]
        col at [gap: 0, padding: 0] {
            each a in attrs { rect a_row [width: 220, height: 20, label: a, align: start] }
        }
        col op [gap: 0, padding: 0] {
            each o in ops { rect o_row [width: 220, height: 20, label: o, align: start] }
        }
    }
}
"#;

#[test]
fn a_template_list_parameter_drives_one_element_per_item() {
    let out = svg(&format!(r#"{CLS}
cls entry [ttl: "TimeEntry", attrs: ["id", "date", "hours"], ops: ["+ approve()"]]"#));
    for id in ["entry_a_row_1", "entry_a_row_2", "entry_a_row_3", "entry_o_row_1"] {
        assert!(has_id(&out, id), "missing {id}: {:?}", ids(&out));
    }
    assert!(!has_id(&out, "entry_a_row_4"));
    for text in ["id", "date", "hours", "+ approve()"] {
        assert!(out.contains(&format!(">{text}<")), "label {text} not drawn");
    }
}

#[test]
fn an_empty_list_makes_nothing() {
    let out = svg(&format!(r#"{CLS}
cls bare [ttl: "Bare"]
cls some [ttl: "Some", attrs: ["x"], ops: []]"#));
    assert!(!ids(&out).iter().any(|i| i.starts_with("bare_a_row") || i.starts_with("bare_o_row")), "{:?}", ids(&out));
    assert!(has_id(&out, "some_a_row_1"));
    assert!(!has_id(&out, "some_o_row_1"));
}

#[test]
fn names_are_unique_across_instances_and_targetable() {
    let out = svg(&format!(r#"{CLS}
cls one [ttl: "One", attrs: ["a", "b"]]
cls two [ttl: "Two", attrs: ["c", "d"]]
constrain two.a_row_2.left = one.a_row_2.left + 300
one.a_row_1 -> two.a_row_1"#));
    let all = ids(&out);
    let mut sorted = all.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(all.len(), sorted.len(), "duplicate ids: {all:?}");
    for id in ["one_a_row_1", "one_a_row_2", "two_a_row_1", "two_a_row_2"] {
        assert!(has_id(&out, id), "missing {id}");
    }
}

#[test]
fn nested_each_chains_the_numbers() {
    let out = svg(r#"
each r in ["a", "b"] {
    row ln [gap: 4] {
        each c in ["x", "y", "z"] { rect cell [width: 20, height: 20, label: "{r}{c}"] }
    }
}"#);
    for id in ["ln_1", "ln_2", "cell_1_1", "cell_1_3", "cell_2_1", "cell_2_3"] {
        assert!(has_id(&out, id), "missing {id}: {:?}", ids(&out));
    }
    for t in ["ax", "bz"] {
        assert!(out.contains(&format!(">{t}<")), "{t}");
    }
}

#[test]
fn the_index_is_zero_based_and_works_in_strings_and_numbers() {
    let out = svg(r#"
each n, i in ["p", "q", "r"] {
    rect bar [width: 40, height: 10, label: "{i}:{n}"]
}"#);
    for t in ["0:p", "1:q", "2:r"] {
        assert!(out.contains(&format!(">{t}<")), "{t} not drawn");
    }
    let out = svg(r#"
col c [gap: 2] {
    each n, i in [10, 20, 30] { rect w [width: i, height: 10] }
}"#);
    assert!(has_id(&out, "w_1") && has_id(&out, "w_3"));
}

#[test]
fn text_item_and_number_items() {
    let out = svg(r#"
col c [gap: 2] {
    each t in ["one", "two"] { text t tx }
    each n in [7, 8] { text n num }
}"#);
    assert!(out.contains(">one<") && out.contains(">two<"));
    assert!(out.contains(">7<") && out.contains(">8<"));
    assert!(has_id(&out, "tx_2") && has_id(&out, "num_1"));
}

#[test]
fn markup_in_items_keeps_working() {
    let out = svg(r#"each s in ["<b>bold</b> word"] { rect m [label: s, width: 120, height: 24] }"#);
    assert!(out.contains("bold"), "{out}");
    assert!(!out.contains("&lt;b&gt;"), "markup was escaped");
}

#[test]
fn top_level_each_over_a_literal_list() {
    let out = svg(r#"
row r [gap: 6] {
    each n in ["a", "b", "c"] { circle dot [size: 12] }
}
constrain dot_1.center_y = dot_3.center_y"#);
    for id in ["dot_1", "dot_2", "dot_3"] {
        assert!(has_id(&out, id));
    }
}

#[test]
fn constraints_inside_the_body_follow_the_renaming() {
    let out = svg(r#"
each n in ["a", "b"] {
    rect bg [fill: none, stroke: foreground-2]
    rect fg [width: 30, height: 10]
    constrain bg contains fg [padding: 5]
}"#);
    assert!(has_id(&out, "bg_1") && has_id(&out, "bg_2") && has_id(&out, "fg_2"));
}

#[test]
fn anchors_declared_inside_each_are_numbered_and_connectable() {
    let src = r#"
template "list" (items: []) {
    col c [gap: 0] {
        each it in items {
            rect r [width: 80, height: 20, label: it]
            anchor out [position: r.right, direction: right]
        }
    }
}
list a [items: ["x", "y"]]
rect z [width: 20, height: 20]
constrain z.left = a.right + 100
a.out_2 -> z.left"#;
    svg(src);
}

#[test]
fn a_list_argument_reaches_the_body_but_not_a_label() {
    let err = render(r#"
template "t" (items: []) { rect b [label: items] }
t x [items: ["a", "b"]]"#)
        .unwrap_err()
        .to_string();
    assert!(err.contains("each"), "{err}");
}

#[test]
fn unknown_list_name_is_a_clear_error() {
    let err = render(r#"
template "t" (items: []) { each i in itms { rect r } }
t x [items: ["a"]]"#)
        .unwrap_err()
        .to_string();
    assert!(err.contains("itms") && err.contains("items"), "{err}");
    let err = render(r#"each a in things { rect r }"#).unwrap_err().to_string();
    assert!(err.contains("things"), "{err}");
}

#[test]
fn each_over_a_non_list_says_what_it_got() {
    let err = render(r#"each a in 5 { rect r }"#).unwrap_err().to_string();
    assert!(err.contains("list") && err.contains("5"), "{err}");
    let err = render(r#"each a in "abc" { rect r }"#).unwrap_err().to_string();
    assert!(err.contains("list") && err.contains("abc"), "{err}");
    let err = render(r#"
template "t" (items: "x") { each i in items { rect r } }
t y"#)
        .unwrap_err()
        .to_string();
    assert!(err.contains("not a list") && err.contains("items"), "{err}");
}

#[test]
fn a_missing_in_is_an_error_that_shows_the_form() {
    let err = render(r#"each a { rect r }"#).unwrap_err().to_string();
    assert!(err.contains("`in`") && err.contains("each a in"), "{err}");
    let err = render(r#"each a ["x"] { rect r }"#).unwrap_err().to_string();
    assert!(err.contains("`in`"), "{err}");
}

#[test]
fn each_and_in_are_reserved() {
    let err = render("rect each").unwrap_err().to_string();
    assert!(err.contains("reserved"), "{err}");
    // `in` stays an ordinary word outside `each` (`ease: in`).
    assert!(render("rect in").is_ok());
}
