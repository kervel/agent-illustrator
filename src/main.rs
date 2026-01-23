//! Agent Illustrator CLI
//!
//! Usage:
//!   agent-illustrator [FILE]       - Render a .ail file to SVG
//!   agent-illustrator              - Read from stdin
//!   cat file.ail | agent-illustrator

use std::env;
use std::fs;
use std::io::{self, Read};

use agent_illustrator::render;

fn main() {
    let args: Vec<String> = env::args().collect();

    let source = if args.len() > 1 {
        // Read from file
        let path = &args[1];
        match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading file '{}': {}", path, e);
                std::process::exit(1);
            }
        }
    } else {
        // Read from stdin
        let mut buffer = String::new();
        match io::stdin().read_to_string(&mut buffer) {
            Ok(_) => buffer,
            Err(e) => {
                eprintln!("Error reading from stdin: {}", e);
                std::process::exit(1);
            }
        }
    };

    match render(&source) {
        Ok(svg) => {
            println!("{}", svg);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
