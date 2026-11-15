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

#[test]
fn index_plus_one_numbers_items_from_one() {
    let out = svg(r#"
each n, i in ["a", "b", "c"] { rect r [width: 40, height: 20, label: "{i+1}. {n}"] }"#);
    for t in ["1. a", "2. b", "3. c"] {
        assert!(out.contains(&format!(">{t}<")), "missing {t}");
    }
}

// ---- loop variables where a name goes --------------------------------

fn connections(svg: &str) -> usize {
    svg.matches("class=\"ai-connection\"").count()
}

/// x, y, width, height of `<rect id="ID" ...>`.
fn rect_of(svg: &str, id: &str) -> (f64, f64, f64, f64) {
    let start = svg.find(&format!("<rect id=\"{id}\"")).unwrap_or_else(|| panic!("no rect {id}"));
    let tag = &svg[start..start + svg[start..].find('>').unwrap()];
    let attr = |k: &str| -> f64 {
        let at = tag.find(&format!(" {k}=\"")).unwrap_or_else(|| panic!("{k} in {tag}")) + k.len() + 3;
        tag[at..at + tag[at..].find('"').unwrap()].parse().unwrap()
    };
    (attr("x"), attr("y"), attr("width"), attr("height"))
}

#[test]
fn a_loop_variable_is_a_connection_endpoint() {
    let out = svg(r#"
row r [gap: 60] { rect srv [label: "server"]  col cs [gap: 20] { rect c1  rect c2  rect c3 } }
each c in [c1, c2, c3] { srv.right -> c.left [label: "data"] }"#);
    assert_eq!(connections(&out), 3);
}

#[test]
fn a_loop_variable_works_in_constrain_and_contains() {
    let out = svg(r#"
row r [gap: 60] { rect a [label: "a"]  rect b [label: "b"] }
each n in [a, b] { constrain n.width = 50 }
rect frame [fill: none, stroke: foreground-2]
each n in [a] { constrain frame contains n, b [padding: 10] }"#);
    let (ax, _, aw, _) = rect_of(&out, "a");
    let (bx, _, bw, _) = rect_of(&out, "b");
    assert_eq!((aw, bw), (50.0, 50.0));
    let (fx, _, fw, _) = rect_of(&out, "frame");
    assert!(fx <= ax - 10.0 + 0.01 && fx + fw >= bx + bw + 10.0 - 0.01, "frame {fx}+{fw} around {ax}..{}", bx + bw);
}

#[test]
fn a_loop_variable_works_in_via_lists_and_anchor_positions() {
    let out = svg(r#"
row r [gap: 80] { rect a [label: "a"]  rect hub [label: "hub"] }
point w1
point w2
constrain w1.center_x = a.center_x
constrain w1.center_y = a.top - 40
constrain w2.center_x = hub.center_x
constrain w2.center_y = w1.center_y
each n in [w1] { a.top -> hub.top [routing: direct, via: [n, w2]] }
template "pair" (outs: []) {
    row two [gap: 20] { rect l [width: 30, height: 20]  rect rr [width: 30, height: 20] }
    each n in outs { anchor out [position: n.bottom, direction: down] }
}
pair p [outs: [rr]]
constrain p.top = hub.bottom + 40
rect z [width: 20, height: 20]
constrain z.top = p.bottom + 40
p.out_1 -> z.top"#);
    assert_eq!(connections(&out), 2);
}

#[test]
fn a_dotted_item_names_a_part_of_an_instance() {
    let out = svg(r#"
template "board" () { rect body [width: 80, height: 40]  circle led [size: 10] }
row r [gap: 80] { board b1  board b2  rect hub [label: "hub"] }
each n in [b1.led, b2.led] { n.top -> hub [routing: direct]  constrain n.center_y = b1.body.center_y }"#);
    assert_eq!(connections(&out), 2);
}

#[test]
fn inside_a_template_loop_names_are_the_instance_parts() {
    let out = svg(r#"
template "panel" (lit: []) {
    row leds [gap: 20] { circle l1 [size: 14]  circle l2 [size: 14]  circle l3 [size: 14] }
    rect bar [width: 120, height: 10]
    constrain bar.top = leds.bottom + 30
    each n in lit { bar -> n }
}
panel p [lit: [l1, l3]]
panel q [lit: [l2]]"#);
    assert_eq!(connections(&out), 3);
}

#[test]
fn a_loop_variable_that_hides_an_element_used_as_a_name_is_an_error() {
    let err = render(r#"
row r { rect a [label: "a"]  rect hub [label: "h"] }
each a in [hub] { a -> hub }"#)
        .unwrap_err()
        .to_string();
    assert!(err.contains("`a` is also the name of an element") && err.contains("rename the loop variable"), "{err}");
    // Used only as a value (a label), the same name is fine.
    assert!(render(r#"
rect a [label: "a"]
each a in ["x"] { rect r [label: a] }"#)
        .is_ok());
}

#[test]
fn a_string_or_the_index_where_a_name_goes_says_what_to_write() {
    let err = render(r#"
row r { rect a [label: "a"]  rect hub [label: "h"] }
each n in ["a"] { n -> hub }"#)
        .unwrap_err()
        .to_string();
    assert!(err.contains("the string \"a\"") && err.contains("without quotes"), "{err}");
    let err = render(r#"
row r { rect a [label: "a"]  rect hub [label: "h"] }
each n, i in [a] { i -> hub }"#)
        .unwrap_err()
        .to_string();
    assert!(err.contains("position") && err.contains("each it, i in"), "{err}");
}

#[test]
fn a_typo_in_the_list_is_reported_at_the_list_item() {
    let src = r#"row r { rect a [label: "a"]  rect hub [label: "h"] }
each n in [a, bx] { n -> hub }"#;
    let err = render(src).unwrap_err().to_string();
    assert!(err.contains("'bx'"), "{err}");
    // The caret sits under `bx` in the list, not under `n` in the body.
    let line = err.lines().find(|l| l.contains("each n in")).expect(&err);
    let caret = err.lines().skip_while(|l| !l.contains("each n in")).nth(1).expect(&err);
    assert_eq!(caret.find('^'), line.find("bx"), "{err}");
}
