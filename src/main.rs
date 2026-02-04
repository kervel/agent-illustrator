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

use agent_illustrator::{render_with_config, render_with_lint, RenderConfig, Stylesheet};

#[derive(Parser)]
#[command(name = "agent-illustrator")]
#[command(about = "Declarative illustration language for AI agents")]
struct Cli {
    /// Input file (reads from stdin if not provided)
    input: Option<PathBuf>,

    /// Stylesheet file for color palette (TOML format)
    #[arg(short, long)]
    stylesheet: Option<PathBuf>,

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

    /// Lint mode: check for layout defects (overlaps, containment violations, etc.)
    #[arg(long)]
    lint: bool,
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

    // If no input file and stdin is a terminal (interactive), show intro help
    if cli.input.is_none() && io::stdin().is_terminal() {
        print_intro();
        return;
    }

    // Load stylesheet
    let stylesheet = match &cli.stylesheet {
        Some(path) => match Stylesheet::from_file(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error loading stylesheet '{}': {}", path.display(), e);
                std::process::exit(1);
            }
        },
        None => Stylesheet::default(),
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

    // Render with stylesheet, debug mode, and trace mode
    let config = RenderConfig::new()
        .with_stylesheet(stylesheet)
        .with_debug(cli.debug)
        .with_trace(cli.trace)
        .with_lint(cli.lint);

    if cli.lint {
        match render_with_lint(&source, config) {
            Ok((svg, lint_warnings)) => {
                println!("{}", svg);
                if lint_warnings.is_empty() {
                    eprintln!("lint: clean");
                } else {
                    for w in &lint_warnings {
                        eprintln!("lint: {}: {}", w.category, w.message);
                    }
                    eprintln!("lint: {} warning(s)", lint_warnings.len());
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
    -s, --stylesheet   Custom color palette (TOML file)
    -d, --debug        Show element bounds and IDs
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
