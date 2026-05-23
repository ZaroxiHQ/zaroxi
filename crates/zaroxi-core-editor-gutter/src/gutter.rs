use zaroxi_kernel_math::Rect;

/// Minimal gutter model: owns a configured width and computes layout helpers.
#[derive(Clone, Debug)]
pub struct GutterModel {
    pub width: u32,
}

impl GutterModel {
    /// Construct a gutter model with the given pixel width.
    pub fn new(width: u32) -> Self {
        GutterModel { width }
    }

    /// Number of pixels reserved at the left of the content for the gutter.
    pub fn content_inset(&self) -> u32 {
        self.width
    }

    /// Compute the gutter rectangle (absolute window-space) given the full editor rect.
    pub fn gutter_rect(&self, editor_rect: Rect) -> Rect {
        // Place gutter at the left side of the editor rect.
        Rect::new(editor_rect.x, editor_rect.y, self.width as f32, editor_rect.height)
    }

    /// Compute the content (text) rect inside the editor after reserving gutter space.
    pub fn content_rect(&self, editor_rect: Rect) -> Rect {
        let inset = self.width as f32;
        let content_w = (editor_rect.width - inset).max(0.0);
        Rect::new(editor_rect.x + inset, editor_rect.y, content_w, editor_rect.height)
    }

    /// Format a 1-based logical line number into a narrow right-aligned label.
    ///
    /// The formatting here is intentionally deterministic and stable: it produces
    /// a 4-character, right-aligned string. Presenters may use this label when
    /// composing gutter paint operations.
    pub fn line_number_string(&self, line_one_based: u32) -> String {
        format!("{:>4}", line_one_based)
    }
}
