//! Geometry of drawn lines: flatten an SVG path `d` into a measured polyline.
//!
//! Everything a motion statement says about a line (`to: 60%`, `to: vertex 2`,
//! `to: st_hotfix`, `move along`) is resolved here, from the exact `d` string
//! the renderer emits, so the numbers match what the browser draws.

use crate::layout::types::Point;

#[derive(Debug, Clone, Default)]
pub struct Polyline {
    pub pts: Vec<Point>,
    /// Cumulative length at each point.
    pub cum: Vec<f64>,
    /// Cumulative length at each command endpoint ("vertex").
    pub vertices: Vec<f64>,
}

impl Polyline {
    pub fn length(&self) -> f64 {
        self.cum.last().copied().unwrap_or(0.0)
    }

    fn push(&mut self, p: Point) {
        let d = match self.pts.last() {
            Some(q) => ((p.x - q.x).powi(2) + (p.y - q.y).powi(2)).sqrt(),
            None => 0.0,
        };
        let c = self.cum.last().copied().unwrap_or(0.0) + d;
        self.pts.push(p);
        self.cum.push(c);
    }

    fn mark_vertex(&mut self) {
        self.vertices.push(self.length());
    }

    pub fn from_points(points: &[Point]) -> Self {
        let mut pl = Polyline::default();
        for p in points {
            pl.push(*p);
            pl.mark_vertex();
        }
        pl
    }

    /// The point at `frac` of the length.
    pub fn point_at(&self, frac: f64) -> Point {
        if self.pts.is_empty() {
            return Point::new(0.0, 0.0);
        }
        let target = frac.clamp(0.0, 1.0) * self.length();
        for i in 1..self.pts.len() {
            if self.cum[i] >= target {
                let seg = self.cum[i] - self.cum[i - 1];
                let t = if seg > 0.0 { (target - self.cum[i - 1]) / seg } else { 0.0 };
                let (a, b) = (self.pts[i - 1], self.pts[i]);
                return Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
            }
        }
        *self.pts.last().unwrap()
    }

    /// Fraction of the length at which vertex `n` sits.
    pub fn vertex_fraction(&self, n: usize) -> Option<f64> {
        let len = self.length();
        self.vertices.get(n).map(|v| if len > 0.0 { v / len } else { 0.0 })
    }

