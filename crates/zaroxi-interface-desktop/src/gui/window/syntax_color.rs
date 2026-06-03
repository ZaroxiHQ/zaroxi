use zaroxi_core_platform_syntax::highlight::{Highlight, HighlightEngine};
use zaroxi_core_platform_syntax::language::LanguageId;
use zaroxi_core_platform_syntax::parser::ParserPool;
use zaroxi_interface_theme::theme::ZaroxiTheme;

/// Colorize editor source lines using tree-sitter syntax highlighting.
/// Returns per-line colored spans as `(text, [r, g, b, a])`.
pub fn colorize_source(lines: &[String]) -> Vec<(String, [f32; 4])> {
    let source = lines.join("\n");
    let sem = ZaroxiTheme::Dark.colors(false);

    // Parse with tree-sitter
    let pool = ParserPool::new();
    let mut parser = match pool.acquire(&LanguageId::Rust) {
        Some(p) => p,
        None => return lines.iter().map(|l| (l.clone(), [0.9, 0.9, 0.9, 1.0])).collect(),
    };

    let tree = match parser.parse(&source, None) {
        Some(t) => t,
        None => return lines.iter().map(|l| (l.clone(), [0.9, 0.9, 0.9, 1.0])).collect(),
    };

    // Highlight
    let engine = HighlightEngine::new();
    let spans = engine.highlight(LanguageId::Rust, &source, &tree).unwrap_or_default();

    // Build byte-offset → color map
    let to_f32 = |c: &zaroxi_interface_theme::Color| -> [f32; 4] { [c.r, c.g, c.b, c.a] };
    let default_color: [f32; 4] = to_f32(&sem.text_primary);

    let highlight_color = |h: Highlight| -> [f32; 4] {
        match h {
            Highlight::Comment => to_f32(&sem.syntax_comment),
            Highlight::String => to_f32(&sem.syntax_string),
            Highlight::Keyword => to_f32(&sem.syntax_keyword),
            Highlight::Function => to_f32(&sem.syntax_function),
            Highlight::Type => to_f32(&sem.syntax_type),
            Highlight::Number => to_f32(&sem.syntax_number),
            Highlight::Constant => to_f32(&sem.syntax_constant),
            Highlight::Variable => to_f32(&sem.syntax_variable),
            Highlight::Operator => to_f32(&sem.syntax_operator),
            Highlight::Attribute => to_f32(&sem.syntax_attribute),
            Highlight::Property => to_f32(&sem.syntax_property),
            Highlight::Namespace => to_f32(&sem.syntax_namespace),
            Highlight::Plain => default_color,
        }
    };

    // Build colored spans per line
    let mut result: Vec<(String, [f32; 4])> = Vec::new();
    let mut byte_offset = 0usize;

    for line in lines {
        let line_end = byte_offset + line.len();
        let line_spans: Vec<_> =
            spans.iter().filter(|s| s.start < line_end && s.end > byte_offset).cloned().collect();

        if line_spans.is_empty() {
            result.push((line.clone(), default_color));
        } else {
            let mut pos = byte_offset;
            for span in &line_spans {
                let seg_start = span.start.max(pos);
                let seg_end = span.end.min(line_end);
                if seg_start > pos && pos < seg_start {
                    let before = &source[pos..seg_start];
                    if !before.is_empty() {
                        result.push((before.to_string(), default_color));
                    }
                }
                if seg_start < seg_end && seg_start >= pos {
                    let text = &source[seg_start..seg_end];
                    if !text.is_empty() {
                        result.push((text.to_string(), highlight_color(span.highlight)));
                    }
                }
                pos = seg_end.max(pos);
            }
            if pos < line_end {
                let after = &source[pos..line_end];
                if !after.is_empty() {
                    result.push((after.to_string(), default_color));
                }
            }
        }

        result.push(("\n".to_string(), default_color));
        byte_offset = line_end + 1;
    }

    // Drop tree before parser
    drop(tree);
    pool.release(&LanguageId::Rust, parser);

    result
}
