//! Integration tests: file-SVG clipart viewBox is auto-trimmed to the artwork bbox.
use agent_illustrator::{render_with_config, RenderConfig};

fn write_clipart(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join("clipart")).unwrap();
    // drawing occupies (30,30)..(70,70) = 16% of a 100x100 viewBox (above the 5% guard)
    std::fs::write(
        dir.join("clipart/box.svg"),
        br#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><rect x="30" y="30" width="40" height="40" fill="black"/></svg>"#,
    )
    .unwrap();
}

fn render_in(dir: &std::path::Path, src: &str) -> String {
    let cfg = RenderConfig::new().with_template_base_path(dir.to_path_buf());
    render_with_config(src, cfg).expect("render")
}

#[test]
fn trim_on_by_default_offsets_content() {
    let dir = std::env::temp_dir().join("ail_trim_default");
    write_clipart(&dir);
    let svg = render_in(
        &dir,
        r#"
template "box_icon" from "clipart/box.svg"
box_icon boximg [width: 40, height: 40]
constrain boximg.center_x = 100
constrain boximg.center_y = 100
"#,
    );
    // Trimmed: the embed transform shifts the content by its bbox origin (-30,-30).
    assert!(
        svg.contains("translate(-30"),
        "expected content offset translate(-30,-30), got:\n{}",
        svg
    );
}

#[test]
fn trim_false_keeps_viewbox() {
    let dir = std::env::temp_dir().join("ail_trim_off");
    write_clipart(&dir);
    let svg = render_in(
        &dir,
        r#"
template "box_icon" from "clipart/box.svg"
box_icon boximg [width: 40, height: 40, trim: false]
constrain boximg.center_x = 100
constrain boximg.center_y = 100
"#,
    );
    assert!(
        !svg.contains("translate(-30"),
        "trim:false must not offset content, got:\n{}",
        svg
    );
}
