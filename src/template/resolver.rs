//! Template resolution - expands template instances into concrete elements

use std::collections::{HashMap, HashSet};

use crate::parser::ast::{
    ConstrainDecl, ConstraintExpr, Document, ElementPath, Identifier, PropertyRef,
    ShapeDecl, ShapeType, Spanned, Statement, StyleModifier, StyleValue, TemplateInstance,
};

use super::registry::{TemplateError, TemplateRegistry};

/// Context for template resolution
#[derive(Debug, Clone)]
pub struct ResolutionContext {
    /// Parameter values for the current resolution
    pub parameters: HashMap<String, StyleValue>,
    /// Instance name prefix for nested templates
    pub name_prefix: String,
    /// Set of templates currently being resolved (for cycle detection)
    pub resolving: HashSet<String>,
}

impl Default for ResolutionContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ResolutionContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            parameters: HashMap::new(),
            name_prefix: String::new(),
            resolving: HashSet::new(),
        }
    }

    /// Create a context with parameters
    pub fn with_parameters(parameters: HashMap<String, StyleValue>) -> Self {
        Self {
            parameters,
            name_prefix: String::new(),
            resolving: HashSet::new(),
        }
    }

    /// Create a nested context for recursive resolution
    pub fn nested(&self, prefix: &str, new_params: HashMap<String, StyleValue>) -> Self {
        let name_prefix = if self.name_prefix.is_empty() {
            prefix.to_string()
        } else {
            format!("{}_{}", self.name_prefix, prefix)
        };

        Self {
            parameters: new_params,
            name_prefix,
            resolving: self.resolving.clone(),
        }
    }

    /// Get a parameter value
    pub fn get_parameter(&self, name: &str) -> Option<&StyleValue> {
        self.parameters.get(name)
    }

    /// Prefix an identifier with the current namespace
    pub fn prefix_name(&self, name: &str) -> String {
        if self.name_prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}_{}", self.name_prefix, name)
        }
    }

    /// Check if a template is currently being resolved (cycle detection)
    pub fn is_resolving(&self, name: &str) -> bool {
        self.resolving.contains(name)
    }

    /// Mark a template as being resolved
    pub fn start_resolving(&mut self, name: &str) {
        self.resolving.insert(name.to_string());
    }

    /// Mark a template as done resolving
    pub fn done_resolving(&mut self, name: &str) {
        self.resolving.remove(name);
    }
}

/// Resolve all template instances in a document
///
/// This function:
/// 1. Collects all template declarations into a registry
/// 2. Expands template instances into their concrete shapes
/// 3. Returns a new document with all templates resolved
pub fn resolve_templates(doc: Document, registry: &mut TemplateRegistry) -> Result<Document, TemplateError> {
    // First pass: collect template declarations
    registry.collect_from_statements(&doc.statements)?;

    // Second pass: resolve template instances
    let mut resolved_statements = Vec::new();
    let mut ctx = ResolutionContext::new();

    for stmt in doc.statements {
        match &stmt.node {
            Statement::TemplateDecl(_) => {
                // Template declarations are consumed by the registry, not rendered
                continue;
            }
            Statement::TemplateInstance(inst) => {
                // Expand the template instance
                let expanded = resolve_instance(inst, &stmt.span, registry, &mut ctx)?;
                resolved_statements.extend(expanded);
            }
            _ => {
                // Recursively resolve any nested template instances
                let resolved = resolve_statement(stmt, registry, &mut ctx)?;
                resolved_statements.push(resolved);
            }
        }
    }

    Ok(Document {
        statements: resolved_statements,
    })
}

