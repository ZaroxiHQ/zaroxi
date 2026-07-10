//! zaroxi-core-editor-minimap — the minimap **rendering data model**.
//!
//! This crate is pure (no rendering, no I/O): it turns document lines into a
//! compact, structure-first [`MinimapProjection`] and provides the scroll/line
//! mapping math the renderer and interaction code share. Keeping it isolated
//! means both the widget painter (`zaroxi-interface-widgets`) and the desktop
//! integration (`zaroxi-interface-desktop`) consume one authoritative model,
//! and the tricky banding / viewport / click-to-line math is unit-testable
//! without a GPU or a running shell.
//!
//! Design principles (a Zed-like minimap in Zaroxi's architecture):
//! - **Structure first, syntax second.** Each row summarizes a source line (or
//!   a band of lines) by its indentation, content occupancy, and a coarse kind
//!   (blank / comment / code). No literal syntax painting — the minimap conveys
//!   code *shape*, indentation rhythm, and blank gaps, not a color rainbow.
//! - **Honest downsampling.** When the document is taller than the minimap, rows
//!   are banded deterministically so the texture stays stable during scrolling.
//! - **Accurate navigation.** Viewport and click mapping are derived from real
//!   scroll state (`scroll_top`, `visible`, `total`), never approximated.

#![forbid(unsafe_code)]

/// Columns of indentation at which the indent fraction saturates to `1.0`.
/// Deeper nesting than this reads as "maximally indented" in the minimap.
const INDENT_REFERENCE_COLS: f32 = 32.0;
/// Content length (in characters, excluding indentation) at which the occupancy
/// fraction saturates to `1.0`.
const OCCUPANCY_REFERENCE_COLS: f32 = 96.0;
/// Minimum occupancy for any non-blank line, so a short but real line still
/// paints a visible (if tiny) mark rather than vanishing.
const MIN_NONBLANK_OCCUPANCY: f32 = 0.08;
/// Default tab width used when a caller does not specify one.
pub const DEFAULT_TAB_WIDTH: usize = 4;

/// Coarse classification of a source line, driving the row's brightness group.
/// Intentionally tiny: the minimap abstracts syntax down to three legible bands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// Whitespace-only line — painted as an empty gap (structure/rhythm).
    Blank,
    /// A comment line — dimmer than code so prose recedes.
    Comment,
    /// A code line — the brightest texture band.
    Code,
}

/// One minimap row: a compressed, render-ready summary of one source line or a
/// band of source lines. All magnitudes are normalized fractions in `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapRow {
    /// Leading-indentation fraction (`0.0` = flush left, `1.0` = deeply nested).
    pub indent: f32,
    /// Content occupancy fraction (`0.0` = empty, `1.0` = long/dense line).
    pub occupancy: f32,
    /// Coarse line kind driving the brightness band.
    pub kind: RowKind,
}

/// Per-line statistics, pre-normalization aggregation unit.
#[derive(Debug, Clone, Copy)]
struct LineStat {
    indent: f32,
    occupancy: f32,
    kind: RowKind,
}

impl LineStat {
    const BLANK: LineStat = LineStat { indent: 0.0, occupancy: 0.0, kind: RowKind::Blank };

    /// Analyze a single source line into normalized structure stats. Pure and
    /// language-agnostic: indentation is measured in columns (tabs expanded),
    /// occupancy is the trimmed content length, and the kind is a conservative
    /// leading-token heuristic (never a full parse — the minimap only needs a
    /// coarse brightness band).
    fn analyze(line: &str, tab_width: usize) -> LineStat {
        let tab_width = tab_width.max(1);
        let mut indent_cols = 0usize;
        let mut content_chars = 0usize;
        let mut seen_content = false;
        for ch in line.chars() {
            if ch == '\n' || ch == '\r' {
                continue;
            }
            if !seen_content {
                match ch {
                    ' ' => {
                        indent_cols += 1;
                        continue;
                    }
                    '\t' => {
                        indent_cols += tab_width;
                        continue;
                    }
                    _ => seen_content = true,
                }
            }
            content_chars += 1;
        }
        if !seen_content {
            return LineStat::BLANK;
        }
        let trimmed = line.trim_start();
        let kind = if is_comment_lead(trimmed) { RowKind::Comment } else { RowKind::Code };
        let indent = (indent_cols as f32 / INDENT_REFERENCE_COLS).clamp(0.0, 1.0);
        let occupancy =
            (content_chars as f32 / OCCUPANCY_REFERENCE_COLS).clamp(MIN_NONBLANK_OCCUPANCY, 1.0);
        LineStat { indent, occupancy, kind }
    }
}

