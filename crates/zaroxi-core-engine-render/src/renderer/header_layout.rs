//! Responsive split-header layout for the status strip.
//!
//! The status strip is a thin (~26px) header-only region rendered as two
//! independently aligned text groups: a left label and a priority-ordered set
//! of right-side segments. This module owns the *fitting* policy only (it does
//! not decide which fields belong where — that is the app's job):
//!
//! * keep both groups when they fit;
//! * otherwise drop the lowest-priority right segments (from the end) first;
//! * if the left label still cannot fit beside the collapsed right group,
//!   truncate it with a trailing ellipsis.
//!
//! It never overlaps the two groups and never relies on manual space padding
//! for alignment — the renderer right-aligns the right group using the measured
//! widths returned here.

/// Separator placed between right-side segments.
pub const RIGHT_SEPARATOR: &str = "  ";

/// Outcome of fitting a split header into the available width.
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderFit {
    /// Left label, possibly ellipsized.
    pub left: String,
    /// Measured width of `left` in logical pixels.
    pub left_width: f32,
    /// Joined right segments that survived collapsing (may be empty).
    pub right: String,
    /// Measured width of `right` in logical pixels.
    pub right_width: f32,
}

fn text_width(s: &str, advance: f32) -> f32 {
    s.chars().count() as f32 * advance
}

/// Fit `left` plus priority-ordered `right_segments` into `avail` logical px.
///
/// `advance` is the per-character width (the status text is monospace). `gap`
/// is the minimum spacing reserved between the two groups. `right_segments` are
/// ordered highest-priority first; the lowest-priority (last) ones are dropped
/// first when space is tight.
pub fn fit_status_header(
    left: &str,
    right_segments: &[String],
    avail: f32,
    advance: f32,
    gap: f32,
) -> HeaderFit {
    if advance <= 0.0 || avail <= 0.0 {
        return HeaderFit {
            left: String::new(),
            left_width: 0.0,
            right: String::new(),
            right_width: 0.0,
        };
    }

    let left_w = text_width(left, advance);

    // Drop lowest-priority right segments (from the end) until both sides fit.
    let mut kept = right_segments.len();
    let mut right = right_segments.join(RIGHT_SEPARATOR);
    loop {
        let effective_gap = if right.is_empty() { 0.0 } else { gap };
        if left_w + effective_gap + text_width(&right, advance) <= avail || kept == 0 {
            break;
        }
        kept -= 1;
        right = right_segments[..kept].join(RIGHT_SEPARATOR);
    }

    let right_width = text_width(&right, advance);
    let effective_gap = if right.is_empty() { 0.0 } else { gap };
    let left_budget = (avail - effective_gap - right_width).max(0.0);

    let left_final =
        if left_w > left_budget { ellipsize(left, left_budget, advance) } else { left.to_string() };

    HeaderFit { left_width: text_width(&left_final, advance), left: left_final, right, right_width }
}

/// A single positioned, clipped text run produced for the status strip.
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderRun {
    /// Text to draw.
    pub text: String,
    /// Draw origin x (logical px).
    pub x: f32,
    /// Clip rect left edge (logical px).
    pub clip_x: f32,
    /// Clip rect width (logical px); always > 0 for an emitted run.
    pub clip_w: f32,
}

/// Plan the concrete left/right header runs for a status strip of width
/// `rect_w` starting at `rect_x`. Returns 0..=2 runs: a left-aligned title and
/// a right-aligned group, each with a valid (non-empty) clip rect and an
/// in-bounds x. The right run is right-aligned but never starts before the left
/// text plus `gap`, and its clip extends to the strip's hard right edge so a
/// small width estimate can never cull it.
pub fn plan_status_header(
    title: &str,
    right_segments: &[String],
    rect_x: f32,
    rect_w: f32,
    pad: f32,
    advance: f32,
    gap: f32,
) -> Vec<HeaderRun> {
    let avail = (rect_w - pad * 2.0).max(0.0);
    let fit = fit_status_header(title, right_segments, avail, advance, gap);

    let inner_left = rect_x + pad;
    let inner_right = rect_x + rect_w - pad;
    let hard_right = rect_x + rect_w;

    let mut runs = Vec::new();

    // Right run x: right-align, but never overlap the left text + gap.
    let right_x = if fit.right.is_empty() {
        inner_right
    } else {
        (inner_right - fit.right_width).max(inner_left + fit.left_width + gap)
    };

    if !fit.left.is_empty() {
        // Left clip ends just before the right run (or the inner right edge).
        let left_clip_right =
            if fit.right.is_empty() { inner_right } else { (right_x - gap * 0.5).max(inner_left) };
        let clip_w = (left_clip_right - inner_left).max(1.0);
        runs.push(HeaderRun { text: fit.left, x: inner_left, clip_x: inner_left, clip_w });
    }

    if !fit.right.is_empty() {
        // Clip from a hair before the run's x (so a sub-pixel boundary can never
        // cull the first glyph) to the strip's hard right edge (so an imperfect
        // width estimate can never clip the run away).
        let clip_x = (right_x - 2.0).max(inner_left);
        let clip_w = (hard_right - clip_x).max(1.0);
        runs.push(HeaderRun { text: fit.right, x: right_x, clip_x, clip_w });
    }

    runs
}

