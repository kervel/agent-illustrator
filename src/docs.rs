//! The documentation shipped in the binary, and `--doc <topic>`: one
//! section at a time, for agents whose context has no room for whole guides.

pub const SKILL: &str = include_str!("../docs/skill.md");
const SKILL_BRIEF_SRC: &str = include_str!("../docs/skill-brief.md");
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
    ("layering", "animation", "Layering and overlaps"),
    ("overlaps", "animation", "Layering and overlaps"),
    ("captions", "animation", "Captions"),
];

/// Other words for the same topics (what an agent would type).
const ALIASES: &[(&str, &str)] = &[
    ("move", "travel"),
    ("fly", "travel"),
    ("pivot", "hinges"),
    ("rotate", "hinges"),
    ("rotation", "hinges"),
    ("stagger", "swarm"),
    ("count", "swarm"),
    ("meter", "swarm"),
    ("many", "swarm"),
    ("timing", "beats"),
    ("when", "events"),
    ("then", "beats"),
    ("highlight", "accent"),
    ("emphasis", "accent"),
    ("mark", "accent"),
    ("unmark", "accent"),
    ("marker", "accent"),
    ("caption", "captions"),
    ("z-order", "layering"),
    ("z_order", "layering"),
    ("order", "layering"),
    ("overlap", "overlaps"),
    ("state", "states"),
    ("set", "states"),
    ("table", "tables"),
    ("layout", "layouts"),
    ("line", "lines"),
    ("draw", "lines"),
    ("show", "entrances"),
    ("appears", "entrances"),
    ("colors", "roles"),
    ("colours", "roles"),
    ("theme", "roles"),
    ("png", "verify"),
    ("lint", "verify"),
    ("icons", "artwork"),
    ("svg", "artwork"),
    ("symbols", "text"),
    ("glyphs", "text"),
    ("marks", "text"),
    ("fonts", "text"),
];

/// The brief, with its topic list filled in from the one table `--doc`
/// uses (they cannot drift apart).
pub fn skill_brief() -> String {
    SKILL_BRIEF_SRC.replace("{TOPICS}", &topics().join(", "))
}

/// The closest topics to a word (for "did you mean").
pub fn suggest(word: &str) -> Vec<&'static str> {
    let w = word.to_lowercase();
    let dist = |a: &str, b: &str| -> usize {
        let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
        let mut prev: Vec<usize> = (0..=b.len()).collect();
        for i in 1..=a.len() {
            let mut cur = vec![i; b.len() + 1];
            for j in 1..=b.len() {
                let c = if a[i - 1] == b[j - 1] { 0 } else { 1 };
                cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + c);
            }
            prev = cur;
        }
        prev[b.len()]
    };
    let mut all: Vec<(&str, usize)> = TOPICS
        .iter()
        .map(|(t, _, _)| *t)
        .chain(ALIASES.iter().map(|(a, _)| *a))
        .map(|t| (t, if t.starts_with(&w) || w.starts_with(t) { 0 } else { dist(t, &w) }))
        .filter(|(_, d)| *d <= 2)
        .collect();
    all.sort_by_key(|(_, d)| *d);
    let mut out: Vec<&str> = Vec::new();
    for (t, _) in all {
        let t = ALIASES.iter().find(|(a, _)| *a == t).map(|(_, to)| *to).unwrap_or(t);
        if !out.contains(&t) {
            out.push(t);
        }
    }
    out.truncate(3);
    out
}

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
    let t = ALIASES.iter().find(|(a, _)| *a == t).map(|(_, to)| to.to_string()).unwrap_or(t);
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