/// Conservative, language-agnostic "line begins a comment" heuristic. Covers the
/// common families (C/Rust/JS `//` `/*` `*`, shell/Python `#`, SQL/Lua `--`,
/// Lisp `;`, HTML/XML `<!--`, LaTeX `%`). False negatives just render as code
/// (the safe default); the minimap never depends on this being exact.
fn is_comment_lead(trimmed: &str) -> bool {
    const LEADS: [&str; 9] = ["//", "/*", "*/", "* ", "#", "--", ";", "<!--", "%"];
    if trimmed == "*" {
        return true;
    }
    LEADS.iter().any(|p| trimmed.starts_with(p))
}

/// A compact, render-ready minimap of a document.
#[derive(Debug, Clone, PartialEq)]
pub struct MinimapProjection {
    /// One entry per minimap row, top-to-bottom. Length is [`Self::source_rows`].
    pub rows: Vec<MinimapRow>,
    /// The document's total line count (the mapping domain).
    pub total_lines: usize,
    /// How many source lines each row represents (`>= 1.0` when banded).
    pub lines_per_row: f32,
    /// True when the projection was built from sampled lines (large-file mode)
    /// rather than the full document, so the renderer can render it slightly
    /// calmer and callers know fidelity is intentional-degraded, not broken.
    pub sampled: bool,
}

impl Default for MinimapProjection {
    fn default() -> Self {
        Self::empty()
    }
}

impl MinimapProjection {
    /// An empty projection (no document / no rows).
    pub fn empty() -> Self {
        MinimapProjection { rows: Vec::new(), total_lines: 0, lines_per_row: 1.0, sampled: false }
    }

    /// Number of minimap rows.
    pub fn source_rows(&self) -> usize {
        self.rows.len()
    }

    /// Whether there is anything to render.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Build a full-fidelity projection from the document's lines.
    ///
    /// `lines` yields the document text one line at a time (without trailing
    /// newline). `total_lines` is the authoritative line count (it may exceed
    /// the yielded count when the document ends in a newline — the tail is
    /// padded with blank rows so the mapping domain stays exact). Rows are
    /// deterministically banded down to at most `max_rows`.
    pub fn from_lines<'a, I: Iterator<Item = &'a str>>(
        lines: I,
        total_lines: usize,
        max_rows: usize,
        tab_width: usize,
    ) -> Self {
        let total = total_lines.max(1);
        let mut stats: Vec<LineStat> = lines.map(|l| LineStat::analyze(l, tab_width)).collect();
        stats.resize(total, LineStat::BLANK);
        Self::from_stats(&stats, total, max_rows, false)
    }

    /// Build a sampled (degraded-fidelity) projection for large files.
    ///
    /// Rather than materialize the whole document, one representative source
    /// line per row is fetched via `fetch(line_index)`. This is `O(rows)`
    /// fetches regardless of document size, so a multi-million-line file
    /// projects in bounded time. Missing lines (fetch returns `None`) render as
    /// blank gaps. The result is marked [`sampled`](Self::sampled).
    pub fn from_sampled<F: FnMut(usize) -> Option<String>>(
        total_lines: usize,
        max_rows: usize,
        tab_width: usize,
        mut fetch: F,
    ) -> Self {
        let total = total_lines.max(1);
        let rows = total.min(max_rows.max(1)).max(1);
        let mut out = Vec::with_capacity(rows);
        for r in 0..rows {
            let line = (r.saturating_mul(total) / rows).min(total - 1);
            let stat =
                fetch(line).map(|s| LineStat::analyze(&s, tab_width)).unwrap_or(LineStat::BLANK);
            out.push(MinimapRow {
                indent: stat.indent,
                occupancy: stat.occupancy,
                kind: stat.kind,
            });
        }
        MinimapProjection {
            rows: out,
            total_lines: total,
            lines_per_row: total as f32 / rows as f32,
            sampled: true,
        }
    }