/// Resolve a single template instance into statements
fn resolve_instance(
    inst: &TemplateInstance,
    span: &std::ops::Range<usize>,
    registry: &mut TemplateRegistry,
    ctx: &mut ResolutionContext,
) -> Result<Vec<Spanned<Statement>>, TemplateError> {
    let template_name = inst.template_name.node.as_str();
    let instance_name = inst.instance_name.node.as_str();

    // Check for circular references
    if ctx.is_resolving(template_name) {
        return Err(TemplateError::CircularReference {
            chain: format!("{} -> {}", ctx.resolving.iter().cloned().collect::<Vec<_>>().join(" -> "), template_name),
        });
    }

    // Get the template definition
    let def = registry.get(template_name).ok_or_else(|| TemplateError::NotFound {
        name: template_name.to_string(),
    })?.clone(); // Clone to avoid borrow issues

    // Build parameter values from arguments and defaults
    let mut param_values: HashMap<String, StyleValue> = HashMap::new();

    // Start with defaults
    for param in &def.parameters {
        param_values.insert(
            param.name.node.0.clone(),
            param.default_value.node.clone(),
        );
    }

    // Override with provided arguments
    for (name, value) in &inst.arguments {
        let param_name = name.node.0.clone();
        if def.has_parameter(&param_name) {
            param_values.insert(param_name, value.node.clone());
        }
        // Note: Extra arguments are silently ignored (could warn in future)
    }

    ctx.start_resolving(template_name);

    let result = match def.source_type {
        crate::parser::ast::TemplateSourceType::Svg => {
            // For SVG templates, create an SvgEmbed shape
            resolve_svg_template(&def, instance_name, span, registry, &param_values)
        }
        crate::parser::ast::TemplateSourceType::Ail => {
            // For AIL templates, load and resolve the external file
            resolve_ail_template(&def, instance_name, span, registry, ctx, &param_values)
        }
        crate::parser::ast::TemplateSourceType::Inline => {
            // For inline templates, expand the body
            resolve_inline_template(&def, instance_name, span, registry, ctx, &param_values)
        }
    };

    ctx.done_resolving(template_name);
    result
}

/// Resolve an SVG file template into an SvgEmbed shape
fn resolve_svg_template(
    def: &super::registry::TemplateDefinition,
    instance_name: &str,
    span: &std::ops::Range<usize>,
    registry: &mut TemplateRegistry,
    _param_values: &HashMap<String, StyleValue>,
) -> Result<Vec<Spanned<Statement>>, TemplateError> {
    // Ensure the SVG content is loaded
    if def.svg_content.is_none() {
        registry.load_svg_template(&def.name)?;
    }

    // Get the loaded definition
    let def = registry.get(&def.name).ok_or_else(|| TemplateError::NotFound {
        name: def.name.clone(),
    })?;

    let content = def.svg_content.clone().unwrap_or_default();
    let (width, height) = def.svg_dimensions.unwrap_or((100.0, 100.0));

    let shape = ShapeDecl {
        shape_type: Spanned::new(
            ShapeType::SvgEmbed {
                content,
                intrinsic_width: Some(width),
                intrinsic_height: Some(height),
            },
            span.clone(),
        ),
        name: Some(Spanned::new(Identifier::new(instance_name), span.clone())),
        modifiers: vec![],
    };

    Ok(vec![Spanned::new(Statement::Shape(shape), span.clone())])
}

/// Resolve an AIL file template by loading and parsing the external file
fn resolve_ail_template(
    def: &super::registry::TemplateDefinition,
    instance_name: &str,
    span: &std::ops::Range<usize>,
    registry: &mut TemplateRegistry,
    ctx: &mut ResolutionContext,
    param_values: &HashMap<String, StyleValue>,
) -> Result<Vec<Spanned<Statement>>, TemplateError> {
    // Get the source path
    let source_path = def.source_path.as_ref().ok_or_else(|| TemplateError::FileNotFound {
        path: std::path::PathBuf::from(&def.name),
    })?;

    // Resolve relative to base path
    let full_path = registry.resolve_path(source_path.to_str().unwrap_or(""));

    // Load the AIL file content
    let content = std::fs::read_to_string(&full_path).map_err(|e| TemplateError::FileReadError {
        path: full_path.clone(),
        message: e.to_string(),
    })?;

    // Parse the AIL content
    let parsed_doc = crate::parser::parse(&content).map_err(|errors| {
        TemplateError::FileReadError {
            path: full_path.clone(),
            message: format!("Parse errors: {:?}", errors),
        }
    })?;

    // Collect any nested template declarations from the AIL file
    registry.collect_from_statements(&parsed_doc.statements)?;

    // Create a nested context for this instance
    let mut nested_ctx = ctx.nested(instance_name, param_values.clone());

    let mut expanded = Vec::new();

    for stmt in parsed_doc.statements {
        match &stmt.node {
            Statement::TemplateDecl(_) => {
                // Template declarations are consumed by the registry, not expanded
                continue;
            }
            Statement::Export(_) => {
                // Exports are metadata, skip during expansion
                continue;
            }
            Statement::TemplateInstance(nested_inst) => {
                // Recursively expand nested template instances
                let nested_expanded = resolve_instance(nested_inst, &stmt.span, registry, &mut nested_ctx)?;
                expanded.extend(nested_expanded);
            }
            _ => {
                // Substitute parameters and prefix identifiers
                let resolved = substitute_parameters(stmt.clone(), param_values, instance_name);
                let resolved = resolve_statement(resolved, registry, &mut nested_ctx)?;
                expanded.push(resolved);
            }
        }
    }

    // If there's only one shape, rename it to the instance name
    if expanded.len() == 1 {
        if let Statement::Shape(mut shape) = expanded[0].node.clone() {
            shape.name = Some(Spanned::new(Identifier::new(instance_name), span.clone()));
            return Ok(vec![Spanned::new(Statement::Shape(shape), span.clone())]);
        }
    }

    Ok(expanded)
}