/// Truncate `s` to fit `budget` logical px, appending an ellipsis when cut.
fn ellipsize(s: &str, budget: f32, advance: f32) -> String {
    if budget < advance {
        return String::new();
    }
    let max_chars = (budget / advance).floor() as usize;
    let total = s.chars().count();
    if total <= max_chars {
        return s.to_string();
    }
    if max_chars == 0 {
        return String::new();
    }
    let keep = max_chars.saturating_sub(1); // reserve one cell for the ellipsis
    let mut out: String = s.chars().take(keep).collect();
    out.push('\u{2026}');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    const ADV: f32 = 8.0;
    const GAP: f32 = 16.0;

    #[test]
    fn wide_keeps_both_groups_separate() {
        let right = segs(&["Ln 1, Col 1", "Sel 5", "Rust"]);
        let fit = fit_status_header("workspace", &right, 1000.0, ADV, GAP);
        assert_eq!(fit.left, "workspace", "left label kept intact when wide");
        assert!(fit.right.contains("Ln 1, Col 1") && fit.right.contains("Rust"));
        // No overlap: left + gap + right fits the available width.
        assert!(fit.left_width + GAP + fit.right_width <= 1000.0);
    }

    #[test]
    fn narrow_drops_lowest_priority_right_first() {
        let right = segs(&["Ln 1, Col 1", "Sel 5", "Rust"]);
        let fit = fit_status_header("ws", &right, 200.0, ADV, GAP);
        assert!(fit.right.contains("Ln 1, Col 1"), "highest-priority right field survives");
        assert!(!fit.right.contains("Rust"), "lowest-priority right field dropped first");
        assert!(fit.left_width + GAP + fit.right_width <= 200.0, "must not overlap");
    }

    #[test]
    fn very_narrow_truncates_left_with_ellipsis() {
        let right = segs(&["Ln 1, Col 1", "Sel 5", "Rust"]);
        let fit = fit_status_header("myworkspace", &right, 40.0, ADV, GAP);
        assert!(
            fit.left.ends_with('\u{2026}'),
            "left label truncated with ellipsis: {:?}",
            fit.left
        );
        assert!(fit.left_width <= 40.0, "truncated left must fit available width");
        assert!(fit.right.is_empty(), "all right fields dropped at extreme narrowness");
    }

    #[test]
    fn no_overlap_invariant_across_widths() {
        let right = segs(&["Ln 1, Col 1", "Sel 5", "Rust", "LF", "Spaces: 4", "UTF-8"]);
        for avail in [60.0_f32, 120.0, 240.0, 480.0, 960.0] {
            let fit = fit_status_header("project-name", &right, avail, ADV, GAP);
            let used =
                fit.left_width + if fit.right.is_empty() { 0.0 } else { GAP } + fit.right_width;
            assert!(used <= avail + 0.01, "overlap at avail={avail}: used={used}");
        }
    }

    #[test]
    fn wide_plan_emits_both_runs_with_valid_clips() {
        let right = segs(&["Ln 1, Col 1", "Sel 5", "Rust"]);
        let runs = plan_status_header("ws", &right, 0.0, 1000.0, 8.0, ADV, GAP);
        assert_eq!(runs.len(), 2, "wide strip emits a left and a right run");
        assert_eq!(runs[0].text, "ws");
        assert!(runs[1].text.contains("Ln 1, Col 1"), "right run carries the right group");
        // Both clips are non-empty and in-bounds.
        for run in &runs {
            assert!(run.clip_w > 0.0, "clip width must be positive: {run:?}");
            assert!(run.x >= 0.0 && run.x <= 1000.0, "x in bounds: {run:?}");
            assert!(run.clip_x + run.clip_w <= 1000.0 + 0.01, "clip within strip: {run:?}");
        }
        // No overlap: left clip ends before the right run begins.
        assert!(runs[0].clip_x + runs[0].clip_w <= runs[1].x + 0.01, "left/right must not overlap");
        // Right run is right-aligned (near the right edge).
        assert!(runs[1].x > 500.0, "right run is right-aligned: {:?}", runs[1]);
    }

    #[test]
    fn narrow_plan_still_emits_right_run_with_fewer_segments() {
        let right = segs(&["Ln 1, Col 1", "Sel 5", "Rust"]);
        let runs = plan_status_header("ws", &right, 0.0, 200.0, 8.0, ADV, GAP);
        assert_eq!(runs.len(), 2, "narrow strip still emits a right run");
        let right_run = &runs[1];
        assert!(right_run.text.contains("Ln 1, Col 1"), "highest-priority field kept");
        assert!(!right_run.text.contains("Rust"), "lowest-priority field dropped");
        assert!(right_run.clip_w > 0.0, "right clip non-empty when right text exists");
    }

    #[test]
    fn very_narrow_plan_emits_left_only() {
        let right = segs(&["Ln 1, Col 1", "Sel 5", "Rust"]);
        let runs = plan_status_header("myworkspace", &right, 0.0, 40.0, 8.0, ADV, GAP);
        assert_eq!(runs.len(), 1, "only the (truncated) left run survives extreme narrowness");
        assert!(runs[0].text.ends_with('\u{2026}'), "left truncated: {:?}", runs[0].text);
        assert!(runs[0].clip_w > 0.0);
    }
}
