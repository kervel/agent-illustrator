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

// ---- records: one item, several fields --------------------------------

const STATES: &str = r#"row r [gap: 80] { rect idle [label: "Idle"]  rect run [label: "Running"]  rect done [label: "Done"] }
"#;

#[test]
fn a_record_field_works_in_labels_strings_endpoints_and_constraints() {
    let out = svg(&format!(r#"{STATES}
each t, i in [{{from: idle, to: run, label: "start"}}, {{from: run, to: done, label: "finish"}}] {{
    t.from -> t.to [label: "{{i+1}}: {{t.label}}"]
    constrain t.to.width = 120
}}
each n in [{{at: done, text: "last", tint: role-ok-soft}}] {{
    rect tag [label: n.text, fill: n.tint, width: 60, height: 20]
    constrain tag.top = n.at.bottom + 10
    text n.text cap [font_size: 10]
    constrain cap.top = tag.bottom + 4
}}"#));
    assert_eq!(connections(&out), 2);
    for t in [">1: start<", ">2: finish<"] {
        assert!(out.contains(t), "missing {t}");
    }
    assert_eq!(out.matches(">last<").count(), 2);
    assert_eq!(rect_of(&out, "run").2, 120.0);
    assert_eq!(rect_of(&out, "done").2, 120.0);
    assert!(out.contains("role-ok-soft"), "the tint field reached fill:");
}

#[test]
fn reserved_words_name_fields() {
    // `from`, `label`, `text` are keywords elsewhere; in a record and after `m.` they are fields.
    let out = svg(&format!(r#"{STATES}
each m in [{{from: idle, to: run, label: "go", text: "t"}}] {{ m.from -> m.to [label: m.label] }}"#));
    assert!(out.contains(">go<"));
    // ... and outside records they keep their meaning.
    svg(r#"row r { rect a [label: "a"]  rect b [label: "b"] }
a.right -> b.left [label: "x"]
constrain b.top = a.top"#);
}

#[test]
fn a_record_field_reaches_a_template_argument() {
    let out = svg(r#"
template "pill" (txt: "") { rect body [label: txt, width: 90, height: 30]  circle dot [size: 6] }
row r [gap: 20] { each s in [{label: "Alpha"}, {label: "Beta"}] { pill p [txt: s.label] } }"#);
    assert!(out.contains(">Alpha<") && out.contains(">Beta<"));
}

#[test]
fn an_unknown_field_lists_the_fields_and_suggests_one() {
    let err = render(&format!(r#"{STATES}
each t in [{{from: idle, to: run, label: "a"}}, {{from: run, to: done, label: "b"}}] {{ t.from -> t.to [label: t.lable] }}"#))
        .unwrap_err()
        .to_string();
    assert!(err.contains("`t.lable`") && err.contains("item 1 has: from, to, label"), "{err}");
    assert!(err.contains("did you mean `label`"), "{err}");
}

#[test]
fn a_field_missing_from_some_items_names_them() {
    let err = render(&format!(r#"{STATES}
each t in [{{from: idle, to: run, label: "a"}}, {{from: run, to: done}}, {{from: done, to: idle}}] {{ t.from -> t.to [label: t.label] }}"#))
        .unwrap_err()
        .to_string();
    assert!(err.contains("items 2 and 3 have no field `label`") && err.contains("add `label: ...`"), "{err}");
}

#[test]
fn a_whole_record_is_an_error_that_lists_its_fields() {
    for body in ["t -> idle", "rect x [label: t]", "text t q", r#"text "{t}" q"#, "constrain t.width = 4"] {
        let err = render(&format!(r#"{STATES}
each t in [{{from: idle, to: run}}] {{ {body} }}"#))
            .unwrap_err()
            .to_string();
        assert!(err.contains("`t` is a whole record; pick a field: t.from, t.to"), "{body}: {err}");
    }
}

#[test]
fn a_quoted_name_in_a_field_says_to_drop_the_quotes() {
    let err = render(&format!(r#"{STATES}
each t in [{{from: "idle", to: run}}] {{ t.from -> t.to }}"#))
        .unwrap_err()
        .to_string();
    assert!(err.contains("`t.from` (item 1) is the string \"idle\"") && err.contains("without quotes"), "{err}");
}

#[test]
fn a_typo_in_a_field_is_reported_at_the_field() {
    let src = format!(r#"{STATES}each t in [{{from: idle, to: rnu}}] {{ t.from -> t.to }}"#);
    let err = render(&src).unwrap_err().to_string();
    let line = err.lines().find(|l| l.contains("each t in")).expect(&err);
    let caret = err.lines().skip_while(|l| !l.contains("each t in")).nth(1).expect(&err);
    assert_eq!(caret.find('^'), line.find("rnu"), "{err}");
}

#[test]
fn a_hyphenated_bare_value_shows_as_written_in_a_string() {
    // `diamond-filled` unquoted parses as a (palette) token, not a plain word:
    // `{r.look}` stayed literally `{r.look}`, with no diagnostic.
    let out = svg(r#"
col list {
    each r in [{look: diamond-filled}, {look: crow}, {look: accent-1}] {
        text "end: {r.look}"
    }
}
"#);
    assert!(out.contains("end: diamond-filled"), "{out}");
    assert!(out.contains("end: crow"), "{out}");
    assert!(out.contains("end: accent-1"), "{out}");
    assert!(!out.contains("{r.look}"), "{out}");
}

#[test]
fn one_unquoted_end_value_sets_the_end_and_shows_as_text() {
    let quoted = svg(r#"
row pair [gap: 120] { rect a  rect b }
each r in [{fe: "diamond-filled"}] {
    a -- b [from_end: r.fe]
    text "from_end: {r.fe}" code
}
"#);
    let bare = svg(r#"
row pair [gap: 120] { rect a  rect b }
each r in [{fe: diamond-filled}] {
    a -- b [from_end: r.fe]
    text "from_end: {r.fe}" code
}
"#);
    assert!(bare.contains("from_end: diamond-filled"), "{bare}");
    assert_eq!(quoted, bare);
}

// ---- S3: indexed names `x.p[k]` ----------------------------------------

const LIVES: &str = r#"
template "life" (ticks: []) {
    col rows [gap: 8, padding: 0] {
        each t in ticks { rect p [width: 20, height: 16] }
    }
}
row actors [gap: 120] {
    life api [ticks: [1, 2, 3]]
    life db [ticks: [1, 2, 3]]
}
"#;

#[test]
fn an_indexed_name_is_the_numbered_copy_everywhere_a_name_goes() {
    let plain = svg(&format!(r#"{LIVES}
point w
constrain w.center_x = api.p_2.center_x + 40
constrain w.center_y = api.p_2.center_y
api.p_1 -> db.p_1 [routing: direct]
api.p_2.right -> db.p_3.left [via: [w]]
rect frame [fill: none, overlaps: all]
constrain frame contains api.p_2, db.p_3 [padding: 4]
"#));
    let indexed = svg(&format!(r#"{LIVES}
point w
constrain w.center_x = api.p[2].center_x + 40
constrain w.center_y = api.p[2].center_y
api.p[1] -> db.p[1] [routing: direct]
api.p[2].right -> db.p[3].left [via: [w]]
rect frame [fill: none, overlaps: all]
constrain frame contains api.p[2], db.p[3] [padding: 4]
"#));
    assert_eq!(plain, indexed);
}

#[test]
fn an_index_may_be_computed_from_the_loop_index() {
    let plain = svg(&format!(r#"{LIVES}
api.p_1 -> db.p_1
api.p_2 -> db.p_2
db.p_2 -> api.p_1
db.p_3 -> api.p_2
"#));
    let computed = svg(&format!(r#"{LIVES}
each m, i in ["a", "b"] {{
    api.p[i+1] -> db.p[i+1]
}}
each m, i in ["a", "b"] {{
    db.p[i+2] -> api.p[i+1]
}}
"#));
    assert_eq!(plain, computed);
}

#[test]
fn a_pipeline_wires_the_copies_of_one_each_with_another() {
    let out = svg(r#"
row flow [gap: 40] {
    each s in ["fetch", "build", "test"] { rect box [label: s] }
}
each k in [1, 2] { box[k].right -> box[k+1].left }
"#);
    let hand = svg(r#"
row flow [gap: 40] {
    each s in ["fetch", "build", "test"] { rect box [label: s] }
}
box_1.right -> box_2.left
box_2.right -> box_3.left
"#);
    assert_eq!(out, hand);
}

#[test]
fn an_index_may_be_a_record_field_even_a_reserved_word() {
    let plain = svg(&format!(r#"{LIVES}
api.p_3 -> db.p_3 [label: "x"]
db.p_1 -> api.p_1 [label: "y"]
"#));
    let fields = svg(&format!(r#"{LIVES}
each m in [{{from: api, to: db, row: 3, label: "x"}}, {{from: db, to: api, row: 1, label: "y"}}] {{
    m.from.p[m.row] -> m.to.p[m.row] [label: m.label]
}}
"#));
    assert_eq!(plain, fields);
}

#[test]
fn a_word_index_names_the_copy_with_that_suffix() {
    let base = r#"
col c { rect p_query  rect p_rows }
rect hub
"#;
    let plain = svg(&format!("{base}p_query -> hub\np_rows -> hub\n"));
    let literal = svg(&format!("{base}p[query] -> hub\np[rows] -> hub\n"));
    let field = svg(&format!("{base}each m in [{{id: query}}, {{id: rows}}] {{ p[m.id] -> hub }}\n"));
    assert_eq!(plain, literal);
    assert_eq!(plain, field);
}

#[test]
fn a_touching_bracket_is_an_index_a_spaced_one_a_modifier_block() {
    // `rect a [x]` and `a -> b [auto]` keep their modifier blocks; a block
    // with `key: value` is never an index, spaced or not.
    let spaced = svg("rect a [fill: red]\nrect b\na -> b [routing: direct]\n");
    let touching = svg("rect a[fill: red]\nrect b\na -> b[routing: direct]\n");
    assert_eq!(spaced, touching);
}

#[test]
fn an_index_from_an_outer_each_reaches_into_a_nested_one() {
    let plain = svg(&format!(r#"{LIVES}
api.p_1 -> db.p_1
api.p_1 -> db.p_2
api.p_2 -> db.p_1
api.p_2 -> db.p_2
"#));
    let nested = svg(&format!(r#"{LIVES}
each a, i in ["x", "y"] {{
    each b, j in ["u", "v"] {{ api.p[i+1] -> db.p[j+1] }}
}}
"#));
    assert_eq!(plain, nested);
}

fn err_of(src: &str) -> String {
    match render(src) {
        Ok(_) => panic!("expected an error\n---\n{src}"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn p_of_i_hints_that_copies_count_from_one() {
    let err = err_of(&format!("{LIVES}\neach m, i in [\"a\", \"b\"] {{ api.p[i] -> db.p[i+1] }}\n"));
    assert!(err.contains("api.p_0"), "{err}");
    assert!(err.contains("`api.p` has copies 1 to 3"), "{err}");
    assert!(err.contains("write api.p[i+1], not api.p[i]"), "{err}");
}

#[test]
fn an_index_past_the_last_copy_names_the_copies_there_are() {
    for line in ["api.p[7] -> db.p[1]", "constrain api.p[7].left = db.p[1].right + 10"] {
        let err = err_of(&format!("{LIVES}\n{line}\n"));
        assert!(err.contains("undefined identifier 'api.p_7'"), "{line}: {err}");
        assert!(err.contains("`api.p` has copies 1 to 3 (api.p_1 … api.p_3); there is no copy 7"), "{line}: {err}");
    }
}

#[test]
fn a_computed_index_outside_its_each_is_an_error() {
    let err = err_of("rect a\nrect b\na.p[i+1] -> b\n");
    assert!(err.contains("`i` is not bound here") && err.contains("only works inside `each m, i in ...`"), "{err}");
    let err = err_of("rect a\nrect b\na.p[m.row] -> b\n");
    assert!(err.contains("`m` is not bound here"), "{err}");
    // A sibling block's variable is not bound either.
    let err = err_of("each m in [a] { rect x }\nrect b\nx.p[m.row] -> b\n");
    assert!(err.contains("`m` is not bound here"), "{err}");
}

#[test]
fn an_index_must_be_a_whole_number_or_a_name() {
    let err = err_of("rect a\nrect b\na[2.5] -> b\n");
    assert!(err.contains("`a[2.5]`: an index is a whole number"), "{err}");
    let err = err_of("rect a\nrect b\na[-1] -> b\n");
    assert!(err.contains("never negative"), "{err}");
    let err = err_of(&format!("{LIVES}\neach m in [{{r: 1.5}}] {{ api.p[m.r] -> db.p[1] }}\n"));
    assert!(err.contains("`api.p[m.r]`: `m.r` (item 1) is 1.5; an index is a whole number"), "{err}");
    let err = err_of(&format!("{LIVES}\neach m, i in [\"a\"] {{ api.p[i-1] -> db.p[1] }}\n"));
    assert!(err.contains("`i-1` (item 1) is -1") && err.contains("never negative"), "{err}");
    let err = err_of(&format!("{LIVES}\neach m in [{{r: \"two words\"}}] {{ api.p[m.r] -> db.p[1] }}\n"));
    assert!(err.contains("is the string \"two words\"; an index is a whole number or a name"), "{err}");
}

#[test]
fn an_indexed_name_works_as_a_template_argument_and_in_a_via_list() {
    let base = format!(r#"{LIVES}
template "mark" (at: "") {{
    circle dot [size: 6]
    constrain dot.center_x = at.center_x
    constrain dot.center_y = at.center_y
}}
col ws {{ each n in [1, 2] {{ rect w [size: 4] }} }}
"#);
    let plain = svg(&format!("{base}mark mk_1 [at: api.p_2]\napi.p_1 -> db.p_1 [via: [w_2]]\n"));
    let literal = svg(&format!("{base}mark mk_1 [at: api.p[2]]\napi.p[1] -> db.p[1] [via: [w[2]]]\n"));
    let computed = svg(&format!(
        "{base}each m, i in [{{r: 2}}] {{\n  mark mk [at: api.p[m.r]]\n  api.p[i+1] -> db.p[i+1] [via: [w[m.r]]]\n}}\n"
    ));
    assert_eq!(plain, literal);
    assert_eq!(plain, computed);
}

// ---- naming copies by the items (keyed copies) ---------------------------

fn render_err(src: &str) -> String {
    match render(src) {
        Ok(_) => panic!("expected an error\n---\n{src}"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn by_a_field_names_the_copies_by_it() {
    let out = svg(r#"
col lanes {
    each l by id in [{id: cust, name: "Customer"}, {id: ops, name: "Operations"}] {
        rect lane [label: l.name]
    }
}
rect hub
lane_ops -> hub
"#);
    assert!(has_id(&out, "lane_cust"), "{:?}", ids(&out));
    assert!(has_id(&out, "lane_ops"));
    assert!(!has_id(&out, "lane_1"));
}

#[test]
fn a_keyed_copy_is_reached_with_an_index() {
    let base = r#"
col lanes {
    each l by id in [{id: cust}, {id: ops}] {
        rect lane
    }
}
rect hub
"#;
    assert_eq!(svg(&format!("{base}lane_ops -> hub\n")), svg(&format!("{base}lane[ops] -> hub\n")));
}

#[test]
fn the_position_still_counts_from_zero_with_keyed_copies() {
    let out = svg(r#"
col c {
    each l, i by id in [{id: a}, {id: b}] {
        rect r [label: "row {i+1}"]
    }
}
"#);
    assert!(has_id(&out, "r_a") && has_id(&out, "r_b"), "{:?}", ids(&out));
    assert!(out.contains(">row 2<"));
}

#[test]
fn a_key_missing_from_an_item_is_an_error() {
    let e = render_err("col c {\n each l by id in [{id: a, n: 1}, {n: 2}] {\n rect r\n }\n}\n");
    assert!(e.contains("item 2 has no `id`"), "{e}");
}

#[test]
fn a_duplicate_id_is_an_error() {
    let e = render_err("col c {\n each l by id in [{id: a}, {id: a}] {\n rect r\n }\n}\n");
    assert!(e.contains("items 1 and 2 both have id `a`"), "{e}");
}

#[test]
fn a_quoted_id_is_an_error() {
    let e = render_err("col c {\n each l by id in [{id: \"a b\"}] {\n rect r\n }\n}\n");
    assert!(e.contains("`id` must be a plain word"), "{e}");
}

#[test]
fn an_id_field_without_by_is_plain_data() {
    // Records often carry an id as data: no `by`, no keying, numbered copies.
    let out = svg(r#"
col c {
    each t in [{id: "T-101", title: "Login"}, {id: "T-101", title: "Logout"}] {
        rect card [label: "{t.id}: {t.title}"]
    }
}
"#);
    assert!(has_id(&out, "card_1") && has_id(&out, "card_2"), "{:?}", ids(&out));
    assert!(out.contains("T-101: Logout"));
}

#[test]
fn by_needs_records() {
    let e = render_err("col c {\n each l by id in [a, b] {\n rect r\n }\n}\n");
    assert!(e.contains("item 1 is not a record"), "{e}");
}

#[test]
fn a_name_indexed_by_the_item_keeps_that_name() {
    // `rect lane[l]` is already one per item: no `_1` is added.
    let out = svg(r#"
col lanes {
    each l in [cust, ops] {
        rect lane[l]
    }
}
rect hub
lane_cust -> hub
"#);
    assert!(has_id(&out, "lane_cust") && has_id(&out, "lane_ops"), "{:?}", ids(&out));
}

#[test]
fn two_items_making_one_indexed_name_is_an_error() {
    let e = render_err("col c {\n each l in [a, a] {\n rect r[l]\n }\n}\n");
    assert!(e.contains("items 1 and 2 both make an element called `r_a`"), "{e}");
}

// ---- S4: `let NAME = [...]`, a list with a name ----------------------

fn labels(svg: &str) -> Vec<String> {
    svg.split('>')
        .filter_map(|s| s.split('<').next())
        .filter(|t| !t.trim().is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn a_let_is_an_each_source_for_several_blocks() {
    let out = svg(r#"
let tr = [{from: a, to: b, on: "go"}, {from: b, to: a, on: "back"}]
row r [gap: 80] { rect a  rect b }
each t in tr { t.from -> t.to [label: t.on] }
col legend { each t, i in tr { text "{i+1}. {t.on}" ev } }
"#);
    assert_eq!(connections(&out), 2);
    assert!(has_id(&out, "ev_1") && has_id(&out, "ev_2"), "{:?}", ids(&out));
    for t in ["go", "back", "1. go", "2. back"] {
        assert!(labels(&out).iter().any(|l| l == t), "missing {t}");
    }
}

#[test]
fn order_does_not_matter() {
    let out = svg(r#"
each n in names { rect box [label: n] }
let names = ["x", "y"]
"#);
    assert!(has_id(&out, "box_1") && has_id(&out, "box_2"), "{:?}", ids(&out));
}

#[test]
fn a_let_is_a_template_argument_a_default_and_reaches_nested_instances() {
    let out = svg(r#"
let msgs = [{id: login, label: "login"}, {id: ok, label: "ok"}]
template "life" (ticks: msgs) {
    col c [gap: 4] { each t by id in ticks { rect p [label: t.label, width: 60, height: 20] } }
}
template "pair" (rows: []) { row r { life x [ticks: rows] } }
row actors [gap: 40] {
    life cl [ticks: msgs]
    life api
}
pair pp [rows: msgs]
cl.p[login] -> api.p[ok]
"#);
    for id in ["cl_p_login", "cl_p_ok", "api_p_login", "api_p_ok", "pp_x_p_login", "pp_x_p_ok"] {
        assert!(has_id(&out, id), "missing {id}: {:?}", ids(&out));
    }
    assert_eq!(connections(&out), 1);
}

#[test]
fn a_template_body_sees_the_lets_and_a_parameter_of_the_same_name_wins() {
    let out = svg(r#"
let xs = ["file"]
template "t" (ys: []) { col c { each a in xs { rect f [label: a] } each b in ys { rect g [label: b] } } }
template "u" (xs: ["param"]) { col c { each a in xs { rect f [label: a] } } }
t one [ys: xs]
u two
"#);
    assert!(has_id(&out, "one_f_1") && has_id(&out, "one_g_1"), "{:?}", ids(&out));
    assert!(labels(&out).iter().any(|l| l == "param"), "parameter should shadow the let");
    assert_eq!(labels(&out).iter().filter(|l| *l == "file").count(), 2);
}

#[test]
fn a_let_gives_star_instances_their_items() {
    let out = svg(r#"
let chips = [{name: "A"}, {name: "B"}, {name: "C"}]
template "chip" (name: "x") { rect c [label: name, width: 40, height: 20] }
row { chip k* [items: chips] }
"#);
    for id in ["k0", "k1", "k2"] {
        assert!(has_id(&out, id), "missing {id}: {:?}", ids(&out));
    }
}

#[test]
fn a_let_is_defined_once() {
    let e = render_err("let tr = [a]\nlet tr = [b]\n");
    assert!(e.contains("`let tr` is defined twice"), "{e}");
}

#[test]
fn a_let_goes_at_the_top_level() {
    let e = render_err("row r {\n let xs = [a]\n rect b\n}\n");
    assert!(e.contains("top level") && e.contains("row"), "{e}");
    let e = render_err("template \"t\" {\n let xs = [a]\n rect b\n}\n");
    assert!(e.contains("top level") && e.contains("template"), "{e}");
    let e = render_err("each n in [a] {\n let xs = [a]\n}\n");
    assert!(e.contains("top level"), "{e}");
}

#[test]
fn a_let_holds_a_list() {
    let e = render_err("let n = 5\n");
    assert!(e.contains("holds a list") && e.contains("the number 5"), "{e}");
    let e = render_err("let r = {a: 1}\n");
    assert!(e.contains("holds a list") || e.contains("Unexpected"), "{e}");
}

#[test]
fn a_let_where_one_value_goes_is_an_error() {
    let e = render_err("let tr = [{a: 1}, {a: 2}]\nrect x [fill: tr]\n");
    assert!(e.contains("`fill: tr`") && e.contains("is data") && e.contains("2 records") && e.contains("each"), "{e}");
    let e = render_err("let tr = [a]\nrect x\ntr -> x\n");
    assert!(e.contains("`tr -> x`") && e.contains("is data"), "{e}");
    let e = render_err("let tr = [a]\neach n in [a] { rect r [label: tr] }\n");
    assert!(e.contains("`label: tr`"), "{e}");
}

#[test]
fn a_let_and_an_element_may_not_share_a_name() {
    let e = render_err("let tr = [a]\nrect tr\n");
    assert!(e.contains("`let tr`") && e.contains("element called `tr`"), "{e}");
}

#[test]
fn let_is_reserved() {
    let e = render_err("rect let\n");
    assert!(e.contains("reserved"), "{e}");
}

#[test]
fn an_unknown_list_lists_the_lets() {
    let e = render_err("let tr = [a]\neach t in rt { rect r }\n");
    assert!(e.contains("no list called `rt`") && e.contains("tr"), "{e}");
    let e = render_err("each t in rt { rect r }\n");
    assert!(e.contains("let"), "{e}");
}

// ---- S6: `where`, only some items -----------------------------------

const TR: &str = r#"
let tr = [
    {from: a, to: b, on: "go"},
    {from: b, to: b, on: "stay"},
    {from: b, to: a, on: "back"},
]
row r [gap: 80] { rect a  rect b }
"#;

#[test]
fn where_splits_one_list_and_keeps_source_positions() {
    let out = svg(&format!(r#"{TR}
each t, i in tr where t.from != t.to {{ t.from -> t.to [label: "{{i+1}} {{t.on}}"] }}
each t, i in tr where t.from == t.to {{ text "{{i+1}} {{t.on}}" self }}
"#));
    assert_eq!(connections(&out), 2);
    let l = labels(&out);
    for t in ["1 go", "3 back", "2 stay"] {
        assert!(l.iter().any(|x| x == t), "missing {t}: {l:?}");
    }
    // The copy is named by its place in the whole list: self_2, not self_1.
    assert!(has_id(&out, "self_2") && !has_id(&out, "self_1"), "{:?}", ids(&out));
}

#[test]
fn where_joins_conditions_with_and_and_compares_words_strings_and_numbers() {
    let out = svg(r#"
col c {
    each m in [{k: call, n: 1, s: "x"}, {k: reply, n: 2, s: "y"}, {k: call, n: 3, s: "y"}]
        where m.k == call and m.s == "y" { rect hit [label: "{m.n}"] }
    each m in [{k: call, n: 1}, {k: reply, n: 2}] where m.k == "call" { rect q }
    each m in [{k: call, n: 1}, {k: reply, n: 2}] where m.n != 1 { rect num }
    each n, i in [a, b, c] where i != 0 { rect skip_first }
}
"#);
    assert!(has_id(&out, "hit_3") && !has_id(&out, "hit_1") && !has_id(&out, "hit_2"), "{:?}", ids(&out));
    // a quoted string and a bare word with the same text are equal
    assert!(has_id(&out, "q_1") && !has_id(&out, "q_2"), "{:?}", ids(&out));
    assert!(has_id(&out, "num_2") && !has_id(&out, "num_1"), "{:?}", ids(&out));
    assert!(has_id(&out, "skip_first_2") && has_id(&out, "skip_first_3") && !has_id(&out, "skip_first_1"));
}

#[test]
fn where_with_by_keeps_the_keys() {
    let out = svg(r#"
col c {
    each m by id in [{id: login, self: no}, {id: check, self: yes}, {id: done, self: no}] where m.self == no {
        rect p
    }
}
"#);
    assert!(has_id(&out, "p_login") && has_id(&out, "p_done") && !has_id(&out, "p_check"), "{:?}", ids(&out));
}

#[test]
fn where_may_compare_with_an_outer_item() {
    let out = svg(r#"
col c { each a in [x, y] { each b in [{k: x}, {k: y}, {k: x}] where b.k == a { rect r } } }
"#);
    assert!(has_id(&out, "r_1_1") && has_id(&out, "r_1_3") && has_id(&out, "r_2_2"), "{:?}", ids(&out));
    assert!(!has_id(&out, "r_1_2") && !has_id(&out, "r_2_1"));
}

#[test]
fn a_field_only_the_matching_items_use_need_only_be_there() {
    let out = svg(r#"
col c {
    each m in [{k: a}, {k: b, extra: "only me"}] where m.k == b { rect r [label: m.extra] }
}
"#);
    assert!(out.contains(">only me<"));
}

#[test]
fn an_unknown_field_in_where_suggests_one() {
    let e = render_err(&format!("{TR}\neach t in tr where t.form != t.to {{ t.from -> t.to }}\n"));
    assert!(e.contains("no item has a field `form`") && e.contains("did you mean `from`"), "{e}");
}

#[test]
fn a_field_missing_from_some_items_in_where_names_them() {
    let e = render_err("col c {\n each m in [{k: a}, {k: b, z: 1}, {k: c}] where m.z == 1 { rect r }\n}\n");
    assert!(e.contains("items 1 and 3 have no field `z`"), "{e}");
}

#[test]
fn a_misspelt_body_field_under_where_says_no_item_has_it() {
    let e = render_err(&format!("{TR}\neach t in tr where t.from == t.to {{ text t.no }}\n"));
    assert!(e.contains("no item has a field `no`"), "{e}");
}

#[test]
fn where_mistakes_say_what_to_write() {
    let e = render_err(&format!("{TR}\neach t in tr where t.from = t.to {{ rect r }}\n"));
    assert!(e.contains("`==`"), "{e}");
    let e = render_err(&format!("{TR}\neach t in tr where t.from == a or t.to == a {{ rect r }}\n"));
    assert!(e.contains("`and` only"), "{e}");
    let e = render_err(&format!("{TR}\neach t in tr where t.from == a && t.to == a {{ rect r }}\n"));
    assert!(e.contains("&&"), "{e}");
    let e = render_err(&format!("{TR}\neach t in tr where kind == a {{ rect r }}\n"));
    assert!(e.contains("does not use the loop variable") && e.contains("t.kind"), "{e}");
}

#[test]
fn where_compares_single_values() {
    let e = render_err("col c {\n each m in [{k: a}] where m.k == [a, b] { rect r }\n}\n");
    assert!(e.contains("compares single values") && e.contains("a list"), "{e}");
}

#[test]
fn where_and_and_stay_ordinary_words_elsewhere() {
    assert!(render("rect where\nrect and\nwhere -> and").is_ok());
}
