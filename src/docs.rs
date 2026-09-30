//! The documentation shipped in the binary, and `--doc <topic>`: one
//! section at a time, for agents whose context has no room for whole guides.

pub const SKILL: &str = include_str!("../docs/skill.md");
pub const SKILL_BRIEF: &str = include_str!("../docs/skill-brief.md");
pub const SKILL_ANIMATION: &str = include_str!("../docs/skill-animation.md");
pub const SKILL_STYLING: &str = include_str!("../docs/skill-styling.md");
pub const SKILL_CLIPART: &str = include_str!("../docs/skill-find-clipart.md");
pub const GRAMMAR: &str = include_str!("../docs/grammar.md");

/// Topic -> (document, the start of the section's heading).
const TOPICS: &[(&str, &str, &str)] = &[
    ("stage", "animation", "The stage"),
    ("constraints", "grammar", "CONSTRAINTS"),
    ("templates", "grammar", "TEMPLATES"),
    ("artwork", "animation", "Artwork with parts"),
    ("windows", "animation", "Windows and panels"),
    ("tables", "animation", "Tables on a slide"),
    ("code", "animation", "Code on a slide"),
    ("beats", "animation", "Beats"),
    ("events", "animation", "Beats"),
    ("entrances", "animation", "Entrances and exits"),
    ("lines", "animation", "Lines that grow"),
    ("travel", "animation", "Travel"),
    ("accent", "animation", "Look here"),
    ("states", "animation", "Components with states"),
    ("layouts", "animation", "Layouts that change"),
    ("hinges", "animation", "Hinges"),
    ("swarm", "animation", "Many things at once"),
    ("effects", "animation", "One-shot effects"),
    ("text", "animation", "Text and numbers"),
    ("selectors", "animation", "Selecting many things"),
    ("macros", "animation", "Reuse: motion macros"),
    ("roles", "animation", "Themes: roles"),
    ("verify", "animation", "Verifying without a browser"),
    ("player", "animation", "The player"),
    ("cookbook", "animation", "Cookbook"),
    ("gotchas", "animation", "Part 3: Gotchas"),
    ("styling", "styling", ""),
    ("clipart", "clipart", ""),
    ("first-scene", "animation", "Your first scene"),
];

fn doc(name: &str) -> &'static str {
    match name {
        "animation" => SKILL_ANIMATION,
        "grammar" => GRAMMAR,
        "styling" => SKILL_STYLING,
        "clipart" => SKILL_CLIPART,
        _ => SKILL,
    }
}

/// The topics `--doc` knows, in order.
pub fn topics() -> Vec<&'static str> {
    TOPICS.iter().map(|(t, _, _)| *t).collect()
}

/// Markdown heading level of a line (`### X` = 3), or the grammar's
/// `TITLE` + `-----` style (level 1).
fn heading(lines: &[&str], i: usize) -> Option<(usize, String)> {
    let l = lines[i];
    if let Some(rest) = l.strip_prefix('#') {
        let level = 1 + rest.chars().take_while(|c| *c == '#').count();
        return Some((level, l.trim_start_matches('#').trim().to_string()));
    }
    if !l.trim().is_empty() && lines.get(i + 1).is_some_and(|n| n.len() >= 3 && n.chars().all(|c| c == '-')) {
        return Some((1, l.trim().to_string()));
    }
    None
}

/// One section: from its heading to the next heading of the same or a
/// higher level. `needle` matches the start of the heading text.
fn section_of(text: &str, needle: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let needle = needle.to_lowercase();
    let start = (0..lines.len()).find(|&i| heading(&lines, i).is_some_and(|(_, h)| h.to_lowercase().starts_with(&needle)))?;
    let (level, _) = heading(&lines, start)?;
    let mut end = lines.len();
    for (j, _) in lines.iter().enumerate().skip(start + 1) {
        if let Some((l, _)) = heading(&lines, j) {
            if l <= level && !(j == start + 1 && lines[j].chars().all(|c| c == '-')) {
                end = j;
                break;
            }
        }
    }
    Some(lines[start..end].join("\n").trim_end().to_string() + "\n")
}

/// `--doc <topic>`: the section, or None for an unknown topic.
pub fn section(topic: &str) -> Option<String> {
    let t = topic.trim().to_lowercase();
    if let Some((_, d, needle)) = TOPICS.iter().find(|(name, _, _)| *name == t) {
        if needle.is_empty() {
            return Some(doc(d).to_string());
        }
        return section_of(doc(d), needle);
    }
    // Any heading in the guides that starts with the words.
    for d in ["animation", "skill", "grammar", "styling", "clipart"] {
        if let Some(s) = section_of(doc(d), &t) {
            return Some(s);
        }
    }
    None
}
