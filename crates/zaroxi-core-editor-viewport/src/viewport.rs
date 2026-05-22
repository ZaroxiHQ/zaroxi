use std::cmp;

/// Tiny, deterministic editor viewport math.
///
/// All positions returned by these helpers are in pixels and line_to_y returns
/// a y-position relative to the content top (i.e. 0.0 corresponds to the top of
/// the content region).
#[derive(Clone, Debug)]
pub struct EditorViewport {
    /// viewport width in pixels (may be unused by simple math but kept for completeness)
    pub width: u32,
    /// viewport height in pixels
    pub height: u32,
    /// logical line height in pixels (floating point to allow fractional metrics)
    pub line_height: f32,
    /// reserved gutter width in pixels (for convenience; used by presenters)
    pub gutter_width: u32,
}

impl EditorViewport {
    /// Create a new EditorViewport.
    pub fn new(width: u32, height: u32, line_height: f32, gutter_width: u32) -> Self {
        EditorViewport {
            width,
            height,
            line_height: if line_height > 0.0 { line_height } else { 1.0 },
            gutter_width,
        }
    }

    /// Compute the inclusive visible line range for the given scroll Y (pixels).
    ///
    /// Behavior:
    /// - scroll_y is the number of pixels the content has been scrolled upward.
    /// - first_line is floor(scroll_y / line_height), clamped to zero.
    /// - last_line is computed so that all lines with any visible pixels inside
    ///   the viewport are included (conservative: uses ceil).
    pub fn visible_line_range(&self, scroll_y: f32) -> (u32, u32) {
        let first = (scroll_y / self.line_height).floor() as i64;
        let first = if first < 0 { 0 } else { first as u32 };
        // number of lines that can fit (at least)
        let lines_fit = ((self.height as f32) / self.line_height).ceil() as u32;
        let last = first.saturating_add(lines_fit.saturating_sub(1));
        (first, last)
    }

    /// How many whole lines roughly fit in the viewport (floor).
    pub fn total_lines_fit(&self) -> u32 {
        ((self.height as f32) / self.line_height).floor() as u32
    }

    /// Map a (0-based) line index to a y-position (relative to content top)
    /// taking the scroll_y into account. The result is in pixels and may be
    /// negative (if the line is above the viewport) or >height (if below).
    pub fn line_to_y(&self, line_index: u32, scroll_y: f32) -> f32 {
        (line_index as f32) * self.line_height - scroll_y
    }

    /// Produce a vector of (line_index, y) for visible lines clipped by total_lines.
    /// y is relative to content top.
    pub fn visible_line_positions(&self, scroll_y: f32, total_lines: u32) -> Vec<(u32, f32)> {
        let (first, last_est) = self.visible_line_range(scroll_y);
        if total_lines == 0 {
            return Vec::new();
        }
        let last = std::cmp::min(last_est, total_lines.saturating_sub(1));
        let mut out = Vec::new();
        for li in first..=last {
            let y = self.line_to_y(li, scroll_y);
            out.push((li, y));
        }
        out
    }
}