    fn from_stats(stats: &[LineStat], total: usize, max_rows: usize, sampled: bool) -> Self {
        let rows = total.min(max_rows.max(1)).max(1);
        let mut out = Vec::with_capacity(rows);
        for r in 0..rows {
            let start = r.saturating_mul(total) / rows;
            let end = (((r + 1).saturating_mul(total) / rows).max(start + 1)).min(total);
            let mut occupancy = 0.0f32;
            let mut indent = 1.0f32;
            let mut has_code = false;
            let mut has_comment = false;
            let mut has_content = false;
            for s in &stats[start..end] {
                match s.kind {
                    RowKind::Code => {
                        has_code = true;
                        has_content = true;
                        indent = indent.min(s.indent);
                        occupancy = occupancy.max(s.occupancy);
                    }
                    RowKind::Comment => {
                        has_comment = true;
                        has_content = true;
                        indent = indent.min(s.indent);
                        occupancy = occupancy.max(s.occupancy);
                    }
                    RowKind::Blank => {}
                }
            }
            let kind = if has_code {
                RowKind::Code
            } else if has_comment {
                RowKind::Comment
            } else {
                RowKind::Blank
            };
            out.push(MinimapRow {
                indent: if has_content { indent } else { 0.0 },
                occupancy,
                kind,
            });
        }
        MinimapProjection {
            rows: out,
            total_lines: total,
            lines_per_row: total as f32 / rows as f32,
            sampled,
        }
    }

    /// Map a document line to its `[0, 1)` vertical fraction in the minimap.
    pub fn line_fraction(&self, line: usize) -> f32 {
        line_fraction(line, self.total_lines)
    }
}

/// Vertical fraction `[0, 1)` of a document line, for overlay placement.
pub fn line_fraction(line: usize, total_lines: usize) -> f32 {
    if total_lines == 0 {
        return 0.0;
    }
    (line as f32 / total_lines as f32).clamp(0.0, 1.0)
}

/// The visible viewport as a `[0, 1]` `(top, bottom)` fraction pair, from the
/// real editor scroll state. This is the primary minimap overlay and MUST be
/// accurate, so it is computed directly from the first-visible line and the
/// visible-line count — never approximated from the cursor.
///
/// The band is clamped to `[0, 1]` and guaranteed non-degenerate (a minimum
/// height) so the indicator is always visible even for very tall documents.
pub fn viewport_fraction(scroll_top: usize, visible: usize, total_lines: usize) -> (f32, f32) {
    if total_lines == 0 {
        return (0.0, 1.0);
    }
    let total = total_lines as f32;
    let visible = visible.max(1) as f32;
    let top = (scroll_top as f32 / total).clamp(0.0, 1.0);
    let bottom = ((scroll_top as f32 + visible) / total).clamp(0.0, 1.0);
    // Guarantee a minimum visible band (2% of height) without exceeding 1.0.
    const MIN_BAND: f32 = 0.02;
    if bottom - top < MIN_BAND {
        let b = (top + MIN_BAND).min(1.0);
        let t = (b - MIN_BAND).max(0.0);
        return (t, b);
    }
    (top, bottom)
}

