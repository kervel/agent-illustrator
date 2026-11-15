//! `--animate-css`: a text element whose wording swaps and whose fill changes.
//!
//! A text element's wordings (`.aitxt-<id>-base`, `-v0`, ...) are the shape
//! itself, so each node also carries the shape hook `.kfp-<id>`. The film CSS
//! wrote one `animation:` rule per selector; the `.kfp-` rule (the fill) came
//! later and replaced the wordings' opacity animation, so the label never
//! swapped. Both must play.

use agent_illustrator::{render_with_config, RenderConfig};

const SOURCE: &str = r#"
text "state: Idle" status [font_size: 18]
keyframe "one" {
    transform status [label: "state: Busy", fill: red]
}
keyframe "two" {
    transform status [label: "state: Done"]
}
"#;

fn film() -> String {
    let mut config = RenderConfig::new();
    config.animate_css = true;
    render_with_config(SOURCE, config).unwrap_or_else(|e| panic!("{e}"))
}

/// The animation names of the last rule whose selector is exactly `sel`.
fn animations_of(svg: &str, sel: &str) -> Vec<String> {
    let rule = svg
        .lines()
        .filter(|l| l.split(" {").next() == Some(sel))
        .last()
        .unwrap_or_else(|| panic!("no rule for {sel}"));
    rule.split("animation:")
        .nth(1)
        .unwrap()
        .split(',')
        .map(|a| a.split_whitespace().next().unwrap().to_string())
        .collect()
}

#[test]
fn a_wording_swap_survives_a_fill_change_on_the_same_text() {
    let svg = film();
    let scope_at = svg.find(".kfp-").expect("a shape hook");
    let kfp = svg[scope_at..].split(' ').next().unwrap().to_string();
    assert!(kfp.ends_with("-status"), "{kfp}");
    let fill = animations_of(&svg, &kfp);
    assert_eq!(fill.len(), 1, "the fill animation: {fill:?}");

    for part in ["base", "v0", "v1"] {
        let variant = kfp.replacen(".kfp-", ".aitxt-", 1) + "-" + part;
        let own = animations_of(&svg, &variant);
        // The node has both classes: the rule that matches both must run
        // the wording's opacity and the fill together.
        let both = animations_of(&svg, &format!("{variant}{kfp}"));
        for a in own.iter().chain(&fill) {
            assert!(both.contains(a), "{variant}{kfp} lacks {a}: {both:?}");
        }
    }
}

#[test]
fn the_variants_carry_both_classes_so_the_compound_rule_applies() {
    let svg = film();
    let variants: Vec<&str> = svg
        .split("<text ")
        .skip(1)
        .filter(|t| t.contains("aitxt-") && t.contains("-status-"))
        .collect();
    assert_eq!(variants.len(), 3);
    for v in variants {
        assert!(v.contains("kfp-"), "a text wording without its shape hook: {v}");
    }
}
