//! `if COND { ... } [else { ... }]`: one of two bodies, chosen before layout.

use agent_illustrator::render;

fn svg(src: &str) -> String {
    render(src).unwrap_or_else(|e| panic!("{e}\n---\n{src}"))
}

fn render_err(src: &str) -> String {
    match render(src) {
        Ok(_) => panic!("expected an error\n---\n{src}"),
        Err(e) => e.to_string(),
    }
}

fn ids(svg: &str) -> Vec<String> {
    svg.split("id=\"").skip(1).filter_map(|s| s.split('"').next()).map(str::to_string).collect()
}

fn has_id(svg: &str, id: &str) -> bool {
    ids(svg).iter().any(|i| i == id)
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

const STEP: &str = r#"
template "step" (name: "", ai: false) {
    rect box [label: name, width: 120, height: 40]
    if ai {
        rect badge [label: "AI", width: 30, height: 16]
        constrain badge.right = box.right - 4
        constrain badge.bottom = box.top - 4
    } else {
        rect manual [width: 8, height: 8]
        constrain manual.right = box.right - 4
        constrain manual.bottom = box.top - 4
    }
}
"#;

#[test]
fn true_draws_the_body_false_the_else() {
    let out = svg(&format!("{STEP}\nstep a [name: \"A\", ai: true]\nstep b [name: \"B\", ai: false]\n"));
    assert!(has_id(&out, "a_badge") && !has_id(&out, "a_manual"), "{:?}", ids(&out));
    assert!(has_id(&out, "b_manual") && !has_id(&out, "b_badge"), "{:?}", ids(&out));
}

#[test]
fn a_default_parameter_decides_when_no_argument_is_given() {
    let out = svg(&format!("{STEP}\nstep a [name: \"A\"]\n"));
    assert!(has_id(&out, "a_manual") && !has_id(&out, "a_badge"), "{:?}", ids(&out));
}

#[test]
fn without_else_a_false_if_draws_nothing() {
    let out = svg("template \"t\" (on: false) {\n rect box\n if on { rect dot [width: 6] }\n}\nt x\nt y [on: true]\n");
    // `x` keeps one shape: it is drawn as the instance itself.
    assert!(has_id(&out, "x") && !has_id(&out, "x_dot"), "{:?}", ids(&out));
    assert!(has_id(&out, "y_dot"), "{:?}", ids(&out));
}

#[test]
fn a_part_name_still_works_when_the_if_leaves_one_shape() {
    // `x` collapses to its one shape, `y` stays a group: `x.box` and `y.box` both resolve.
    let out = svg(
        "template \"t\" (on: false) {\n rect box [width: 50]\n if on { rect dot [width: 6] }\n}\nt x\nt y [on: true]\nconstrain y.box.left = x.box.right + 40\nx.box -> y.box\n",
    );
    assert!(has_id(&out, "x") && has_id(&out, "y_box"), "{:?}", ids(&out));
}

#[test]
fn a_string_true_and_false_work_and_a_string_is_true_unless_empty() {
    let src = "template \"t\" (on: \"\") {\n rect box\n if on { rect dot [width: 6] }\n}\nt a [on: \"true\"]\nt b [on: \"false\"]\nt c [on: \"yes\"]\nt d\n";
    let out = svg(src);
    assert!(has_id(&out, "a_dot") && has_id(&out, "c_dot"), "{:?}", ids(&out));
    assert!(!has_id(&out, "b_dot") && !has_id(&out, "d_dot"), "{:?}", ids(&out));
}

#[test]
fn comparisons_and_and_work_on_parameters() {
    let src = "template \"t\" (kind: call, big: false) {\n rect box\n if kind == call and big == true { rect big_call }\n if kind != call { rect other }\n}\nt a [big: true]\nt b\nt c [kind: reply]\n";
    let out = svg(src);
    assert!(has_id(&out, "a_big_call"), "{:?}", ids(&out));
    assert!(!has_id(&out, "b_big_call") && !has_id(&out, "b_other"), "{:?}", ids(&out));
    assert!(has_id(&out, "c_other") && !has_id(&out, "c_big_call"), "{:?}", ids(&out));
}

#[test]
fn nested_ifs_and_else_if() {
    let src = "template \"t\" (a: false, b: false) {\n rect box\n if a {\n  if b { rect ab } else { rect a_only }\n } else if b { rect b_only } else { rect none_set }\n}\nt w [a: true, b: true]\nt x [a: true]\nt y [b: true]\nt z\n";
    let out = svg(src);
    for (inst, want) in [("w", "w_ab"), ("x", "x_a_only"), ("y", "y_b_only"), ("z", "z_none_set")] {
        assert!(has_id(&out, want), "{inst}: {:?}", ids(&out));
    }
    assert!(!has_id(&out, "w_a_only") && !has_id(&out, "z_b_only"), "{:?}", ids(&out));
}

#[test]
fn inside_each_reading_fields_and_the_index() {
    let src = r#"
each m, i in [{n: "a", ai: true}, {n: "b", ai: false}, {n: "c", ai: true}] {
    if m.ai { rect flag [width: 10] } else { circle plain [size: 10] }
    if i == 0 { rect first [width: 4] }
}"#;
    let out = svg(src);
    for want in ["flag_1", "plain_2", "flag_3", "first_1"] {
        assert!(has_id(&out, want), "{want}: {:?}", ids(&out));
    }
    for gone in ["plain_1", "flag_2", "first_2", "first_3"] {
        assert!(!has_id(&out, gone), "{gone}: {:?}", ids(&out));
    }
}

#[test]
fn inside_each_in_a_template_mixing_a_field_and_a_parameter() {
    let src = r#"
template "list" (items: [], mode: "x") {
    col c {
        each m in items {
            rect item [width: 60, height: 20]
            if m.kind == mode { rect hit [width: 6, height: 6] }
        }
    }
}
list l [items: [{kind: x}, {kind: y}]]
list k [items: [{kind: x}, {kind: y}], mode: y]"#;
    let out = svg(src);
    assert!(has_id(&out, "l_hit_1") && !has_id(&out, "l_hit_2"), "{:?}", ids(&out));
    assert!(has_id(&out, "k_hit_2") && !has_id(&out, "k_hit_1"), "{:?}", ids(&out));
}

#[test]
fn a_parameter_passed_on_to_a_nested_template() {
    let src = format!("{STEP}\ntemplate \"lane\" (ai: false) {{\n step s [name: \"x\", ai: ai]\n}}\nlane l1 [ai: true]\nlane l2\n");
    let out = svg(&src);
    assert!(has_id(&out, "l1_s_badge") && has_id(&out, "l2_s_manual"), "{:?}", ids(&out));
}

#[test]
fn a_row_closes_up_where_an_if_draws_nothing() {
    let with = svg(
        "template \"t\" (on: false) {\n row r [gap: 10] {\n  rect a [width: 50, height: 20]\n  if on { rect mid [width: 70, height: 20] }\n  rect b [width: 50, height: 20]\n }\n}\nt x [on: true]\n",
    );
    let without = svg(
        "template \"t\" (on: false) {\n row r [gap: 10] {\n  rect a [width: 50, height: 20]\n  if on { rect mid [width: 70, height: 20] }\n  rect b [width: 50, height: 20]\n }\n}\nt x\n",
    );
    let plain = svg("template \"t\" () {\n row r [gap: 10] {\n  rect a [width: 50, height: 20]\n  rect b [width: 50, height: 20]\n }\n}\nt x\n");
    let (ax, ..) = rect_of(&with, "x_a");
    let (bx_with, ..) = rect_of(&with, "x_b");
    let (ax2, ..) = rect_of(&without, "x_a");
    let (bx_without, ..) = rect_of(&without, "x_b");
    // With the middle box, b sits one box and one gap further right.
    assert!((bx_with - ax - (bx_without - ax2) - 80.0).abs() < 0.5, "with {bx_with}, without {bx_without}");
    // Without it, the row is exactly the row that never had it.
    assert_eq!(rect_of(&without, "x_b"), rect_of(&plain, "x_b"));
    assert_eq!(rect_of(&without, "x_a"), rect_of(&plain, "x_a"));
}

#[test]
fn an_if_works_as_a_child_of_a_row_at_the_top_level() {
    let out = svg("row r [gap: 10] {\n rect a\n each k in [1, 2] { if k == 2 { rect b } }\n rect c\n}\n");
    assert!(has_id(&out, "b_2") && !has_id(&out, "b_1"), "{:?}", ids(&out));
    let out = svg("group g {\n rect a\n if false { rect b }\n if true { rect c }\n}\n");
    assert!(has_id(&out, "c") && !has_id(&out, "b"), "{:?}", ids(&out));
}

// ---- errors ------------------------------------------------------------

#[test]
fn a_reference_to_a_part_the_if_left_out_names_the_condition() {
    let e = render_err(&format!("{STEP}\nstep a [name: \"A\"]\nrect note\nnote -> a.badge\n"));
    assert!(e.contains("`a.badge` only exists when `ai` is true"), "{e}");
    assert!(e.contains('^'), "caret: {e}");
    let e = render_err(&format!("{STEP}\nstep a [name: \"A\", ai: true]\nrect note\nconstrain note.left = a.manual.right\n"));
    assert!(e.contains("`a.manual` only exists when `ai` is false"), "{e}");
    // A top-level element, and a copy that an `each` made.
    let e = render_err("if false { rect y }\nrect z\nconstrain y.left = z.left\n");
    assert!(e.contains("`y` only exists when `false` is true"), "{e}");
    let e = render_err("each m in [{ai: true}, {ai: false}] { if m.ai { rect flag } }\nrect z\nflag_2 -> z\n");
    assert!(e.contains("`flag_2` only exists when `m.ai` is true"), "{e}");
}

#[test]
fn a_misspelt_name_in_a_condition_suggests_the_parameter() {
    let e = render_err("template \"t\" (ai: false) {\n rect b\n if ia { rect c }\n}\nt x\n");
    assert!(e.contains("nothing called `ia`") && e.contains("did you mean `ai`"), "{e}");
    let e = render_err("template \"t\" (kind: call) {\n rect b\n if knd == call { rect c }\n}\nt x\n");
    assert!(e.contains("did you mean `kind`"), "{e}");
    // A loop variable is suggested too, and a misspelt field lists the fields.
    let e = render_err("each m in [{kind: a}] { if mm.kind == a { rect c } }\n");
    assert!(e.contains("did you mean `m`"), "{e}");
    let e = render_err("each m in [{kind: a}] { if m.knd == a { rect c } }\n");
    assert!(e.contains("did you mean `kind`"), "{e}");
}

#[test]
fn a_bare_if_needs_true_or_false() {
    let e = render_err("template \"t\" (mode: fast) {\n rect b\n if mode { rect c }\n}\nt x\n");
    assert!(e.contains("`mode` is the word `fast`") && e.contains("if mode == fast"), "{e}");
    let e = render_err("template \"t\" (n: 3) {\n rect b\n if n { rect c }\n}\nt x\n");
    assert!(e.contains("the number 3"), "{e}");
    let e = render_err("let tr = [1]\nif tr { rect c }\n");
    assert!(e.contains("a list"), "{e}");
    let e = render_err("each m in [{k: a}] { if m { rect c } }\n");
    assert!(e.contains("whole record"), "{e}");
}

#[test]
fn malformed_ifs_get_a_caret() {
    for (src, want) in [
        ("if !ai { rect c }\n", "no `!`"),
        ("if ai = true { rect c }\n", "not `=`"),
        ("if a == b or c == d { rect c }\n", "`and` only"),
        ("if a == b && c == d { rect c }\n", "no `&&`"),
        ("if { rect c }\n", "needs a condition"),
        ("rect a\nelse { rect c }\n", "`else` goes right after"),
        ("if true { rect a } else rect c\n", "`else` takes a body"),
    ] {
        let e = render_err(src);
        assert!(e.contains(want) && e.contains('^'), "{src}: {e}");
    }
}

#[test]
fn if_and_else_are_reserved() {
    for w in ["if", "else"] {
        let e = render_err(&format!("rect {w}\n"));
        assert!(e.contains("reserved"), "{e}");
    }
    assert!(agent_illustrator::error::RESERVED.contains(&"if"));
    assert!(agent_illustrator::error::RESERVED.contains(&"else"));
}

#[test]
fn a_let_inside_an_if_is_an_error() {
    let e = render_err("if true { let x = [1] }\n");
    assert!(e.contains("inside an `if`"), "{e}");
}

#[test]
fn one_name_in_both_bodies_is_one_element() {
    let out = svg("if true { rect mark [width: 10] } else { rect mark [width: 20] }\neach m in [{a: true}, {a: false}] { if m.a { rect mk [width: 10] } else { circle mk [size: 10] } }\nmark -> mk_2\n");
    for id in ["mark", "mk_1", "mk_2"] {
        assert_eq!(ids(&out).iter().filter(|i| *i == id).count(), 1, "{id}: {:?}", ids(&out));
    }
}

#[test]
fn a_keyframe_reference_to_a_dropped_part_names_the_condition() {
    let e = render_err(&format!("{STEP}\nstep s2 [name: \"b\"]\nkeyframe \"k\" {{ show s2.badge }}\n"));
    assert!(e.contains("`s2.badge` only exists when `ai` is true"), "{e}");
}
