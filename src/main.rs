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

use agent_illustrator::{
    frame_names, render_with_config, render_with_lint, ImageHrefMode, LintCategory, RenderConfig,
    Stylesheet,
};

/// Render every keyframe into `dir` as `<NN>-<name>.svg`.
///
/// One command instead of a shell loop over hand-copied frame names, so
/// checking that every frame renders correctly stays a single step.
fn write_all_frames(source: &str, config: RenderConfig, dir: &std::path::Path) {
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

        let path = dir.join(format!("{:02}-{}.svg", i, sanitize_filename(name)));
        if let Err(e) = fs::write(&path, svg) {
            eprintln!("Error writing '{}': {}", path.display(), e);
            std::process::exit(1);
        }
        println!("{}", path.display());
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
#[command(version)]
struct Cli {
    /// Input file (reads from stdin if not provided)
    input: Option<PathBuf>,

    /// [Deprecated: use --stylesheet-css] TOML color palette file
    #[arg(short, long)]
    stylesheet: Option<PathBuf>,

    /// CSS file to inject into the SVG <style> block
    #[arg(long)]
    stylesheet_css: Option<PathBuf>,

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

    /// Output clipart search sub-skill document
    #[arg(long)]
    skill_find_clipart: bool,

    /// Output styling and CSS sub-skill document
    #[arg(long)]
    skill_styling: bool,

    /// Lint mode: check for layout defects (overlaps, containment violations, etc.)
    #[arg(long)]
    lint: bool,

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

    /// Render every keyframe as a static SVG into this directory,
    /// as <NN>-<name>.svg. The directory is created if needed.
    #[arg(long, value_name = "DIR")]
    frames_to_dir: Option<PathBuf>,

    /// List the keyframe names in the input and exit
    #[arg(long)]
    list_frames: bool,

    /// Embed minimal JS for self-contained animated playback
    #[arg(long)]
    animate: bool,

    /// Use pure CSS animation (no JS, works in GitLab/GitHub READMEs)
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
    let custom_css = match &cli.stylesheet_css {
        Some(path) => match fs::read_to_string(path) {
            Ok(css) => Some(css),
            Err(e) => {
                eprintln!("Error loading CSS '{}': {}", path.display(), e);
                std::process::exit(1);
            }
        },
        None => None,
    };

    // Render with stylesheet, debug mode, and trace mode
    let mut config = RenderConfig::new()
        .with_stylesheet(stylesheet)
        .with_debug(cli.debug)
        .with_trace(cli.trace)
        .with_lint(cli.lint)
        .with_image_href_mode(cli.image_href.into());
    config.frame = cli.frame.clone();
    config.animate = cli.animate;
    config.animate_css = cli.animate_css;
    config.no_frame_css = cli.no_frame_css;
    if let Some(css) = custom_css {
        config = config.with_custom_css(css);
    }
    // Set template base path to input file's directory for relative imports
    if let Some(path) = &cli.input {
        if let Some(parent) = path.parent() {
            config = config.with_template_base_path(parent.to_path_buf());
        }
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

    if let Some(dir) = &cli.frames_to_dir {
        if cli.frame.is_some() || cli.animate || cli.animate_css {
            eprintln!("Error: --frames-to-dir cannot be combined with --frame or --animate");
            std::process::exit(2);
        }
        write_all_frames(&source, config, dir);
        return;
    }

    if cli.lint {
        match render_with_lint(&source, config) {
            Ok((svg, lint_warnings)) => {
                println!("{}", svg);
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
                    if hidden > 0 {
                        eprintln!(
                            "lint: {} warning(s), {} filtered out",
                            shown.len(),
                            hidden
                        );
                    } else {
                        eprintln!("lint: {} warning(s)", shown.len());
                    }
                    std::process::exit(1);
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
                println!("{}", svg);
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
}

fn print_skill_find_clipart() {
    print!("{}", include_str!("../docs/skill-find-clipart.md"));
}

fn print_skill_styling() {
    print!("{}", include_str!("../docs/skill-styling.md"));
}
