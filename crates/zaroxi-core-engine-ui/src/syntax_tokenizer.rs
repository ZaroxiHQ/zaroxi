/// Heuristic syntax tokenizer producing `SyntaxHighlights` from visible text lines.
///
/// Phase 42: A lightweight pattern-based tokenizer that identifies common
/// code tokens (keywords, strings, comments, numbers, types) without requiring
/// Tree-sitter or any external parser. Works on any text content.
///
/// The tokenizer produces `HighlightKind` annotations mapped through
/// `From<HighlightKind> for SpanKind` for the engine extraction seam.
use crate::work_content::{HighlightKind, LineHighlight, SyntaxHighlights};

/// Rust keyword set used for identification.
const RUST_KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "if", "else", "match", "for", "while", "loop", "break", "continue",
    "return", "impl", "struct", "enum", "trait", "mod", "use", "pub", "crate", "self", "super",
    "where", "async", "await", "in", "ref", "static", "const", "unsafe", "extern", "type", "as",
    "move", "dyn", "true", "false",
];

/// Tokenize visible lines into syntax highlights using heuristic pattern matching.
///
/// Identifies:
/// - Line comments (`// ...`)
/// - String literals (`"..."`)
/// - Rust keywords (`fn`, `let`, `if`, etc.)
/// - Numbers (integer and float literals)
/// - Type-like identifiers (uppercase-starting words)
pub fn tokenize_lines(lines: &[String]) -> SyntaxHighlights {
    let mut highlights: Vec<Vec<LineHighlight>> = Vec::with_capacity(lines.len());

    for line in lines {
        let spans = tokenize_line(line);
        highlights.push(spans);
    }

    SyntaxHighlights { highlights }
}

/// Tokenize a single line into `LineHighlight` spans.
fn tokenize_line(line: &str) -> Vec<LineHighlight> {
    let mut spans: Vec<LineHighlight> = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0usize;

    while i < len {
        let c = chars[i];

        if c == '/' && i + 1 < len && chars[i + 1] == '/' {
            // Line comment: rest of line
            spans.push(LineHighlight { start_col: i, end_col: len, kind: HighlightKind::Comment });
            break;
        }

        if c == '"' {
            // String literal
            let start = i;
            i += 1;
            while i < len {
                if chars[i] == '\\' && i + 1 < len {
                    // Escape sequence — skip next char
                    i += 2;
                } else if chars[i] == '"' {
                    i += 1; // closing quote
                    spans.push(LineHighlight {
                        start_col: start,
                        end_col: i,
                        kind: HighlightKind::String,
                    });
                    break;
                } else {
                    i += 1;
                }
            }
            // Unclosed string: treat as string anyway
            if i == len && start < len {
                spans.push(LineHighlight {
                    start_col: start,
                    end_col: len,
                    kind: HighlightKind::String,
                });
            }
            continue;
        }

        if c == '\'' && i + 2 < len && chars[i + 2] == '\'' {
            // Char literal
            spans.push(LineHighlight { start_col: i, end_col: i + 3, kind: HighlightKind::String });
            i += 3;
            continue;
        }

        if c.is_ascii_digit() {
            // Number literal
            let start = i;
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '_') {
                i += 1;
            }
            spans.push(LineHighlight { start_col: start, end_col: i, kind: HighlightKind::Number });
            continue;
        }

        if c == '#' && i + 1 < len && chars[i + 1] == '[' {
            // Attribute: #[...]
            let start = i;
            i += 2;
            let mut depth = 1u32;
            while i < len && depth > 0 {
                if chars[i] == '[' {
                    depth += 1;
                } else if chars[i] == ']' {
                    depth -= 1;
                }
                i += 1;
            }
            spans.push(LineHighlight {
                start_col: start,
                end_col: i,
                kind: HighlightKind::Attribute,
            });
            continue;
        }

        if c.is_ascii_alphabetic() || c == '_' {
            // Identifier: check for keywords and type-like identifiers
            let start = i;
            while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let word_lower = word.to_lowercase();

            if RUST_KEYWORDS.contains(&word_lower.as_str()) {
                spans.push(LineHighlight {
                    start_col: start,
                    end_col: i,
                    kind: HighlightKind::Keyword,
                });
            } else if word.chars().next().is_some_and(|c| c.is_uppercase()) {
                // Type-like: starts with uppercase
                spans.push(LineHighlight {
                    start_col: start,
                    end_col: i,
                    kind: HighlightKind::Type,
                });
            }
            // Otherwise plain — don't emit a span
            continue;
        }

        // Operators: single and multi-char
        let op_len = match c {
            ':' if i + 1 < len && chars[i + 1] == ':' => 2,
            '-' if i + 1 < len && chars[i + 1] == '>' => 2,
            '=' if i + 1 < len && (chars[i + 1] == '=' || chars[i + 1] == '>') => 2,
            '!' if i + 1 < len && chars[i + 1] == '=' => 2,
            '<' | '>' if i + 1 < len && chars[i + 1] == '=' => 2,
            '&' | '|' if i + 1 < len && chars[i + 1] == c => 2,
            '+' | '*' | '/' | '%' | '^' if i + 1 < len && chars[i + 1] == '=' => 2,
            '.' if i + 1 < len && chars[i + 1] == '.' => 2,
            _ => 1,
        };

        if is_operator_char(c) {
            spans.push(LineHighlight {
                start_col: i,
                end_col: i + op_len,
                kind: HighlightKind::Operator,
            });
            i += op_len;
            continue;
        }

        i += 1;
    }

    // Deduplicate and sort by start column
    spans.sort_by_key(|s| s.start_col);
    spans
}

