//! Structured diagnostics: what a front end (WebAssembly, Python) shows an
//! author or feeds back to an agent, instead of one error string.
//!
//! [`check`] renders and lints in one call and returns the output plus every
//! diagnostic: a render error (syntax, layout, template, missing file) as an
//! `error`, and each lint finding with its category's severity.

use crate::layout::lint::LintWarning;
use crate::layout::LayoutError;
use crate::template::TemplateError;
use crate::{ParseError, RenderConfig, RenderError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

/// A 1-based line/column range in the source (end exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Stable, kebab-case: `syntax`, `undefined-identifier`, `missing-file`,
    /// ..., or a lint category (`overlap`, `label-overflow`, ...).
    pub code: String,
    /// One line, without the location.
    pub message: String,
    pub range: Option<SourceRange>,
    /// What would fix it, when known (`did you mean: server?`).
    pub hint: Option<String>,
    /// Keyframes a lint finding occurs in; empty when frame-independent.
    pub frames: Vec<String>,
    /// The elements a lint finding is about (as the message names them:
    /// dotted paths for parts), so a front end can point at them.
    pub elements: Vec<String>,
    /// The whole report as the CLI prints it: location, source line, caret.
    pub text: String,
}

/// The result of [`check`]: the output when the render succeeded, and every
/// diagnostic. `output` is present with lint findings, including lint
/// errors; it is absent only when the render itself failed.
#[derive(Debug, Clone)]
pub struct Report {
    pub output: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    /// No diagnostic of `error` severity.
    pub fn ok(&self) -> bool {
        self.output.is_some() && !self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }
}

/// Render `source` (and lint it, when `config.lint`), collecting diagnostics.
pub fn check(source: &str, config: RenderConfig) -> Report {
    match crate::render_pipeline(source, config) {
        Ok((output, warnings)) => Report { output: Some(output), diagnostics: warnings.iter().map(from_lint).collect() },
        Err(e) => Report { output: None, diagnostics: from_error(&e, source) },
    }
}

fn range(source: &str, span: &std::ops::Range<usize>) -> SourceRange {
    let at = |offset: usize| {
        let before = &source[..floor_char_boundary(source, offset)];
        let line = before.matches('\n').count() + 1;
        let column = before[before.rfind('\n').map(|i| i + 1).unwrap_or(0)..].chars().count() + 1;
        (line, column)
    };
    let (line, column) = at(span.start);
    let (end_line, end_column) = at(span.end.max(span.start + 1));
    SourceRange { line, column, end_line, end_column }
}