    /// Fraction of the length of the point nearest `p`, and its distance.
    pub fn project(&self, p: Point) -> (f64, f64) {
        let len = self.length();
        let mut best = (0.0, f64::INFINITY);
        for i in 1..self.pts.len() {
            let (a, b) = (self.pts[i - 1], self.pts[i]);
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let l2 = dx * dx + dy * dy;
            let t = if l2 > 0.0 {
                (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let q = Point::new(a.x + dx * t, a.y + dy * t);
            let dist = ((p.x - q.x).powi(2) + (p.y - q.y).powi(2)).sqrt();
            if dist < best.1 {
                let at = self.cum[i - 1] + (self.cum[i] - self.cum[i - 1]) * t;
                best = (if len > 0.0 { at / len } else { 0.0 }, dist);
            }
        }
        best
    }

    /// Points from `f0` to `f1` of the length (either direction).
    pub fn sub_points(&self, f0: f64, f1: f64, samples: usize) -> Vec<Point> {
        (0..=samples)
            .map(|i| self.point_at(f0 + (f1 - f0) * i as f64 / samples as f64))
            .collect()
    }
}

fn tokenize(d: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let b = d.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c.is_ascii_alphabetic() && c != 'e' && c != 'E' {
            out.push(Tok::Cmd(c));
            i += 1;
        } else if c.is_ascii_digit() || c == '-' || c == '+' || c == '.' {
            let start = i;
            let mut seen_dot = c == '.';
            i += 1;
            while i < b.len() {
                let ch = b[i] as char;
                if ch.is_ascii_digit() {
                    i += 1;
                } else if ch == '.' && !seen_dot {
                    seen_dot = true;
                    i += 1;
                } else if (ch == 'e' || ch == 'E') && i + 1 < b.len() {
                    i += 1;
                    if b[i] == b'-' || b[i] == b'+' {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            if let Ok(v) = d[start..i].parse() {
                out.push(Tok::Num(v));
            }
        } else {
            i += 1;
        }
    }
    out
}

#[derive(Debug, Clone, Copy)]
enum Tok {
    Cmd(char),
    Num(f64),
}

const STEPS: usize = 24;

/// Flatten a path `d` (all subpaths concatenated, in drawing order).
pub fn flatten_d(d: &str) -> Polyline {
    let toks = tokenize(d);
    let mut pl = Polyline::default();
    let mut i = 0;
    let mut cmd = 'M';
    let mut cur = Point::new(0.0, 0.0);
    let mut start = cur;
    let mut last_ctrl: Option<Point> = None;
    let num = |i: &mut usize| -> Option<f64> {
        match toks.get(*i) {
            Some(Tok::Num(v)) => {
                *i += 1;
                Some(*v)
            }
            _ => None,
        }
    };
    let cubic = |pl: &mut Polyline, p0: Point, c1: Point, c2: Point, p: Point| {
        for s in 1..=STEPS {
            let t = s as f64 / STEPS as f64;
            let u = 1.0 - t;
            pl.push(Point::new(
                u * u * u * p0.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * p.x,
                u * u * u * p0.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * p.y,
            ));
        }
    };
    let quad = |pl: &mut Polyline, p0: Point, c: Point, p: Point| {
        for s in 1..=STEPS {
            let t = s as f64 / STEPS as f64;
            let u = 1.0 - t;
            pl.push(Point::new(
                u * u * p0.x + 2.0 * u * t * c.x + t * t * p.x,
                u * u * p0.y + 2.0 * u * t * c.y + t * t * p.y,
            ));
        }
    };
    while i < toks.len() {
        if let Tok::Cmd(c) = toks[i] {
            cmd = c;
            i += 1;
            if c == 'Z' || c == 'z' {
                pl.push(start);
                pl.mark_vertex();
                cur = start;
                last_ctrl = None;
                continue;
            }
        }
        let rel = cmd.is_ascii_lowercase();
        let base = if rel { cur } else { Point::new(0.0, 0.0) };
        let before = i;
        match cmd.to_ascii_uppercase() {
            'M' => {
                let (Some(x), Some(y)) = (num(&mut i), num(&mut i)) else { break };
                cur = Point::new(base.x + x, base.y + y);
                start = cur;
                pl.push(cur);
                pl.mark_vertex();
                // Subsequent pairs are implicit line-tos.
                cmd = if rel { 'l' } else { 'L' };
                last_ctrl = None;
            }
            'L' => {
                let (Some(x), Some(y)) = (num(&mut i), num(&mut i)) else { break };
                cur = Point::new(base.x + x, base.y + y);
                pl.push(cur);
                pl.mark_vertex();
                last_ctrl = None;
            }
            'H' => {
                let Some(x) = num(&mut i) else { break };
                cur = Point::new(if rel { cur.x + x } else { x }, cur.y);
                pl.push(cur);
                pl.mark_vertex();
                last_ctrl = None;
            }
            'V' => {
                let Some(y) = num(&mut i) else { break };
                cur = Point::new(cur.x, if rel { cur.y + y } else { y });
                pl.push(cur);
                pl.mark_vertex();
                last_ctrl = None;
            }
            'C' => {
                let v: Vec<f64> = (0..6).filter_map(|_| num(&mut i)).collect();
                if v.len() < 6 {
                    break;
                }
                let c1 = Point::new(base.x + v[0], base.y + v[1]);
                let c2 = Point::new(base.x + v[2], base.y + v[3]);
                let p = Point::new(base.x + v[4], base.y + v[5]);
                cubic(&mut pl, cur, c1, c2, p);
                pl.mark_vertex();
                last_ctrl = Some(c2);
                cur = p;
            }
            'S' => {
                let v: Vec<f64> = (0..4).filter_map(|_| num(&mut i)).collect();
                if v.len() < 4 {
                    break;
                }
                let c1 = last_ctrl.map(|c| Point::new(2.0 * cur.x - c.x, 2.0 * cur.y - c.y)).unwrap_or(cur);
                let c2 = Point::new(base.x + v[0], base.y + v[1]);
                let p = Point::new(base.x + v[2], base.y + v[3]);
                cubic(&mut pl, cur, c1, c2, p);
                pl.mark_vertex();
                last_ctrl = Some(c2);
                cur = p;
            }
            'Q' => {
                let v: Vec<f64> = (0..4).filter_map(|_| num(&mut i)).collect();
                if v.len() < 4 {
                    break;
                }
                let c = Point::new(base.x + v[0], base.y + v[1]);
                let p = Point::new(base.x + v[2], base.y + v[3]);
                quad(&mut pl, cur, c, p);
                pl.mark_vertex();
                last_ctrl = Some(c);
                cur = p;
            }
            'T' => {
                let v: Vec<f64> = (0..2).filter_map(|_| num(&mut i)).collect();
                if v.len() < 2 {
                    break;
                }
                let c = last_ctrl.map(|c| Point::new(2.0 * cur.x - c.x, 2.0 * cur.y - c.y)).unwrap_or(cur);
                let p = Point::new(base.x + v[0], base.y + v[1]);
                quad(&mut pl, cur, c, p);
                pl.mark_vertex();
                last_ctrl = Some(c);
                cur = p;
            }
            'A' => {
                let v: Vec<f64> = (0..7).filter_map(|_| num(&mut i)).collect();
                if v.len() < 7 {
                    break;
                }
                let p = Point::new(base.x + v[5], base.y + v[6]);
                arc(&mut pl, cur, v[0], v[1], v[2], v[3] != 0.0, v[4] != 0.0, p);
                pl.mark_vertex();
                last_ctrl = None;
                cur = p;
            }
            _ => {
                i += 1;
            }
        }
        if i == before {
            i += 1;
        }
    }
    pl
}

/// SVG elliptical arc (endpoint parameterisation, per SVG spec F.6).
#[allow(clippy::too_many_arguments)]
fn arc(pl: &mut Polyline, p0: Point, rx: f64, ry: f64, phi_deg: f64, large: bool, sweep: bool, p: Point) {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx == 0.0 || ry == 0.0 {
        pl.push(p);
        return;
    }
    let phi = phi_deg.to_radians();
    let (cp, sp) = (phi.cos(), phi.sin());
    let dx = (p0.x - p.x) / 2.0;
    let dy = (p0.y - p.y) / 2.0;
    let x1 = cp * dx + sp * dy;
    let y1 = -sp * dx + cp * dy;
    let lam = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lam > 1.0 {
        rx *= lam.sqrt();
        ry *= lam.sqrt();
    }
    let num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut coef = if den > 0.0 { (num / den).max(0.0).sqrt() } else { 0.0 };
    if large == sweep {
        coef = -coef;
    }
    let cx1 = coef * rx * y1 / ry;
    let cy1 = -coef * ry * x1 / rx;
    let cx = cp * cx1 - sp * cy1 + (p0.x + p.x) / 2.0;
    let cy = sp * cx1 + cp * cy1 + (p0.y + p.y) / 2.0;
    let ang = |ux: f64, uy: f64, vx: f64, vy: f64| {
        let a = (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
        a
    };
    let t1 = ang(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut dt = ang((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry);
    if !sweep && dt > 0.0 {
        dt -= std::f64::consts::TAU;
    } else if sweep && dt < 0.0 {
        dt += std::f64::consts::TAU;
    }
    for s in 1..=STEPS {
        let t = t1 + dt * s as f64 / STEPS as f64;
        let (ct, st) = (t.cos(), t.sin());
        pl.push(Point::new(cx + rx * cp * ct - ry * sp * st, cy + rx * sp * ct + ry * cp * st));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_polylines() {
        let pl = flatten_d("M0 0 L100 0 L100 100");
        assert!((pl.length() - 200.0).abs() < 1e-9);
        assert_eq!(pl.vertex_fraction(1), Some(0.5));
        let p = pl.point_at(0.75);
        assert!((p.x - 100.0).abs() < 1e-9 && (p.y - 50.0).abs() < 1e-9);
        let (f, _) = pl.project(Point::new(40.0, 10.0));
        assert!((f - 0.2).abs() < 1e-9);
    }

    #[test]
    fn flattens_arcs() {
        // Half circle of radius 50: length ~ 157
        let pl = flatten_d("M0 0 A50 50 0 0 1 100 0");
        assert!((pl.length() - std::f64::consts::PI * 50.0).abs() < 0.5, "{}", pl.length());
    }
}
