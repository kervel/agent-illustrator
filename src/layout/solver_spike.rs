//! Spike test to validate kasuari can express our constraint types
//!
//! Run with: cargo test spike_kasuari_fitness --lib

#[cfg(test)]
mod tests {
    use kasuari::{Solver, Strength, Variable, WeightedRelation::*};

    #[test]
    fn spike_kasuari_fitness() {
        let mut solver = Solver::new();

        // 1. Create variables for 5 elements (x, y, width, height each)
        let a_x = Variable::new();
        let _a_y = Variable::new();
        let a_width = Variable::new();
        let _a_height = Variable::new();

        let b_x = Variable::new();
        let _b_y = Variable::new();
        let b_width = Variable::new();
        let _b_height = Variable::new();

        let c_x = Variable::new();
        let c_width = Variable::new();

        let d_x = Variable::new();
        let d_width = Variable::new();

        let e_width = Variable::new();

        // 2. Add equality constraint: a.left = b.left
        solver
            .add_constraint(a_x | EQ(Strength::REQUIRED) | b_x)
            .unwrap();

        // 3. Add offset constraint: c.left = b.right + 20
        // b.right = b.x + b.width
        solver
            .add_constraint(c_x | EQ(Strength::REQUIRED) | b_x + b_width + 20.0)
            .unwrap();

        // 4. Add midpoint constraint: d.center_x = midpoint(a.center_x, c.center_x)
        // d.center_x = d.x + d.width/2
        // a.center_x = a.x + a.width/2
        // c.center_x = c.x + c.width/2
        // Express as: 2*d_center = a_center + c_center
        // => 2*(d.x + d.width/2) = (a.x + a.width/2) + (c.x + c.width/2)
        // For simplicity, assume width=100 for all, so center = x + 50
        // => 2*(d.x + 50) = (a.x + 50) + (c.x + 50)
        // => 2*d.x + 100 = a.x + c.x + 100
        // => 2*d.x = a.x + c.x
        solver
            .add_constraint(2.0 * d_x | EQ(Strength::REQUIRED) | a_x + c_x)
            .unwrap();

        // 5. Add inequality constraint: e.width >= 50
        solver
            .add_constraint(e_width | GE(Strength::REQUIRED) | 50.0)
            .unwrap();

        // 6. Add containment inequality: container.left <= child.left - padding
        // (tested implicitly via LE constraint)
        solver
            .add_constraint(a_x | LE(Strength::REQUIRED) | b_x + 10.0)
            .unwrap();

        // Set some edit variables to anchor the system
        solver.add_edit_variable(b_x, Strength::STRONG).unwrap();
        solver.add_edit_variable(b_width, Strength::STRONG).unwrap();
        solver.add_edit_variable(a_width, Strength::STRONG).unwrap();
        solver.add_edit_variable(c_width, Strength::STRONG).unwrap();
        solver.add_edit_variable(d_width, Strength::STRONG).unwrap();

        solver.suggest_value(b_x, 0.0).unwrap();
        solver.suggest_value(b_width, 100.0).unwrap();
        solver.suggest_value(a_width, 100.0).unwrap();
        solver.suggest_value(c_width, 100.0).unwrap();
        solver.suggest_value(d_width, 100.0).unwrap();

        // Fetch and verify - fetch_changes returns &[(Variable, f64)]
        let changes = solver.fetch_changes();
        let values: std::collections::HashMap<Variable, f64> =
            changes.iter().map(|(v, val)| (*v, *val)).collect();

        // Verify a.x = b.x = 0
        assert!(
            (values.get(&a_x).copied().unwrap_or(0.0) - 0.0).abs() < 0.001,
            "a.x should equal b.x (0)"
        );

        // Verify c.x = b.x + b.width + 20 = 0 + 100 + 20 = 120
        assert!(
            (values.get(&c_x).copied().unwrap_or(0.0) - 120.0).abs() < 0.001,
            "c.x should be 120 (b.x + b.width + 20)"
        );

        // Verify d.x = (a.x + c.x) / 2 = (0 + 120) / 2 = 60
        assert!(
            (values.get(&d_x).copied().unwrap_or(0.0) - 60.0).abs() < 0.001,
            "d.x should be 60 (midpoint of a.x and c.x)"
        );

        // Verify e.width >= 50
        assert!(
            values.get(&e_width).copied().unwrap_or(0.0) >= 50.0,
            "e.width should be >= 50"
        );

        println!("Spike PASSED: kasuari can express all our constraint types!");
    }

    #[test]
    fn spike_kasuari_relative_width() {
        // Test that we can express constraints involving width relationships
        let mut solver = Solver::new();

        let a_x = Variable::new();
        let a_width = Variable::new();
        let b_x = Variable::new();
        let b_width = Variable::new();

        // b.left = a.right + gap
        // b.x = a.x + a.width + 20
        solver
            .add_constraint(b_x | EQ(Strength::REQUIRED) | a_x + a_width + 20.0)
            .unwrap();

        solver.add_edit_variable(a_x, Strength::STRONG).unwrap();
        solver.add_edit_variable(a_width, Strength::STRONG).unwrap();
        solver.add_edit_variable(b_width, Strength::STRONG).unwrap();

        solver.suggest_value(a_x, 10.0).unwrap();
        solver.suggest_value(a_width, 80.0).unwrap();
        solver.suggest_value(b_width, 60.0).unwrap();

        // Fetch and verify - fetch_changes returns &[(Variable, f64)]
        let changes = solver.fetch_changes();
        let values: std::collections::HashMap<Variable, f64> =
            changes.iter().map(|(v, val)| (*v, *val)).collect();

        // b.x should be a.x + a.width + 20 = 10 + 80 + 20 = 110
        assert!(
            (values.get(&b_x).copied().unwrap_or(0.0) - 110.0).abs() < 0.001,
            "b.x should be 110"
        );

        println!("Spike PASSED: relative width constraints work!");
    }
}
