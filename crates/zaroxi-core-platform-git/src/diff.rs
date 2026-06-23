//! Pure, dependency-free line-level diffing.
//!
//! This module contains no git or I/O: it turns two text blobs (a baseline and
//! the current buffer) into line-range [`DiffHunk`]s and per-line [`ChangedLine`]
//! markers ready for editor gutter / minimap / cockpit overlays. Keeping it pure
//! makes it fast and fully unit-testable, and lets the git layer ([`crate::repo`])
//! worry only about *fetching* the baseline.
//!
//! The algorithm trims the common prefix and suffix (so typical small edits are
//! cheap regardless of file size) and runs an LCS only over the differing middle.
//! A cell-count cap bounds worst-case cost on pathological inputs so the UI is
//! never blocked by a runaway diff.

/// Maximum LCS DP cells (`old_mid * new_mid`) before falling back to a single
/// coarse "modified" hunk. ~4M cells keeps the table under ~16 MB (`u32`).
const MAX_LCS_CELLS: usize = 4_000_000;

/// Classification of a contiguous changed block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// Lines present in the new file but not the old (pure insertion).
    Added,
    /// Lines present in the old file but not the new (pure deletion).
    Removed,
    /// A block where old lines were replaced by new lines.
    Modified,
}

/// A contiguous changed region, expressed as line ranges in **both** sides.
///
/// All line numbers are 0-based. `new_start..new_start+new_len` is the range in
/// the current file; `old_start..old_start+old_len` is the range in the baseline.
/// A pure addition has `old_len == 0`; a pure removal has `new_len == 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffHunk {
    /// Whether the block was added, removed, or modified.
    pub kind: ChangeKind,
    /// First affected line in the new (current) file.
    pub new_start: usize,
    /// Number of new-file lines in the block (`0` for a pure removal).
    pub new_len: usize,
    /// First affected line in the old (baseline) file.
    pub old_start: usize,
    /// Number of old-file lines in the block (`0` for a pure addition).
    pub old_len: usize,
}

/// A single changed line in the current file, for gutter/minimap rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangedLine {
    /// 0-based line in the current file (for a pure removal, the line where the
    /// deletion occurred).
    pub line: usize,
    /// `true` for added/modified lines, `false` for a removal marker.
    pub added: bool,
}

/// Split a document into lines without allocating per line.
///
/// An empty document is zero lines (so "empty file" vs "has content" diffs as a
/// pure addition). Otherwise lines are separated by `'\n'`, matching the editor's
/// document contract (a trailing newline yields a final empty line).
fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() { Vec::new() } else { text.split('\n').collect() }
}

/// Compute line-level [`DiffHunk`]s between `old` (baseline) and `new` (current).
///
/// The result is ordered by position in the new file and contains only changed
/// regions (unchanged lines produce no hunks).
pub fn diff_lines(old: &str, new: &str) -> Vec<DiffHunk> {
    let a = split_lines(old);
    let b = split_lines(new);

    // Trim the common prefix.
    let mut prefix = 0;
    while prefix < a.len() && prefix < b.len() && a[prefix] == b[prefix] {
        prefix += 1;
    }
    // Trim the common suffix (without overlapping the prefix).
    let mut suffix = 0;
    while suffix < a.len() - prefix
        && suffix < b.len() - prefix
        && a[a.len() - 1 - suffix] == b[b.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let a_mid = &a[prefix..a.len() - suffix];
    let b_mid = &b[prefix..b.len() - suffix];

    if a_mid.is_empty() && b_mid.is_empty() {
        return Vec::new();
    }
    if a_mid.is_empty() {
        return vec![DiffHunk {
            kind: ChangeKind::Added,
            new_start: prefix,
            new_len: b_mid.len(),
            old_start: prefix,
            old_len: 0,
        }];
    }
    if b_mid.is_empty() {
        return vec![DiffHunk {
            kind: ChangeKind::Removed,
            new_start: prefix,
            new_len: 0,
            old_start: prefix,
            old_len: a_mid.len(),
        }];
    }

    let n = a_mid.len();
    let m = b_mid.len();

    // Bound worst-case cost: collapse to a single modified block.
    if n.saturating_mul(m) > MAX_LCS_CELLS {
        return vec![DiffHunk {
            kind: ChangeKind::Modified,
            new_start: prefix,
            new_len: m,
            old_start: prefix,
            old_len: n,
        }];
    }

    let ops = lcs_ops(a_mid, b_mid);
    coalesce(&ops, prefix)
}

/// An edit operation in an LCS alignment.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Equal,
    Delete,
    Insert,
}

/// Produce the LCS edit script (`Equal`/`Delete`/`Insert`) aligning `a` to `b`.
fn lcs_ops(a: &[&str], b: &[&str]) -> Vec<Op> {
    let n = a.len();
    let m = b.len();
    let width = m + 1;
    // dp[i*width + j] = LCS length of a[i..], b[j..].
    let mut dp = vec![0u32; (n + 1) * width];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i * width + j] = if a[i] == b[j] {
                dp[(i + 1) * width + (j + 1)] + 1
            } else {
                dp[(i + 1) * width + j].max(dp[i * width + (j + 1)])
            };
        }
    }

    let mut ops = Vec::with_capacity(n + m);
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if a[i] == b[j] {
            ops.push(Op::Equal);
            i += 1;
            j += 1;
        } else if dp[(i + 1) * width + j] >= dp[i * width + (j + 1)] {
            ops.push(Op::Delete);
            i += 1;
        } else {
            ops.push(Op::Insert);
            j += 1;
        }
    }
    while i < n {
        ops.push(Op::Delete);
        i += 1;
    }
    while j < m {
        ops.push(Op::Insert);
        j += 1;
    }
    ops
}

