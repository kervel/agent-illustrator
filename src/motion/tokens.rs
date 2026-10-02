//! Motion design tokens: durations, eases and distances, read from the
//! stylesheet CSS so a deck is retimed in one place.
//!
//! ```css
//! :root { --ail-motion-normal: .5s; --ail-ease-pop: cubic-bezier(.34,1.56,.64,1); }
//! ```

use std::collections::HashMap;

use super::ease::Ease;

#[derive(Debug, Clone)]
pub struct MotionTokens {
    pub durations: HashMap<String, f64>,
    pub eases: HashMap<String, Ease>,
    /// Travel distance of rise/drop/fall/nudge/shake, in px.
    pub distance: f64,
    /// CSS custom properties (colours) for interpolating `var(--x)`.
    pub vars: HashMap<String, String>,
    /// `--ail-motion-speed: 1.3`: everything plays 1.3 times as fast
    /// (durations, delays, staggers, `at`, event shifts, `[auto, after:]`).
    pub speed: f64,
}

impl Default for MotionTokens {
    fn default() -> Self {
        let mut durations = HashMap::new();
        durations.insert("instant".into(), 0.0);
        durations.insert("fast".into(), 0.25);
        durations.insert("normal".into(), 0.5);
        durations.insert("slow".into(), 0.9);
        let mut eases = HashMap::new();
        eases.insert("linear".into(), Ease::Linear);
        // back-out overshoot
        eases.insert("pop".into(), Ease::Bezier(0.34, 1.56, 0.64, 1.0));
        // power3-out
        eases.insert("settle".into(), Ease::Bezier(0.215, 0.61, 0.355, 1.0));
        eases.insert("out".into(), Ease::Bezier(0.215, 0.61, 0.355, 1.0));
        // in-out
        eases.insert("glide".into(), Ease::Bezier(0.65, 0.0, 0.35, 1.0));
        eases.insert("snap".into(), Ease::Bezier(0.2, 0.0, 0.0, 1.0));
        // power2-in
        eases.insert("in".into(), Ease::Bezier(0.55, 0.0, 1.0, 0.45));
        MotionTokens { durations, eases, distance: 12.0, vars: HashMap::new(), speed: 1.0 }
    }
}

/// Every `--name: value;` declaration in a CSS text, in order.
pub fn css_custom_properties(css: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = css.as_bytes();
    let mut i = 0;
    while let Some(pos) = css[i..].find("--") {
        let start = i + pos + 2;
        // Must begin a declaration: preceded by `{`, `;` or whitespace.
        let prev = if i + pos == 0 { b' ' } else { bytes[i + pos - 1] };
        if !(prev == b'{' || prev == b';' || prev.is_ascii_whitespace()) {
            i = start;
            continue;
        }
        let rest = &css[start..];
        let Some(colon) = rest.find(':') else { break };
        let name = rest[..colon].trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            i = start;
            continue;
        }
        let after = &rest[colon + 1..];
        let end = after.find([';', '}']).unwrap_or(after.len());
        out.push((name.to_string(), after[..end].trim().to_string()));
        i = start + colon + 1 + end;
    }
    out
}

fn parse_seconds(v: &str) -> Option<f64> {
    let v = v.trim();
    if let Some(ms) = v.strip_suffix("ms") {
        return ms.trim().parse::<f64>().ok().map(|x| x / 1000.0);
    }
    v.strip_suffix('s').unwrap_or(v).trim().parse().ok()
}

impl MotionTokens {
    /// Tokens from the palette plus any custom CSS (later wins).
    pub fn from_styles(colors: &HashMap<String, String>, custom_css: Option<&str>) -> Self {
        let mut t = MotionTokens::default();
        for (k, v) in colors {
            t.vars.insert(k.clone(), v.clone());
        }
        if let Some(css) = custom_css {
            for (name, value) in css_custom_properties(css) {
                if let Some(d) = name.strip_prefix("ail-motion-") {
                    if d == "speed" {
                        if let Ok(k) = value.trim().parse::<f64>() {
                            if k > 0.0 {
                                t.speed = k;
                            }
                        }
                    } else if d == "distance" {
                        if let Ok(px) = value.trim_end_matches("px").trim().parse() {
                            t.distance = px;
                        }
                    } else if let Some(s) = parse_seconds(&value) {
                        t.durations.insert(d.to_string(), s);
                    }
                } else if let Some(e) = name.strip_prefix("ail-ease-") {
                    if let Some(ease) = Ease::parse_css(&value) {
                        t.eases.insert(e.to_string(), ease);
                    }
                } else {
                    t.vars.insert(name, value);
                }
            }
        }
        t
    }

