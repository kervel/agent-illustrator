//! Agent Illustrator CLI
//!
//! Usage:
//!   agent-illustrator [OPTIONS] [FILE]
//!
//! Options:
//!   -s, --stylesheet <FILE>  Stylesheet file for color palette (TOML format)
//!   -h, --help               Print help

use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use clap::Parser;

use agent_illustrator::{render_with_config, RenderConfig, Stylesheet};

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
}

fn main() {
    let cli = Cli::parse();

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

    // Render with stylesheet and debug mode
    let config = RenderConfig::new()
        .with_stylesheet(stylesheet)
        .with_debug(cli.debug);
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
