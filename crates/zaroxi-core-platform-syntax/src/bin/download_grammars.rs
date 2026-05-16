//! Command-line tool to download and install Tree-sitter grammars.
//!
//! This binary checks which grammars are missing from the runtime directory
//! and downloads/compiles them. It uses the `cc` crate to compile C source
//! files into shared libraries.
//!
//! Usage:
//!   cargo run --bin download_grammars [install|check|list]
//!
//! Commands:
//!   install   Download and compile all missing grammars (default)
//!   check     Check which grammars are installed
//!   list      List all available grammars

use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("install");

    match command {
        "install" => {
            eprintln!("Checking for missing grammars...");
            let installed = zaroxi_lang_syntax::grammar_registry::install_missing_grammars();
            if installed.is_empty() {
                eprintln!("All grammars are already installed.");
            } else {
                eprintln!("Installed {} grammars: {:?}", installed.len(), installed);
            }
        }
        "check" => {
            let registry = zaroxi_lang_syntax::grammar_registry::GrammarRegistry::global();
            let mut missing = Vec::new();
            let mut present = Vec::new();

            for lang_id in registry.language_ids() {
                if zaroxi_lang_syntax::grammar_registry::is_grammar_installed(lang_id) {
                    present.push(lang_id);
                } else {
                    missing.push(lang_id);
                }
            }

            eprintln!("Installed grammars ({}): {:?}", present.len(), present);
            eprintln!("Missing grammars ({}): {:?}", missing.len(), missing);
        }
        "list" => {
            let registry = zaroxi_lang_syntax::grammar_registry::GrammarRegistry::global();
            eprintln!("Available grammars:");
            for lang_id in registry.language_ids() {
                let info = registry.get(lang_id).unwrap();
                let installed = if zaroxi_lang_syntax::grammar_registry::is_grammar_installed(lang_id) {
                    "[installed]"
                } else {
                    "[missing]"
                };
                eprintln!("  {} {} - {} ({})", installed, lang_id, info.name, info.extensions.join(", "));
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Usage: cargo run --bin download_grammars [install|check|list]");
            process::exit(1);
        }
    }
}
