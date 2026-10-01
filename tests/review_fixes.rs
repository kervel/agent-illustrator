//! Fixes from the discoverability review and the reveal-deck integration.

use agent_illustrator::{render_with_config, render_with_lint, step_frames, RenderConfig};

fn render(src: &str) -> Result<String, String> {
    render_with_config(src, RenderConfig::new()).map_err(|e| e.to_string())
}

fn lint(src: &str) -> Vec<String> {
    let (_, w) = render_with_lint(src, RenderConfig::new().with_lint(true)).expect("renders");
    w.into_iter().map(|w| w.message).collect()
}

#[test]
fn a_reserved_word_as_a_name_is_an_error_at_the_name() {
    let e = render("circle a\ncircle b\npath line [through: [a, b], fill: none]\n").unwrap_err();
    assert!(e.contains("`line` is a reserved word") && e.contains("line 3, column 6"), "{e}");
    let e = render("rect row\n").unwrap_err();
    assert!(e.contains("`row` is a reserved word"), "{e}");
}

#[test]
fn a_dotted_path_names_the_only_part_of_a_one_shape_template() {
    // The part is drawn as the instance itself; `f.val` still names it.
    let svg = render(
        "template \"fr\" () {\n  rect val [label: \"84\"]\n}\nfr f\nrect other\n\
         constrain f.val.left = other.right + 20\nkeyframe \"a\" { }\nkeyframe \"b\" { hide f.val }\n",
    )
    .expect("f.val resolves");
    assert!(svg.contains(r#"id="f""#));
}

#[test]
fn a_macro_argument_without_the_part_it_needs_is_explained_at_the_call() {
    let e = render(
        "import \"ail:motion/git\"\nrect folder\ncircle st [appears: later]\n\
         path track [through: [folder, st], drawn: 0, fill: none]\nkeyframe \"a\" { }\n\
         keyframe \"b\" { commit(folder, st, track) }\n",
    )
    .unwrap_err();
    assert!(e.contains("commit() -> snapshot() uses a part `dot` of `st`"), "{e}");
    assert!(e.contains("line 6"), "points at the call: {e}");
    assert!(e.contains("git_station"), "names the template that fits: {e}");
}

#[test]
fn the_git_library_works_on_its_own_templates() {
    let src = r#"
import "ail:motion/git"
group folder {
    rect folder_bg [fill: background-1, stroke: foreground-1]
    col files { git_file f_cart [name: "cart.py", value: "57 lines"] }
}
constrain folder_bg contains files [padding: 8]
path track [through: [c1.dot, c2.dot], drawn: 0, stroke: accent-1, stroke_width: 12, fill: none]
row history [gap: 60, align: dot] {
    git_station c1 [name: "Login page", meta: "anna"]
    git_station c2 [name: "Fix cart", meta: "ben"]
}
constrain history.left = folder.right + 80
constrain c1.dot.center_y = folder.center_y
keyframe "start" { }
keyframe "first" { commit(folder, c1, track) }
keyframe "second" { change(f_cart, "58 lines"); then { commit(folder, c2, track) } }
"#;
    assert!(lint(src).is_empty(), "{:?}", lint(src));
}

#[test]
fn stations_named_by_their_own_label_get_room() {
    let msgs = lint(
        r#"
path trk [through: [a, b, c], stroke: blue, fill: none]
circle a [size: 24, label: "Login page for customers", label_position: below]
circle b [size: 24, label: "Dark mode everywhere", label_position: below]
circle c [size: 24, label: "Fix price rounding", label_position: below]
constrain a.center_x = 0
constrain a.center_y = 0
constrain b.center_y = a.center_y
constrain c.center_y = a.center_y
"#,
    );
    assert!(msgs.is_empty(), "{msgs:?}");
}

#[test]
fn a_draw_that_shrinks_a_line_is_reported() {
    let msgs = lint(
        "circle a\ncircle b\npath trk [through: [a, b], stroke: blue, fill: none]\n\
         keyframe \"k\" { draw trk [to: a] }\n",
    );
    assert!(msgs.iter().any(|m| m.contains("shrinks it")), "{msgs:?}");
}

#[test]
fn swapped_label_variants_do_not_follow_a_sibling_with_a_fill() {
    // A host rule `[fill=dark] + .ai-label` meant for "label after its shape"
    // turned a swapped variant white after a dark-coloured base text.
    let mut cfg = RenderConfig::new();
    cfg.animate = true;
    let svg = render_with_config(
        "rect card [width: 200, height: 60, fill: background-1, label: \"210\", label_fill: foreground-1]\n\
         rect chip [width: 80, height: 60, fill: foreground-1, label: \"py\"]\n\
         keyframe \"a\" { }\nkeyframe \"b\" { transform card [label: \"246\", swap: roll]; transform chip [label: \"css\", swap: roll] }\n",
        cfg,
    )
    .expect("renders");
    let lines: Vec<&str> = svg.lines().map(str::trim).collect();
    for (i, l) in lines.iter().enumerate() {
        if l.contains("-v0") && l.starts_with("<text") {
            assert!(lines[i - 1].starts_with("<g class=\"ai-label-variants"), "{}", lines[i - 1]);
            assert!(!l.contains(" fill="), "the colour is on the wrapper: {l}");
        }
    }
    // The chip's labels (base and variant) are marked for light text.
    assert_eq!(svg.matches("ai-on-dark").count(), 1 + 2, "rule + base + variant:\n{svg}");
}

#[test]
fn steps_group_auto_frames_with_the_click_before_them() {
    let steps = step_frames(
        "rect a\nkeyframe \"one\" { }\nkeyframe \"two\" { }\nkeyframe \"more\" [auto] { }\nkeyframe \"three\" { }\n",
    )
    .unwrap();
    assert_eq!(steps, vec![vec!["one"], vec!["two", "more"], vec!["three"]]);
}
