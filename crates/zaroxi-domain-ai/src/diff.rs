//! Structured diff engine.
//!
//! Computes insert/delete/replace operations between two text strings,
//! using a line-based diff approach. Also parses code blocks from AI responses.

use std::collections::HashMap;

use crate::actions::DiffChange;

pub fn compute_diff(original: &str, modified: &str) -> Vec<DiffChange> {
    if original == modified {
        return Vec::new();
    }
    if original.is_empty() {
        return vec![DiffChange::Insert { index: 0, text: modified.to_string() }];
    }
    if modified.is_empty() {
        return vec![DiffChange::Delete { start: 0, end: original.len() }];
    }

    let orig_lines: Vec<&str> = original.lines().collect();
    let mod_lines: Vec<&str> = modified.lines().collect();

    let line_ratio = orig_lines.len().max(1) as f64 / mod_lines.len().max(1) as f64;
    if line_ratio < 0.3 || line_ratio > 3.0 {
        return vec![DiffChange::Replace {
            start: 0,
            end: original.len(),
            text: modified.to_string(),
        }];
    }

    let changes = line_diff(&orig_lines, &mod_lines);
    let char_offsets = line_char_offsets(original);
    let mut result = Vec::new();

    for change in &changes {
        result.push(compute_char_diff(change, &char_offsets, original.len()));
    }

    if result.len() == 1 {
        if let DiffChange::Replace { start, end, .. } = &result[0] {
            if *start == 0 && *end >= original.len() - 1 {
                return result;
            }
        }
    }
    result
}

fn compute_char_diff(
    change: &DiffChange,
    offsets: &HashMap<usize, usize>,
    text_len: usize,
) -> DiffChange {
    let get_offset = |idx: usize| -> usize { offsets.get(&idx).copied().unwrap_or(text_len) };
    let get_start = |idx: usize| -> usize { offsets.get(&idx).copied().unwrap_or(0) };

    match change {
        DiffChange::Insert { index, text } => {
            DiffChange::Insert { index: get_offset(*index), text: text.clone() }
        }
        DiffChange::Delete { start, end } => {
            DiffChange::Delete { start: get_start(*start), end: get_offset(*end) }
        }
        DiffChange::Replace { start, end, text } => DiffChange::Replace {
            start: get_start(*start),
            end: get_offset(*end),
            text: text.clone(),
        },
    }
}

fn line_diff(orig: &[&str], modified: &[&str]) -> Vec<DiffChange> {
    let mut changes = Vec::new();

    let mut prefix = 0;
    while prefix < orig.len() && prefix < modified.len() && orig[prefix] == modified[prefix] {
        prefix += 1;
    }

    let mut suffix = 0;
    while suffix < orig.len() - prefix && suffix < modified.len() - prefix {
        let oi = orig.len() - 1 - suffix;
        let mi = modified.len() - 1 - suffix;
        if orig[oi] != modified[mi] {
            break;
        }
        suffix += 1;
    }

    let orig_start = prefix;
    let orig_end = orig.len() - suffix;
    let mod_start = prefix;
    let mod_end = modified.len() - suffix;

    let new_lines: Vec<&str> = modified[mod_start..mod_end].to_vec();
    let text = new_lines.join("\n");

    if orig_start < orig_end || mod_start < mod_end {
        if orig_start < orig_end {
            if mod_start < mod_end {
                changes.push(DiffChange::Replace { start: orig_start, end: orig_end, text });
            } else {
                changes.push(DiffChange::Delete { start: orig_start, end: orig_end });
            }
        } else if mod_start < mod_end {
            let insert_text =
                if orig_start > 0 && !text.is_empty() { format!("\n{text}") } else { text };
            changes.push(DiffChange::Insert { index: orig_start, text: insert_text });
        }
    }
    changes
}

fn line_char_offsets(text: &str) -> HashMap<usize, usize> {
    let mut offsets = HashMap::new();
    offsets.insert(0, 0);
    let mut idx = 0;
    for (line_no, line) in text.lines().enumerate() {
        offsets.insert(line_no, idx);
        idx += line.len() + 1;
    }
    offsets.insert(text.lines().count(), text.len());
    offsets
}

pub fn parse_diff_from_response(response: &str, original: &str) -> Vec<DiffChange> {
    let cleaned = extract_code_block(response);
    if cleaned != original {
        return compute_diff(original, &cleaned);
    }
    if response.contains("fn ") || response.contains("pub ") || response.contains("class ") {
        let code = response.lines().filter(|l| !l.starts_with("//")).collect::<Vec<_>>().join("\n");
        if code != original {
            return compute_diff(original, &code);
        }
    }
    Vec::new()
}

fn extract_code_block(text: &str) -> String {
    let mut in_block = false;
    let mut block_lines: Vec<&str> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_block {
                break;
            }
            in_block = true;
            continue;
        }
        if in_block {
            block_lines.push(line);
        }
    }
    if !block_lines.is_empty() { block_lines.join("\n") } else { text.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_text_produces_no_diff() {
        let diff = compute_diff("hello", "hello");
        assert!(diff.is_empty());
    }

    #[test]
    fn insert_at_beginning() {
        let diff = compute_diff("world", "hello world");
        assert!(!diff.is_empty());
        let result = apply_changes("world", &diff);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn delete_from_middle() {
        let diff = compute_diff("hello beautiful world", "hello world");
        let result = apply_changes("hello beautiful world", &diff);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn full_replace_when_line_ratio_bad() {
        let diff = compute_diff("a\nb\nc\nd\ne\nf\ng\nh\ni\nj", "xyz");
        assert_eq!(diff.len(), 1);
        assert!(matches!(diff[0], DiffChange::Replace { .. }));
    }

    #[test]
    fn compute_diff_empty_original() {
        let diff = compute_diff("", "new content");
        assert_eq!(diff.len(), 1);
        let result = apply_changes("", &diff);
        assert_eq!(result, "new content");
    }

    #[test]
    fn single_line_change() {
        let orig = "fn hello() {\n    println!(\"hello\");\n}";
        let modified = "fn hello() {\n    println!(\"hi\");\n}";
        let diff = compute_diff(orig, modified);
        let result = apply_changes(orig, &diff);
        assert!(result.contains("println!(\"hi\")"), "should update the print: {result}");
    }

    fn apply_changes(text: &str, changes: &[DiffChange]) -> String {
        use crate::actions::DiffResult;
        let diff = DiffResult {
            buffer_id: "test".into(),
            changes: changes.to_vec(),
            full_replacement: None,
            summary: String::new(),
        };
        diff.apply_to(text).unwrap_or_else(|| text.to_string())
    }
}