/// Map a minimap click/drag fraction `[0, 1]` to a target first-visible line,
/// CENTERING the viewport on the clicked position (the Zed-like feel: click a
/// spot and it moves to the middle of the editor), then clamping so the top and
/// bottom of the document remain reachable without overscroll.
pub fn top_line_for_fraction(fraction: f32, visible: usize, total_lines: usize) -> usize {
    if total_lines == 0 {
        return 0;
    }
    let frac = fraction.clamp(0.0, 1.0);
    let visible = visible.max(1);
    let center = (frac * total_lines as f32).round() as usize;
    let max_scroll = total_lines.saturating_sub(visible);
    center.saturating_sub(visible / 2).min(max_scroll)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(indent: f32, occ: f32) -> MinimapRow {
        MinimapRow { indent, occupancy: occ, kind: RowKind::Code }
    }

    #[test]
    fn empty_document_projects_to_empty_or_single_blank() {
        let p = MinimapProjection::from_lines(std::iter::empty(), 0, 500, 4);
        // total clamps to 1 -> a single blank row.
        assert_eq!(p.source_rows(), 1);
        assert_eq!(p.rows[0].kind, RowKind::Blank);
        assert!(!p.sampled);
    }

    #[test]
    fn single_line_maps_one_to_one() {
        let p = MinimapProjection::from_lines(["fn main() {}"].into_iter(), 1, 500, 4);
        assert_eq!(p.source_rows(), 1);
        assert_eq!(p.rows[0].kind, RowKind::Code);
        assert!(p.rows[0].occupancy > 0.0);
        assert_eq!(p.rows[0].indent, 0.0);
        assert!((p.lines_per_row - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn short_document_is_one_row_per_line() {
        let src = ["fn a() {", "    let x = 1;", "", "    // note", "}"];
        let p = MinimapProjection::from_lines(src.into_iter(), src.len(), 500, 4);
        assert_eq!(p.source_rows(), src.len());
        assert_eq!(p.rows[0].kind, RowKind::Code);
        assert!(p.rows[1].indent > 0.0, "indented line reads as nested");
        assert_eq!(p.rows[2].kind, RowKind::Blank);
        assert_eq!(p.rows[3].kind, RowKind::Comment);
    }

    #[test]
    fn tall_document_bands_down_to_max_rows() {
        let lines: Vec<String> = (0..1000).map(|i| format!("line {i} = value;")).collect();
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        let p = MinimapProjection::from_lines(refs.into_iter(), 1000, 200, 4);
        assert_eq!(p.source_rows(), 200, "must band down to max_rows");
        assert!((p.lines_per_row - 5.0).abs() < 0.01, "1000/200 = 5 lines per row");
        // Every row covers real code here.
        assert!(p.rows.iter().all(|r| r.kind == RowKind::Code));
    }

    #[test]
    fn banding_covers_all_source_lines_without_gaps() {
        // Deterministic banding must partition [0, total) with no gaps/overlap
        // for arbitrary total/rows ratios.
        for total in [1usize, 7, 50, 999, 1000] {
            for max_rows in [1usize, 3, 64, 200, 5000] {
                let stats = vec![LineStat::BLANK; total];
                let p = MinimapProjection::from_stats(&stats, total, max_rows, false);
                let rows = p.source_rows();
                assert!(rows >= 1 && rows <= total.min(max_rows.max(1)).max(1));
                // Re-derive bands and confirm full coverage.
                let mut covered = 0usize;
                for r in 0..rows {
                    let start = r * total / rows;
                    let end = (((r + 1) * total / rows).max(start + 1)).min(total);
                    assert!(end > start);
                    assert_eq!(start, covered, "no gap between bands (total={total}, rows={rows})");
                    covered = end;
                }
                assert_eq!(covered, total, "bands cover the whole document");
            }
        }
    }

    #[test]
    fn viewport_fraction_is_accurate_and_bounded() {
        // Top of a 100-line doc, 20 visible.
        let (t, b) = viewport_fraction(0, 20, 100);
        assert!((t - 0.0).abs() < 1e-6);
        assert!((b - 0.2).abs() < 1e-6);
        // Scrolled to line 40.
        let (t, b) = viewport_fraction(40, 20, 100);
        assert!((t - 0.4).abs() < 1e-6);
        assert!((b - 0.6).abs() < 1e-6);
        // Clamped at the bottom.
        let (t, b) = viewport_fraction(95, 20, 100);
        assert!(b <= 1.0 && t >= 0.0 && t < b);
    }

    #[test]
    fn viewport_fraction_guarantees_min_band_for_huge_docs() {
        // 1M lines, 30 visible -> raw band ~0.00003; must widen to the min band.
        let (t, b) = viewport_fraction(500_000, 30, 1_000_000);
        assert!(b - t >= 0.019, "min band enforced, got {}", b - t);
        assert!(t >= 0.0 && b <= 1.0);
    }

    #[test]
    fn empty_doc_viewport_is_full_height() {
        assert_eq!(viewport_fraction(0, 10, 0), (0.0, 1.0));
    }

    #[test]
    fn click_fraction_centers_and_clamps() {
        // Click near top: cannot scroll above 0.
        assert_eq!(top_line_for_fraction(0.0, 20, 100), 0);
        // Click at middle: center 50 - half(10) = 40.
        assert_eq!(top_line_for_fraction(0.5, 20, 100), 40);
        // Click at bottom: clamp to max_scroll (100-20 = 80).
        assert_eq!(top_line_for_fraction(1.0, 20, 100), 80);
        // Short doc that fits: always 0.
        assert_eq!(top_line_for_fraction(0.7, 200, 100), 0);
    }

    #[test]
    fn sampled_projection_is_bounded_and_marked() {
        // A 1M-line "file": sampling must produce exactly max_rows and mark it.
        let mut fetched = 0usize;
        let p = MinimapProjection::from_sampled(1_000_000, 300, 4, |line| {
            fetched += 1;
            Some(format!("    row at {line}"))
        });
        assert_eq!(p.source_rows(), 300);
        assert_eq!(fetched, 300, "exactly O(rows) fetches, not O(file)");
        assert!(p.sampled);
        assert!(p.rows.iter().all(|r| r.kind == RowKind::Code && r.indent > 0.0));
    }

    #[test]
    fn sampled_missing_lines_render_as_blank() {
        let p = MinimapProjection::from_sampled(1000, 100, 4, |_| None);
        assert!(p.rows.iter().all(|r| r.kind == RowKind::Blank));
        assert!(p.sampled);
    }

    #[test]
    fn comment_heuristic_covers_common_families() {
        for (line, expect_comment) in [
            ("// rust", true),
            ("/* c */", true),
            (" * doc", true),
            ("# shell", true),
            ("-- sql", true),
            ("; lisp", true),
            ("<!-- html", true),
            ("let x = 1;", false),
            ("fn main() {}", false),
        ] {
            let s = LineStat::analyze(line, 4);
            let got = s.kind == RowKind::Comment;
            assert_eq!(got, expect_comment, "line {line:?}");
        }
    }

    #[test]
    fn indentation_and_occupancy_normalize_monotonically() {
        let shallow = LineStat::analyze("x", 4);
        let deep = LineStat::analyze("                x", 4);
        assert!(deep.indent > shallow.indent);
        let short = code(0.0, LineStat::analyze("ab", 4).occupancy);
        let long = code(0.0, LineStat::analyze(&"a".repeat(120), 4).occupancy);
        assert!(long.occupancy > short.occupancy);
        assert!(long.occupancy <= 1.0 && short.occupancy >= MIN_NONBLANK_OCCUPANCY);
    }

    #[test]
    fn trailing_newline_tail_is_padded_blank() {
        // 3 yielded lines but total_lines = 4 (document ends with '\n').
        let p = MinimapProjection::from_lines(["a", "b", "c"].into_iter(), 4, 500, 4);
        assert_eq!(p.source_rows(), 4);
        assert_eq!(p.rows[3].kind, RowKind::Blank);
    }
}
