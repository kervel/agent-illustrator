//! Integration tests for the Agent Illustrator parser

use agent_illustrator::parse;

#[test]
fn test_simple_shapes() {
    let input = r#"
        rect server
        circle db [fill: blue]
        server -> db
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 3);
}

#[test]
fn test_layout_container() {
    let input = r#"
        row {
            rect a
            rect b
            rect c [fill: red]
        }
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 1);
}

#[test]
fn test_nested_groups() {
    let input = r#"
        group datacenter {
            col {
                group rack1 {
                    rect server1
                    rect server2
                }
                group rack2 {
                    rect server3
                }
            }
        }
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 1);
}

#[test]
fn test_connections_with_labels() {
    let input = r#"
        rect client
        rect server
        client -> server [label: "HTTP", style: dashed]
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 3);
}

#[test]
fn test_constraints() {
    let input = r#"
        rect server
        rect client
        place client right-of server
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 3);
}

#[test]
fn test_icon_shapes() {
    let input = r#"
        icon "server" myserver [fill: gray]
        icon "database" db1
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 2);
}

#[test]
fn test_all_connection_types() {
    let input = r#"
        rect a
        rect b
        a -> b
        a <- b
        a <-> b
        a -- b
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 6);
}

#[test]
fn test_all_layout_types() {
    let input = r#"
        row { rect a }
        col { rect b }
        grid { rect c }
        stack { rect d }
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 4);
}

#[test]
fn test_all_constraint_relations() {
    let input = r#"
        rect anchor
        rect subject
        place subject right-of anchor
        place subject left-of anchor
        place subject above anchor
        place subject below anchor
        place subject inside anchor
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 7);
}

#[test]
fn test_comments_ignored() {
    let input = r#"
        // This is a comment
        rect server
        /* This is a
           multi-line comment */
        rect client
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 2);
}

#[test]
fn test_complex_real_world_example() {
    let input = r#"
        // Server architecture diagram
        group frontend {
            row {
                icon "browser" user1
                icon "browser" user2
                icon "browser" user3
            }
        }

        group backend {
            col {
                icon "server" lb [label: "Load Balancer"]
                row {
                    icon "server" app1
                    icon "server" app2
                }
            }
        }

        group data {
            icon "database" db [fill: #336699]
        }

        // Connections
        user1 -> lb
        user2 -> lb
        user3 -> lb
        lb -> app1
        lb -> app2
        app1 -> db [label: "SQL"]
        app2 -> db [label: "SQL"]

        // Layout hints
        place backend below frontend
        place data below backend
    "#;

    let doc = parse(input).expect("Should parse");
    // 3 groups + 7 connections + 2 constraints = 12 statements
    assert_eq!(doc.statements.len(), 12);
}

#[test]
fn test_error_reporting() {
    let input = "rect [invalid";
    let result = parse(input);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(!errors.is_empty());
}

#[test]
fn test_empty_document() {
    let input = "";
    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 0);
}

#[test]
fn test_whitespace_only() {
    let input = "   \n\t\n   ";
    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 0);
}

