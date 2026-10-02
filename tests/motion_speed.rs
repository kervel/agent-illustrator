//! `--ail-motion-speed`: one token retimes the whole choreography, explicit
//! seconds, delays, staggers and event shifts included.

use agent_illustrator::{render_with_config, RenderConfig};

const SCENE: &str = r#"
row r [gap: 40] {
    rect a [width: 60, height: 40, appears: later]
    rect b [width: 60, height: 40, appears: later]
    rect c [width: 60, height: 40, appears: later]
}
keyframe "k" [auto, after: 0.6] {
    show a, b, c [enter: pop, stagger: 0.2, duration: 0.7]
    when c shown + 0.3 { pulse a [delay: 0.1] }
    at 1.5 { pulse b }
}
"#;

/// Every number followed by `s` in the timeline.
fn seconds(css: Option<&str>) -> Vec<f64> {
    let mut cfg = RenderConfig::new();
    cfg.timeline = true;
    if let Some(c) = css {
        cfg = cfg.with_custom_css(c.to_string());
    }
    let t = render_with_config(SCENE, cfg).expect("timeline");
    t.split(|c: char| c.is_whitespace() || c == '(' || c == ',' || c == '@' || c == '+')
        .filter_map(|w| w.strip_suffix('s').and_then(|n| n.parse::<f64>().ok()))
        .collect()
}

#[test]
fn speed_divides_every_time() {
    let normal = seconds(None);
    let fast = seconds(Some(":root { --ail-motion-speed: 2; }"));
    assert!(normal.len() > 5, "{normal:?}");
    assert_eq!(normal.len(), fast.len());
    for (n, f) in normal.iter().zip(&fast) {
        assert!((n / 2.0 - f).abs() < 0.011, "{n}s at speed 2 should be {}s, got {f}s", n / 2.0);
    }
}
