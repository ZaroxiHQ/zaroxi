#![allow(dead_code)]
#![allow(unused_imports)]
// Editor view composition: composes gutter + viewport math to produce a stable
// layout for the editor content region and visible-line positions.

mod view;
pub use view::{EditorView, EditorViewLayout, EditorViewState};

pub mod render_contract;
pub use render_contract::{EditorRenderContract, EditorRenderMetrics};

#[cfg(test)]
mod tests {
    use crate::{EditorView, EditorViewLayout};
    use zaroxi_core_editor_gutter::GutterModel;
    use zaroxi_core_editor_viewport::EditorViewport;
    use zaroxi_kernel_math::Rect;

    #[test]
    fn layout_produces_positions() {
        let viewport = EditorViewport::new(800, 200, 16.0, 48);
        let gutter = GutterModel::new(48);
        let ev = EditorView::new(viewport, gutter);
        let editor_rect = Rect::new(10.0, 20.0, 780.0, 200.0);
        let layout = ev.layout(editor_rect, 0.0, 500);
        assert!(!layout.line_positions.is_empty());
        assert!(layout.content_rect.width <= editor_rect.width);
    }
}
