//! Template resolution - expands template instances into concrete elements

use std::collections::{HashMap, HashSet};

use crate::parser::ast::{
    AnchorDecl, AnchorPosition, ConstrainDecl, ConstraintExpr, Document, ElementPath, GroupDecl,
    Identifier, PropertyRef, ShapeDecl, ShapeType, Spanned, Statement, StyleKey, StyleModifier,
    MotionNode, MotionVerb, StyleValue, TemplateInstance,
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
pub fn resolve_templates(
    doc: Document,
    registry: &mut TemplateRegistry,
) -> Result<Document, TemplateError> {
    // First pass: collect template declarations
    registry.collect_from_statements(&doc.statements)?;

    // `let tr = [...]`: the file's named lists. They leave the document here.
    let mut statements = Vec::with_capacity(doc.statements.len());
    for st in doc.statements {
        match st.node {
            Statement::Let(l) => {
                registry.lets.insert(l.name.node.0, l.value.node);
            }
            other => statements.push(Spanned::new(other, st.span)),
        }
    }
    if !registry.lets.is_empty() {
        misused_lets(&statements, &registry.lets, registry, &HashSet::new())?;
    }

    // `each` at the top level repeats over literal lists and lets
    let doc_statements = super::each::expand_each(statements, &registry.lets)?;
    if !registry.lets.is_empty() {
        // A let and an element of the same name: `[a: tr]` could mean either.
        let declared = super::each::declared_names(&doc_statements);
        let mut clash: Vec<&String> = registry.lets.keys().filter(|n| declared.contains(*n)).collect();
        clash.sort();
        if let Some(n) = clash.first() {
            return Err(TemplateError::Each {
                message: format!(
                    "`let {}`: there is also an element called `{}`; a template argument `[x: {}]` could mean either, so give the list another name (e.g. `{}_list`)",
                    n, n, n, n
                ),
            });
        }
    }

    // Second pass: resolve template instances
    let mut resolved_statements = Vec::new();
    let mut ctx = ResolutionContext::new();

    for stmt in doc_statements {
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

    let mut aliases = doc.aliases;
    aliases.extend(registry.collapsed_parts.iter().cloned());
    Ok(Document {
        statements: resolved_statements,
        aliases,
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
            chain: format!(
                "{} -> {}",
                ctx.resolving
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" -> "),
                template_name
            ),
        });
    }

    // Get the template definition
    let def = registry
        .get(template_name)
        .ok_or_else(|| TemplateError::NotFound {
            name: template_name.to_string(),
        })?
        .clone(); // Clone to avoid borrow issues

    // Build parameter values from arguments and defaults
    let mut param_values: HashMap<String, StyleValue> = HashMap::new();

    // `[ticks: tr]` or a default `(ticks: tr)` with `let tr = [...]`: the list.
    let lets = &registry.lets;
    let with_lets = |v: &StyleValue| -> StyleValue {
        match v {
            StyleValue::Identifier(id) => lets.get(&id.0).cloned().unwrap_or_else(|| v.clone()),
            _ => v.clone(),
        }
    };

    // Start with defaults
    for param in &def.parameters {
        param_values.insert(param.name.node.0.clone(), with_lets(&param.default_value.node));
    }

    // Override with provided arguments
    let mut defaulted: HashSet<String> = param_values.keys().cloned().collect();
    for (name, value) in &inst.arguments {
        let param_name = name.node.0.clone();
        if def.has_parameter(&param_name) {
            defaulted.remove(&param_name);
            param_values.insert(param_name, with_lets(&value.node));
        }
        // Note: Extra arguments are silently ignored (could warn in future)
    }

    ctx.start_resolving(template_name);

    // Convert instance arguments to modifiers (excluding template parameters)
    let template_param_names: Vec<String> = def
        .parameters
        .iter()
        .map(|p| p.name.node.0.clone())
        .collect();
    let instance_modifiers = arguments_to_modifiers(&inst.arguments, &template_param_names, span);

    let result = match def.source_type {
        crate::parser::ast::TemplateSourceType::Svg => {
            // For SVG templates, create an SvgEmbed shape; `arm.pivot: left`
            // on the instance reaches its part `arm` as for any component.
            resolve_svg_template(&def, instance_name, span, registry, &instance_modifiers).map(|mut stmts| {
                let overrides: Vec<(String, String, Spanned<StyleValue>)> = instance_modifiers
                    .iter()
                    .filter_map(|m| match &m.node.key.node {
                        StyleKey::Custom(k) if k.contains('.') => {
                            let (part, key) = k.split_once('.').unwrap();
                            Some((format!("{}_{}", instance_name, part), key.to_string(), m.node.value.clone()))
                        }
                        _ => None,
                    })
                    .collect();
                if !overrides.is_empty() {
                    override_parts(&mut stmts, &overrides);
                }
                stmts
            })
        }
        crate::parser::ast::TemplateSourceType::Ail => {
            // For AIL templates, load and resolve the external file
            resolve_ail_template(&def, instance_name, span, registry, ctx, &param_values)
        }
        crate::parser::ast::TemplateSourceType::Raster => {
            // For raster image templates, create a RasterImage shape
            resolve_raster_template(&def, instance_name, span, registry, &instance_modifiers)
        }
        crate::parser::ast::TemplateSourceType::Inline => {
            // For inline templates, expand the body
            resolve_inline_template(&def, instance_name, span, registry, ctx, &param_values, &defaulted, &instance_modifiers)
        }
    };

    ctx.done_resolving(template_name);
    result
}

/// Read `[trim: false]` from instance modifiers (default true). Accepts keyword/ident
/// `true`/`false` and numeric 0/non-0. `trim` parses as `StyleKey::Custom("trim")`.
fn read_trim_flag(modifiers: &[Spanned<StyleModifier>]) -> bool {
    for m in modifiers {
        let is_trim = matches!(&m.node.key.node, StyleKey::Custom(k) if k == "trim");
        if is_trim {
            return match &m.node.value.node {
                StyleValue::Keyword(s) => s != "false",
                StyleValue::Identifier(Identifier(s)) => s != "false",
                StyleValue::Number { value, .. } => *value != 0.0,
                _ => true,
            };
        }
    }
    true
}

