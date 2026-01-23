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
            // The col should contain: micro group, connection, meso group, connection, macro group
            // = 5 children
            assert_eq!(
                layout.children.len(),
                5,
                "Col layout should have 5 children"
            );
        }
        _ => panic!("Expected top-level layout"),
    }
}
