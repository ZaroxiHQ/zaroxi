/*!
Render state helpers — content hashing, editor-data caching.

Extracted from app.rs to keep render-preparation logic out of the
winit event loop.
*/

use crate::gui::ShellWorkContent;
use crate::gui::window::editor::EditorContentData;
use zaroxi_core_platform_syntax::parser::ParserPool;
use zaroxi_interface_theme::theme::SemanticColors;

/// Compute a fast non-cryptographic hash over editor-body line lengths.
///
/// Zero is returned when there is no editor content — this acts as
/// an "uncacheable" sentinel.
pub(crate) fn compute_lines_hash(work_content: &Option<ShellWorkContent>) -> u64 {
    work_content
        .as_ref()
        .and_then(|wc| wc.editor_body.as_ref())
        .map(|cv| {
            let mut h: u64 = 0;
            for line in cv.lines.iter() {
                h = h.wrapping_mul(31).wrapping_add(line.len() as u64);
            }
            h
        })
        .unwrap_or(0)
}

/// Returns true when a cached `EditorContentData` is still valid for
/// the given content hash.
pub(crate) fn should_use_editor_cache(lines_hash: u64, cached_hash: u64) -> bool {
    lines_hash > 0 && lines_hash == cached_hash
}

/// Resolve editor content data, populating or using caches.
///
/// Takes specific fields (not the entire `GuiApp`) so that the borrow
/// checker can distinguish from concurrent field-level borrows (e.g.
/// `self.maybe_window.as_mut()` inside `RedrawRequested`).
pub(crate) fn prepare_editor_data(
    work_content: &Option<ShellWorkContent>,
    cached_editor_data: &mut Option<EditorContentData>,
    cached_editor_lines_hash: &mut u64,
    parser_pool: &ParserPool,
    sem: &SemanticColors,
) -> EditorContentData {
    let lines_hash = compute_lines_hash(work_content);
    let use_cache = should_use_editor_cache(lines_hash, *cached_editor_lines_hash);

    if use_cache {
        cached_editor_data.clone().unwrap_or_else(|| {
            super::super::presenters::shape_editor_content(work_content, sem, parser_pool)
        })
    } else {
        let data = super::super::presenters::shape_editor_content(work_content, sem, parser_pool);
        *cached_editor_data = Some(data.clone());
        *cached_editor_lines_hash = lines_hash;
        data
    }
}
