//! Orthogonal connections leaving one point the same way branch off at one
//! place, even when their targets are not level.

use agent_illustrator::render;

/// The `M x y L x y ...` points of every routed connection path.
fn paths(svg: &str) -> Vec<Vec<(f64, f64)>> {
    svg.split("d=\"M")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .filter(|d| d.contains('L') && !d.contains('A') && !d.contains('C') && !d.contains('Z'))
        .map(|d| {
            let nums: Vec<f64> = d
                .replace('L', " ")
                .split_whitespace()
                .filter_map(|n| n.parse().ok())
                .collect();
            nums.chunks(2).filter(|c| c.len() == 2).map(|c| (c[0], c[1])).collect()
        })
        .collect()
}

#[test]
fn a_fan_out_branches_at_one_place() {
    let src = r#"
rect hub [width: 120, height: 60]
rect a [width: 80, height: 80]
rect b [width: 80, height: 40]
rect c [width: 80, height: 60]
constrain hub.left = 200
constrain hub.top = 0
constrain a.left = 0
constrain a.top = 160
constrain b.left = 220
constrain b.top = 200
constrain c.left = 440
constrain c.top = 180
hub.bottom -> a.top [routing: orthogonal]
hub.bottom -> b.top [routing: orthogonal]
hub.bottom -> c.top [routing: orthogonal]
"#;
    let svg = render(src).expect("renders");
    let zs: Vec<Vec<(f64, f64)>> = paths(&svg).into_iter().filter(|p| p.len() == 4).collect();
    assert!(zs.len() >= 2, "expected bent fan-out routes, got {:?}", paths(&svg));
    let ys: Vec<f64> = zs.iter().map(|p| p[1].1).collect();
    assert!(ys.windows(2).all(|w| (w[0] - w[1]).abs() < 0.5), "branch points differ: {ys:?}");
}