/// Resolve an SVG file template into an SvgEmbed shape
fn resolve_svg_template(
    def: &super::registry::TemplateDefinition,
    instance_name: &str,
    span: &std::ops::Range<usize>,
    registry: &mut TemplateRegistry,
    instance_modifiers: &[Spanned<StyleModifier>],
) -> Result<Vec<Spanned<Statement>>, TemplateError> {
    // Ensure the SVG content is loaded
    if def.svg_content.is_none() {
        registry.load_svg_template(&def.name)?;
    }

    // Get the loaded definition
    let def = registry
        .get(&def.name)
        .ok_or_else(|| TemplateError::NotFound {
            name: def.name.clone(),
        })?;

    let content = def.svg_content.clone().unwrap_or_default();

    // Trim is on by default; `[trim: false]` on the instance disables it. When trimming,
    // use the artwork's content bbox for sizing/anchors and offset the content so it
    // fills the element rect. Falls back to the raw viewBox when no trimmed bbox exists.
    let trim_enabled = read_trim_flag(instance_modifiers);
    let ((width, height), (offset_x, offset_y)) = match (trim_enabled, def.svg_trimmed) {
        (true, Some((mx, my, w, h))) => ((w, h), (mx, my)),
        _ => (def.svg_dimensions.unwrap_or((100.0, 100.0)), (0.0, 0.0)),
    };

    // A file whose elements carry ids is a component: one part per id.
    if let Some(parts) = super::svg_parts::analyze(&content) {
        let (tw, th) = (width, height);
        let explicit = |k: StyleKey| {
            instance_modifiers.iter().find(|m| m.node.key.node == k).and_then(|m| match &m.node.value.node {
                StyleValue::Number { value, .. } => Some(*value),
                _ => None,
            })
        };
        let scale = explicit(StyleKey::Width)
            .map(|w| w / tw)
            .or_else(|| explicit(StyleKey::Height).map(|h| h / th))
            .unwrap_or(1.0);
        let mut values = Vec::new();
        let mut group_modifiers = Vec::new();
        for m in instance_modifiers {
            match &m.node.key.node {
                StyleKey::Width | StyleKey::Height => {}
                StyleKey::Custom(k) if k == "trim" => {}
                StyleKey::Custom(k) if parts.vars.contains(k) => {
                    let css = crate::layout::types::ResolvedStyles::color_to_css(&m.node.value.node)
                        .or_else(|| match &m.node.value.node {
                            StyleValue::String(s) => Some(s.clone()),
                            _ => None,
                        });
                    if let Some(css) = css {
                        values.push((k.clone(), css));
                    }
                }
                _ => group_modifiers.push(m.clone()),
            }
        }
        return Ok(super::svg_parts::instance_statements(
            &parts,
            &content,
            instance_name,
            (offset_x, offset_y, tw, th),
            scale,
            &values,
            group_modifiers,
            span,
        ));
    }

    let shape = ShapeDecl {
        shape_type: Spanned::new(
            ShapeType::SvgEmbed {
                content,
                intrinsic_width: Some(width),
                intrinsic_height: Some(height),
                offset_x,
                offset_y,
            },
            span.clone(),
        ),
        name: Some(Spanned::new(Identifier::new(instance_name), span.clone())),
        modifiers: instance_modifiers.to_vec(),
    };

    Ok(vec![Spanned::new(Statement::Shape(shape), span.clone())])
}