    pub fn duration(&self, name: &str) -> Option<f64> {
        self.durations.get(name).copied()
    }

    pub fn ease(&self, name: &str) -> Option<Ease> {
        self.eases.get(name).copied()
    }

    /// The CSS that declares every token, so the page and the compiler agree.
    pub fn css(&self) -> String {
        let mut keys: Vec<_> = self.durations.iter().collect();
        keys.sort_by(|a, b| a.0.cmp(b.0));
        let mut s = String::from(":root {");
        for (k, v) in keys {
            s.push_str(&format!(" --ail-motion-{}: {}s;", k, v));
        }
        s.push_str(" }");
        s
    }

    /// Resolve a colour (hex, `var(--x)`, or a CSS name) to RGB.
    pub fn rgb(&self, c: &str) -> Option<(f64, f64, f64)> {
        let mut c = c.trim().to_string();
        for _ in 0..8 {
            if let Some(inner) = c.strip_prefix("var(--").and_then(|x| x.strip_suffix(')')) {
                let name = inner.split(',').next().unwrap_or(inner).trim();
                c = self.vars.get(name)?.trim().to_string();
            } else {
                break;
            }
        }
        parse_color(&c)
    }
}

pub fn parse_color(c: &str) -> Option<(f64, f64, f64)> {
    let c = c.trim();
    if let Some(h) = c.strip_prefix('#') {
        let h: String = if h.len() == 3 {
            h.chars().flat_map(|ch| [ch, ch]).collect()
        } else {
            h.to_string()
        };
        if h.len() < 6 {
            return None;
        }
        let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok().map(|v| v as f64);
        return Some((p(0)?, p(2)?, p(4)?));
    }
    if let Some(inner) = c.strip_prefix("rgb(").or_else(|| c.strip_prefix("rgba(")) {
        let v: Vec<f64> = inner
            .trim_end_matches(')')
            .split([',', ' '])
            .filter(|s| !s.is_empty())
            .filter_map(|x| x.trim().parse().ok())
            .collect();
        if v.len() >= 3 {
            return Some((v[0], v[1], v[2]));
        }
    }
    Some(match c {
        "white" => (255.0, 255.0, 255.0),
        "black" => (0.0, 0.0, 0.0),
        "red" => (255.0, 0.0, 0.0),
        "green" => (0.0, 128.0, 0.0),
        "blue" => (0.0, 0.0, 255.0),
        "gray" | "grey" => (128.0, 128.0, 128.0),
        "yellow" => (255.0, 255.0, 0.0),
        "orange" => (255.0, 165.0, 0.0),
        "purple" => (128.0, 0.0, 128.0),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_tokens_from_css() {
        let css = ":root { --ail-motion-normal: .4s; --ail-ease-pop: cubic-bezier(.3,1.8,.6,1); --main: #2346D8; }";
        let t = MotionTokens::from_styles(&HashMap::new(), Some(css));
        assert_eq!(t.duration("normal"), Some(0.4));
        assert_eq!(t.ease("pop"), Some(Ease::Bezier(0.3, 1.8, 0.6, 1.0)));
        assert_eq!(t.rgb("var(--main)"), Some((35.0, 70.0, 216.0)));
    }

    #[test]
    fn speed_is_a_factor_not_a_duration() {
        let t = MotionTokens::from_styles(&HashMap::new(), Some(":root { --ail-motion-speed: 1.3; }"));
        assert_eq!(t.speed, 1.3);
        assert_eq!(t.duration("speed"), None);
    }
}
