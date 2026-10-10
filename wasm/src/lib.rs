//! WebAssembly bindings for Agent Illustrator.
//!
//! The JS side never touches a filesystem: files the document references
//! (`import`, `template ... from`, `code [file: ...]`, images) are supplied with
//! [`Illustrator::set_file`]. [`Illustrator::dependencies`] lists what is still
//! missing, and every render reports the files it tried to read and could not.
//! The TypeScript wrapper (`ts/index.ts`) builds the fetch-and-retry loop on top.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use agent_illustrator::vfs::{self, Files};
use agent_illustrator::diagnostics::{self, Diagnostic};
use agent_illustrator::{ImageHrefMode, RenderConfig};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct RenderOptions {
    /// CSS layered on top of the default palette (`--stylesheet-css`).
    stylesheet_css: Option<String>,
    /// `"none"` (static), `"js"` (embedded player), `"css"` (pure CSS loop).
    animate: Option<String>,
    no_frame_css: bool,
    frame: Option<String>,
    at: Option<String>,
    crop_to_content: Option<f64>,
    lint: bool,
    debug: bool,
    /// `"base64"` (default: images inlined from supplied files) or `"verbatim"`.
    image_href: Option<String>,
    /// Output the motion timeline / storyboard as text instead of SVG.
    timeline: bool,
    states: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticOut {
    severity: &'static str,
    code: String,
    message: String,
    line: Option<usize>,
    column: Option<usize>,
    end_line: Option<usize>,
    end_column: Option<usize>,
    hint: Option<String>,
    frames: Vec<String>,
    text: String,
}

impl From<Diagnostic> for DiagnosticOut {
    fn from(d: Diagnostic) -> Self {
        DiagnosticOut {
            severity: d.severity.as_str(),
            code: d.code,
            message: d.message,
            line: d.range.map(|r| r.line),
            column: d.range.map(|r| r.column),
            end_line: d.range.map(|r| r.end_line),
            end_column: d.range.map(|r| r.end_column),
            hint: d.hint,
            frames: d.frames,
            text: d.text,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderOutput {
    ok: bool,
    output: Option<String>,
    diagnostics: Vec<DiagnosticOut>,
    missing_files: Vec<String>,
}

#[derive(Serialize)]
struct Dependencies {
    needed: Vec<String>,
    missing: Vec<String>,
}

fn js_err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

/// A renderer plus the in-memory files its documents can reference.
#[wasm_bindgen]
pub struct Illustrator {
    files: Rc<Files>,
    base: Option<PathBuf>,
}

impl Default for Illustrator {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl Illustrator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Illustrator {
        Illustrator { files: Rc::new(BTreeMap::new()), base: None }
    }

    /// Directory the document lives in: its relative references resolve
    /// against it (`""` for the root).
    #[wasm_bindgen(js_name = setBasePath)]
    pub fn set_base_path(&mut self, base: &str) {
        self.base = (!base.is_empty()).then(|| PathBuf::from(base));
    }

    /// Supply a file (path relative to the root, as `dependencies` names it).
    #[wasm_bindgen(js_name = setFile)]
    pub fn set_file(&mut self, path: &str, contents: &[u8]) {
        Rc::make_mut(&mut self.files).insert(vfs::normalize(Path::new(path)), contents.to_vec());
    }

    #[wasm_bindgen(js_name = removeFile)]
    pub fn remove_file(&mut self, path: &str) -> bool {
        Rc::make_mut(&mut self.files).remove(&vfs::normalize(Path::new(path))).is_some()
    }

    #[wasm_bindgen(js_name = clearFiles)]
    pub fn clear_files(&mut self) {
        self.files = Rc::new(BTreeMap::new());
    }

    #[wasm_bindgen(js_name = filePaths)]
    pub fn file_paths(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }

    /// `{ needed, missing }`: every file `source` references (transitively
    /// through supplied files), and those not supplied yet.
    pub fn dependencies(&self, source: &str) -> Result<JsValue, JsError> {
        let (needed, missing) = vfs::dependencies(source, self.base.as_deref(), &self.files);
        serde_wasm_bindgen::to_value(&Dependencies { needed, missing }).map_err(js_err)
    }

    /// Render `source` (and lint it, with `lint: true`). Never throws on a bad
    /// document: the result carries `ok`, `output` (SVG or text, absent when
    /// the render failed), `diagnostics` and `missingFiles`.
    pub fn render(&self, source: &str, options: JsValue) -> Result<JsValue, JsError> {
        let opts: RenderOptions = if options.is_undefined() || options.is_null() {
            RenderOptions::default()
        } else {
            serde_wasm_bindgen::from_value(options).map_err(js_err)?
        };
        let config = self.config(&opts)?;
        let (report, missing_files) = vfs::with_files(self.files.clone(), || diagnostics::check(source, config));
        let out = RenderOutput {
            ok: report.ok(),
            output: report.output,
            diagnostics: report.diagnostics.into_iter().map(DiagnosticOut::from).collect(),
            missing_files,
        };
        serde_wasm_bindgen::to_value(&out).map_err(js_err)
    }

    /// Keyframe names, in order.
    #[wasm_bindgen(js_name = frameNames)]
    pub fn frame_names(&self, source: &str) -> Result<Vec<String>, JsError> {
        let files = self.files.clone();
        vfs::with_files(files, || agent_illustrator::frame_names(source)).0.map_err(js_err)
    }

    fn config(&self, o: &RenderOptions) -> Result<RenderConfig, JsError> {
        let mut c = RenderConfig::new().with_debug(o.debug).with_lint(o.lint);
        c.image_href_mode = match o.image_href.as_deref() {
            None | Some("base64") => ImageHrefMode::Base64,
            Some("verbatim") => ImageHrefMode::Verbatim,
            Some(other) => return Err(JsError::new(&format!("imageHref: unknown mode '{}'", other))),
        };
        match o.animate.as_deref() {
            None | Some("none") => {}
            Some("js") => c.animate = true,
            Some("css") => c.animate_css = true,
            Some(other) => return Err(JsError::new(&format!("animate: expected none, js or css, got '{}'", other))),
        }
        c.no_frame_css = o.no_frame_css;
        c.frame = o.frame.clone();
        c.at = o.at.clone();
        c.crop = o.crop_to_content;
        c.timeline = o.timeline;
        c.states = o.states;
        c.custom_css = o.stylesheet_css.clone().filter(|s| !s.trim().is_empty());
        if let Some(b) = &self.base {
            c = c.with_template_base_path(b.clone());
        }
        Ok(c)
    }
}

thread_local! {
    static LAST_PANIC: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Keep the panic message: a panic traps the call, and the JS side asks for
/// the message with `takePanic` to report something better than "unreachable".
#[wasm_bindgen(start)]
fn start() {
    std::panic::set_hook(Box::new(|info| {
        LAST_PANIC.with(|p| *p.borrow_mut() = Some(info.to_string()));
    }));
}

/// The message of the last panic, once.
#[wasm_bindgen(js_name = takePanic)]
pub fn take_panic() -> Option<String> {
    LAST_PANIC.with(|p| p.borrow_mut().take())
}

/// The library version (release tag, or `git describe` for a local build).
#[wasm_bindgen]
pub fn version() -> String {
    agent_illustrator::VERSION.to_string()
}

/// The language reference: `"skill"`, `"brief"`, `"grammar"`, `"animation"`,
/// `"styling"`, or a `--doc` topic.
#[wasm_bindgen]
pub fn docs(topic: &str) -> Option<String> {
    use agent_illustrator::docs;
    match topic {
        "skill" => Some(docs::SKILL.to_string()),
        "brief" => Some(docs::skill_brief()),
        "grammar" => Some(docs::GRAMMAR.to_string()),
        "animation" => Some(docs::SKILL_ANIMATION.to_string()),
        "styling" => Some(docs::SKILL_STYLING.to_string()),
        _ => docs::section(topic),
    }
}

/// Topics `docs` accepts besides the whole documents.
#[wasm_bindgen(js_name = docTopics)]
pub fn doc_topics() -> Vec<String> {
    agent_illustrator::docs::topics().into_iter().map(String::from).collect()
}

/// The standalone player script (`--player-js`), for SVGs rendered with
/// `noFrameCss` driven by a host page.
#[wasm_bindgen(js_name = playerJs)]
pub fn player_js() -> String {
    agent_illustrator::motion::render::PLAYER_JS.to_string()
}