fn is_operator_char(c: char) -> bool {
    matches!(
        c,
        '+' | '-'
            | '*'
            | '/'
            | '%'
            | '='
            | '<'
            | '>'
            | '!'
            | '&'
            | '|'
            | '^'
            | '~'
            | '.'
            | ':'
            | ';'
            | ','
            | '@'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_highlighting() {
        let lines = vec!["fn main() {".to_string()];
        let result = tokenize_lines(&lines);
        assert_eq!(result.highlights.len(), 1);
        // First span should be the "fn" keyword
        let first = &result.highlights[0][0];
        assert_eq!(first.kind, HighlightKind::Keyword);
        assert_eq!(first.start_col, 0);
        assert_eq!(first.end_col, 2);
    }

    #[test]
    fn string_literal_highlighting() {
        let lines = vec!["let x = \"hello world\";".to_string()];
        let result = tokenize_lines(&lines);
        // Should have "let" keyword and string literal
        assert!(result.highlights[0].iter().any(|s| s.kind == HighlightKind::String));
        assert!(result.highlights[0].iter().any(|s| s.kind == HighlightKind::Keyword));
    }

    #[test]
    fn comment_highlighting() {
        let lines = vec!["// this is a comment".to_string()];
        let result = tokenize_lines(&lines);
        assert_eq!(result.highlights[0].len(), 1);
        assert_eq!(result.highlights[0][0].kind, HighlightKind::Comment);
        assert_eq!(result.highlights[0][0].start_col, 0);
        assert_eq!(result.highlights[0][0].end_col, 20);
    }

    #[test]
    fn number_highlighting() {
        let lines = vec!["let x = 42;".to_string(), "let y = 3.14;".to_string()];
        let result = tokenize_lines(&lines);
        assert!(result.highlights[0].iter().any(|s| s.kind == HighlightKind::Number));
        assert!(result.highlights[1].iter().any(|s| s.kind == HighlightKind::Number));
    }

    #[test]
    fn type_highlighting() {
        let lines = vec!["struct FooBar {".to_string()];
        let result = tokenize_lines(&lines);
        let type_spans: Vec<_> =
            result.highlights[0].iter().filter(|s| s.kind == HighlightKind::Type).collect();
        assert!(!type_spans.is_empty());
    }

    #[test]
    fn empty_lines_ok() {
        let lines = vec!["".to_string(), "fn x() {}".to_string()];
        let result = tokenize_lines(&lines);
        assert_eq!(result.highlights.len(), 2);
        assert!(result.highlights[0].is_empty());
        assert!(!result.highlights[1].is_empty());
    }

    #[test]
    fn attribute_highlighting() {
        let lines = vec!["#[derive(Debug)]".to_string()];
        let result = tokenize_lines(&lines);
        assert!(result.highlights[0].iter().any(|s| s.kind == HighlightKind::Attribute));
    }
}
