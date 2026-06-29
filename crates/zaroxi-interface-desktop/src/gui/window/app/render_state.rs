use std::collections::HashMap;

use crate::gui::ShellWorkContent;
use crate::gui::window::editor::EditorContentData;
use zaroxi_core_editor_largefile::DocumentBuffer;
use zaroxi_core_editor_rope::Rope;
use zaroxi_core_platform_syntax::highlight::HighlightSpan;
use zaroxi_interface_theme::theme::SemanticColors;

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

pub(crate) fn should_use_editor_cache(lines_hash: u64, cached_hash: u64) -> bool {
    lines_hash > 0 && lines_hash == cached_hash
}

pub(crate) fn editor_spans_debug_enabled() -> bool {
    std::env::var("ZAROXI_DEBUG_EDITOR_SPANS").as_deref() == Ok("1")
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_editor_data(
    work_content: &Option<ShellWorkContent>,
    cached_editor_data: &mut Option<EditorContentData>,
    cached_editor_lines_hash: &mut u64,
    cached_editor_spans_version: &mut u64,
    spans: &[HighlightSpan],
    spans_version: u64,
    sem: &SemanticColors,
    line_syntax_cache: &mut HashMap<(usize, u64), Vec<(String, [f32; 4])>>,
    cached_line_hashes: &mut Vec<u64>,
    large_file_mode: bool,
    visible_line_range: Option<(usize, usize)>,
    rope: Option<&Rope>,
    _doc_buffer: Option<&DocumentBuffer>,
    buffer_version: u64,
    wrap_chars_per_row: usize,
    total_lines_override: Option<usize>,
) -> EditorContentData {
    // Single path for both Rope and PieceTable backends.
    // For large files the rope holds only the viewport window, so
    // content hashing and per-line hashing are O(1).  For small files
    // the rope holds the full document.  Syntax spans apply equally.
    let mut lines_hash = compute_lines_hash(work_content);
    if let Some((start, end)) = visible_line_range {
        lines_hash = lines_hash.wrapping_mul(31).wrapping_add(start as u64);
        lines_hash = lines_hash.wrapping_mul(31).wrapping_add(end as u64);
    }
    // Mix buffer_version so edits always invalidate the cache for
    // large files (where the content hash computed from the viewport
    // window may not change on a mid-viewport edit).
    if large_file_mode {
        lines_hash = lines_hash.wrapping_mul(31).wrapping_add(buffer_version);
    }
    let per_line_hashes = compute_per_line_hashes(work_content);

    let cache_valid = should_use_editor_cache(lines_hash, *cached_editor_lines_hash)
        && spans_version == *cached_editor_spans_version;

    if cache_valid {
        if let Some(cached) = cached_editor_data {
            if editor_spans_debug_enabled() {
                eprintln!(
                    "ZAROXI_DEBUG_EDITOR_SPANS: cache_hit spans_version={} has_spans={} large_file_mode={}",
                    spans_version,
                    cached.editor_spans.is_some(),
                    large_file_mode,
                );
            }
            return cached.clone();
        }
    }

    // Always use the incremental shaper since both backends now
    // have a viewport-sized rope supporting per-line hashes and
    // syntax highlighting within the visible window.
    let data = super::super::presenters::shape_editor_content_incremental(
        work_content,
        sem,
        spans,
        line_syntax_cache,
        &per_line_hashes,
        cached_line_hashes,
        visible_line_range,
        rope,
        wrap_chars_per_row,
        total_lines_override,
    );

    if editor_spans_debug_enabled() {
        eprintln!(
            "ZAROXI_DEBUG_EDITOR_SPANS: rebuild spans_in={} spans_version={} editor_spans_segments={:?} visible_range={:?} large_file_mode={}",
            spans.len(),
            spans_version,
            data.editor_spans.as_ref().map(|s| s.len()),
            data.visible_line_range,
            large_file_mode,
        );
    }

    if std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1") {
        eprintln!(
            "ZAROXI_DEBUG_LARGE_FILE: prepare lines={} bytes={} hash={:016x} has_spans={} visible_range={:?}",
            data.total_lines,
            data.editor_body_text.len(),
            lines_hash,
            data.editor_spans.is_some(),
            data.visible_line_range,
        );
    }

    *cached_editor_data = Some(data.clone());
    *cached_editor_lines_hash = lines_hash;
    *cached_editor_spans_version = spans_version;
    *cached_line_hashes = per_line_hashes;

    data
}
