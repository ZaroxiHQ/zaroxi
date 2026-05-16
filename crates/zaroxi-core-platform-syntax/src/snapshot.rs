//! Syntax snapshot for UI consumption.

use crate::highlight::{Highlight, HighlightSpan};
use std::collections::HashMap;

/// A snapshot of syntax information for a document
#[derive(Debug, Clone)]
pub struct SyntaxSnapshot {
    /// Highlight spans grouped by line
    highlights_by_line: HashMap<usize, Vec<LineHighlight>>,
    /// Whether syntax highlighting is available
    pub has_syntax: bool,
}

/// A highlight within a specific line
#[derive(Debug, Clone)]
pub struct LineHighlight {
    /// Start column in the line
    pub start_col: usize,
    /// End column in the line
    pub end_col: usize,
    /// Highlight type
    pub highlight: Highlight,
}

impl SyntaxSnapshot {
    /// Create a new syntax snapshot from highlight spans
    pub fn new(highlights: Vec<HighlightSpan>, text: &str) -> Self {
        let mut highlights_by_line = HashMap::new();
        
        // Split text into lines to map byte offsets to line/column
        let lines: Vec<&str> = text.lines().collect();
        let mut line_starts = Vec::new();
        let mut current_byte = 0;
        
        for line in &lines {
            line_starts.push(current_byte);
            current_byte += line.len() + 1; // +1 for newline
        }
        
        for span in &highlights {
            // Find which line this span starts in
            if let Some(line_idx) = line_starts.iter().position(|&start| start <= span.start) {
                let line_start = line_starts[line_idx];
                let start_col = span.start - line_start;
                let end_col = span.end - line_start;
                
                let line_highlight = LineHighlight {
                    start_col,
                    end_col,
                    highlight: span.highlight,
                };
                
                highlights_by_line
                    .entry(line_idx)
                    .or_insert_with(Vec::new)
                    .push(line_highlight);
            }
        }
        
        // Sort highlights within each line
        for highlights in highlights_by_line.values_mut() {
            highlights.sort_by_key(|h| h.start_col);
        }
        
        Self {
            highlights_by_line,
            has_syntax: !highlights.is_empty(),
        }
    }

    /// Get highlights for a specific line
    pub fn highlights_for_line(&self, line: usize) -> Option<&[LineHighlight]> {
        self.highlights_by_line
            .get(&line)
            .map(|v| v.as_slice())
    }

    /// Check if syntax highlighting is available
    pub fn has_syntax(&self) -> bool {
        self.has_syntax
    }
}

impl Default for SyntaxSnapshot {
    fn default() -> Self {
        Self {
            highlights_by_line: HashMap::new(),
            has_syntax: false,
        }
    }
}