/// Resolve an inline template by expanding its body
fn resolve_inline_template(
    def: &super::registry::TemplateDefinition,
    instance_name: &str,
    span: &std::ops::Range<usize>,
    registry: &mut TemplateRegistry,
    ctx: &mut ResolutionContext,
    param_values: &HashMap<String, StyleValue>,
) -> Result<Vec<Spanned<Statement>>, TemplateError> {
    let body = match &def.body {
        Some(b) => b.clone(),
        None => return Ok(vec![]),
    };

    // Create a nested context for this instance
    let mut nested_ctx = ctx.nested(instance_name, param_values.clone());

    let mut expanded = Vec::new();

    for stmt in body {
        match &stmt.node {
            Statement::Export(_) => {
                // Exports are metadata, skip during expansion
                continue;
            }
            Statement::TemplateInstance(nested_inst) => {
                // Recursively expand nested template instances
                let nested_expanded = resolve_instance(nested_inst, &stmt.span, registry, &mut nested_ctx)?;
                expanded.extend(nested_expanded);
            }
            _ => {
                // Substitute parameters and prefix identifiers
                let resolved = substitute_parameters(stmt.clone(), param_values, instance_name);
                let resolved = resolve_statement(resolved, registry, &mut nested_ctx)?;
                expanded.push(resolved);
            }
        }
    }

    // If there's only one shape, rename it to the instance name
    // If there are multiple, wrap them in a group
    if expanded.len() == 1 {
        // Rename the single element to the instance name
        if let Statement::Shape(mut shape) = expanded[0].node.clone() {
            shape.name = Some(Spanned::new(Identifier::new(instance_name), span.clone()));
            return Ok(vec![Spanned::new(Statement::Shape(shape), span.clone())]);
        }
    }

    // Multiple elements: wrap in a group (or return as-is for now)
    // TODO: Consider wrapping in a group with the instance name
    Ok(expanded)
}

/// Resolve a statement recursively
fn resolve_statement(
    stmt: Spanned<Statement>,
    registry: &mut TemplateRegistry,
    ctx: &mut ResolutionContext,
) -> Result<Spanned<Statement>, TemplateError> {
    match stmt.node {
        Statement::Layout(mut layout) => {
            let mut resolved_children = Vec::new();
            for child in layout.children {
                match &child.node {
                    Statement::TemplateInstance(inst) => {
                        let expanded = resolve_instance(inst, &child.span, registry, ctx)?;
                        resolved_children.extend(expanded);
                    }
                    _ => {
                        let resolved = resolve_statement(child, registry, ctx)?;
                        resolved_children.push(resolved);
                    }
                }
            }
            layout.children = resolved_children;
            Ok(Spanned::new(Statement::Layout(layout), stmt.span))
        }
        Statement::Group(mut group) => {
            let mut resolved_children = Vec::new();
            for child in group.children {
                match &child.node {
                    Statement::TemplateInstance(inst) => {
                        let expanded = resolve_instance(inst, &child.span, registry, ctx)?;
                        resolved_children.extend(expanded);
                    }
                    _ => {
                        let resolved = resolve_statement(child, registry, ctx)?;
                        resolved_children.push(resolved);
                    }
                }
            }
            group.children = resolved_children;
            Ok(Spanned::new(Statement::Group(group), stmt.span))
        }
        Statement::Label(inner) => {
            let resolved_inner = resolve_statement(Spanned::new(*inner, stmt.span.clone()), registry, ctx)?;
            Ok(Spanned::new(Statement::Label(Box::new(resolved_inner.node)), stmt.span))
        }
        // Other statements pass through unchanged
        _ => Ok(stmt),
    }
}

