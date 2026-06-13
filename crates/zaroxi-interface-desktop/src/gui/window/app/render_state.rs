/*!
Render state helpers — content hashing, editor-data caching.

Extracted from app.rs to keep render-preparation logic out of the
winit event loop.  Now supports per-line syntax caching so that
a single-line edit only recomputes spans for the changed line.
*/

use std::collections::HashMap;

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

/// Compute per-line content hashes (fnv-like) for incremental syntax caching.
pub(crate) fn compute_per_line_hashes(work_content: &Option<ShellWorkContent>) -> Vec<u64> {
    work_content
        .as_ref()
        .and_then(|wc| wc.editor_body.as_ref())
        .map(|cv| {
            cv.lines
                .iter()
                .map(|line| {
                    let mut h: u64 = 0xcbf29ce484222325;
                    for &b in line.as_bytes() {
                        h ^= b as u64;
                        h = h.wrapping_mul(0x100000001b3);
                    }
                    h
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Returns true when a cached `EditorContentData` is still valid for
/// the given content hash.
pub(crate) fn should_use_editor_cache(lines_hash: u64, cached_hash: u64) -> bool {
    lines_hash > 0 && lines_hash == cached_hash
}

/// Resolve editor content data, populating or using caches.
///
/// Now supports incremental per-line syntax caching: only lines whose content
/// hash changed are re-colored.  The cache is keyed by (line_index, line_hash)
/// and lives in `line_syntax_cache`.
pub(crate) fn prepare_editor_data(
    work_content: &Option<ShellWorkContent>,
    cached_editor_data: &mut Option<EditorContentData>,
    cached_editor_lines_hash: &mut u64,
    parser_pool: &ParserPool,
    sem: &SemanticColors,
    line_syntax_cache: &mut HashMap<(usize, u64), Vec<(String, [f32; 4])>>,
    cached_line_hashes: &mut Vec<u64>,
) -> EditorContentData {
    let lines_hash = compute_lines_hash(work_content);
    let per_line_hashes = compute_per_line_hashes(work_content);

    // If the coarse hash is unchanged, return the fully cached result.
    if should_use_editor_cache(lines_hash, *cached_editor_lines_hash) {
        return cached_editor_data.clone().unwrap_or_else(|| {
            super::super::presenters::shape_editor_content(work_content, sem, parser_pool)
        });
    }

    // Build editor data with incremental per-line span caching.
    let data = super::super::presenters::shape_editor_content_incremental(
        work_content,
        sem,
        parser_pool,
        line_syntax_cache,
        &per_line_hashes,
        cached_line_hashes,
    );

    *cached_editor_data = Some(data.clone());
    *cached_editor_lines_hash = lines_hash;
    *cached_line_hashes = per_line_hashes;

    data
}