#[test]
fn test_hex_colors() {
    let input = r#"
        rect a [fill: #fff]
        rect b [fill: #ff0000]
        rect c [stroke: #abc]
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 3);
}

#[test]
fn test_numeric_modifiers() {
    let input = r#"
        rect a [opacity: 0.5, stroke_width: 2]
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 1);
}

#[test]
fn test_connection_routing_direct() {
    // Feature: Direct (diagonal) routing for connections
    let input = r#"
        rect a
        rect b
        a -> b [routing: direct]
    "#;

    let doc = parse(input).expect("Should parse connection with direct routing");
    assert_eq!(doc.statements.len(), 3);

    // Verify the connection has the routing modifier
    match &doc.statements[2].node {
        agent_illustrator::parser::ast::Statement::Connection(conn) => {
            assert_eq!(conn.modifiers.len(), 1);
            assert!(matches!(
                conn.modifiers[0].node.key.node,
                agent_illustrator::parser::ast::StyleKey::Routing
            ));
            match &conn.modifiers[0].node.value.node {
                agent_illustrator::parser::ast::StyleValue::Keyword(k) => {
                    assert_eq!(k, "direct");
                }
                _ => panic!("Expected keyword value"),
            }
        }
        _ => panic!("Expected connection"),
    }
}

#[test]
fn test_connection_routing_orthogonal() {
    // Explicit orthogonal routing (same as default)
    let input = r#"
        rect a
        rect b
        a -> b [routing: orthogonal]
    "#;

    let doc = parse(input).expect("Should parse connection with orthogonal routing");
    assert_eq!(doc.statements.len(), 3);
}

#[test]
fn test_undirected_connection_with_routing() {
    // Undirected connection with direct routing
    let input = r#"
        rect a
        rect b
        a -- b [routing: direct]
    "#;

    let doc = parse(input).expect("Should parse undirected connection with routing");
    assert_eq!(doc.statements.len(), 3);
}

#[test]
fn test_railway_junction_with_direct_routing() {
    // Feature: Direct routing for railway junction diagrams
    // This example demonstrates diagonal connections for track switches
    let input = include_str!("../examples/railway-junction-direct.ail");

    let doc = parse(input).expect("Railway junction with direct routing should parse");

    // Count connections with direct routing
    fn count_direct_routing(
        stmts: &[agent_illustrator::parser::ast::Spanned<
            agent_illustrator::parser::ast::Statement,
        >],
    ) -> usize {
        let mut count = 0;
        for stmt in stmts {
            if let agent_illustrator::parser::ast::Statement::Connection(conn) = &stmt.node {
                for modifier in &conn.modifiers {
                    if matches!(
                        modifier.node.key.node,
                        agent_illustrator::parser::ast::StyleKey::Routing
                    ) {
                        if let agent_illustrator::parser::ast::StyleValue::Keyword(k) =
                            &modifier.node.value.node
                        {
                            if k == "direct" {
                                count += 1;
                            }
                        }
                    }
                }
            } else if let agent_illustrator::parser::ast::Statement::Layout(l) = &stmt.node {
                count += count_direct_routing(&l.children);
            } else if let agent_illustrator::parser::ast::Statement::Group(g) = &stmt.node {
                count += count_direct_routing(&g.children);
            }
        }
        count
    }

    let direct_count = count_direct_routing(&doc.statements);
    assert!(
        direct_count >= 5,
        "Expected at least 5 direct routing connections, found {}",
        direct_count
    );
}

#[test]
fn test_railway_topology_smoke_test() {
    // Feature 002: Railway Topology Smoke Test
    // This test verifies that the reference DSL document parses correctly
    let input = include_str!("../examples/railway-topology.ail");

    let doc = parse(input).expect("Railway topology document should parse");

    // Document should have:
    // - 1 col layout (diagram) containing everything
    assert_eq!(
        doc.statements.len(),
        1,
        "Should have 1 top-level statement (col layout)"
    );

    // Verify it's a layout
    match &doc.statements[0].node {
        agent_illustrator::parser::ast::Statement::Layout(layout) => {
            // The col should contain:
            // - micro group
            // - meso group
            // - macro group
            // - agg_label text
            // - simp_label text
            // - 2 connections
            // - 4 constrain statements (2 for labels, 2 for text)
            // = 11 children
            assert_eq!(
                layout.children.len(),
                11,
                "Col layout should have 11 children"
            );
        }
        _ => panic!("Expected top-level layout"),
    }
}

#[test]
fn test_text_shape_basic() {
    // Feature: Text shape primitive
    // Basic text element parsing
    let input = r#"text "Hello World" foo"#;

    let doc = parse(input).expect("Should parse text shape");
    assert_eq!(doc.statements.len(), 1);

    match &doc.statements[0].node {
        agent_illustrator::parser::ast::Statement::Shape(shape) => {
            match &shape.shape_type.node {
                agent_illustrator::parser::ast::ShapeType::Text { content } => {
                    assert_eq!(content, "Hello World");
                }
                _ => panic!("Expected text shape type"),
            }
            assert_eq!(shape.name.as_ref().unwrap().node.as_str(), "foo");
        }
        _ => panic!("Expected shape statement"),
    }
}

#[test]
fn test_text_shape_with_styles() {
    // Text element with fill color and font size
    let input = r#"text "Styled Text" styled_text [fill: red, font_size: 24]"#;

    let doc = parse(input).expect("Should parse text shape with styles");
    assert_eq!(doc.statements.len(), 1);

    match &doc.statements[0].node {
        agent_illustrator::parser::ast::Statement::Shape(shape) => {
            assert_eq!(shape.modifiers.len(), 2);
            // Verify fill modifier
            let has_fill = shape.modifiers.iter().any(|m| {
                matches!(
                    m.node.key.node,
                    agent_illustrator::parser::ast::StyleKey::Fill
                )
            });
            assert!(has_fill, "Should have fill modifier");
            // Verify font_size modifier
            let has_font_size = shape.modifiers.iter().any(|m| {
                matches!(
                    m.node.key.node,
                    agent_illustrator::parser::ast::StyleKey::FontSize
                )
            });
            assert!(has_font_size, "Should have font_size modifier");
        }
        _ => panic!("Expected shape statement"),
    }
}

#[test]
fn test_text_shapes_with_connection() {
    // Text elements can be connected like other shapes
    let input = r#"
        text "Label A" a
        text "Label B" b
        a -> b
    "#;

    let doc = parse(input).expect("Should parse text shapes with connection");
    assert_eq!(doc.statements.len(), 3);

    // Verify first text element
    match &doc.statements[0].node {
        agent_illustrator::parser::ast::Statement::Shape(shape) => {
            assert!(matches!(
                shape.shape_type.node,
                agent_illustrator::parser::ast::ShapeType::Text { .. }
            ));
        }
        _ => panic!("Expected shape statement"),
    }

    // Verify connection
    match &doc.statements[2].node {
        agent_illustrator::parser::ast::Statement::Connection(conn) => {
            assert_eq!(conn.from.node.as_str(), "a");
            assert_eq!(conn.to.node.as_str(), "b");
        }
        _ => panic!("Expected connection statement"),
    }
}

#[test]
fn test_text_shape_in_layout() {
    // Text elements can be used inside layouts
    let input = r#"
        row {
            text "First" first
            text "Second" second
            text "Third" third
        }
    "#;

    let doc = parse(input).expect("Should parse text shapes in layout");
    assert_eq!(doc.statements.len(), 1);

    match &doc.statements[0].node {
        agent_illustrator::parser::ast::Statement::Layout(layout) => {
            assert_eq!(layout.children.len(), 3);
            // Verify all children are text shapes
            for child in &layout.children {
                match &child.node {
                    agent_illustrator::parser::ast::Statement::Shape(shape) => {
                        assert!(matches!(
                            shape.shape_type.node,
                            agent_illustrator::parser::ast::ShapeType::Text { .. }
                        ));
                    }
                    _ => panic!("Expected shape statement"),
                }
            }
        }
        _ => panic!("Expected layout statement"),
    }
}

// ============================================================================
// Label Statement Tests
// ============================================================================

#[test]
fn test_label_block_form_in_group() {
    // Feature: Label statement with block form inside a group
    let input = r#"
        group mygroup {
            label { text "Group Label" }
            rect a
            rect b
        }
    "#;

    let doc = parse(input).expect("Should parse label block form");
    assert_eq!(doc.statements.len(), 1);

    match &doc.statements[0].node {
        agent_illustrator::parser::ast::Statement::Group(g) => {
            assert_eq!(g.children.len(), 3);
            // First child should be a Label statement
            match &g.children[0].node {
                agent_illustrator::parser::ast::Statement::Label(inner) => match inner.as_ref() {
                    agent_illustrator::parser::ast::Statement::Shape(s) => {
                        assert!(matches!(
                            s.shape_type.node,
                            agent_illustrator::parser::ast::ShapeType::Text { .. }
                        ));
                    }
                    _ => panic!("Expected shape inside label"),
                },
                _ => panic!("Expected label statement"),
            }
        }
        _ => panic!("Expected group statement"),
    }
}

#[test]
fn test_label_inline_form_in_group() {
    // Feature: Label statement with inline form inside a group
    let input = r#"
        group mygroup {
            label: text "Inline Label" lbl [font_size: 18]
            rect a
        }
    "#;

    let doc = parse(input).expect("Should parse label inline form");
    assert_eq!(doc.statements.len(), 1);

    match &doc.statements[0].node {
        agent_illustrator::parser::ast::Statement::Group(g) => {
            assert_eq!(g.children.len(), 2);
            // First child should be a Label statement
            match &g.children[0].node {
                agent_illustrator::parser::ast::Statement::Label(inner) => match inner.as_ref() {
                    agent_illustrator::parser::ast::Statement::Shape(s) => {
                        match &s.shape_type.node {
                            agent_illustrator::parser::ast::ShapeType::Text { content } => {
                                assert_eq!(content, "Inline Label");
                            }
                            _ => panic!("Expected text shape"),
                        }
                        assert_eq!(s.modifiers.len(), 1);
                    }
                    _ => panic!("Expected shape inside label"),
                },
                _ => panic!("Expected label statement"),
            }
        }
        _ => panic!("Expected group statement"),
    }
}

#[test]
fn test_label_with_any_shape() {
    // Feature: Any shape can be used as a label
    let input = r#"
        group mygroup {
            label { rect marker [fill: red, width: 10, height: 10] }
            row {
                circle c1
                circle c2
            }
        }
    "#;

    let doc = parse(input).expect("Should parse label with rectangle shape");
    assert_eq!(doc.statements.len(), 1);

    match &doc.statements[0].node {
        agent_illustrator::parser::ast::Statement::Group(g) => {
            assert_eq!(g.children.len(), 2);
            // First child should be a Label with a rectangle inside
            match &g.children[0].node {
                agent_illustrator::parser::ast::Statement::Label(inner) => match inner.as_ref() {
                    agent_illustrator::parser::ast::Statement::Shape(s) => {
                        assert!(matches!(
                            s.shape_type.node,
                            agent_illustrator::parser::ast::ShapeType::Rectangle
                        ));
                        assert_eq!(s.name.as_ref().unwrap().node.as_str(), "marker");
                    }
                    _ => panic!("Expected shape inside label"),
                },
                _ => panic!("Expected label statement"),
            }
        }
        _ => panic!("Expected group statement"),
    }
}

#[test]
fn test_label_modifier_backward_compatible() {
    // Feature: Old [label: "text"] modifier should still work
    let input = r#"
        group mygroup [label: "Old Style"] {
            rect a
        }
        rect standalone [label: "Standalone"]
    "#;

    let doc = parse(input).expect("Should parse label modifier");
    assert_eq!(doc.statements.len(), 2);

    // Check the group's modifier
    match &doc.statements[0].node {
        agent_illustrator::parser::ast::Statement::Group(g) => {
            assert_eq!(g.modifiers.len(), 1);
            assert!(matches!(
                g.modifiers[0].node.key.node,
                agent_illustrator::parser::ast::StyleKey::Label
            ));
        }
        _ => panic!("Expected group statement"),
    }

    // Check the standalone rect's modifier
    match &doc.statements[1].node {
        agent_illustrator::parser::ast::Statement::Shape(s) => {
            assert_eq!(s.modifiers.len(), 1);
            assert!(matches!(
                s.modifiers[0].node.key.node,
                agent_illustrator::parser::ast::StyleKey::Label
            ));
        }
        _ => panic!("Expected shape statement"),
    }
}

#[test]
fn test_label_in_layout_container() {
    // Feature: Label statement inside layout containers
    let input = r#"
        row myrow {
            label { text "Row Title" }
            rect a
            rect b
        }
    "#;

    let doc = parse(input).expect("Should parse label in layout container");
    assert_eq!(doc.statements.len(), 1);

    match &doc.statements[0].node {
        agent_illustrator::parser::ast::Statement::Layout(l) => {
            assert_eq!(l.children.len(), 3);
            // First child should be a Label statement
            match &l.children[0].node {
                agent_illustrator::parser::ast::Statement::Label(inner) => match inner.as_ref() {
                    agent_illustrator::parser::ast::Statement::Shape(s) => {
                        assert!(matches!(
                            s.shape_type.node,
                            agent_illustrator::parser::ast::ShapeType::Text { .. }
                        ));
                    }
                    _ => panic!("Expected shape inside label"),
                },
                _ => panic!("Expected label statement"),
            }
        }
        _ => panic!("Expected layout statement"),
    }
}

// ============================================================================
// Connection Label Position Tests
// ============================================================================

#[test]
fn test_connection_label_position_right() {
    // Feature: Connection label positioned to the right
    let input = r#"
        rect a
        rect b
        a -> b [label: "Right Label", label_position: right]
    "#;

    let doc = parse(input).expect("Should parse connection with label_position: right");
    assert_eq!(doc.statements.len(), 3);

    match &doc.statements[2].node {
        agent_illustrator::parser::ast::Statement::Connection(conn) => {
            assert_eq!(conn.modifiers.len(), 2);
            // Verify label_position modifier exists
            let has_label_position = conn.modifiers.iter().any(|m| {
                matches!(
                    m.node.key.node,
                    agent_illustrator::parser::ast::StyleKey::LabelPosition
                )
            });
            assert!(has_label_position, "Should have label_position modifier");
        }
        _ => panic!("Expected connection statement"),
    }
}

#[test]
fn test_connection_label_position_left() {
    // Feature: Connection label positioned to the left
    let input = r#"
        rect a
        rect b
        a -> b [label: "Left Label", label_position: left]
    "#;

    let doc = parse(input).expect("Should parse connection with label_position: left");
    assert_eq!(doc.statements.len(), 3);

    match &doc.statements[2].node {
        agent_illustrator::parser::ast::Statement::Connection(conn) => {
            let label_position_modifier = conn.modifiers.iter().find(|m| {
                matches!(
                    m.node.key.node,
                    agent_illustrator::parser::ast::StyleKey::LabelPosition
                )
            });
            assert!(
                label_position_modifier.is_some(),
                "Should have label_position modifier"
            );
            match &label_position_modifier.unwrap().node.value.node {
                agent_illustrator::parser::ast::StyleValue::Keyword(k) => {
                    assert_eq!(k, "left");
                }
                _ => panic!("Expected keyword value for label_position"),
            }
        }
        _ => panic!("Expected connection statement"),
    }
}

#[test]
fn test_connection_label_position_center() {
    // Feature: Connection label positioned at center
    let input = r#"
        rect a
        rect b
        a -> b [label: "Center Label", label_position: center]
    "#;

    let doc = parse(input).expect("Should parse connection with label_position: center");
    assert_eq!(doc.statements.len(), 3);

    match &doc.statements[2].node {
        agent_illustrator::parser::ast::Statement::Connection(conn) => {
            let label_position_modifier = conn.modifiers.iter().find(|m| {
                matches!(
                    m.node.key.node,
                    agent_illustrator::parser::ast::StyleKey::LabelPosition
                )
            });
            assert!(
                label_position_modifier.is_some(),
                "Should have label_position modifier"
            );
            match &label_position_modifier.unwrap().node.value.node {
                agent_illustrator::parser::ast::StyleValue::Keyword(k) => {
                    assert_eq!(k, "center");
                }
                _ => panic!("Expected keyword value for label_position"),
            }
        }
        _ => panic!("Expected connection statement"),
    }
}

#[test]
fn test_connection_label_without_position() {
    // Feature: Connection label without explicit position (auto-detect)
    let input = r#"
        rect a
        rect b
        a -> b [label: "Auto Position"]
    "#;

    let doc = parse(input).expect("Should parse connection without label_position");
    assert_eq!(doc.statements.len(), 3);

    match &doc.statements[2].node {
        agent_illustrator::parser::ast::Statement::Connection(conn) => {
            // Should only have label modifier, no label_position
            assert_eq!(conn.modifiers.len(), 1);
            assert!(matches!(
                conn.modifiers[0].node.key.node,
                agent_illustrator::parser::ast::StyleKey::Label
            ));
        }
        _ => panic!("Expected connection statement"),
    }
}

// ============================================================================
// Symbolic Color Tests
// ============================================================================

#[test]
fn test_symbolic_colors() {
    use agent_illustrator::render;

    // Test parsing and rendering of symbolic colors
    let input = r#"
        rect server [fill: foreground-1, stroke: accent-1]
        rect client [fill: background-2, stroke: text-dark]
    "#;

    let svg = render(input).expect("Should render symbolic colors");

    // Check that CSS custom properties are used in the output
    assert!(
        svg.contains("var(--foreground-1)"),
        "Should use CSS variable for foreground-1"
    );
    assert!(
        svg.contains("var(--accent-1)"),
        "Should use CSS variable for accent-1"
    );
    assert!(
        svg.contains("var(--background-2)"),
        "Should use CSS variable for background-2"
    );
    assert!(
        svg.contains("var(--text-dark)"),
        "Should use CSS variable for text-dark"
    );

    // Check that the style block contains the CSS custom property definitions
    assert!(svg.contains("<style>"), "Should have style block");
    assert!(
        svg.contains("--foreground-1:"),
        "Should define foreground-1 in style"
    );
}

#[test]
fn test_symbolic_colors_with_stylesheet() {
    use agent_illustrator::{render_with_config, RenderConfig, Stylesheet};

    let input = r#"rect box [fill: foreground-1]"#;

    // Create a custom stylesheet
    let stylesheet_toml = r##"
[colors]
foreground-1 = "#ff0000"
"##;
    let stylesheet = Stylesheet::from_str(stylesheet_toml).expect("Should parse stylesheet");

    let config = RenderConfig::new().with_stylesheet(stylesheet);
    let svg = render_with_config(input, config).expect("Should render with custom stylesheet");

    // Check that the custom color is in the style block
    assert!(
        svg.contains("--foreground-1: #ff0000"),
        "Should use custom color from stylesheet"
    );
    // The fill attribute should use the CSS variable
    assert!(
        svg.contains("var(--foreground-1)"),
        "Should reference CSS variable"
    );
}

#[test]
fn test_mixed_colors() {
    use agent_illustrator::render;

    // Test mixing symbolic, hex, and named colors
    let input = r#"
        rect a [fill: foreground-1]
        rect b [fill: #ff0000]
        rect c [fill: blue]
    "#;

    let svg = render(input).expect("Should render mixed colors");

    // Symbolic color should use CSS variable
    assert!(svg.contains("var(--foreground-1)"));
    // Hex color should pass through directly
    assert!(svg.contains(r##"fill="#ff0000""##));
    // Named color should pass through directly
    assert!(svg.contains(r#"fill="blue""#));
}

#[test]
fn test_role_label_in_group() {
    // Feature: Using [role: label] modifier instead of label { } syntax
    let input = r#"
        group mygroup {
            text "Group Label" [role: label]
            rect a
            rect b
        }
    "#;

    let doc = parse(input).expect("Should parse role label");
    assert_eq!(doc.statements.len(), 1);

    match &doc.statements[0].node {
        agent_illustrator::parser::ast::Statement::Group(g) => {
            // Should have 3 children: text with role:label, rect a, rect b
            assert_eq!(g.children.len(), 3);

            // First child should be a Shape with role: label modifier
            match &g.children[0].node {
                agent_illustrator::parser::ast::Statement::Shape(s) => {
                    // Check it has the role: label modifier
                    let has_role_label = s.modifiers.iter().any(|m| {
                        matches!(
                            &m.node.key.node,
                            agent_illustrator::parser::ast::StyleKey::Role
                        ) && matches!(
                            &m.node.value.node,
                            agent_illustrator::parser::ast::StyleValue::Keyword(k) if k == "label"
                        )
                    });
                    assert!(has_role_label, "Should have role: label modifier");
                }
                _ => panic!("Expected shape statement"),
            }
        }
        _ => panic!("Expected group statement"),
    }
}

#[test]
fn test_role_label_rendering_in_group() {
    use agent_illustrator::render;

    // Feature: Elements with [role: label] should be rendered as labels
    let input = r#"
        group mygroup {
            text "My Label" [role: label]
            rect a
        }
    "#;

    let svg = render(input).expect("Should render role label");

    // The text "My Label" should be in the output
    assert!(svg.contains("My Label"), "Should contain the label text");
}

#[test]
fn test_role_label_in_layout_container() {
    use agent_illustrator::render;

    // Feature: Elements with [role: label] in layout containers
    let input = r#"
        row myrow {
            text "Row Label" [role: label]
            rect a
            rect b
        }
    "#;

    let svg = render(input).expect("Should render role label in layout");

    // The text "Row Label" should be in the output
    assert!(svg.contains("Row Label"), "Should contain the label text");
}

#[test]
fn test_connection_label_identifier_reference() {
    use agent_illustrator::render;

    // Feature: Connection labels can reference text shapes by identifier
    let input = r#"
        text "HTTP Request" http_label
        rect server
        rect client
        server -> client [label: http_label]
    "#;

    let svg = render(input).expect("Should render connection with label reference");

    // The text "HTTP Request" should appear on the connection
    assert!(
        svg.contains("HTTP Request"),
        "Connection should use referenced text shape's content as label"
    );
}

#[test]
fn test_connection_label_string_still_works() {
    use agent_illustrator::render;

    // Feature: String labels should still work (backward compatibility)
    let input = r#"
        rect server
        rect client
        server -> client [label: "Query"]
    "#;

    let svg = render(input).expect("Should render connection with string label");

    // The text "Query" should appear on the connection
    assert!(svg.contains("Query"), "String labels should still work");
}

// ============================================================================
// Constraint Solver Integration Tests (Feature 005)
// ============================================================================

#[test]
fn test_constrain_statement_parses() {
    // Test that constrain statements parse correctly
    let input = r#"
        rect a
        rect b
        constrain a.left = b.left
    "#;

    let doc = parse(input).expect("Should parse constrain statement");
    assert_eq!(doc.statements.len(), 3);

    // Verify the last statement is a Constrain
    use agent_illustrator::parser::ast::Statement;
    assert!(matches!(doc.statements[2].node, Statement::Constrain(_)));
}

#[test]
fn test_constrain_with_offset_parses() {
    let input = r#"
        rect a
        rect b
        constrain a.x = b.x + 100
    "#;

    let doc = parse(input).expect("Should parse constrain with offset");
    assert_eq!(doc.statements.len(), 3);
}

#[test]
fn test_constrain_inequality_parses() {
    let input = r#"
        rect a [width: 50]
        constrain a.width >= 100
    "#;

    let doc = parse(input).expect("Should parse constrain inequality");
    assert_eq!(doc.statements.len(), 2);
}

#[test]
fn test_constrain_midpoint_parses() {
    let input = r#"
        rect a
        rect b
        rect c
        constrain c.x = midpoint(a, b)
    "#;

    let doc = parse(input).expect("Should parse constrain midpoint");
    assert_eq!(doc.statements.len(), 4);
}

#[test]
fn test_constrain_contains_parses() {
    let input = r#"
        rect container [width: 200, height: 200]
        rect a [width: 50]
        rect b [width: 50]
        constrain container contains a, b [padding: 10]
    "#;

    let doc = parse(input).expect("Should parse constrain contains");
    assert_eq!(doc.statements.len(), 4);
}

#[test]
fn test_constrain_renders_without_error() {
    use agent_illustrator::render;

    // Test that a document with constrain statements renders without error
    let input = r#"
        rect a [width: 50, height: 50]
        rect b [width: 50, height: 50]
        constrain a.left = b.left
    "#;

    let svg = render(input).expect("Should render with constrain statement");
    assert!(svg.contains("<svg"), "Output should be valid SVG");
    assert!(svg.contains("<rect"), "Should contain rect elements");
}

#[test]
fn test_constrain_with_nested_path_parses() {
    let input = r#"
        group g1 {
            rect a
        }
        group g2 {
            rect b
        }
        constrain g1.a.left = g2.b.left
    "#;

    let doc = parse(input).expect("Should parse constrain with nested paths");
    assert_eq!(doc.statements.len(), 3);
}

#[test]
fn test_constraint_collector_collects_constraints() {
    use agent_illustrator::layout::{ConstraintCollector, LayoutConfig};

    let input = r#"
        rect a [width: 100]
        rect b
        constrain a.left = b.left
        constrain b.width >= 50
    "#;

    let doc = parse(input).unwrap();
    let mut collector = ConstraintCollector::new(LayoutConfig::default());
    collector.collect(&doc);

    // Should have:
    // - 1 intrinsic constraint (a.width = 100)
    // - 1 equality constraint (a.left = b.left)
    // - 1 inequality constraint (b.width >= 50)
    assert!(
        collector.constraints.len() >= 3,
        "Should collect at least 3 constraints, got {}",
        collector.constraints.len()
    );
}

#[test]
fn test_existing_row_layout_still_works() {
    use agent_illustrator::render;

    // Verify that existing row layout functionality is preserved
    let input = r#"
        row {
            rect a [width: 50]
            rect b [width: 50]
            rect c [width: 50]
        }
    "#;

    let svg = render(input).expect("Row layout should still work");
    assert!(svg.contains("<svg"), "Output should be valid SVG");
    assert!(svg.contains("<rect"), "Should contain rect elements");
}

// align keyword has been removed - use constrain instead

// ==================== Template Tests ====================

#[test]
fn test_template_inline_basic() {
    // Test that inline templates can be defined and instantiated
    use agent_illustrator::render;

    let input = r#"
        template "box" {
            rect shape [fill: blue]
        }
        box mybox
    "#;

    let svg = render(input).expect("Template should render");
    assert!(svg.contains("<svg"), "Output should be valid SVG");
    assert!(svg.contains("<rect"), "Should contain rect element from template");
    assert!(svg.contains(r#"id="mybox""#), "Should have instance name as ID");
}

#[test]
fn test_template_with_parameters() {
    // Test that template parameters are substituted correctly
    use agent_illustrator::render;

    let input = r#"
        template "colored_box" (fill: blue) {
            rect shape [fill: fill]
        }
        colored_box red_box [fill: red]
    "#;

    let svg = render(input).expect("Parameterized template should render");
    assert!(svg.contains("<svg"), "Output should be valid SVG");
    // The fill should be red, overriding the default blue
    assert!(svg.contains(r#"fill="red""#), "Should have overridden fill color");
}

#[test]
fn test_template_default_parameters() {
    // Test that template default parameters work when not overridden
    use agent_illustrator::render;

    let input = r#"
        template "colored_box" (fill: blue) {
            rect shape [fill: fill]
        }
        colored_box default_box
    "#;

    let svg = render(input).expect("Template with defaults should render");
    assert!(svg.contains("<svg"), "Output should be valid SVG");
    // The fill should be blue (the default)
    // Note: "blue" is parsed as a keyword, not a color
    assert!(svg.contains("<rect"), "Should contain rect element");
}

#[test]
fn test_template_in_layout() {
    // Test that template instances work inside layouts
    use agent_illustrator::render;

    let input = r#"
        template "server" {
            rect box [fill: gray]
        }
        row {
            server server1
            server server2
            server server3
        }
    "#;

    let svg = render(input).expect("Templates in layout should render");
    assert!(svg.contains("<svg"), "Output should be valid SVG");
    // Should have multiple rect elements
    let rect_count = svg.matches("<rect").count();
    assert!(rect_count >= 3, "Should have at least 3 rect elements from template instances");
}

#[test]
fn test_template_with_multiple_elements() {
    // Test templates that contain multiple elements
    use agent_illustrator::render;

    let input = r#"
        template "labeled_server" {
            rect box [fill: blue, size: 50]
            text "Server" title
        }
        labeled_server myserver
    "#;

    let svg = render(input).expect("Multi-element template should render");
    assert!(svg.contains("<svg"), "Output should be valid SVG");
    assert!(svg.contains("<rect"), "Should contain rect from template");
    assert!(svg.contains("<text"), "Should contain text from template");
}

#[test]
fn test_template_preserves_connections() {
    // Test that connections work alongside templates
    use agent_illustrator::render;

    let input = r#"
        template "node" {
            rect shape [fill: lightblue]
        }
        node a
        node b
        a -> b
    "#;

    let svg = render(input).expect("Templates with connections should render");
    assert!(svg.contains("<svg"), "Output should be valid SVG");
    // Should have connection (path element for arrow)
    assert!(svg.contains("<path"), "Should contain path for connection");
}

#[test]
fn test_template_file_svg_declaration() {
    // Test that SVG file template declarations parse correctly (but don't render without the file)
    use agent_illustrator::parse;

    let input = r#"
        template "icon" from "assets/icon.svg"
    "#;

    let doc = parse(input).expect("SVG template declaration should parse");
    assert_eq!(doc.statements.len(), 1);

    match &doc.statements[0].node {
        agent_illustrator::parser::ast::Statement::TemplateDecl(decl) => {
            assert_eq!(decl.name.node.as_str(), "icon");
            assert_eq!(decl.source_type, agent_illustrator::parser::ast::TemplateSourceType::Svg);
        }
        _ => panic!("Expected TemplateDecl"),
    }
}

#[test]
fn test_export_declaration() {
    // Test that export declarations parse within templates
    use agent_illustrator::parse;

    let input = r#"
        template "connector" {
            circle port_left
            circle port_right
            export port_left, port_right
        }
    "#;

    let doc = parse(input).expect("Template with exports should parse");
    assert_eq!(doc.statements.len(), 1);
}

// =============================================================================
// Phase 5: SVG Import Tests (T029)
// =============================================================================

#[test]
fn test_svg_template_import_end_to_end() {
    use agent_illustrator::{render_with_config, RenderConfig};
    use std::io::Write;

    // Create a temporary directory and SVG file
    let temp_dir = std::env::temp_dir().join("agent_illustrator_test_svg");
    std::fs::create_dir_all(&temp_dir).expect("Should create temp dir");

    let svg_content = r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
        <circle cx="50" cy="50" r="40" fill="blue"/>
    </svg>"#;

    let svg_path = temp_dir.join("test_icon.svg");
    let mut file = std::fs::File::create(&svg_path).expect("Should create SVG file");
    file.write_all(svg_content.as_bytes()).expect("Should write SVG content");

    // Use "myicon" as template name (not "icon" which is a keyword)
    let input = r#"
        template "myicon" from "test_icon.svg"
        myicon inst1
    "#;

    let config = RenderConfig::new()
        .with_template_base_path(temp_dir.clone());

    let result = render_with_config(input, config);

    // Cleanup
    let _ = std::fs::remove_file(&svg_path);
    let _ = std::fs::remove_dir(&temp_dir);

    let svg = result.expect("SVG template import should render");
    assert!(svg.contains("<svg"), "Output should be valid SVG");
    // The embedded content should be present (circle element)
    assert!(svg.contains("circle"), "Should contain the embedded SVG circle");
    // Should have the embed group
    assert!(svg.contains("ai-svg-embed"), "Should have svg-embed class");
}

#[test]
fn test_svg_template_multiple_instances() {
    use agent_illustrator::{render_with_config, RenderConfig};
    use std::io::Write;

    let temp_dir = std::env::temp_dir().join("agent_illustrator_test_svg_multi");
    std::fs::create_dir_all(&temp_dir).expect("Should create temp dir");

    let svg_content = r#"<svg viewBox="0 0 50 50" xmlns="http://www.w3.org/2000/svg">
        <rect x="5" y="5" width="40" height="40" fill="red"/>
    </svg>"#;

    let svg_path = temp_dir.join("box.svg");
    let mut file = std::fs::File::create(&svg_path).expect("Should create SVG file");
    file.write_all(svg_content.as_bytes()).expect("Should write SVG content");

    let input = r#"
        template "box" from "box.svg"
        row {
            box b1
            box b2
            box b3
        }
    "#;

    let config = RenderConfig::new()
        .with_template_base_path(temp_dir.clone());

    let result = render_with_config(input, config);

    // Cleanup
    let _ = std::fs::remove_file(&svg_path);
    let _ = std::fs::remove_dir(&temp_dir);

    let svg = result.expect("Multiple SVG instances should render");
    // Count occurrences of rect elements (should be 3)
    let rect_count = svg.matches("<rect").count();
    assert!(rect_count >= 3, "Should have at least 3 rect elements (found {})", rect_count);
}

#[test]
fn test_svg_template_with_size_modifiers() {
    use agent_illustrator::{render_with_config, RenderConfig};
    use std::io::Write;

    let temp_dir = std::env::temp_dir().join("agent_illustrator_test_svg_size");
    std::fs::create_dir_all(&temp_dir).expect("Should create temp dir");

    let svg_content = r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
        <circle cx="50" cy="50" r="45"/>
    </svg>"#;

    let svg_path = temp_dir.join("circleicon.svg");
    let mut file = std::fs::File::create(&svg_path).expect("Should create SVG file");
    file.write_all(svg_content.as_bytes()).expect("Should write SVG content");

    // Use "circleicon" as template name (not "circle" which is a keyword)
    let input = r#"
        template "circleicon" from "circleicon.svg"
        circleicon c1 [width: 200, height: 200]
    "#;

    let config = RenderConfig::new()
        .with_template_base_path(temp_dir.clone());

    let result = render_with_config(input, config);

    // Cleanup
    let _ = std::fs::remove_file(&svg_path);
    let _ = std::fs::remove_dir(&temp_dir);

    let svg = result.expect("SVG template with size modifiers should render");
    assert!(svg.contains("scale"), "Should contain scale transform for sizing");
}

#[test]
fn test_svg_template_aspect_ratio_preserved() {
    use agent_illustrator::{render_with_config, RenderConfig};
    use std::io::Write;

    let temp_dir = std::env::temp_dir().join("agent_illustrator_test_svg_aspect");
    std::fs::create_dir_all(&temp_dir).expect("Should create temp dir");

    // SVG with 2:1 aspect ratio
    let svg_content = r#"<svg viewBox="0 0 200 100" xmlns="http://www.w3.org/2000/svg">
        <rect x="0" y="0" width="200" height="100" fill="green"/>
    </svg>"#;

    let svg_path = temp_dir.join("wide.svg");
    let mut file = std::fs::File::create(&svg_path).expect("Should create SVG file");
    file.write_all(svg_content.as_bytes()).expect("Should write SVG content");

    let input = r#"
        template "wide" from "wide.svg"
        wide w1
    "#;

    let config = RenderConfig::new()
        .with_template_base_path(temp_dir.clone());

    let result = render_with_config(input, config);

    // Cleanup
    let _ = std::fs::remove_file(&svg_path);
    let _ = std::fs::remove_dir(&temp_dir);

    let svg = result.expect("SVG template should preserve aspect ratio");
    assert!(svg.contains("<svg"), "Output should be valid SVG");
    // The template should render with the intrinsic dimensions
    assert!(svg.contains("rect"), "Should contain the embedded rect");
}

#[test]
fn test_svg_template_file_not_found_error() {
    use agent_illustrator::{render_with_config, RenderConfig};

    let temp_dir = std::env::temp_dir().join("agent_illustrator_test_svg_notfound");
    std::fs::create_dir_all(&temp_dir).expect("Should create temp dir");

    let input = r#"
        template "missing" from "nonexistent.svg"
        missing m1
    "#;

    let config = RenderConfig::new()
        .with_template_base_path(temp_dir.clone());

    let result = render_with_config(input, config);

    // Cleanup
    let _ = std::fs::remove_dir(&temp_dir);

    assert!(result.is_err(), "Missing SVG file should cause an error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found") || err_msg.contains("read") || err_msg.contains("nonexistent"),
        "Error should indicate file issue: {}",
        err_msg
    );
}

// =============================================================================
// Phase 6: AIL Import & Exports Tests (T031-T034)
// =============================================================================

#[test]
fn test_ail_template_import_basic() {
    use agent_illustrator::{render_with_config, RenderConfig};
    use std::io::Write;

    // Create a temporary directory and AIL file
    let temp_dir = std::env::temp_dir().join("agent_illustrator_test_ail_basic");
    std::fs::create_dir_all(&temp_dir).expect("Should create temp dir");

    // Use simpler AIL content - just shapes without text
    let ail_content = r#"
        rect box [fill: blue]
        circle port [size: 20]
    "#;

    let ail_path = temp_dir.join("server.ail");
    let mut file = std::fs::File::create(&ail_path).expect("Should create AIL file");
    file.write_all(ail_content.as_bytes()).expect("Should write AIL content");

    let input = r#"
        template "server" from "server.ail"
        server s1
        server s2
    "#;

    let config = RenderConfig::new()
        .with_template_base_path(temp_dir.clone());

    let result = render_with_config(input, config);

    // Cleanup
    let _ = std::fs::remove_file(&ail_path);
    let _ = std::fs::remove_dir(&temp_dir);

    let svg = result.expect("AIL template import should render");
    assert!(svg.contains("<svg"), "Output should be valid SVG");
    // Should have elements from both instances
    assert!(svg.contains("rect"), "Should contain rect elements from template");
}

#[test]
fn test_ail_template_with_exports() {
    use agent_illustrator::{render_with_config, RenderConfig};
    use std::io::Write;

    let temp_dir = std::env::temp_dir().join("agent_illustrator_test_ail_exports");
    std::fs::create_dir_all(&temp_dir).expect("Should create temp dir");

    let ail_content = r#"
        rect body [fill: gray]
        circle port_left [size: 10]
        circle port_right [size: 10]
        export port_left, port_right
    "#;

    let ail_path = temp_dir.join("connector.ail");
    let mut file = std::fs::File::create(&ail_path).expect("Should create AIL file");
    file.write_all(ail_content.as_bytes()).expect("Should write AIL content");

    let input = r#"
        template "connector" from "connector.ail"
        connector c1
    "#;

    let config = RenderConfig::new()
        .with_template_base_path(temp_dir.clone());

    let result = render_with_config(input, config);

    // Cleanup
    let _ = std::fs::remove_file(&ail_path);
    let _ = std::fs::remove_dir(&temp_dir);

    let svg = result.expect("AIL template with exports should render");
    assert!(svg.contains("<svg"), "Output should be valid SVG");
    assert!(svg.contains("rect"), "Should contain rect element");
    assert!(svg.contains("ellipse") || svg.contains("circle"), "Should contain circle elements");
}

#[test]
fn test_ail_template_nested_import() {
    use agent_illustrator::{render_with_config, RenderConfig};
    use std::io::Write;

    let temp_dir = std::env::temp_dir().join("agent_illustrator_test_ail_nested");
    std::fs::create_dir_all(&temp_dir).expect("Should create temp dir");

    // First AIL file - a simple component
    let component_ail = r#"
        rect box [fill: red]
    "#;

    // Second AIL file - uses the first component
    let container_ail = r#"
        template "component" from "component.ail"
        row {
            component inner1
            component inner2
        }
    "#;

    let component_path = temp_dir.join("component.ail");
    let mut file = std::fs::File::create(&component_path).expect("Should create component AIL");
    file.write_all(component_ail.as_bytes()).expect("Should write component content");

    let container_path = temp_dir.join("container.ail");
    let mut file = std::fs::File::create(&container_path).expect("Should create container AIL");
    file.write_all(container_ail.as_bytes()).expect("Should write container content");

    let input = r#"
        template "container" from "container.ail"
        container c1
    "#;

    let config = RenderConfig::new()
        .with_template_base_path(temp_dir.clone());

    let result = render_with_config(input, config);

    // Cleanup
    let _ = std::fs::remove_file(&component_path);
    let _ = std::fs::remove_file(&container_path);
    let _ = std::fs::remove_dir(&temp_dir);

    let svg = result.expect("Nested AIL template import should render");
    assert!(svg.contains("<svg"), "Output should be valid SVG");
    // Should have two rect elements from the nested components
    let rect_count = svg.matches("<rect").count();
    assert!(rect_count >= 2, "Should have at least 2 rects from nested components (found {})", rect_count);
}

#[test]
fn test_ail_template_circular_dependency_error() {
    use agent_illustrator::{render_with_config, RenderConfig};

    let temp_dir = std::env::temp_dir().join("agent_illustrator_test_ail_circular");
    std::fs::create_dir_all(&temp_dir).expect("Should create temp dir");

    // Create circular dependency via inline template self-reference
    // Template "recursive" uses itself - this should be caught during resolution
    let input = r#"
        template "recursive" {
            rect box
            recursive nested
        }
        recursive inst
    "#;

    let config = RenderConfig::new()
        .with_template_base_path(temp_dir.clone());

    let result = render_with_config(input, config);

    // Cleanup
    let _ = std::fs::remove_dir(&temp_dir);

    assert!(result.is_err(), "Circular dependency should cause an error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.to_lowercase().contains("circular") || err_msg.contains("Circular"),
        "Error should mention circular dependency: {}",
        err_msg
    );
}

#[test]
fn test_ail_template_file_not_found_error() {
    use agent_illustrator::{render_with_config, RenderConfig};

    let temp_dir = std::env::temp_dir().join("agent_illustrator_test_ail_notfound");
    std::fs::create_dir_all(&temp_dir).expect("Should create temp dir");

    let input = r#"
        template "missing" from "nonexistent.ail"
        missing m1
    "#;

    let config = RenderConfig::new()
        .with_template_base_path(temp_dir.clone());

    let result = render_with_config(input, config);

    // Cleanup
    let _ = std::fs::remove_dir(&temp_dir);

    assert!(result.is_err(), "Missing AIL file should cause an error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found") || err_msg.contains("read") || err_msg.contains("nonexistent"),
        "Error should indicate file issue: {}",
        err_msg
    );
}