/// Substitute parameter references in a statement
fn substitute_parameters(
    stmt: Spanned<Statement>,
    params: &HashMap<String, StyleValue>,
    prefix: &str,
) -> Spanned<Statement> {
    match stmt.node {
        Statement::Shape(mut shape) => {
            // Prefix the shape name
            if let Some(ref mut name) = shape.name {
                name.node = Identifier::new(format!("{}_{}", prefix, name.node.0));
            }
            // Substitute parameters in modifiers
            shape.modifiers = substitute_modifiers(&shape.modifiers, params);
            Spanned::new(Statement::Shape(shape), stmt.span)
        }
        Statement::Layout(mut layout) => {
            // Prefix the layout name
            if let Some(ref mut name) = layout.name {
                name.node = Identifier::new(format!("{}_{}", prefix, name.node.0));
            }
            // Substitute in children
            layout.children = layout.children
                .into_iter()
                .map(|c| substitute_parameters(c, params, prefix))
                .collect();
            layout.modifiers = substitute_modifiers(&layout.modifiers, params);
            Spanned::new(Statement::Layout(layout), stmt.span)
        }
        Statement::Group(mut group) => {
            // Prefix the group name
            if let Some(ref mut name) = group.name {
                name.node = Identifier::new(format!("{}_{}", prefix, name.node.0));
            }
            // Substitute in children
            group.children = group.children
                .into_iter()
                .map(|c| substitute_parameters(c, params, prefix))
                .collect();
            group.modifiers = substitute_modifiers(&group.modifiers, params);
            Spanned::new(Statement::Group(group), stmt.span)
        }
        Statement::Connection(mut conn) => {
            // Prefix the connection endpoints
            conn.from.node = Identifier::new(format!("{}_{}", prefix, conn.from.node.0));
            conn.to.node = Identifier::new(format!("{}_{}", prefix, conn.to.node.0));
            conn.modifiers = substitute_modifiers(&conn.modifiers, params);
            Spanned::new(Statement::Connection(conn), stmt.span)
        }
        Statement::Constrain(decl) => {
            // Prefix all element references in the constraint expression
            let new_expr = prefix_constraint_expr(&decl.expr, prefix);
            Spanned::new(
                Statement::Constrain(ConstrainDecl { expr: new_expr }),
                stmt.span,
            )
        }
        // Other statements pass through
        _ => stmt,
    }
}

/// Prefix all element references in a constraint expression
fn prefix_constraint_expr(expr: &ConstraintExpr, prefix: &str) -> ConstraintExpr {
    match expr {
        ConstraintExpr::Equal { left, right } => ConstraintExpr::Equal {
            left: prefix_property_ref(left, prefix),
            right: prefix_property_ref(right, prefix),
        },
        ConstraintExpr::EqualWithOffset {
            left,
            right,
            offset,
        } => ConstraintExpr::EqualWithOffset {
            left: prefix_property_ref(left, prefix),
            right: prefix_property_ref(right, prefix),
            offset: *offset,
        },
        ConstraintExpr::Constant { left, value } => ConstraintExpr::Constant {
            left: prefix_property_ref(left, prefix),
            value: *value,
        },
        ConstraintExpr::GreaterOrEqual { left, value } => ConstraintExpr::GreaterOrEqual {
            left: prefix_property_ref(left, prefix),
            value: *value,
        },
        ConstraintExpr::LessOrEqual { left, value } => ConstraintExpr::LessOrEqual {
            left: prefix_property_ref(left, prefix),
            value: *value,
        },
        ConstraintExpr::Midpoint {
            target,
            a,
            b,
            offset,
        } => ConstraintExpr::Midpoint {
            target: prefix_property_ref(target, prefix),
            a: prefix_identifier(a, prefix),
            b: prefix_identifier(b, prefix),
            offset: *offset,
        },
        ConstraintExpr::Contains {
            container,
            elements,
            padding,
        } => ConstraintExpr::Contains {
            container: prefix_identifier(container, prefix),
            elements: elements
                .iter()
                .map(|e| prefix_identifier(e, prefix))
                .collect(),
            padding: *padding,
        },
    }
}

