//! Agent Illustrator CLI
//!
//! Usage:
//!   agent-illustrator [OPTIONS] [FILE]
//!
//! Options:
//!   -s, --stylesheet <FILE>  Stylesheet file for color palette (TOML format)
//!   -g, --grammar            Show language grammar reference
//!   -e, --examples           Show annotated examples
//!   --skill                  Output LLM-optimized skill document
//!   -h, --help               Print help

use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

use clap::Parser;

mod serve;

use agent_illustrator::{
    frame_names, render_with_config, render_with_lint, ImageHrefMode, LintCategory, RenderConfig,
    Stylesheet,
};

/// `-q`: progress and notices are not printed.
static QUIET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn quiet() -> bool {
    QUIET.load(std::sync::atomic::Ordering::Relaxed)
}

/// Render every keyframe into `dir` as `<NN>-<name>.svg`.
///
/// One command instead of a shell loop over hand-copied frame names, so
/// checking that every frame renders correctly stays a single step.
/// Rasterise to PNG (notes on fonts go to stderr).
fn to_png(svg: &str, scale: f32) -> Vec<u8> {
    match agent_illustrator::raster::svg_to_png(svg, scale) {
        Ok(r) => {
            for n in r.notes.iter().filter(|_| !quiet()) {
                eprintln!("png: {}", n);
            }
            r.png
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// The SVG, or with `--png` the picture; to `-o FILE`, or stdout.
fn emit(svg: &str, png: bool, output: &Option<PathBuf>, scale: f32) {
    let bytes = if png { to_png(svg, scale) } else { svg.as_bytes().to_vec() };
    match output {
        Some(path) => {
            if let Err(e) = fs::write(path, bytes) {
                eprintln!("Error writing '{}': {}", path.display(), e);
                std::process::exit(1);
            }
        }
        None => {
            use std::io::Write;
            let _ = std::io::stdout().write_all(&bytes);
        }
    }
}

fn write_all_frames(source: &str, config: RenderConfig, dir: &std::path::Path, png: Option<f32>) {
    let names = match frame_names(source) {
        Ok(names) => names,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    if names.is_empty() {
        eprintln!("Error: --frames-to-dir requires keyframes in the input");
        std::process::exit(1);
    }
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!("Error creating directory '{}': {}", dir.display(), e);
        std::process::exit(1);
    }

    for (i, name) in names.iter().enumerate() {
        // Select by index: a frame named "2" would otherwise be ambiguous.
        let mut frame_config = config.clone();
        frame_config.frame = Some(i.to_string());

        let svg = match render_with_config(source, frame_config) {
            Ok(svg) => svg,
            Err(e) => {
                eprintln!("Error rendering frame '{}': {}", name, e);
                std::process::exit(1);
            }
        };

        let (path, bytes) = match png {
            Some(scale) => (dir.join(format!("{:02}-{}.png", i, sanitize_filename(name))), to_png(&svg, scale)),
            None => (dir.join(format!("{:02}-{}.svg", i, sanitize_filename(name))), svg.into_bytes()),
        };
        if let Err(e) = fs::write(&path, bytes) {
            eprintln!("Error writing '{}': {}", path.display(), e);
            std::process::exit(1);
        }
        if !quiet() {
            println!("{}", path.display());
        }
    }
}

/// Make a frame name safe to use as a filename.
fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "frame".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Resolve category names from the CLI, exiting with a helpful message on typos.
fn parse_lint_categories(names: &[String]) -> Vec<LintCategory> {
    names
        .iter()
        .map(|name| {
            LintCategory::parse(name.trim()).unwrap_or_else(|| {
                eprintln!(
                    "Error: unknown lint category '{}'. Valid categories: {}",
                    name,
                    LintCategory::all_names()
                );
                std::process::exit(2);
            })
        })
        .collect()
}

#[derive(Parser)]
#[command(name = "agent-illustrator")]
#[command(about = "Declarative illustration language for AI agents")]
#[command(version = env!("AI_VERSION"))]
struct Cli {
    /// Input file (reads from stdin if not provided)
    input: Option<PathBuf>,

    /// [Deprecated: use --stylesheet-css] TOML color palette file
    #[arg(short, long)]
    stylesheet: Option<PathBuf>,

    /// CSS file to inject into the SVG <style> block. Repeat to layer them
    /// in order (a theme, then deck overrides): later files win.
    #[arg(long)]
    stylesheet_css: Vec<PathBuf>,

    /// Debug mode: show container bounds and element IDs
    #[arg(short, long)]
    debug: bool,

    /// Trace mode: show internal constraint solver and routing debug output
    #[arg(short, long)]
    trace: bool,

    /// Show language grammar reference
    #[arg(short, long)]
    grammar: bool,

    /// Show annotated examples
    #[arg(short, long)]
    examples: bool,

    /// Output LLM-optimized skill document for agent integration
    #[arg(long)]
    skill: bool,

    /// Output animation sub-skill document
    #[arg(long)]
    skill_animation: bool,

    /// The one-page version for agents with little context: the loop, a
    /// whole scene, the idioms, and the topics `--doc` can fetch
    #[arg(long)]
    skill_brief: bool,

    /// Print one section of the guides (e.g. `--doc tables`, `--doc
    /// accent`); an unknown topic lists them all
    #[arg(long, value_name = "TOPIC")]
    doc: Option<String>,

    /// Output clipart search sub-skill document
    #[arg(long)]
    skill_find_clipart: bool,

    /// Output styling and CSS sub-skill document
    #[arg(long)]
    skill_styling: bool,

    /// Lint mode: check for layout defects (overlaps, containment violations, etc.)
    #[arg(long)]
    lint: bool,

    /// With --lint: also fail (exit 1) on warnings, not only on errors
    #[arg(long)]
    lint_strict: bool,

    /// Only report these lint categories (comma-separated, repeatable).
    /// Run --lint-categories to list the valid names.
    #[arg(long, value_delimiter = ',')]
    lint_category: Vec<String>,

    /// Suppress these lint categories (comma-separated, repeatable)
    #[arg(long, value_delimiter = ',')]
    lint_exclude: Vec<String>,

    /// List the lint category names and exit
    #[arg(long)]
    lint_categories: bool,

    /// How raster image paths (from "template X from file.png") appear in SVG output.
    /// Use 'base64' to embed images directly in the SVG for fully self-contained output.
    /// Use 'verbatim' (default) to keep paths as written in the AIL source.
    #[arg(long, value_enum, default_value_t = ImageHrefArg::Verbatim)]
    image_href: ImageHrefArg,

    /// Render a single keyframe as a static SVG (by index or name)
    #[arg(long)]
    frame: Option<String>,

    /// With --frame: render a mid-motion still this far into that frame,
    /// sampled from the same tracks the browser player uses
    /// (`0.35s`, `350ms`, or `50%` of the frame's duration)
    #[arg(long, value_name = "T")]
    at: Option<String>,

    /// Render a contact sheet of one keyframe (by index or name), sampled at
    /// 0/25/50/75/100% of its duration
    #[arg(long, value_name = "FRAME")]
    frames_strip: Option<String>,

    /// Print the compiled motion as a readable table (per frame: start,
    /// duration, ease, statement, target, and each property's from -> to)
    #[arg(long)]
    timeline: bool,

    /// Print the storyboard: per click step, which elements are visible and
    /// what changes (appears, disappears, properties, moves, draws), as the
    /// file declares it. Check it against your STORYBOARD comment.
    #[arg(long)]
    states: bool,

    /// Print the compiled motion manifest (tracks) as JSON
    #[arg(long)]
    timeline_json: bool,

    /// Print the motion player JavaScript (for hosts embedding inline SVG)
    #[arg(long)]
    player_js: bool,

    /// Render every keyframe as a static SVG into this directory,
    /// as <NN>-<name>.svg. The directory is created if needed.
    #[arg(long, value_name = "DIR")]
    frames_to_dir: Option<PathBuf>,

    /// Write a PNG instead of the SVG (to -o FILE, or stdout). Works with
    /// --frame (and --at), --frames-strip and --frames-to-dir (one PNG per
    /// frame). Uses the fonts bundled in the binary and any @font-face the
    /// stylesheet embeds as a data URI; no browser needed.
    #[arg(long)]
    png: bool,

    /// Quiet: no progress or notices (the files --frames-to-dir writes, font
    /// notes for --png). Errors and the output you asked for still appear.
    #[arg(short = 'q', long)]
    quiet: bool,

    /// Write the output to FILE instead of stdout
    #[arg(short = 'o', long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Zoom for --png (2 = twice the size, for checking details)
    #[arg(long, default_value_t = 1.0)]
    scale: f32,

    /// List the keyframe names in the input and exit
    #[arg(long)]
    list_frames: bool,

    /// List the click steps and exit: one line per step, its index and its
    /// frames (the frame a click plays, then any `[auto]` frames that follow
    /// by themselves). A host needs one fragment per step.
    #[arg(long)]
    list_steps: bool,

    /// Make the picture the content, not the stage: the union of what every
    /// frame shows, plus this margin (default 24). One box for all frames, so
    /// nothing jumps; for embedding under a host's own header.
    #[arg(long, value_name = "PAD", num_args = 0..=1, default_missing_value = "24")]
    crop_to_content: Option<f64>,

    /// Live preview in the browser: the real player, arrow keys step, a
    /// scrubber for the current frame, reload on save (the file, its folder,
    /// the stylesheets). Default port 8420.
    #[arg(long, value_name = "PORT", num_args = 0..=1, default_missing_value = "8420")]
    serve: Option<u16>,

    /// Write an HTML page showing click-step STEP (its frame and the [auto]
    /// frames after it) sampled every --film-every seconds by the browser
    /// player: a film strip for spotting flicker and jumps.
    #[arg(long, value_name = "STEP")]
    film: Option<usize>,

    /// Sampling interval for --film, in seconds.
    #[arg(long, value_name = "SECONDS", default_value = "0.05")]
    film_every: f64,

    /// Embed minimal JS for self-contained animated playback
    #[arg(long)]
    animate: bool,

    /// The whole choreography as a self-playing pure-CSS loop (no JS: plays
    /// inside an <img>, e.g. a GitLab/GitHub README)
    #[arg(long)]
    animate_css: bool,

    /// Skip the auto-generated `.frame-X` CSS rules. Element/connection
    /// `kf-*` / `conn-*` classes and `data-frames` are still emitted, so an
    /// external runtime (e.g. reveal.js) can drive frame transitions by
    /// toggling a `frame-<name>` class on the SVG root.
    #[arg(long)]
    no_frame_css: bool,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum ImageHrefArg {
    /// Keep the image path exactly as written in the AIL source (e.g. "../assets/logo.png")
    Verbatim,
    /// Normalize the path relative to the current working directory, removing ".." segments
    Rewrite,
    /// Resolve to a fully qualified absolute filesystem path
    Absolute,
    /// Read the image file and embed its contents directly in the SVG as a data URI, making the SVG fully self-contained with no external file dependencies
    Base64,
}

impl From<ImageHrefArg> for ImageHrefMode {
    fn from(arg: ImageHrefArg) -> Self {
        match arg {
            ImageHrefArg::Verbatim => ImageHrefMode::Verbatim,
            ImageHrefArg::Rewrite => ImageHrefMode::Rewrite,
            ImageHrefArg::Absolute => ImageHrefMode::Absolute,
            ImageHrefArg::Base64 => ImageHrefMode::Base64,
        }
    }
}

fn main() {
    let cli = Cli::parse();
    QUIET.store(cli.quiet, std::sync::atomic::Ordering::Relaxed);

    // Handle documentation flags first
    if cli.grammar {
        print_grammar();
        return;
    }

    if cli.examples {
        print_examples();
        return;
    }

    if cli.skill {
        print_skill();
        return;
    }

    if cli.skill_brief {
        print!("{}", agent_illustrator::docs::skill_brief());
        return;
    }

    if let Some(topic) = &cli.doc {
        match agent_illustrator::docs::section(topic) {
            Some(s) => print!("{}", s),
            None => {
                let near = agent_illustrator::docs::suggest(topic);
                if !near.is_empty() {
                    eprintln!("no section '{}'. Did you mean: {}?", topic, near.join(", "));
                }
                eprintln!("Topics: {}", agent_illustrator::docs::topics().join(", "));
                std::process::exit(2);
            }
        }
        return;
    }

    if cli.skill_animation {
        print_skill_animation();
        return;
    }

    if cli.skill_find_clipart {
        print_skill_find_clipart();
        return;
    }

    if cli.skill_styling {
        print_skill_styling();
        return;
    }

    if cli.player_js {
        print!("{}", agent_illustrator::motion::render::PLAYER_JS);
        return;
    }

    if cli.lint_categories {
        for category in LintCategory::ALL {
            println!("{}", category);
        }
        return;
    }

    let include_categories = parse_lint_categories(&cli.lint_category);
    let exclude_categories = parse_lint_categories(&cli.lint_exclude);

    // If no input file and stdin is a terminal (interactive), show intro help
    if cli.input.is_none() && io::stdin().is_terminal() {
        print_intro();
        return;
    }

    // Load stylesheet
    // When --stylesheet-css is provided without --stylesheet, use an empty TOML
    // stylesheet so the CSS file is the sole source of styling variables.
    if cli.stylesheet.is_some() {
        eprintln!("warning: --stylesheet is deprecated, use --stylesheet-css instead");
    }
    let stylesheet = match &cli.stylesheet {
        Some(path) => match Stylesheet::from_file(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error loading stylesheet '{}': {}", path.display(), e);
                std::process::exit(1);
            }
        },
        None => {
            // Always use default palette for CSS variable definitions.
            // --stylesheet-css adds custom CSS rules on top, not replacements.
            Stylesheet::default()
        }
    };

    // Read input
    let source = match &cli.input {
        Some(path) => match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading file '{}': {}", path.display(), e);
                std::process::exit(1);
            }
        },
        None => {
            let mut buffer = String::new();
            match io::stdin().read_to_string(&mut buffer) {
                Ok(_) => buffer,
                Err(e) => {
                    eprintln!("Error reading from stdin: {}", e);
                    std::process::exit(1);
                }
            }
        }
    };

    // Load custom CSS
    // Several stylesheets layer in order (later wins).
    let custom_css = match serve::load_css(&cli.stylesheet_css) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // Render with stylesheet, debug mode, and trace mode
    let mut config = RenderConfig::new()
        .with_stylesheet(stylesheet)
        .with_debug(cli.debug)
        .with_trace(cli.trace)
        .with_lint(cli.lint || cli.lint_strict)
        .with_image_href_mode(cli.image_href.into());
    config.frame = cli.frame.clone();
    config.animate = cli.animate;
    config.animate_css = cli.animate_css;
    config.no_frame_css = cli.no_frame_css;
    config.at = cli.at.clone();
    config.crop = cli.crop_to_content;
    config.timeline = cli.timeline;
    config.states = cli.states;
    config.timeline_json = cli.timeline_json;
    if let Some(css) = custom_css {
        config = config.with_custom_css(css);
    }
    // Set template base path to input file's directory for relative imports
    if let Some(path) = &cli.input {
        if let Some(parent) = path.parent() {
            config = config.with_template_base_path(parent.to_path_buf());
        }
    }

    if let Some(port) = cli.serve {
        let Some(input) = cli.input.clone() else {
            eprintln!("Error: --serve needs an input file (it watches it for changes)");
            std::process::exit(2);
        };
        if let Err(e) = serve::serve(input, cli.stylesheet_css.clone(), config, port) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    if let Some(step) = cli.film {
        let mut c = config;
        c.animate = true;
        match render_with_config(&source, c) {
            Ok(svg) => print!("{}", serve::film(&svg, step, cli.film_every.max(0.01))),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if cli.list_steps {
        match agent_illustrator::step_frames(&source) {
            Ok(steps) if steps.is_empty() => eprintln!("no keyframes in input"),
            Ok(steps) => {
                for (i, frames) in steps.iter().enumerate() {
                    println!("{}\t{}", i, frames.join(" "));
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if cli.list_frames {
        match frame_names(&source) {
            Ok(names) if names.is_empty() => eprintln!("no keyframes in input"),
            Ok(names) => {
                for (i, name) in names.iter().enumerate() {
                    println!("{}\t{}", i, name);
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if let Some(frame) = &cli.frames_strip {
        match agent_illustrator::render_frames_strip(&source, config, frame) {
            Ok(svg) => emit(&svg, cli.png, &cli.output, cli.scale),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if let Some(dir) = &cli.frames_to_dir {
        if cli.frame.is_some() || cli.animate || cli.animate_css {
            eprintln!("Error: --frames-to-dir cannot be combined with --frame or --animate");
            std::process::exit(2);
        }
        write_all_frames(&source, config, dir, cli.png.then_some(cli.scale));
        return;
    }

    let lint = cli.lint || cli.lint_strict;
    if lint {
        match render_with_lint(&source, config) {
            Ok((_svg, lint_warnings)) => {
                // Lint is a report: the SVG is not printed, so `--lint` can
                // gate a build without anyone redirecting stdout.
                let total = lint_warnings.len();
                let shown: Vec<_> = lint_warnings
                    .iter()
                    .filter(|w| {
                        (include_categories.is_empty() || include_categories.contains(&w.category))
                            && !exclude_categories.contains(&w.category)
                    })
                    .collect();
                if shown.is_empty() {
                    if total == 0 {
                        eprintln!("lint: clean");
                    } else {
                        eprintln!("lint: clean in selected categories ({} filtered out)", total);
                    }
                } else {
                    for w in &shown {
                        eprintln!(
                            "lint: {}{}: {}",
                            w.category,
                            w.frame_suffix(),
                            w.message
                        );
                    }
                    let hidden = total - shown.len();
                    let errors = shown.iter().filter(|w| w.category.is_error()).count();
                    let warnings = shown.len() - errors;
                    let filtered = if hidden > 0 { format!(", {} filtered out", hidden) } else { String::new() };
                    eprintln!("lint: {} error(s), {} warning(s){}", errors, warnings, filtered);
                    // Errors fail the build; warnings only with --lint-strict.
                    if errors > 0 || cli.lint_strict {
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        match render_with_config(&source, config) {
            Ok(svg) => {
                if cli.png || cli.output.is_some() {
                    emit(&svg, cli.png, &cli.output, cli.scale);
                } else {
                    println!("{}", svg);
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn print_intro() {
    println!(
        r#"Agent Illustrator - Declarative illustration language for AI agents

USAGE:
    agent-illustrator [OPTIONS] [FILE]
    echo '<code>' | agent-illustrator

OPTIONS:
    -g, --grammar      Show language grammar reference
    -e, --examples     Show annotated examples
    --skill            Output LLM skill document (for embedding in agent context)
    --skill-brief      The one-page version (little context); --doc TOPIC for one section
    --stylesheet-css   CSS stylesheet for colors and visual styling
    -s, --stylesheet   [Deprecated] TOML color palette
    -d, --debug        Show element bounds and IDs
    --lint             Report layout defects (--lint-categories lists the kinds)
    --frames-to-dir    Render every keyframe into a directory
    -h, --help         Print help

QUICK START:
    echo 'row {{ rect a  rect b }}  a -> b' | agent-illustrator > output.svg

This creates two rectangles in a row with a connecting arrow.
Run --grammar for syntax reference or --examples for more patterns."#
    );
}

fn print_grammar() {
    print!("{}", include_str!("../docs/grammar.md"));
}

fn print_examples() {
    print!("{}", include_str!("../docs/examples.md"));
}

fn print_skill() {
    print!("{}", include_str!("../docs/skill.md"));
}

fn print_skill_animation() {
    print!("{}", include_str!("../docs/skill-animation.md"));
    // The built-in library ships in the binary; show it, contract and all.
    print!(
        "\n## Appendix: the built-in `ail:motion/git` library\n\n```\n{}```\n",
        include_str!("motion/lib/git.ail")
    );
}

fn print_skill_find_clipart() {
    print!("{}", include_str!("../docs/skill-find-clipart.md"));
}

fn print_skill_styling() {
    print!("{}", include_str!("../docs/skill-styling.md"));
}