/// Resolve a raster image template into a RasterImage shape
fn resolve_raster_template(
    def: &super::registry::TemplateDefinition,
    instance_name: &str,
    span: &std::ops::Range<usize>,
    registry: &TemplateRegistry,
    instance_modifiers: &[Spanned<StyleModifier>],
) -> Result<Vec<Spanned<Statement>>, TemplateError> {
    // Get the source path for the raster image
    let source_path = def
        .source_path
        .as_ref()
        .ok_or_else(|| TemplateError::FileNotFound {
            path: std::path::PathBuf::from(&def.name),
        })?;

    // Resolve image href according to the configured mode
    let href = registry.resolve_image_href(source_path.to_str().unwrap_or(""));

    let shape = ShapeDecl {
        shape_type: Spanned::new(ShapeType::RasterImage { path: href }, span.clone()),
        name: Some(Spanned::new(Identifier::new(instance_name), span.clone())),
        modifiers: instance_modifiers.to_vec(),
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
    let source_path = def
        .source_path
        .as_ref()
        .ok_or_else(|| TemplateError::FileNotFound {
            path: std::path::PathBuf::from(&def.name),
        })?;

    // Resolve relative to base path
    let full_path = registry.resolve_path(source_path.to_str().unwrap_or(""));

    // Load the AIL file content
    let content =
        crate::vfs::read_to_string(&full_path).map_err(|e| TemplateError::FileReadError {
            path: full_path.clone(),
            message: e.to_string(),
        })?;

    // Parse the AIL content
    let parsed_doc =
        crate::parser::parse(&content).map_err(|errors| TemplateError::FileReadError {
            path: full_path.clone(),
            message: format!("Parse errors: {:?}", errors),
        })?;

    if parsed_doc.statements.iter().any(|s| matches!(s.node, Statement::Let(_))) {
        return Err(TemplateError::Each {
            message: format!(
                "{}: `let` works in the file being drawn, not in a template file; pass the list in as a template argument instead (template \"{}\" (items: []) ... [items: tr])",
                full_path.display(),
                def.name
            ),
        });
    }

    // Collect any nested template declarations from the AIL file
    registry.collect_from_statements(&parsed_doc.statements)?;

    // Create a nested context for this instance
    let mut nested_ctx = ctx.nested(instance_name, param_values.clone());

    let mut expanded = Vec::new();

    for stmt in super::each::expand_each(parsed_doc.statements, param_values)? {
        match &stmt.node {
            Statement::TemplateDecl(_) => {
                // Template declarations are consumed by the registry, not expanded
                continue;
            }
            Statement::Export(_) => {
                // Exports are metadata, skip during expansion
                continue;
            }
            Statement::TemplateInstance(_) => {
                // A component inside a component: its name is scoped by the
                // outer instance (`hub_hist`, addressed as `hub.hist`), and its
                // arguments may use the outer template's parameters.
                let scoped = substitute_parameters(stmt.clone(), param_values, instance_name);
                let Statement::TemplateInstance(nested_inst) = &scoped.node else { unreachable!() };
                let nested_expanded =
                    resolve_instance(nested_inst, &stmt.span, registry, &mut nested_ctx)?;
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
            if let Some(part) = &shape.name {
                registry.collapsed_parts.push((part.node.0.clone(), instance_name.to_string()));
            }
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
    defaulted: &HashSet<String>,
    instance_modifiers: &[Spanned<StyleModifier>],
) -> Result<Vec<Spanned<Statement>>, TemplateError> {
    // `check tests [opacity: 0.3]`: the whole instance is faint;
    // `[appears: later]`: it enters later.
    let group_modifiers: Vec<Spanned<StyleModifier>> = instance_modifiers
        .iter()
        .filter(|m| {
            matches!(&m.node.key.node, StyleKey::Opacity)
                || matches!(&m.node.key.node, StyleKey::Custom(k) if k == "appears" || k == "overlaps")
        })
        .cloned()
        .collect();
    // The file's lets are lists here too; a parameter of the same name wins.
    let mut lists = registry.lets.clone();
    lists.extend(param_values.iter().map(|(k, v)| (k.clone(), v.clone())));
    let mut body = match &def.body {
        Some(b) => super::each::expand_each(b.clone(), &lists)?,
        None => return Ok(vec![]),
    };
    let own = own_part_defaults(param_values, defaulted, &body, instance_name);
    let param_values = &own;
    // `machine anna [hist.appears: later]`: a modifier for one part of this
    // instance (`hist.d4.appears` reaches into a nested component).
    let overrides: Vec<(String, String, Spanned<StyleValue>)> = instance_modifiers
        .iter()
        .filter_map(|m| match &m.node.key.node {
            StyleKey::Custom(k) if k.contains('.') => {
                let (part, key) = k.split_once('.').unwrap();
                Some((part.to_string(), key.to_string(), m.node.value.clone()))
            }
            _ => None,
        })
        .collect();
    if !overrides.is_empty() {
        override_parts(&mut body, &overrides);
    }

    // Anchors of the template, wherever the body declares them: at the top,
    // in a row or column, or inside an `each` (one per item).
    let body_anchors = take_anchors(&mut body);

    // Create a nested context for this instance
    let mut nested_ctx = ctx.nested(instance_name, param_values.clone());

    let mut expanded = Vec::new();
    let mut states = Vec::new();

    for stmt in body {
        match &stmt.node {
            Statement::Export(_) | Statement::AnchorDecl(_) => {
                // Exports and anchor declarations are metadata, skip during expansion
                // Anchors are processed separately and attached to the group
                continue;
            }
            Statement::ComponentState { name, body, .. } => {
                // Kept for `set <instance> <state>`: owned by this instance,
                // with the template's parameters filled in.
                states.push(Spanned::new(
                    Statement::ComponentState {
                        name: name.clone(),
                        body: substitute_motion_params(body, param_values),
                        owner: instance_name.to_string(),
                    },
                    stmt.span.clone(),
                ));
            }
            Statement::TemplateInstance(_) => {
                // A component inside a component: its name is scoped by the
                // outer instance (`hub_hist`, addressed as `hub.hist`), and its
                // arguments may use the outer template's parameters.
                let scoped = substitute_parameters(stmt.clone(), param_values, instance_name);
                let Statement::TemplateInstance(nested_inst) = &scoped.node else { unreachable!() };
                let nested_expanded =
                    resolve_instance(nested_inst, &stmt.span, registry, &mut nested_ctx)?;
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

    // Prefix anchor declarations with instance name (Feature 009)
    let prefixed_anchors: Vec<_> = body_anchors
        .iter()
        .map(|a| prefix_anchor_decl(a, instance_name))
        .collect();

    // If there's only one shape and no custom anchors, rename it to the instance name
    // If there are multiple, or if there are custom anchors, wrap in a group
    if expanded.len() == 1 && prefixed_anchors.is_empty() && group_modifiers.is_empty() {
        // Rename the single element to the instance name
        if let Statement::Shape(mut shape) = expanded[0].node.clone() {
            if let Some(part) = &shape.name {
                registry.collapsed_parts.push((part.node.0.clone(), instance_name.to_string()));
            }
            shape.name = Some(Spanned::new(Identifier::new(instance_name), span.clone()));
            let mut out = vec![Spanned::new(Statement::Shape(shape), span.clone())];
            out.extend(states);
            return Ok(out);
        }
    }
    expanded.extend(states);

    // Multiple elements or custom anchors: wrap them in a group with the instance name
    // This allows the instance name to be used in connections and constraints
    // Custom anchors are attached to the group for layout resolution
    let group = GroupDecl {
        name: Some(Spanned::new(Identifier::new(instance_name), span.clone())),
        children: expanded,
        modifiers: group_modifiers,
        anchors: prefixed_anchors,
        is_template_instance: true,
    };
    Ok(vec![Spanned::new(Statement::Group(group), span.clone())])
}

/// `fill: tr`, `label: tr`, `tr -> x`, `[note: tr]` (not a parameter) with
/// `let tr = [...]`: a list where one value or one element goes. `shadow`
/// are names that mean something else here (a template's parameters, loop
/// variables).
fn misused_lets(
    stmts: &[Spanned<Statement>],
    lets: &HashMap<String, StyleValue>,
    registry: &TemplateRegistry,
    shadow: &HashSet<String>,
) -> Result<(), TemplateError> {
    let is_let = |name: &str| -> Option<String> {
        let head = name.split('.').next().unwrap_or(name);
        (!shadow.contains(head) && lets.contains_key(head)).then(|| head.to_string())
    };
    let fail = |written: String, name: &str| -> TemplateError {
        let what = match lets.get(name) {
            Some(StyleValue::List(items)) => {
                let records = items.iter().filter(|i| matches!(i.node, StyleValue::Record(_))).count();
                if records == items.len() && records > 0 {
                    format!("a list of {} record{}", records, if records == 1 { "" } else { "s" })
                } else {
                    format!("a list of {} item{}", items.len(), if items.len() == 1 { "" } else { "s" })
                }
            }
            _ => "a list".to_string(),
        };
        let var = name.chars().next().unwrap_or('x');
        TemplateError::Each {
            message: format!(
                "`{}`: `{}` is data (a let: {}); use it in `each {} in {} {{ ... }}` or as a template argument (`[items: {}]`)",
                written, name, what, var, name, name
            ),
        }
    };
    let check_value = |key: &str, v: &StyleValue| -> Result<(), TemplateError> {
        let one = |n: &str| match is_let(n) {
            Some(l) => Err(fail(format!("{}: {}", key, n), &l)),
            None => Ok(()),
        };
        match v {
            StyleValue::Identifier(id) => one(&id.0),
            StyleValue::IdentifierList(ids) => ids.iter().try_for_each(|i| one(&i.0)),
            StyleValue::List(items) => items.iter().try_for_each(|i| match &i.node {
                StyleValue::Identifier(id) => one(&id.0),
                _ => Ok(()),
            }),
            _ => Ok(()),
        }
    };
    let check_mods = |mods: &[Spanned<StyleModifier>]| -> Result<(), TemplateError> {
        mods.iter().try_for_each(|m| check_value(&m.node.key.node.source_name(), &m.node.value.node))
    };
    for st in stmts {
        match &st.node {
            Statement::Shape(sh) => {
                check_mods(&sh.modifiers)?;
                if let ShapeType::Path(p) = &sh.shape_type.node {
                    check_mods(&p.modifiers)?;
                }
            }
            Statement::Layout(l) => {
                check_mods(&l.modifiers)?;
                misused_lets(&l.children, lets, registry, shadow)?;
            }
            Statement::Group(g) => {
                check_mods(&g.modifiers)?;
                misused_lets(&g.children, lets, registry, shadow)?;
            }
            Statement::Connection(cs) => {
                for c in cs {
                    for end in [&c.from, &c.to] {
                        if let Some(l) = is_let(&end.element.node.0) {
                            return Err(fail(format!("{} -> {}", c.from.element.node.0, c.to.element.node.0), &l));
                        }
                    }
                    check_mods(&c.modifiers)?;
                }
            }
            Statement::TemplateInstance(inst) => {
                let def = registry.get(inst.template_name.node.as_str());
                for (k, v) in &inst.arguments {
                    let is_param = def.is_some_and(|d| d.has_parameter(&k.node.0));
                    // `k* [items: tr]` was filled in by the parser.
                    if !is_param && k.node.0 != "items" {
                        check_value(&k.node.0, &v.node)?;
                    }
                }
            }
            Statement::Each(e) => {
                let mut inner = shadow.clone();
                inner.insert(e.var.node.0.clone());
                if let Some(ix) = &e.index {
                    inner.insert(ix.node.0.clone());
                }
                misused_lets(&e.body, lets, registry, &inner)?;
            }
            Statement::TemplateDecl(t) => {
                if let Some(body) = &t.body {
                    let mut inner = shadow.clone();
                    inner.extend(t.parameters.iter().map(|p| p.name.node.0.clone()));
                    misused_lets(body, lets, registry, &inner)?;
                }
            }
            Statement::Label(inner) => {
                misused_lets(&[Spanned::new((**inner).clone(), st.span.clone())], lets, registry, shadow)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Pull every `anchor` declaration out of a template body (top level and
/// inside rows/columns), in order.
fn take_anchors(stmts: &mut Vec<Spanned<Statement>>) -> Vec<AnchorDecl> {
    let mut found = Vec::new();
    let mut keep = Vec::with_capacity(stmts.len());
    for mut st in std::mem::take(stmts) {
        match &mut st.node {
            Statement::AnchorDecl(a) => {
                found.push(a.clone());
                continue;
            }
            Statement::Layout(l) => found.extend(take_anchors(&mut l.children)),
            _ => {}
        }
        keep.push(st);
    }
    *stmts = keep;
    found
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
            let mut extracted_anchors = Vec::new();
            for child in group.children {
                match &child.node {
                    Statement::TemplateInstance(inst) => {
                        let expanded = resolve_instance(inst, &child.span, registry, ctx)?;
                        resolved_children.extend(expanded);
                    }
                    Statement::AnchorDecl(anchor) => {
                        // Extract anchor declarations to group.anchors
                        // They should not appear in children for layout
                        extracted_anchors.push(anchor.clone());
                    }
                    _ => {
                        let resolved = resolve_statement(child, registry, ctx)?;
                        resolved_children.push(resolved);
                    }
                }
            }
            group.children = resolved_children;
            // Merge extracted anchors with any existing anchors
            group.anchors.extend(extracted_anchors);
            Ok(Spanned::new(Statement::Group(group), stmt.span))
        }
        Statement::Label(inner) => {
            let resolved_inner =
                resolve_statement(Spanned::new(*inner, stmt.span.clone()), registry, ctx)?;
            Ok(Spanned::new(
                Statement::Label(Box::new(resolved_inner.node)),
                stmt.span,
            ))
        }
        // Other statements pass through unchanged
        _ => Ok(stmt),
    }
}

/// Substitute parameter references in a statement
/// Give the named parts of a template body the instance's overrides.
fn override_parts(stmts: &mut [Spanned<Statement>], overrides: &[(String, String, Spanned<StyleValue>)]) {
    for st in stmts {
        let span = st.span.clone();
        match &mut st.node {
            Statement::Shape(s) => {
                let name = s.name.as_ref().map(|n| n.node.0.clone());
                for (part, key, value) in overrides {
                    if name.as_deref() == Some(part.as_str()) && !key.contains('.') {
                        let k = crate::parser::style_key_named(key);
                        s.modifiers.retain(|m| m.node.key.node != k);
                        s.modifiers.push(Spanned::new(
                            StyleModifier { key: Spanned::new(k, span.clone()), value: value.clone() },
                            span.clone(),
                        ));
                    }
                }
            }
            Statement::TemplateInstance(inst) => {
                for (part, key, value) in overrides {
                    if inst.instance_name.node.0 == *part {
                        inst.arguments.retain(|(k, _)| k.node.0 != *key);
                        inst.arguments.push((Spanned::new(Identifier::new(key.as_str()), span.clone()), value.clone()));
                    }
                }
            }
            Statement::Group(g) => {
                if let Some(n) = g.name.as_ref().map(|n| n.node.0.clone()) {
                    for (part, key, value) in overrides {
                        if n == *part && !key.contains('.') {
                            let k = crate::parser::style_key_named(key);
                            g.modifiers.push(Spanned::new(
                                StyleModifier { key: Spanned::new(k, span.clone()), value: value.clone() },
                                span.clone(),
                            ));
                        }
                    }
                }
                override_parts(&mut g.children, overrides)
            }
            Statement::Layout(l) => {
                if let Some(n) = l.name.as_ref().map(|n| n.node.0.clone()) {
                    for (part, key, value) in overrides {
                        if n == *part && !key.contains('.') {
                            let k = crate::parser::style_key_named(key);
                            l.modifiers.push(Spanned::new(
                                StyleModifier { key: Spanned::new(k, span.clone()), value: value.clone() },
                                span.clone(),
                            ));
                        }
                    }
                }
                override_parts(&mut l.children, overrides)
            }
            _ => {}
        }
    }
}

/// A state's transforms may use the template's parameters as values
/// (`transform txt [label: done_text]`).
fn substitute_motion_params(nodes: &[Spanned<MotionNode>], params: &HashMap<String, StyleValue>) -> Vec<Spanned<MotionNode>> {
    nodes
        .iter()
        .map(|n| {
            let node = match &n.node {
                MotionNode::Stmt(st) => {
                    let mut st = st.clone();
                    if let MotionVerb::Transform { modifiers, .. } = &mut st.verb {
                        for m in modifiers {
                            if let StyleValue::Identifier(id) = &m.node.value.node {
                                if let Some(v) = params.get(&id.0) {
                                    m.node.value.node = v.clone();
                                }
                            }
                        }
                    }
                    MotionNode::Stmt(st)
                }
                MotionNode::Then(b) => MotionNode::Then(substitute_motion_params(b, params)),
                MotionNode::After(t, b) => MotionNode::After(*t, substitute_motion_params(b, params)),
                MotionNode::At(t, b) => MotionNode::At(*t, substitute_motion_params(b, params)),
                MotionNode::When(e, t, b) => MotionNode::When(e.clone(), *t, substitute_motion_params(b, params)),
                MotionNode::Beat(nm, b) => MotionNode::Beat(nm.clone(), substitute_motion_params(b, params)),
            };
            Spanned::new(node, n.span.clone())
        })
        .collect()
}

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
            // Also prefix path names (they're inside PathDecl, not ShapeDecl.name)
            if let ShapeType::Path(ref mut path_decl) = shape.shape_type.node {
                if let Some(ref mut name) = path_decl.name {
                    name.node = Identifier::new(format!("{}_{}", prefix, name.node.0));
                }
            }
            // `text "{name}" t`: a template's text can say its parameters.
            if let ShapeType::Text { ref mut content } = shape.shape_type.node {
                for (k, v) in params {
                    let needle = format!("{{{}}}", k);
                    if content.contains(&needle) {
                        let text = match v {
                            StyleValue::String(s) | StyleValue::Keyword(s) => s.clone(),
                            StyleValue::Number { value, .. } => format!("{}", value),
                            StyleValue::Identifier(id) => id.0.clone(),
                            _ => continue,
                        };
                        *content = content.replace(&needle, &text);
                    }
                }
            }
            // `overlaps:` is scoped before substitution: a name that is a
            // parameter is whatever was passed in (outside the template), any
            // other name is this instance's own part.
            let overlaps = shape
                .modifiers
                .iter()
                .find(|m| matches!(&m.node.key.node, StyleKey::Custom(k) if k == "overlaps"))
                .map(|m| scope_overlaps(&m.node.value, params, prefix));
            // Substitute parameters in modifiers
            shape.modifiers = substitute_modifiers(&shape.modifiers, params);
            if let Some(v) = overlaps {
                for m in shape.modifiers.iter_mut() {
                    if matches!(&m.node.key.node, StyleKey::Custom(k) if k == "overlaps") {
                        m.node.value = v.clone();
                    }
                }
            }
            // A line routed through siblings names them: scope those names too.
            let scope_list = |mods: &mut Vec<Spanned<StyleModifier>>| {
                for m in mods.iter_mut() {
                    // `overlaps:` was scoped above.
                    if matches!(&m.node.key.node, StyleKey::Custom(k) if k == "overlaps") {
                        continue;
                    }
                    // `caption_of: dot` names a sibling inside the template.
                    if matches!(&m.node.key.node, StyleKey::CaptionOf) {
                        if let StyleValue::Identifier(id) = &m.node.value.node {
                            let local = id.0.replace('.', "_");
                            m.node.value.node = StyleValue::Identifier(Identifier::new(format!("{}_{}", prefix, local)));
                        }
                        continue;
                    }
                    // `clip: bg` names the sibling whose shape it is drawn inside.
                    if matches!(&m.node.key.node, StyleKey::Custom(k) if k == "clip") {
                        if let StyleValue::Identifier(id) = &m.node.value.node {
                            let local = id.0.replace('.', "_");
                            m.node.value.node = StyleValue::Identifier(Identifier::new(format!("{}_{}", prefix, local)));
                        }
                        continue;
                    }
                    if matches!(&m.node.key.node, StyleKey::Custom(k) if k == "drawn") {
                        if let StyleValue::Identifier(id) = &m.node.value.node {
                            if id.0 != "none" {
                                let local = id.0.replace('.', "_");
                                m.node.value.node = StyleValue::Identifier(Identifier::new(format!("{}_{}", prefix, local)));
                            }
                        }
                        continue;
                    }
                    if !matches!(&m.node.key.node, StyleKey::Custom(k) if k == "through") {
                        continue;
                    }
                    if let StyleValue::List(items) = &mut m.node.value.node {
                        for it in items.iter_mut() {
                            if let StyleValue::Identifier(id) = &it.node {
                                let local = id.0.replace('.', "_");
                                it.node = StyleValue::Identifier(Identifier::new(format!("{}_{}", prefix, local)));
                            }
                        }
                    }
                }
            };
            scope_list(&mut shape.modifiers);
            if let ShapeType::Path(ref mut path_decl) = shape.shape_type.node {
                path_decl.modifiers = substitute_modifiers(&path_decl.modifiers, params);
                scope_list(&mut path_decl.modifiers);
            }
            Spanned::new(Statement::Shape(shape), stmt.span)
        }
        Statement::Layout(mut layout) => {
            // Prefix the layout name
            if let Some(ref mut name) = layout.name {
                name.node = Identifier::new(format!("{}_{}", prefix, name.node.0));
            }
            // Substitute in children
            layout.children = layout
                .children
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
            group.children = group
                .children
                .into_iter()
                .map(|c| substitute_parameters(c, params, prefix))
                .collect();
            group.modifiers = substitute_modifiers(&group.modifiers, params);
            Spanned::new(Statement::Group(group), stmt.span)
        }
        Statement::Connection(mut conns) => {
            // Prefix all connection endpoints
            // Feature 009: AnchorReference.element contains the identifier
            for conn in &mut conns {
                let refs = element_refs(params);
                conn.from.element = prefix_identifier(&conn.from.element, prefix, &refs);
                conn.to.element = prefix_identifier(&conn.to.element, prefix, &refs);
                // `a.right -> b.left as w` names a part of this instance
                // (`acme.w`), like any other part.
                if let Some(n) = &mut conn.name {
                    n.node = Identifier::new(format!("{}_{}", prefix, n.node.0));
                }
                conn.modifiers = substitute_modifiers(&conn.modifiers, params);
            }
            Spanned::new(Statement::Connection(conns), stmt.span)
        }
        Statement::Constrain(decl) => {
            // Prefix all element references in the constraint expression
            let new_expr = prefix_constraint_expr(&decl.expr, prefix, &element_refs(params));
            Spanned::new(
                Statement::Constrain(ConstrainDecl { expr: new_expr, name: decl.name.clone() }),
                stmt.span,
            )
        }
        Statement::TemplateInstance(mut inst) => {
            inst.instance_name.node = Identifier::new(format!("{}_{}", prefix, inst.instance_name.node.0));
            inst.arguments = inst
                .arguments
                .into_iter()
                .map(|(k, v)| {
                    let v = match &v.node {
                        StyleValue::Identifier(id) => match params.get(id.as_str()) {
                            Some(p) => Spanned::new(p.clone(), v.span.clone()),
                            None => v,
                        },
                        _ => v,
                    };
                    (k, v)
                })
                .collect();
            Spanned::new(Statement::TemplateInstance(inst), stmt.span)
        }
        // Other statements pass through
        _ => stmt,
    }
}

/// Element-reference parameters of an instance: parameter name -> element it names.
type Refs = HashMap<String, String>;

/// `overlaps: hd` / `[hd, on]` inside a template: `all` stays, a parameter
/// becomes what was passed in (a name or a list of names, from outside), and
/// any other name is the instance's own part (`a_hd`).
fn scope_overlaps(v: &Spanned<StyleValue>, params: &HashMap<String, StyleValue>, prefix: &str) -> Spanned<StyleValue> {
    let one = |id: &Identifier, span: &crate::parser::ast::Span| -> Vec<Spanned<StyleValue>> {
        if id.0 == "all" {
            return vec![Spanned::new(StyleValue::Identifier(id.clone()), span.clone())];
        }
        match params.get(&id.0) {
            Some(StyleValue::List(items)) => items.clone(),
            Some(other) => vec![Spanned::new(other.clone(), span.clone())],
            None => vec![Spanned::new(
                StyleValue::Identifier(Identifier::new(format!("{}_{}", prefix, id.0.replace('.', "_")))),
                span.clone(),
            )],
        }
    };
    let node = match &v.node {
        StyleValue::Identifier(id) => {
            let mut out = one(id, &v.span);
            if out.len() == 1 {
                out.pop().unwrap().node
            } else {
                StyleValue::List(out)
            }
        }
        StyleValue::List(items) => StyleValue::List(
            items
                .iter()
                .flat_map(|it| match &it.node {
                    StyleValue::Identifier(id) => one(id, &it.span),
                    _ => vec![it.clone()],
                })
                .collect(),
        ),
        other => other.clone(),
    };
    Spanned::new(node, v.span.clone())
}

/// `template "life" (until: rows) { col rows {...}; hd -- until }`: a parameter
/// left at its default, whose default names one of the template's own parts
/// (`rows`, `rows.bottom`, `l.rows`), means that part of *this* instance
/// (`cl_rows`). A value the caller passes is a name from outside, as always.
fn own_part_defaults(
    param_values: &HashMap<String, StyleValue>,
    defaulted: &HashSet<String>,
    body: &[Spanned<Statement>],
    instance_name: &str,
) -> HashMap<String, StyleValue> {
    let own = super::each::declared_names(body);
    param_values
        .iter()
        .map(|(k, v)| {
            let v = match v {
                StyleValue::Identifier(id)
                    if defaulted.contains(k) && own.contains(id.0.split('.').next().unwrap_or("")) =>
                {
                    StyleValue::Identifier(Identifier::new(format!("{}_{}", instance_name, id.0)))
                }
                _ => v.clone(),
            };
            (k.clone(), v)
        })
        .collect()
}

fn element_refs(params: &HashMap<String, StyleValue>) -> Refs {
    params
        .iter()
        .filter_map(|(k, v)| match v {
            StyleValue::Identifier(id) => Some((k.clone(), id.0.clone())),
            _ => None,
        })
        .collect()
}

/// Prefix all element references in a constraint expression
fn prefix_constraint_expr(expr: &ConstraintExpr, prefix: &str, refs: &Refs) -> ConstraintExpr {
    match expr {
        ConstraintExpr::Equal { left, right } => ConstraintExpr::Equal {
            left: prefix_property_ref(left, prefix, refs),
            right: prefix_property_ref(right, prefix, refs),
        },
        ConstraintExpr::EqualWithOffset {
            left,
            right,
            offset,
        } => ConstraintExpr::EqualWithOffset {
            left: prefix_property_ref(left, prefix, refs),
            right: prefix_property_ref(right, prefix, refs),
            offset: *offset,
        },
        ConstraintExpr::Constant { left, value } => ConstraintExpr::Constant {
            left: prefix_property_ref(left, prefix, refs),
            value: *value,
        },
        ConstraintExpr::GreaterOrEqual { left, value } => ConstraintExpr::GreaterOrEqual {
            left: prefix_property_ref(left, prefix, refs),
            value: *value,
        },
        ConstraintExpr::LessOrEqual { left, value } => ConstraintExpr::LessOrEqual {
            left: prefix_property_ref(left, prefix, refs),
            value: *value,
        },
        ConstraintExpr::Midpoint {
            target,
            a,
            b,
            offset,
            a_edge,
            b_edge,
        } => ConstraintExpr::Midpoint {
            target: prefix_property_ref(target, prefix, refs),
            a: prefix_identifier(a, prefix, refs),
            b: prefix_identifier(b, prefix, refs),
            offset: *offset,
            a_edge: a_edge.clone(),
            b_edge: b_edge.clone(),
        },
        ConstraintExpr::Contains {
            container,
            elements,
            padding,
            padding_x,
            padding_y,
        } => ConstraintExpr::Contains {
            container: prefix_identifier(container, prefix, refs),
            elements: elements
                .iter()
                .map(|e| prefix_identifier(e, prefix, refs))
                .collect(),
            padding: *padding,
            padding_x: *padding_x,
            padding_y: *padding_y,
        },
    }
}

/// Prefix a property reference
fn prefix_property_ref(prop_ref: &PropertyRef, prefix: &str, refs: &Refs) -> PropertyRef {
    PropertyRef {
        element: prefix_element_path(&prop_ref.element, prefix, refs),
        property: prop_ref.property.clone(),
    }
}

/// Prefix an element path (add prefix to the first segment)
fn prefix_element_path(path: &Spanned<ElementPath>, prefix: &str, refs: &Refs) -> Spanned<ElementPath> {
    let mut new_segments = path.node.segments.clone();
    if let Some(target) = new_segments.first().and_then(|f| refs.get(&f.node.0)) {
        // An element-reference parameter: the element the instance was given
        let spliced = super::names::splice_path(&path.node, target, &new_segments[0].span);
        return Spanned::new(spliced, path.span.clone());
    }
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
fn prefix_identifier(id: &Spanned<Identifier>, prefix: &str, refs: &Refs) -> Spanned<Identifier> {
    if let Some(target) = refs.get(super::names::head_of(&id.node.0).0) {
        // `contains` and midpoint operands name parts with `_` (`x_p_1`)
        let text = super::names::splice_dotted(&id.node.0, target).replace('.', "_");
        return Spanned::new(Identifier::new(text), id.span.clone());
    }
    Spanned::new(
        Identifier::new(format!("{}_{}", prefix, id.node.0)),
        id.span.clone(),
    )
}

/// Substitute parameter references in modifiers
/// `"release: {rname}"` with the template's parameters filled in.
fn interpolate(text: &str, params: &HashMap<String, StyleValue>) -> String {
    let mut out = text.to_string();
    for (k, v) in params {
        let needle = format!("{{{}}}", k);
        if out.contains(&needle) {
            let value = match v {
                StyleValue::String(s) | StyleValue::Keyword(s) => s.clone(),
                StyleValue::Number { value, .. } => format!("{}", value),
                StyleValue::Identifier(id) => id.0.clone(),
                _ => continue,
            };
            out = out.replace(&needle, &value);
        }
    }
    out
}

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
                // `label: "release: {rname}"`: parameters inside a string.
                StyleValue::String(text) if text.contains('{') => {
                    Spanned::new(StyleValue::String(interpolate(text, params)), m.node.value.span.clone())
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

/// Convert template instance arguments to style modifiers
/// Arguments that match template parameters are filtered out (they're used for substitution)
/// Remaining arguments are converted to modifiers for the resulting shape
fn arguments_to_modifiers(
    arguments: &[(Spanned<Identifier>, Spanned<StyleValue>)],
    template_params: &[String],
    span: &std::ops::Range<usize>,
) -> Vec<Spanned<StyleModifier>> {
    arguments
        .iter()
        .filter(|(name, _)| !template_params.contains(&name.node.0))
        .map(|(name, value)| {
            let key = match name.node.0.as_str() {
                "fill" => StyleKey::Fill,
                "stroke" => StyleKey::Stroke,
                "stroke_width" => StyleKey::StrokeWidth,
                "opacity" => StyleKey::Opacity,
                "fill_opacity" => StyleKey::FillOpacity,
                "stroke_opacity" => StyleKey::StrokeOpacity,
                "label" => StyleKey::Label,
                "font_size" => StyleKey::FontSize,
                "class" => StyleKey::Class,
                "gap" => StyleKey::Gap,
                "size" => StyleKey::Size,
                "width" => StyleKey::Width,
                "height" => StyleKey::Height,
                "routing" => StyleKey::Routing,
                "role" => StyleKey::Role,
                "x" => StyleKey::X,
                "y" => StyleKey::Y,
                "stroke_dasharray" => StyleKey::StrokeDasharray,
                "rotation" | "rotate" => StyleKey::Rotation,
                "z_order" => StyleKey::ZOrder,
                "pointer" => StyleKey::Pointer,
                "align" => StyleKey::Align,
                "label_fill" => StyleKey::LabelFill,
                other => StyleKey::Custom(other.to_string()),
            };
            Spanned::new(
                StyleModifier {
                    key: Spanned::new(key, name.span.clone()),
                    value: value.clone(),
                },
                span.clone(),
            )
        })
        .collect()
}

/// Prefix element references in an anchor declaration (Feature 009)
fn prefix_anchor_decl(anchor: &AnchorDecl, prefix: &str) -> AnchorDecl {
    let prefixed_position = match &anchor.position {
        AnchorPosition::PropertyRef(prop_ref) => {
            AnchorPosition::PropertyRef(prefix_property_ref(prop_ref, prefix, &Refs::new()))
        }
        AnchorPosition::PropertyRefWithOffset { prop_ref, offset } => {
            AnchorPosition::PropertyRefWithOffset {
                prop_ref: prefix_property_ref(prop_ref, prefix, &Refs::new()),
                offset: *offset,
            }
        }
    };

    AnchorDecl {
        name: anchor.name.clone(),
        position: prefixed_position,
        direction: anchor.direction.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(doc: &Document) -> Vec<String> {
        fn go(stmts: &[Spanned<Statement>], out: &mut Vec<String>) {
            for s in stmts {
                match &s.node {
                    Statement::Shape(sh) => out.extend(sh.name.iter().map(|n| n.node.0.clone())),
                    Statement::Layout(l) => {
                        out.extend(l.name.iter().map(|n| n.node.0.clone()));
                        go(&l.children, out);
                    }
                    Statement::Group(g) => {
                        out.extend(g.name.iter().map(|n| n.node.0.clone()));
                        go(&g.children, out);
                    }
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        go(&doc.statements, &mut out);
        out
    }

    #[test]
    fn each_expands_a_list_parameter_with_numbered_names() {
        let source = r#"
            template "list" (items: []) {
                col c { each it in items { rect rw [label: it] } }
            }
            list a [items: ["x", "y", "z"]]
            list b
        "#;
        let doc = parse(source).expect("parse");
        let mut registry = TemplateRegistry::new();
        let resolved = resolve_templates(doc, &mut registry).expect("resolve");
        let n = names(&resolved);
        for want in ["a_c", "a_rw_1", "a_rw_2", "a_rw_3", "b"] {
            assert!(n.contains(&want.to_string()), "{want} in {n:?}");
        }
        assert!(!n.iter().any(|x| x.starts_with("b_rw")), "{n:?}");
    }

    #[test]
    fn each_nested_names_chain() {
        let source = r#"each a in ["p", "q"] { each b in ["x", "y"] { rect cell } }"#;
        let doc = parse(source).expect("parse");
        let resolved = resolve_templates(doc, &mut TemplateRegistry::new()).expect("resolve");
        assert_eq!(names(&resolved), vec!["cell_1_1", "cell_1_2", "cell_2_1", "cell_2_2"]);
    }

    #[test]
    fn each_substitutes_the_item_and_the_index() {
        let source = r#"each a, i in ["p", 3] { rect r [label: a, width: i] }"#;
        let doc = parse(source).expect("parse");
        let resolved = resolve_templates(doc, &mut TemplateRegistry::new()).expect("resolve");
        let Statement::Shape(s) = &resolved.statements[0].node else { panic!() };
        let label = s.modifiers.iter().find(|m| m.node.key.node == StyleKey::Label).unwrap();
        assert_eq!(label.node.value.node, StyleValue::String("p".into()));
        let Statement::Shape(s) = &resolved.statements[1].node else { panic!() };
        let w = s.modifiers.iter().find(|m| m.node.key.node == StyleKey::Width).unwrap();
        assert_eq!(w.node.value.node, StyleValue::Number { value: 1.0, unit: None });
    }

    #[test]
    fn each_unknown_list_is_an_error() {
        let doc = parse("each a in nothing { rect r }").expect("parse");
        let err = resolve_templates(doc, &mut TemplateRegistry::new()).unwrap_err().to_string();
        assert!(err.contains("nothing"), "{err}");
    }
    use crate::parser::ast::StyleKey;
    use crate::parser::parse;

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
                let fill_mod = s
                    .modifiers
                    .iter()
                    .find(|m| matches!(m.node.key.node, StyleKey::Fill));
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

    // Feature 009: Template anchor tests (T023)

    #[test]
    fn test_template_with_anchors_resolves_to_group() {
        // When a template has custom anchors, it should always expand to a group
        // (even with a single element) so the anchors can be attached
        let source = r#"
            template "server" {
                rect body [width: 100, height: 60]
                anchor input [position: body.left, direction: left]
                anchor output [position: body.right, direction: right]
            }
            server myserver
        "#;

        let doc = parse(source).expect("Should parse");
        let mut registry = TemplateRegistry::new();
        let resolved = resolve_templates(doc, &mut registry).expect("Should resolve");

        assert_eq!(resolved.statements.len(), 1);
        match &resolved.statements[0].node {
            Statement::Group(g) => {
                assert_eq!(g.name.as_ref().unwrap().node.as_str(), "myserver");
                // Should have one child (the body rect)
                assert_eq!(g.children.len(), 1);
                // Should have 2 custom anchors
                assert_eq!(g.anchors.len(), 2);
                // Check anchor names and that element refs are prefixed
                let anchor_names: Vec<_> = g.anchors.iter().map(|a| a.name.node.as_str()).collect();
                assert!(anchor_names.contains(&"input"));
                assert!(anchor_names.contains(&"output"));
            }
            other => panic!("Expected Group with anchors, got {:?}", other),
        }
    }

    #[test]
    fn test_template_anchors_are_prefixed() {
        // Verify that element references in anchors are prefixed with instance name
        let source = r#"
            template "box" {
                rect content [width: 80]
                anchor top_port [position: content.top]
            }
            box mybox
        "#;

        let doc = parse(source).expect("Should parse");
        let mut registry = TemplateRegistry::new();
        let resolved = resolve_templates(doc, &mut registry).expect("Should resolve");

        match &resolved.statements[0].node {
            Statement::Group(g) => {
                let anchor = &g.anchors[0];
                // The anchor position should reference the prefixed element name
                match &anchor.position {
                    AnchorPosition::PropertyRef(pr) => {
                        let element_name = pr.element.node.segments[0].node.as_str();
                        // The element reference should be prefixed with the instance name
                        assert!(
                            element_name.starts_with("mybox_"),
                            "Element ref should be prefixed, got: {}",
                            element_name
                        );
                    }
                    AnchorPosition::PropertyRefWithOffset { prop_ref, .. } => {
                        let element_name = prop_ref.element.node.segments[0].node.as_str();
                        assert!(
                            element_name.starts_with("mybox_"),
                            "Element ref should be prefixed, got: {}",
                            element_name
                        );
                    }
                }
            }
            other => panic!("Expected Group, got {:?}", other),
        }
    }
}
