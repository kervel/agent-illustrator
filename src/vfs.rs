//! File access for the render pipeline.
//!
//! Every file the pipeline reads (`import`, `template ... from`, `code
//! [file: ...]`, base64-inlined images) goes through [`read`]. Normally that is
//! the real filesystem. Inside [`with_files`] it is a caller-supplied set of
//! files instead (sans-io): a read of a path that is not in the set fails, and
//! the path is recorded so the caller can supply it and render again. This is
//! how the WebAssembly build resolves imports — the host fetches the files.
//!
//! [`dependencies`] answers the same question without rendering: which files
//! does this document need, and which of them have not been supplied yet.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use crate::parser::ast::{Statement, StyleValue, TemplateSourceType};

/// A set of files keyed by normalized relative path (`dir/file.ail`).
pub type Files = BTreeMap<String, Vec<u8>>;

struct Overlay {
    files: Rc<Files>,
    missing: BTreeSet<String>,
}

thread_local! {
    static OVERLAY: RefCell<Option<Overlay>> = const { RefCell::new(None) };
}

/// `a/./b/../c` → `a/c`, separators as `/`. Leading `..` that cannot be
/// collapsed are kept.
pub fn normalize(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.last().is_some_and(|p| p != "..") {
                    parts.pop();
                } else {
                    parts.push("..".into());
                }
            }
            Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
            Component::RootDir => parts.push(String::new()),
            Component::Prefix(p) => parts.push(p.as_os_str().to_string_lossy().into_owned()),
        }
    }
    parts.join("/")
}

/// Run `f` with `files` as the only filesystem. Returns `f`'s result and the
/// paths `f` tried to read that were not in `files`.
pub fn with_files<R>(files: Rc<Files>, f: impl FnOnce() -> R) -> (R, Vec<String>) {
    let previous = OVERLAY.with(|o| o.borrow_mut().replace(Overlay { files, missing: BTreeSet::new() }));
    let result = f();
    let overlay = OVERLAY.with(|o| std::mem::replace(&mut *o.borrow_mut(), previous));
    (result, overlay.map(|o| o.missing.into_iter().collect()).unwrap_or_default())
}

/// Read a file: from the overlay inside [`with_files`], else from disk.
pub fn read(path: &Path) -> io::Result<Vec<u8>> {
    let from_overlay = OVERLAY.with(|o| {
        o.borrow_mut().as_mut().map(|overlay| {
            let key = normalize(path);
            match overlay.files.get(&key) {
                Some(bytes) => Ok(bytes.clone()),
                None => {
                    overlay.missing.insert(key.clone());
                    Err(io::Error::new(io::ErrorKind::NotFound, format!("{} was not supplied", key)))
                }
            }
        })
    });
    from_overlay.unwrap_or_else(|| std::fs::read(path))
}

/// [`read`], as UTF-8 text.
pub fn read_to_string(path: &Path) -> io::Result<String> {
    String::from_utf8(read(path)?).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Every file `source` needs, transitively through the files already in
/// `files` (an imported file's own imports count once it is supplied).
/// Paths are normalized and relative to `base`, the way [`with_files`] looks
/// them up. Returns `(needed, missing)`; `missing` is the part of `needed`
/// not in `files`.
pub fn dependencies(source: &str, base: Option<&Path>, files: &Files) -> (Vec<String>, Vec<String>) {
    let mut needed = BTreeSet::new();
    let mut queue = vec![source.to_string()];
    while let Some(src) = queue.pop() {
        let Ok(doc) = crate::parser::parse(&src) else { continue };
        let mut found = Vec::new();
        collect(&doc.statements, &mut found);
        for (rel, recurse) in found {
            let key = normalize(&base.map(|b| b.join(&rel)).unwrap_or_else(|| PathBuf::from(&rel)));
            if !needed.insert(key.clone()) || !recurse {
                continue;
            }
            if let Some(text) = files.get(&key).and_then(|b| String::from_utf8(b.clone()).ok()) {
                queue.push(text);
            }
        }
    }
    let missing = needed.iter().filter(|k| !files.contains_key(*k)).cloned().collect();
    (needed.into_iter().collect(), missing)
}

/// File references in `stmts`: `(path, is AIL that may reference more)`.
fn collect(stmts: &[crate::parser::ast::Spanned<Statement>], out: &mut Vec<(String, bool)>) {
    for st in stmts {
        match &st.node {
            Statement::Import(p) if !p.node.starts_with("ail:") => out.push((p.node.clone(), true)),
            Statement::TemplateDecl(t) => {
                if let Some(p) = &t.source_path {
                    out.push((p.node.clone(), t.source_type == TemplateSourceType::Ail));
                }
                if let Some(body) = &t.body {
                    collect(body, out);
                }
            }
            Statement::TemplateInstance(i) if i.template_name.node.0 == "code" => {
                for (k, v) in &i.arguments {
                    if k.node.0 == "file" {
                        if let StyleValue::String(s) = &v.node {
                            out.push((s.clone(), false));
                        }
                    }
                }
            }
            Statement::Layout(l) => collect(&l.children, out),
            Statement::Group(g) => collect(&g.children, out),
            Statement::Label(inner) => {
                collect(&[crate::parser::ast::Spanned { node: (**inner).clone(), span: st.span.clone() }], out)
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_collapses_dots() {
        assert_eq!(normalize(Path::new("a/./b/../c.ail")), "a/c.ail");
        assert_eq!(normalize(Path::new("../x")), "../x");
    }

    #[test]
    fn overlay_records_missing_reads() {
        let mut files = Files::new();
        files.insert("lib/a.ail".into(), b"rect a".to_vec());
        let (r, missing) = with_files(Rc::new(files), || {
            let a = read_to_string(Path::new("lib/./a.ail")).unwrap();
            let b = read(Path::new("lib/b.ail"));
            (a, b.is_err())
        });
        assert_eq!(r, ("rect a".to_string(), true));
        assert_eq!(missing, vec!["lib/b.ail".to_string()]);
    }

    #[test]
    fn dependencies_follow_supplied_imports() {
        let src = "import \"deck.ail\"\ntemplate \"icon\" from \"art/icon.svg\"\nrect a";
        let mut files = Files::new();
        let (_, missing) = dependencies(src, None, &files);
        assert_eq!(missing, vec!["art/icon.svg".to_string(), "deck.ail".to_string()]);
        files.insert("deck.ail".into(), b"import \"more.ail\"\nimport \"ail:motion/git\"".to_vec());
        let (needed, missing) = dependencies(src, None, &files);
        assert_eq!(needed, vec!["art/icon.svg", "deck.ail", "more.ail"]);
        assert_eq!(missing, vec!["art/icon.svg", "more.ail"]);
    }
}
