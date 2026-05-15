use std::env;
use zaroxi_lang_syntax::grammar_builder;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: build-grammar <language-id>");
        std::process::exit(1);
    }

    let language_id = &args[1];
    println!("Building grammar for {}...", language_id);

    match grammar_builder::build_and_install_grammar(language_id) {
        Ok(_) => {
            println!("Successfully built grammar for {}", language_id);
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Failed to build grammar: {}", e);
            std::process::exit(1);
        }
    }
}
