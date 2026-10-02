//! `--skill-brief` and `--doc`: the short path through the documentation
//! stays true (its scene is a real, clean scene) and complete (every topic it
//! names can be fetched).

use agent_illustrator::docs;
use agent_illustrator::{render_with_config, render_with_lint, RenderConfig};

fn brief_scene() -> String {
    let d = docs::skill_brief();
    let d = d.as_str();
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
    let brief = docs::skill_brief();
    let line = brief.lines().skip_while(|l| !l.starts_with("Topics:")).collect::<Vec<_>>().join(" ");
    let named: Vec<String> = line
        .trim_start_matches("Topics:")
        .split('.')
        .next()
        .unwrap()
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
    let words = docs::skill_brief().split_whitespace().count();
    assert!(words < 1200, "the brief has grown to {words} words; move detail into --doc topics");
}

#[test]
fn the_brief_lists_exactly_the_topics_doc_knows() {
    let brief = docs::skill_brief();
    for t in docs::topics() {
        assert!(brief.contains(t), "the brief does not list {t}");
    }
}

#[test]
fn other_words_find_the_same_sections() {
    assert_eq!(docs::section("move"), docs::section("travel"));
    assert_eq!(docs::section("z-order"), docs::section("layering"));
    assert!(docs::suggest("stagr").contains(&"swarm") || docs::suggest("stagr").contains(&"stage"));
}

#[test]
fn the_text_section_lists_the_safe_marks() {
    let text = agent_illustrator::docs::section("text").expect("text section");
    assert!(text.contains(agent_illustrator::raster::SAFE_MARKS), "{text}");
    assert!(agent_illustrator::docs::section("symbols").is_some());
}

#[test]
fn every_entry_point_says_z_order_exists() {
    // An agent that reads only one of these must still learn that layering
    // can be said explicitly, and where to read more.
    use agent_illustrator::docs;
    let examples = include_str!("../docs/examples.md");
    for (name, text) in [
        ("--skill-brief", docs::skill_brief()),
        ("--skill", docs::SKILL.to_string()),
        ("--skill-animation", docs::SKILL_ANIMATION.to_string()),
        ("--grammar", docs::GRAMMAR.to_string()),
        ("--examples", examples.to_string()),
    ] {
        assert!(text.contains("z_order"), "{name} never mentions z_order");
        assert!(text.contains("scene("), "{name} never mentions z_order: scene(N)");
        assert!(text.contains("[z_order:") || text.contains("z_order: N]"), "{name} never shows a layer change in motion");
    }
    for w in ["zorder", "z-index", "front", "behind", "under", "stacking", "paint", "z_order", "z-order"] {
        assert_eq!(docs::section(w), docs::section("layering"), "--doc {w}");
    }
}