/// Walk an edit script, grouping consecutive non-equal ops into [`DiffHunk`]s.
/// `offset` is the common-prefix length (added to both line counters).
fn coalesce(ops: &[Op], offset: usize) -> Vec<DiffHunk> {
    let mut hunks = Vec::new();
    let mut old_idx = offset;
    let mut new_idx = offset;
    let mut k = 0;
    while k < ops.len() {
        if ops[k] == Op::Equal {
            old_idx += 1;
            new_idx += 1;
            k += 1;
            continue;
        }
        let old_start = old_idx;
        let new_start = new_idx;
        let mut dels = 0;
        let mut ins = 0;
        while k < ops.len() && ops[k] != Op::Equal {
            match ops[k] {
                Op::Delete => {
                    dels += 1;
                    old_idx += 1;
                }
                Op::Insert => {
                    ins += 1;
                    new_idx += 1;
                }
                Op::Equal => unreachable!(),
            }
            k += 1;
        }
        let kind = if dels > 0 && ins > 0 {
            ChangeKind::Modified
        } else if ins > 0 {
            ChangeKind::Added
        } else {
            ChangeKind::Removed
        };
        hunks.push(DiffHunk { kind, new_start, new_len: ins, old_start, old_len: dels });
    }
    hunks
}

/// Flatten [`DiffHunk`]s into per-line [`ChangedLine`] markers in the new file.
///
/// Added and modified lines become `added: true` markers (one per new line);
/// a pure removal becomes a single `added: false` marker at the line where the
/// deletion occurred. This is the directly-renderable form for a change gutter
/// or a semantic minimap lane.
pub fn changed_lines(hunks: &[DiffHunk]) -> Vec<ChangedLine> {
    let mut out = Vec::new();
    for h in hunks {
        match h.kind {
            ChangeKind::Added | ChangeKind::Modified => {
                for line in h.new_start..h.new_start + h.new_len {
                    out.push(ChangedLine { line, added: true });
                }
            }
            ChangeKind::Removed => {
                out.push(ChangedLine { line: h.new_start, added: false });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_text_has_no_hunks() {
        assert!(diff_lines("a\nb\nc", "a\nb\nc").is_empty());
        assert!(diff_lines("", "").is_empty());
    }

    #[test]
    fn pure_addition_at_end() {
        let h = diff_lines("a\nb", "a\nb\nc\nd");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].kind, ChangeKind::Added);
        assert_eq!((h[0].new_start, h[0].new_len, h[0].old_len), (2, 2, 0));
    }

    #[test]
    fn pure_removal_in_middle() {
        let h = diff_lines("a\nb\nc\nd", "a\nd");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].kind, ChangeKind::Removed);
        assert_eq!((h[0].old_start, h[0].old_len, h[0].new_len), (1, 2, 0));
        assert_eq!(h[0].new_start, 1);
    }

    #[test]
    fn modification_in_place() {
        let h = diff_lines("a\nFOO\nc", "a\nBAR\nc");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].kind, ChangeKind::Modified);
        assert_eq!((h[0].new_start, h[0].new_len, h[0].old_len), (1, 1, 1));
    }

    #[test]
    fn empty_baseline_is_all_added() {
        let h = diff_lines("", "x\ny\nz");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].kind, ChangeKind::Added);
        assert_eq!((h[0].new_start, h[0].new_len), (0, 3));
    }

    #[test]
    fn multiple_separated_hunks() {
        let old = "1\n2\n3\n4\n5";
        let new = "1\nX\n3\n4\nY\nZ";
        let h = diff_lines(old, new);
        // Line 2 modified (2->X); line 5 modified + added (5 -> Y, Z).
        assert!(h.len() >= 2, "expected separate hunks, got {h:?}");
        assert_eq!(h[0].new_start, 1);
        assert!(h.iter().any(|x| x.new_start >= 4));
    }

    #[test]
    fn changed_lines_flattening() {
        let old = "a\nb\nc";
        let new = "a\nB\nc\nd";
        let hunks = diff_lines(old, new);
        let lines = changed_lines(&hunks);
        // Line 1 modified (added marker) and line 3 added.
        assert!(lines.iter().any(|c| c.line == 1 && c.added));
        assert!(lines.iter().any(|c| c.line == 3 && c.added));
    }

    #[test]
    fn changed_lines_marks_removal() {
        let hunks = diff_lines("a\nb\nc", "a\nc");
        let lines = changed_lines(&hunks);
        assert_eq!(lines.len(), 1);
        assert!(!lines[0].added);
        assert_eq!(lines[0].line, 1);
    }

    #[test]
    fn large_unchanged_file_with_small_edit_is_cheap() {
        // 100k identical lines + one changed line; prefix/suffix trim keeps this
        // to a tiny LCS, so it must return a single small hunk.
        let mut old = String::new();
        let mut new = String::new();
        for i in 0..100_000 {
            old.push_str(&format!("line {i}\n"));
            if i == 50_000 {
                new.push_str("line CHANGED\n");
            } else {
                new.push_str(&format!("line {i}\n"));
            }
        }
        let h = diff_lines(&old, &new);
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].kind, ChangeKind::Modified);
        assert_eq!(h[0].new_start, 50_000);
    }
}
