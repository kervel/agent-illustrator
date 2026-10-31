//! An empty label takes no room, and facing edges that overlap are joined by
//! one straight segment.

use agent_illustrator::render;

fn attr(tag: &str, name: &str) -> f64 {
    let at = tag.find(&format!(" {name}=\"")).expect(name) + name.len() + 3;
    tag[at..].split('"').next().unwrap().parse().unwrap()
}

fn tag_with_id<'a>(svg: &'a str, id: &str) -> &'a str {
    let at = svg.find(&format!("id=\"{id}\"")).expect(id);
    let start = svg[..at].rfind('<').unwrap();
    let end = svg[at..].find('>').unwrap() + at;
    &svg[start..end]
}

const CLS: &str = r#"
template "cls" (ttl: "", ops: "") {
    col box [gap: 0, padding: 0] {
        rect hd [width: 100, label: ttl]
        rect op [width: 100, label: ops]
    }
}
"#;

#[test]
fn empty_template_parameter_label_takes_no_height() {
    let src = format!("{CLS}\ncls a [ttl: \"A\", ops: \"\"]\ncls b [ttl: \"B\", ops: \"x\"]");
    let svg = render(&src).expect("renders");
    assert_eq!(attr(tag_with_id(&svg, "a_op"), "height"), 0.0);
    assert!(attr(tag_with_id(&svg, "b_op"), "height") > 20.0);
}

#[test]
fn explicit_size_keeps_an_empty_labelled_rect() {
    let svg = render(r#"rect r [label: "", width: 50, height: 40]"#).expect("renders");
    assert_eq!(attr(tag_with_id(&svg, "r"), "height"), 40.0);
    let svg = render(r#"rect r [label: "", size: 40]"#).expect("renders");
    assert_eq!(attr(tag_with_id(&svg, "r"), "width"), 40.0);
}

#[test]
fn rect_without_label_keeps_default_size() {
    let svg = render(r#"rect r [width: 50]"#).expect("renders");
    assert!(attr(tag_with_id(&svg, "r"), "height") > 20.0);
}

#[test]
fn overlapping_facing_edges_route_straight() {
    let svg = render(
        r#"row r [gap: 100, align: top] {
    rect a [width: 80, height: 40]
    rect b [width: 80, height: 200]
}
a -> b"#,
    )
    .expect("renders");
    let at = svg.find("class=\"ai-connection\"").expect("connection");
    let d = svg[at..].split(" d=\"").nth(1).unwrap().split('"').next().unwrap();
    assert_eq!(d.matches('L').count(), 1, "one segment: {d}");
    let nums: Vec<&str> = d.split(|c: char| c == 'M' || c == 'L' || c == ' ').filter(|t| !t.is_empty()).collect();
    assert_eq!(nums[1], nums[3], "horizontal: {d}");
}
