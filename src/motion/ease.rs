//! Easing curves: CSS cubic-bezier, evaluated identically in Rust (for the
//! native `--at` sampler) and in the browser (the player hands the same
//! `cubic-bezier(...)` string to the Web Animations API).

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ease {
    Linear,
    Bezier(f64, f64, f64, f64),
}

impl Ease {
    pub fn eval(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match *self {
            Ease::Linear => t,
            Ease::Bezier(x1, y1, x2, y2) => {
                if t <= 0.0 {
                    return 0.0;
                }
                if t >= 1.0 {
                    return 1.0;
                }
                let bez = |a: f64, b: f64, s: f64| {
                    let u = 1.0 - s;
                    3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
                };
                let dbez = |a: f64, b: f64, s: f64| {
                    let u = 1.0 - s;
                    3.0 * u * u * a + 6.0 * u * s * (b - a) + 3.0 * s * s * (1.0 - b)
                };
                // Newton, then bisection as a fallback.
                let mut s = t;
                for _ in 0..8 {
                    let x = bez(x1, x2, s) - t;
                    let d = dbez(x1, x2, s);
                    if x.abs() < 1e-7 {
                        return bez(y1, y2, s);
                    }
                    if d.abs() < 1e-6 {
                        break;
                    }
                    s = (s - x / d).clamp(0.0, 1.0);
                }
                let (mut lo, mut hi) = (0.0, 1.0);
                s = t;
                for _ in 0..60 {
                    let x = bez(x1, x2, s);
                    if (x - t).abs() < 1e-7 {
                        break;
                    }
                    if x < t {
                        lo = s;
                    } else {
                        hi = s;
                    }
                    s = (lo + hi) / 2.0;
                }
                bez(y1, y2, s)
            }
        }
    }

    /// The accelerating half of a motion that runs through a midpoint (a
    /// flip turning edge-on): an ease-out curve reflected into its ease-in,
    /// an in-out curve's first half, an ease-in as it is.
    pub fn in_half(&self) -> Ease {
        match *self {
            Ease::Linear => Ease::Linear,
            Ease::Bezier(x1, y1, x2, y2) => {
                let fast_start = y1 > x1; // ease-out family
                let slow_end = y2 < x2; // ease-in family
                if fast_start && !slow_end {
                    // Reflect through the centre: out -> in.
                    Ease::Bezier(1.0 - x2, 1.0 - y2, 1.0 - x1, 1.0 - y1)
                } else if !fast_start && !slow_end && y2 > x2 {
                    // In-out: keep the slow start, accelerate to the end.
                    Ease::Bezier(x1, y1, 1.0, 1.0)
                } else {
                    *self
                }
            }
        }
    }

    /// The decelerating half (the mirror of `in_half`).
    pub fn out_half(&self) -> Ease {
        match self.in_half() {
            Ease::Linear => Ease::Linear,
            Ease::Bezier(x1, y1, x2, y2) => Ease::Bezier(1.0 - x2, 1.0 - y2, 1.0 - x1, 1.0 - y1),
        }
    }

    pub fn css(&self) -> String {
        match *self {
            Ease::Linear => "linear".to_string(),
            Ease::Bezier(a, b, c, d) => format!("cubic-bezier({},{},{},{})", r4(a), r4(b), r4(c), r4(d)),
        }
    }

    /// Parse `cubic-bezier(a, b, c, d)` or `linear`.
    pub fn parse_css(s: &str) -> Option<Ease> {
        let s = s.trim();
        if s == "linear" {
            return Some(Ease::Linear);
        }
        let inner = s.strip_prefix("cubic-bezier(")?.strip_suffix(')')?;
        let v: Vec<f64> = inner.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        if v.len() == 4 {
            Some(Ease::Bezier(v[0], v[1], v[2], v[3]))
        } else {
            None
        }
    }
}

fn r4(x: f64) -> f64 {
    (x * 10000.0).round() / 10000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezier_endpoints_and_overshoot() {
        let pop = Ease::Bezier(0.34, 1.56, 0.64, 1.0);
        assert_eq!(pop.eval(0.0), 0.0);
        assert_eq!(pop.eval(1.0), 1.0);
        assert!(pop.eval(0.6) > 1.0, "pop overshoots");
        let lin = Ease::Bezier(0.0, 0.0, 1.0, 1.0);
        assert!((lin.eval(0.3) - 0.3).abs() < 1e-4);
    }
}
