//! `--skill-brief` and `--doc`: the short path through the documentation
//! stays true (its scene is a real, clean scene) and complete (every topic it
//! names can be fetched).

use agent_illustrator::docs;
use agent_illustrator::{render_with_config, render_with_lint, RenderConfig};

fn brief_scene() -> String {
    let d = docs::SKILL_BRIEF;
    let start = d.find("```ail\n").expect("an ```ail block") + "```ail\n".len();
    let end = start + d[start..].find("```").unwrap();
    d[start..end].to_string()
}

#[test]
fn the_brief_scene_lints_clean_and_has_its_steps() {
    let (_, w) = render_with_lint(&brief_scene(), RenderConfig::new().with_lint(true)).expect("renders");
    let w: Vec<String> = w.into_iter().map(|w| w.message).collect();
    assert!(w.is_empty(), "{w:?}");
    let mut c = RenderConfig::new();
    c.states = true;
    let s = render_with_config(&brief_scene(), c).unwrap();
    assert!(s.contains("start") && s.contains("clone"), "{s}");
}

#[test]
fn every_topic_the_brief_names_can_be_fetched() {
    let line = docs::SKILL_BRIEF.lines().skip_while(|l| !l.starts_with("Topics:")).collect::<Vec<_>>().join(" ");
    let named: Vec<String> = line
        .trim_start_matches("Topics:")
        .split(',')
        .map(|t| t.trim().trim_end_matches('.').to_string())
        .filter(|t| !t.is_empty())
        .collect();
    assert!(named.len() > 20, "{named:?}");
    for t in named {
        let s = docs::section(&t).unwrap_or_else(|| panic!("--doc {t} finds nothing"));
        assert!(s.lines().count() >= 3, "--doc {t} is too short: {s}");
    }
}

#[test]
fn a_topic_is_a_section_not_a_whole_guide() {
    for t in docs::topics() {
        let s = docs::section(t).unwrap_or_else(|| panic!("--doc {t}"));
        if t != "styling" && t != "clipart" {
            assert!(s.lines().count() < 200, "--doc {t} returns {} lines", s.lines().count());
        }
    }
}

#[test]
fn the_brief_is_brief() {
    let words = docs::SKILL_BRIEF.split_whitespace().count();
    assert!(words < 1200, "the brief has grown to {words} words; move detail into --doc topics");
}
