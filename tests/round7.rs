//! Code blocks, SVG artwork with parts, colour roles, host metadata, crop.

use agent_illustrator::{render_with_config, render_with_lint, RenderConfig};

fn render(src: &str) -> String {
    render_with_config(src, RenderConfig::new()).unwrap_or_else(|e| panic!("{e}"))
}

fn animate(src: &str) -> String {
    let mut c = RenderConfig::new();
    c.animate = true;
    render_with_config(src, c).unwrap_or_else(|e| panic!("{e}"))
}

fn timeline(src: &str) -> String {
    let mut c = RenderConfig::new();
    c.timeline = true;
    render_with_config(src, c).unwrap_or_else(|e| panic!("{e}"))
}

fn lint(src: &str) -> Vec<String> {
    let (_, w) = render_with_lint(src, RenderConfig::new().with_lint(true)).expect("renders");
    w.into_iter().map(|w| w.message).collect()
}

const CODE: &str = r#"
code m [lang: python, title: "cart.py", source: "def total(cart):\n    s = 0\n    return s"]
rect under [width: 40, height: 20]
constrain under.top = m.bg.bottom + 10
constrain under.left = m.bg.left
"#;

#[test]
fn a_code_block_has_highlighted_numbered_lines_as_parts() {
    let svg = render(CODE);
    for part in ["m_line1", "m_line2", "m_line3", "m_title", "m_bg"] {
        assert!(svg.contains(&format!(r#"id="{part}""#)), "{part}");
    }
    assert!(svg.contains("var(--code-keyword)"), "keywords are coloured by token");
    assert!(lint(CODE).is_empty(), "{:?}", lint(CODE));
}

#[test]
fn removing_a_line_closes_the_block_and_what_follows_moves_up() {
    let src = format!("{CODE}\nkeyframe \"a\" {{ }}\nkeyframe \"b\" {{ remove m.line[2] }}\n");
    let t = timeline(&src);
    assert!(t.contains("m.line2 (shape) height: 28px -> 0px"), "{t}");
    assert!(t.contains("under translate"), "the element under the block follows: {t}");
}

#[test]
fn insert_adds_collapsed_lines_and_opens_them() {
    let src = format!(
        "{CODE}\nkeyframe \"a\" {{ }}\nkeyframe \"b\" {{ insert m after line 2 [name: extra, source: \"x = 1\\ny = 2\"] }}\n"
    );
    let svg = render(&src);
    assert!(svg.contains(r#"id="m_extra1""#) && svg.contains(r#"id="m_extra2""#));
    let t = timeline(&src);
    assert!(t.contains("show [enter: expand] m.extra1"), "{t}");
    assert!(t.contains("m.extra1 (shape) height: 0px -> 28px"), "{t}");
}

#[test]
fn a_source_transform_rehighlights_and_keeps_the_line_number() {
    let src = format!(
        "{CODE}\nkeyframe \"a\" {{ }}\nkeyframe \"b\" {{ transform m.line1 [source: \"def total(cart, coupon=None):\", swap: roll] }}\n"
    );
    let svg = animate(&src);
    assert!(svg.contains("coupon"), "the new wording is in the output");
}

#[test]
fn a_diff_block_tints_rows_and_marks_the_change() {
    let svg = render(r#"code d [diff: "-    return s\n+    return round(s, 2)"]"#);
    assert!(svg.contains("var(--code-del-bg)") && svg.contains("var(--code-add-bg)"));
    assert!(svg.contains("font-weight=\"bold\""), "the changed segment is bold");
}

#[test]
fn a_role_is_a_colour() {
    let svg = render("rect a [fill: role-primary, stroke: role-ink]");
    assert!(svg.contains("var(--role-primary)") && svg.contains("--role-primary: var(--accent-1)"));
}

#[test]
fn overlaps_declares_an_intended_overlap() {
    let base = "rect card [width: 100, height: 60]\ncircle badge [size: 20, fill: red OVERLAPS]\nconstrain badge.center_x = card.left\nconstrain badge.center_y = card.top\n";
    assert!(lint(&base.replace(" OVERLAPS", "")).iter().any(|m| m.contains("overlap")));
    assert!(!lint(&base.replace(" OVERLAPS", ", overlaps: card")).iter().any(|m| m.contains("overlap")));
}

#[test]
fn keyframe_title_and_note_reach_the_manifest() {
    let svg = animate("rect a\nkeyframe \"one\" [title: \"First\", note: \"say hi\"] { }\nkeyframe \"two\" { hide a }\n");
    assert!(svg.contains(r#""title":"First","note":"say hi""#), "manifest frames carry them");
}

#[test]
fn svg_artwork_parts_are_addressable_and_recolourable() {
    let dir = std::env::temp_dir().join(format!("ail-parts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("doc.svg"),
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
  <rect id="sheet" x="2" y="2" width="96" height="116" fill="#fff" stroke="#000" stroke-width="3"/>
  <g id="bars" stroke-width="8" stroke-linecap="round">
    <line id="bar1" x1="20" y1="30" x2="70" y2="30" stroke="var(--b1, #ccc)"/>
  </g>
</svg>"##,
    )
    .unwrap();
    let src = "template \"doc\" from \"doc.svg\"\ndoc d [b1: role-primary]\ncircle q [size: 10, overlaps: d]\n\
               constrain q.center_x = d.sheet.left\nconstrain q.center_y = d.sheet.top\n\
               keyframe \"a\" { }\nkeyframe \"b\" { transform d.bar1 [stroke: role-error] }\n";
    let cfg = || RenderConfig::new().with_template_base_path(dir.clone());
    let svg = render_with_config(src, cfg()).unwrap();
    assert!(svg.contains(r#"id="d_sheet""#) && svg.contains(r#"id="d_bar1""#) && svg.contains(r#"id="d_bars""#));
    assert!(svg.contains("var(--role-primary)"), "the instance argument coloured the bar");
    assert!(svg.contains(r#"stroke="inherit""#), "the part takes its paint from its group");
    let mut c = cfg();
    c.frame = Some("b".into());
    let frame = render_with_config(src, c).unwrap();
    assert!(frame.contains(r#"stroke="var(--role-error)""#), "a transform recolours the part");
}

#[test]
fn crop_to_content_is_one_box_for_every_frame() {
    let src = "rect stage [width: 1000, height: 600, canvas: true, fill: white]\nrect a [width: 50, height: 50]\n\
               rect b [width: 50, height: 50]\nconstrain a.left = stage.left + 100\nconstrain a.top = stage.top + 100\n\
               constrain b.left = a.right + 200\nconstrain b.top = a.top\n\
               keyframe \"one\" { hide b }\nkeyframe \"two\" { show b }\n";
    let vb = |frame: Option<&str>| {
        let mut c = RenderConfig::new();
        c.crop = Some(10.0);
        c.frame = frame.map(str::to_string);
        let svg = render_with_config(src, c).unwrap();
        svg.split("viewBox=\"").nth(1).unwrap().split('"').next().unwrap().to_string()
    };
    assert_eq!(vb(Some("one")), vb(Some("two")));
    assert_eq!(vb(Some("one")), "90 90 320 70", "a, b and the margin; not the stage");
}
