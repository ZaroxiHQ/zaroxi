use super::types::Selection;

/// Selection helpers: normalization and emptiness checks.
impl Selection {
    /// Normalize selection into (start_line, start_col, end_line, end_col)
    /// where the start is document-before the end. Both start and end are
    /// 0-based and end is the exclusive end location.
    pub fn normalized(&self) -> (usize, usize, usize, usize) {
        if (self.anchor_line, self.anchor_col) <= (self.active_line, self.active_col) {
            (self.anchor_line, self.anchor_col, self.active_line, self.active_col)
        } else {
            (self.active_line, self.active_col, self.anchor_line, self.anchor_col)
        }
    }

    /// Convenience: check if selection is empty.
    pub fn is_empty(&self) -> bool {
        self.normalized() == (self.anchor_line, self.anchor_col, self.anchor_line, self.anchor_col)
    }
}
