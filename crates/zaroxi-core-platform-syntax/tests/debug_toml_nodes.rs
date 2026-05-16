use tree_sitter::Parser;
use zaroxi_lang_syntax::language::LanguageId;

fn print_node_types(
    node: &tree_sitter::Node,
    source: &str,
    depth: usize,
    seen: &mut std::collections::HashSet<String>,
) {
    let kind = node.kind();
    seen.insert(kind.to_string());

    let start = node.start_byte();
    let end = node.end_byte();
    let text = &source[start..end];
    println!("{}{}: '{}' ({}..{})", "  ".repeat(depth), kind, text, start, end);

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_node_types(&child, source, depth + 1, seen);
    }
}

#[test]
fn debug_toml_nodes() {
    let source = r#"
[package]
name = "neote"
version = "0.1.0"
edition = "2024"

[dependencies]
iced = { version = "0.13", features = ["multi-window"] }
tree-sitter = "0.21"

[profile.release]
opt-level = 3
lto = true

# This is a comment
boolean_value = true
integer_value = 42
float_value = 3.14
string_value = "hello"
"#;

    let mut parser = Parser::new();
    let language =
        LanguageId::Toml.tree_sitter_language().expect("TOML language should be available");
    parser.set_language(&language).expect("Failed to set TOML language");

    let tree = parser.parse(source, None).expect("Failed to parse TOML");
    let root_node = tree.root_node();

    println!("Root node type: {}", root_node.kind());
    let mut seen_types = std::collections::HashSet::new();
    print_node_types(&root_node, source, 0, &mut seen_types);
    println!("\nUnique node types found:");
    let mut types: Vec<_> = seen_types.into_iter().collect();
    types.sort();
    for t in types {
        println!("  {}", t);
    }
}