fn floor_char_boundary(s: &str, mut i: usize) -> usize {
    i = i.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// `line L, column C: message\n  source\n  ^^^` → `message`.
fn first_line(text: &str) -> String {
    let line = text.lines().next().unwrap_or("");
    match line.strip_prefix("line ").and_then(|rest| rest.split_once(": ")) {
        Some((loc, msg)) if loc.contains("column") => msg.to_string(),
        _ => line.to_string(),
    }
}

fn error(code: &str, message: String, range: Option<SourceRange>, hint: Option<String>, text: String) -> Diagnostic {
    Diagnostic { severity: Severity::Error, code: code.into(), message, range, hint, frames: Vec::new(), elements: Vec::new(), text }
}

/// One diagnostic per error in `e` (a syntax error can come with several).
pub fn from_error(e: &RenderError, source: &str) -> Vec<Diagnostic> {
    match e {
        RenderError::Parse(errors) => errors
            .iter()
            .map(|pe| {
                let ParseError::Syntax { span, message, expected } = pe;
                let mut shown = expected.clone();
                shown.sort();
                shown.dedup();
                let hint = (!shown.is_empty()).then(|| format!("expected: {}", shown.join(", ")));
                error("syntax", first_line(message), Some(range(source, span)), hint, message.clone())
            })
            .collect(),
        RenderError::Layout(le) => {
            let text = crate::locate_error(RenderError::Layout(clone_layout(le)), source).to_string();
            let text = text.strip_prefix("layout error: ").unwrap_or(&text).to_string();
            let (code, span, hint): (&str, Option<&std::ops::Range<usize>>, Option<String>) = match le {
                LayoutError::Located { span, .. } => ("layout", Some(span), None),
                LayoutError::UndefinedIdentifier { span, suggestions, .. } => (
                    "undefined-identifier",
                    Some(span),
                    (!suggestions.is_empty()).then(|| format!("did you mean: {}?", suggestions.join(", "))),
                ),
                LayoutError::PathNotFound { span, suggestions, .. } => (
                    "path-not-found",
                    Some(span),
                    (!suggestions.is_empty()).then(|| format!("did you mean: {}?", suggestions.join(", "))),
                ),
                LayoutError::ConflictingConstraints { .. } => ("conflicting-constraints", None, None),
                LayoutError::CircularConstraint { .. } => ("circular-constraint", None, None),
                LayoutError::InvalidLayout { .. } => ("invalid-layout", None, None),
                LayoutError::InvalidKeyframeName { span, .. } => ("invalid-keyframe-name", Some(span), None),
                LayoutError::InvalidLabel { span, .. } => ("invalid-label", Some(span), None),
                LayoutError::SolverError(_) => ("solver", None, None),
                LayoutError::InvalidAnchor { span, .. } => ("invalid-anchor", Some(span), None),
                LayoutError::ValidationError(m) if is_missing_file(m) => ("missing-file", None, None),
                LayoutError::ValidationError(_) => ("validation", None, None),
            };
            let span = span.filter(|s| s.end <= source.len() && s.start < s.end);
            vec![error(code, first_line(&text), span.map(|s| range(source, s)), hint, text)]
        }
        RenderError::Template(te) => {
            let code = match te {
                TemplateError::FileNotFound { .. } | TemplateError::FileReadError { .. } => "missing-file",
                TemplateError::NotFound { .. } => "unknown-template",
                TemplateError::Duplicate { .. } => "duplicate-template",
                TemplateError::MissingParameter { .. } => "missing-parameter",
                TemplateError::InvalidParameterType { .. } => "invalid-parameter",
                TemplateError::InvalidSvg { .. } => "invalid-svg",
                TemplateError::CircularReference { .. } => "circular-template",
                TemplateError::ExportNotFound { .. } => "unknown-export",
                #[allow(unreachable_patterns)]
                _ => "template",
            };
            let text = te.to_string();
            vec![error(code, first_line(&text), None, None, text)]
        }
    }
}

/// `import "x"` and `code [file: ...]` report unreadable files as validation
/// errors; they are missing files all the same.
fn is_missing_file(message: &str) -> bool {
    message.contains("cannot read")
}

fn clone_layout(e: &LayoutError) -> LayoutError {
    match e {
        LayoutError::Located { message, span } => LayoutError::Located { message: message.clone(), span: span.clone() },
        LayoutError::UndefinedIdentifier { name, span, suggestions } => {
            LayoutError::UndefinedIdentifier { name: name.clone(), span: span.clone(), suggestions: suggestions.clone() }
        }
        LayoutError::PathNotFound { path, span, suggestions } => {
            LayoutError::PathNotFound { path: path.clone(), span: span.clone(), suggestions: suggestions.clone() }
        }
        // locate_error only rewrites the variants above.
        other => LayoutError::ValidationError(other.to_string()),
    }
}

pub fn from_lint(w: &LintWarning) -> Diagnostic {
    let severity = if w.category.is_error() { Severity::Error } else { Severity::Warning };
    Diagnostic {
        severity,
        code: w.category.to_string(),
        message: w.message.clone(),
        range: None,
        hint: None,
        frames: w.frames.clone(),
        elements: lint_elements(w),
        text: format!("lint: {}{}: {}", w.category, w.frame_suffix(), w.message),
    }
}

/// The names a finding quotes (`"api"`, `"browser→api"` → both ends), in
/// the message's order so its subject comes first; else the pair it is about.
fn lint_elements(w: &LintWarning) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (i, quoted) in w.message.split('"').enumerate() {
        if i % 2 == 0 || quoted.is_empty() || quoted.contains(char::is_whitespace) {
            continue;
        }
        for name in quoted.split('→') {
            if !name.is_empty() && !out.iter().any(|n| n == name) {
                out.push(name.to_string());
            }
        }
    }
    if out.is_empty() {
        if let Some((a, b)) = &w.pair {
            out = vec![a.clone(), b.clone()];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_error_has_a_range_and_a_short_message() {
        let r = check("rect a\nrow { rect b ", RenderConfig::default());
        assert!(!r.ok());
        let d = &r.diagnostics[0];
        assert_eq!(d.code, "syntax");
        assert_eq!(d.range.unwrap().line, 2);
        assert!(!d.message.starts_with("line "));
        assert!(d.text.contains("line 2"));
    }

    #[test]
    fn undefined_identifier_suggests() {
        let r = check("rect server\nservr -> server", RenderConfig::default());
        let d = &r.diagnostics[0];
        assert_eq!(d.code, "undefined-identifier");
        assert_eq!(d.range.unwrap().line, 2);
        assert!(d.hint.as_deref().unwrap_or("").contains("server"), "{:?}", d);
    }

    #[test]
    fn missing_import_is_a_missing_file() {
        let (r, missing) = crate::vfs::with_files(Default::default(), || {
            check("import \"deck.ail\"\nrect a", RenderConfig::default())
        });
        assert_eq!(r.diagnostics[0].code, "missing-file");
        assert_eq!(missing, vec!["deck.ail".to_string()]);
    }

    #[test]
    fn overlap_names_its_elements() {
        let src = "rect api [width: 120]\nrect db [width: 120]\nconstrain db.left = api.left + 30\nconstrain db.top = api.top + 10";
        let r = check(src, RenderConfig::default().with_lint(true));
        let d = r.diagnostics.iter().find(|d| d.code == "overlap").expect("an overlap");
        assert_eq!(d.elements, vec!["api".to_string(), "db".to_string()]);
    }

    #[test]
    fn a_label_finding_names_the_label_owner_first() {
        let src = "rect api [label: \"API\"]\nrect note [label: \"A note\"]\nconstrain note.center_x = api.center_x + 20\nconstrain note.center_y = api.center_y + 8";
        let r = check(src, RenderConfig::default().with_lint(true));
        let d = r.diagnostics.iter().find(|d| d.message.starts_with("label on")).expect("a straddle");
        assert_eq!(d.elements.first().map(String::as_str), Some("note"), "{:?}", d);
    }

    #[test]
    fn clean_render_is_ok() {
        let r = check("row { rect a rect b }", RenderConfig::default().with_lint(true));
        assert!(r.ok(), "{:?}", r.diagnostics);
        assert!(r.output.unwrap().contains("<svg"));
    }
}