/// Prefix a property reference
fn prefix_property_ref(prop_ref: &PropertyRef, prefix: &str) -> PropertyRef {
    PropertyRef {
        element: prefix_element_path(&prop_ref.element, prefix),
        property: prop_ref.property.clone(),
    }
}

/// Prefix an element path (add prefix to the first segment)
fn prefix_element_path(path: &Spanned<ElementPath>, prefix: &str) -> Spanned<ElementPath> {
    let mut new_segments = path.node.segments.clone();
    if !new_segments.is_empty() {
        let first = &new_segments[0];
        new_segments[0] = Spanned::new(
            Identifier::new(format!("{}_{}", prefix, first.node.0)),
            first.span.clone(),
        );
    }
    Spanned::new(
        ElementPath {
            segments: new_segments,
        },
        path.span.clone(),
    )
}

/// Prefix a single identifier
fn prefix_identifier(id: &Spanned<Identifier>, prefix: &str) -> Spanned<Identifier> {
    Spanned::new(
        Identifier::new(format!("{}_{}", prefix, id.node.0)),
        id.span.clone(),
    )
}

/// Substitute parameter references in modifiers
fn substitute_modifiers(
    modifiers: &[Spanned<StyleModifier>],
    params: &HashMap<String, StyleValue>,
) -> Vec<Spanned<StyleModifier>> {
    modifiers
        .iter()
        .map(|m| {
            let new_value = match &m.node.value.node {
                StyleValue::Identifier(id) => {
                    // Check if this identifier is a parameter reference
                    if let Some(param_value) = params.get(id.as_str()) {
                        Spanned::new(param_value.clone(), m.node.value.span.clone())
                    } else {
                        m.node.value.clone()
                    }
                }
                _ => m.node.value.clone(),
            };

            Spanned::new(
                StyleModifier {
                    key: m.node.key.clone(),
                    value: new_value,
                },
                m.span.clone(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::parser::ast::StyleKey;

    #[test]
    fn test_resolve_inline_template() {
        let source = r#"
            template "box" {
                rect shape [fill: blue]
            }
            box mybox
        "#;

        let doc = parse(source).expect("Should parse");
        let mut registry = TemplateRegistry::new();
        let resolved = resolve_templates(doc, &mut registry).expect("Should resolve");

        // Should have one statement (the expanded template instance)
        assert_eq!(resolved.statements.len(), 1);

        // The statement should be a Shape with name "mybox"
        match &resolved.statements[0].node {
            Statement::Shape(s) => {
                assert_eq!(s.name.as_ref().unwrap().node.as_str(), "mybox");
            }
            other => panic!("Expected Shape, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_template_with_params() {
        let source = r#"
            template "box" (fill: blue, size: 50) {
                rect shape [fill: fill, size: size]
            }
            box mybox [fill: red, size: 100]
        "#;

        let doc = parse(source).expect("Should parse");
        let mut registry = TemplateRegistry::new();
        let resolved = resolve_templates(doc, &mut registry).expect("Should resolve");

        assert_eq!(resolved.statements.len(), 1);
        match &resolved.statements[0].node {
            Statement::Shape(s) => {
                // Check that fill was substituted
                let fill_mod = s.modifiers.iter().find(|m| matches!(m.node.key.node, StyleKey::Fill));
                assert!(fill_mod.is_some());
                // The value should be the keyword "red" (from the instance)
                match &fill_mod.unwrap().node.value.node {
                    StyleValue::Keyword(k) => assert_eq!(k, "red"),
                    other => panic!("Expected Keyword, got {:?}", other),
                }
            }
            other => panic!("Expected Shape, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_preserves_other_statements() {
        let source = r#"
            template "box" {
                rect shape
            }
            rect standalone
            box mybox
            standalone -> mybox
        "#;

        let doc = parse(source).expect("Should parse");
        let mut registry = TemplateRegistry::new();
        let resolved = resolve_templates(doc, &mut registry).expect("Should resolve");

        // Should have 3 statements: standalone rect, expanded box, connection
        assert_eq!(resolved.statements.len(), 3);
    }

    #[test]
    fn test_template_not_found_error() {
        let source = "unknown_template myinstance";

        let doc = parse(source).expect("Should parse");
        let mut registry = TemplateRegistry::new();
        let result = resolve_templates(doc, &mut registry);

        assert!(matches!(result, Err(TemplateError::NotFound { .. })));
    }
}
