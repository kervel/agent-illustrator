//! `keyframe` inside `each`: one keyframe per item, named from the item
//! (`"step{i+1}"`, `"{s.id}"`), acting on what the item names.

use agent_illustrator::{frame_names, render, render_with_config, step_frames, RenderConfig};

fn frame(src: &str, name: &str) -> String {
    render_with_config(src, RenderConfig { frame: Some(name.into()), ..Default::default() })
        .unwrap_or_else(|e| panic!("{e}\n---\n{src}"))
}

fn render_err(src: &str) -> String {
    match render(src) {
        Ok(_) => panic!("expected an error\n---\n{src}"),
        Err(e) => e.to_string(),
    }
}

fn has_id(svg: &str, id: &str) -> bool {
    svg.contains(&format!("id=\"{id}\""))
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

const STEPS: &str = r#"
let steps = [{id: plan, t: "Plan", c: role-ok}, {id: build, t: "Build", c: role-warn}, {id: ship, t: "Ship", c: role-error}]
row r [gap: 20] {
  each s by id in steps { rect box [label: "{s.t}", width: 80, height: 40] }
}
keyframe "start" { hide box[plan], box[build], box[ship] }
"#;

#[test]
fn each_item_makes_a_keyframe_named_from_the_item() {
    let src = format!("{STEPS}\neach s, i in steps {{ keyframe \"step{{i+1}}\" {{ show box[s.id] }} }}\n");
    assert_eq!(frame_names(&src).unwrap(), vec!["start", "step1", "step2", "step3"]);
    assert_eq!(step_frames(&src).unwrap().len(), 4);
    let f1 = frame(&src, "step1");
    assert!(has_id(&f1, "box_plan") && !has_id(&f1, "box_build"), "step1 shows plan only");
    let f2 = frame(&src, "step2");
    assert!(has_id(&f2, "box_plan") && has_id(&f2, "box_build") && !has_id(&f2, "box_ship"));
}

#[test]
fn a_field_names_the_keyframe() {
    let src = format!("{STEPS}\neach s in steps {{ keyframe \"{{s.id}}\" [title: \"{{s.t}}\"] {{ show box[s.id] }} }}\n");
    assert_eq!(frame_names(&src).unwrap(), vec!["start", "plan", "build", "ship"]);
    let f = frame(&src, "build");
    assert!(has_id(&f, "box_build") && has_id(&f, "box_plan") && !has_id(&f, "box_ship"));
}

#[test]
fn a_keyframe_acts_on_the_parts_its_own_item_made() {
    // `box` declared in the same body is this item's `box_k`.
    let src = r#"
let steps = [{t: "A"}, {t: "B"}]
each s in steps {
  rect box [label: "{s.t}", width: 60, height: 30]
}
keyframe "start" { hide box_1, box_2 }
each s, i in steps {
  rect tag [width: 10, height: 10]
  keyframe "tag{i+1}" { hide tag }
}
"#;
    let f = frame(src, "tag1");
    assert!(!has_id(&f, "tag_1") && has_id(&f, "tag_2"), "tag1 hides this item's tag (tag_1) only");
}

#[test]
fn transform_values_and_constraints_come_from_the_item() {
    let src = r#"
let moves = [{id: a, dx: 30}, {id: b, dx: 90}]
rect base [width: 40, height: 40]
rect dot [width: 10, height: 10]
constrain dot.left = base.right + 5
constrain dot.top = base.top
each m in moves {
  keyframe "{m.id}" { constrain dot.left = base.right + m.dx }
}
"#;
    let base = rect_of(&frame(src, "a"), "base");
    let a = rect_of(&frame(src, "a"), "dot");
    let b = rect_of(&frame(src, "b"), "dot");
    assert_eq!(a.0 - (base.0 + base.2), 30.0);
    assert_eq!(b.0 - (base.0 + base.2), 90.0);
}

#[test]
fn two_items_making_one_keyframe_name_is_an_error() {
    let src = format!("{STEPS}\neach s in steps {{ keyframe \"same\" {{ show box[s.id] }} }}\n");
    let err = render_err(&src);
    assert!(
        err.contains("items 1 and 2 both make a keyframe called \"same\"") && err.contains("step{i+1}"),
        "{err}"
    );
}

#[test]
fn the_documented_example_renders_and_is_findable() {
    let section = agent_illustrator::docs::section("keyframes-from-data").expect("--doc keyframes-from-data");
    for w in ["keyframe-each", "walkthrough", "steps"] {
        assert_eq!(agent_illustrator::docs::section(w).as_deref(), Some(section.as_str()), "--doc {w}");
    }
    let start = section.find("```\n").unwrap() + 4;
    let src = &section[start..start + section[start..].find("```").unwrap()];
    assert_eq!(frame_names(src).unwrap(), vec!["start", "step1", "step2", "step3"]);
    for mode in [(true, false), (false, true)] {
        let cfg = RenderConfig { animate: mode.0, animate_css: mode.1, ..Default::default() };
        render_with_config(src, cfg).unwrap();
    }
}

#[test]
fn a_misspelt_field_in_a_keyframe_is_reported() {
    let src = format!("{STEPS}\neach s in steps {{ keyframe \"{{s.id}}\" {{ show box[s.ide] }} }}\n");
    let err = render_err(&src);
    assert!(err.contains("no item has a field `ide`") && err.contains("did you mean `id`?"), "{err}");
}
