//! Integration tests: --animate-css plays the compiled motion as plain CSS
//! (one looping timeline, no script), so it runs inside an `<img>`.
use agent_illustrator::{render_with_config, RenderConfig};

const SRC: &str = r#"
rect box [width: 100, height: 50, label: "B"]
rect other [width: 100, height: 50]
rect tok [width: 30, height: 20]
constrain box.center_x = 150
constrain box.center_y = 100
constrain other.center_x = 450
constrain other.center_y = 100
constrain tok.center_x = 300
constrain tok.center_y = 200
box.right -> other.left as feed
keyframe "idle" { hide tok }
keyframe "grow" { show tok; transform box [width: 260] }
keyframe "move" { transform box [dx: 40] }
"#;

fn animate_css(src: &str) -> String {
    let cfg = RenderConfig {
        animate_css: true,
        ..Default::default()
    };
    render_with_config(src, cfg).expect("render")
}

/// The `animation:` rule of the selector ending in `suffix` (e.g. `-box`).
fn rule<'a>(svg: &'a str, prefix: &str, suffix: &str) -> Vec<&'a str> {
    svg.lines()
        .filter(|l| l.starts_with(prefix) && l.contains(&format!("{} {{ animation:", suffix)))
        .collect()
}

/// Every value of `prop` in the keyframes the rule plays.
fn values(svg: &str, rule: &str, prop: &str) -> Vec<String> {
    let names: Vec<&str> = rule
        .split("animation:")
        .nth(1)
        .unwrap()
        .split(',')
        .filter_map(|a| a.split_whitespace().next())
        .collect();
    let mut out = Vec::new();
    for n in names {
        let Some(start) = svg.find(&format!("@keyframes {} {{", n)) else { continue };
        let body = &svg[start..start + svg[start..].find('\n').unwrap()];
        for part in body.split(&format!("{}: ", prop)).skip(1) {
            out.push(part.split(';').next().unwrap().to_string());
        }
    }
    out
}

#[test]
fn a_moved_element_travels() {
    let svg = animate_css(SRC);
    let r = rule(&svg, ".kf-", "-box");
    assert_eq!(r.len(), 1, "{r:?}");
    let t = values(&svg, r[0], "translate");
    assert!(t.iter().any(|v| v.starts_with("40px")), "box ends 40px right: {t:?}");
    assert!(t.iter().any(|v| v.starts_with("0px")), "and starts home: {t:?}");
}

#[test]
fn a_resized_element_grows_smoothly() {
    let svg = animate_css(SRC);
    let r = rule(&svg, ".kfp-", "-box");
    let w = values(&svg, r[0], "width");
    assert!(w.contains(&"100px".to_string()) && w.contains(&"260px".to_string()), "{w:?}");
    assert!(w.len() > 4, "in between values, not a jump: {w:?}");
}

#[test]
fn one_rule_per_element_lists_every_channel() {
    // tok fades in AND rises: CSS `animation` is one property, so both must
    // be in a single rule (a second rule would replace the first).
    let svg = animate_css(SRC);
    let r = rule(&svg, ".kf-", "-tok");
    assert_eq!(r.len(), 1, "{r:?}");
    let o = values(&svg, r[0], "opacity");
    assert!(o.first().is_some_and(|v| v == "0") && o.last().is_some_and(|v| v == "1"), "{o:?}");
    assert!(!values(&svg, r[0], "translate").is_empty(), "{}", r[0]);
}

#[test]
fn a_following_connection_reshapes() {
    let svg = animate_css(SRC);
    let r = rule(&svg, ".aid-", "-feed");
    assert_eq!(r.len(), 1, "{r:?}");
    let d = values(&svg, r[0], "d");
    let distinct: std::collections::HashSet<&String> = d.iter().collect();
    assert!(distinct.len() > 2, "the connection follows the box: {d:?}");
}

#[test]
fn a_hidden_connection_stays_hidden_while_its_end_moves() {
    let svg = animate_css(
        r#"
rect llmbox [width: 60, height: 30, label: "L"]
rect tok [width: 30, height: 20, label: "t"]
constrain llmbox.center_x = 100
constrain llmbox.center_y = 60
constrain tok.center_x = 100
constrain tok.center_y = 200 as tok_home
llmbox.bottom -> tok.top as arr
keyframe "idle" { hide tok, arr }
keyframe "point" { show tok, arr }
keyframe "moved" { hide arr; disable tok_home; constrain tok.center_x = 400; constrain tok.center_y = 60 }
"#,
    );
    let r: Vec<&str> = svg.lines().filter(|l| l.starts_with(".conn-") && l.contains("-arr { animation:")).collect();
    assert_eq!(r.len(), 1, "one visibility rule for the arrow: {r:?}");
    let o = values(&svg, r[0], "opacity");
    assert_eq!(o.first().map(String::as_str), Some("0"));
    assert!(o.contains(&"1".to_string()));
    assert_eq!(o.last().map(String::as_str), Some("0"), "hidden at the end: {o:?}");
}

#[test]
fn no_script_and_one_looping_timeline() {
    let svg = animate_css(SRC);
    assert!(!svg.contains("<script"), "plays in an <img>: no script");
    let durations: std::collections::HashSet<&str> = svg
        .lines()
        .filter(|l| l.contains("{ animation:"))
        .flat_map(|l| l.split_whitespace().filter(|w| w.ends_with('s') && w.chars().next().is_some_and(|c| c.is_ascii_digit())))
        .collect();
    assert_eq!(durations.len(), 1, "every channel on one clock: {durations:?}");
    assert!(svg.contains("linear infinite"));
}
