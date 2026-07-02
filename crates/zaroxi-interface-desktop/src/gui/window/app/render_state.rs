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

/// A stable render-artifact owner key: canonical active file path + render
/// source kind. Every reusable editor render payload is keyed (at least) by
/// this so a payload built for one file/source can never be reused for
/// another, regardless of matching geometry, line count, or content hash.
pub(crate) fn editor_render_owner_key(owner_path: Option<&str>, large_file_mode: bool) -> String {
    let path = owner_path.map(|s| s.strip_prefix("buf:").unwrap_or(s)).unwrap_or("<none>");
    let source = if large_file_mode { "large" } else { "rope" };
    format!("{path}\u{1}{source}")
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
    line_syntax_cache: &mut crate::gui::window::syntax_color::LineSyntaxCache,
    cached_line_hashes: &mut Vec<u64>,
    large_file_mode: bool,
    visible_line_range: Option<(usize, usize)>,
    rope: Option<&Rope>,
    _doc_buffer: Option<&DocumentBuffer>,
    buffer_version: u64,
    wrap_chars_per_row: usize,
    total_lines_override: Option<usize>,
    // Canonical content owner (active file) + true-owner-switch epoch. Both
    // are folded into the cache key so no artifact from a previous owner can
    // be reused after a file switch — geometry/hash matches are never enough.
    owner_path: Option<&str>,
    content_generation: u64,
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
    // Fold the canonical owner identity + owner-switch epoch into the key.
    // This is the hard guarantee against cross-file reuse: two different
    // files (or the same path before/after a true owner switch) can never
    // collide onto the same cache entry even if their content hashes match.
    let owner_key = editor_render_owner_key(owner_path, large_file_mode);
    for &b in owner_key.as_bytes() {
        lines_hash = lines_hash.wrapping_mul(31).wrapping_add(b as u64);
    }
    lines_hash = lines_hash.wrapping_mul(1099511628211).wrapping_add(content_generation);
    let per_line_hashes = compute_per_line_hashes(work_content);

    let cache_valid = should_use_editor_cache(lines_hash, *cached_editor_lines_hash)
        && spans_version == *cached_editor_spans_version;

    if cache_valid && let Some(cached) = cached_editor_data {
        if std::env::var("ZAROXI_DEBUG_TABS").as_deref() == Ok("1") {
            eprintln!("ZAROXI_TABS: editor_cache_reuse owner={owner_key} allowed=true",);
        }
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
    if std::env::var("ZAROXI_DEBUG_TABS").as_deref() == Ok("1") {
        eprintln!(
            "ZAROXI_TABS: editor_payload_build owner={owner_key} source={} version={}",
            if large_file_mode { "large_file_viewport" } else { "rope" },
            content_generation,
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::ShellWorkContent;
    use zaroxi_core_editor_rope::Rope;
    use zaroxi_interface_theme::theme::SemanticColors;

    fn wc_with(lines: &[&str]) -> Option<ShellWorkContent> {
        let wc = ShellWorkContent {
            editor_body: Some(zaroxi_core_engine_ui::ContentView {
                lines: lines.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            }),
            ..Default::default()
        };
        Some(wc)
    }

    #[allow(clippy::too_many_arguments)]
    fn call(
        wc: &Option<ShellWorkContent>,
        rope: &Rope,
        cache: &mut Option<EditorContentData>,
        lines_hash: &mut u64,
        spans_version: &mut u64,
        owner: Option<&str>,
        content_generation: u64,
    ) -> u64 {
        let sem = SemanticColors::dark();
        let mut lsc = crate::gui::window::syntax_color::LineSyntaxCache::new();
        let mut clh = Vec::new();
        prepare_editor_data(
            wc,
            cache,
            lines_hash,
            spans_version,
            &[],
            0,
            &sem,
            &mut lsc,
            &mut clh,
            false,
            Some((
                0,
                wc.as_ref()
                    .and_then(|w| w.editor_body.as_ref())
                    .map(|b| b.lines.len())
                    .unwrap_or(0),
            )),
            Some(rope),
            None,
            1,
            80,
            None,
            owner,
            content_generation,
        );
        *lines_hash
    }

    #[test]
    fn owner_key_distinguishes_files_sources_and_normalizes_prefix() {
        // Different files -> different keys, even with identical everything else.
        assert_ne!(
            editor_render_owner_key(Some("/a.rs"), false),
            editor_render_owner_key(Some("/b.rs"), false),
        );
        // Same path, different render source -> different key.
        assert_ne!(
            editor_render_owner_key(Some("/a.rs"), false),
            editor_render_owner_key(Some("/a.rs"), true),
        );
        // `buf:` transport prefix is normalized: same document -> same key.
        assert_eq!(
            editor_render_owner_key(Some("buf:/a.rs"), false),
            editor_render_owner_key(Some("/a.rs"), false),
        );
    }

    #[test]
    fn same_geometry_different_file_forces_rebuild() {
        // Case 1 & 2: identical geometry + identical line count, different file.
        // The cached payload for file A must NOT be reusable for file B.
        let lines = ["fn main() {}", "// same shape"];
        let wc = wc_with(&lines);
        let rope = Rope::new(&lines.join("\n"));
        let mut cache = None;
        let mut lines_hash = 0u64;
        let mut spans_version = 0u64;

        let key_a =
            call(&wc, &rope, &mut cache, &mut lines_hash, &mut spans_version, Some("/a.rs"), 0);
        assert!(cache.is_some());
        // Same owner + same content => cache key stable (reuse path).
        let key_a2 =
            call(&wc, &rope, &mut cache, &mut lines_hash, &mut spans_version, Some("/a.rs"), 0);
        assert_eq!(key_a, key_a2, "same file + geometry must reuse");
        // Different owner, identical geometry => key MUST change (no reuse).
        let key_b =
            call(&wc, &rope, &mut cache, &mut lines_hash, &mut spans_version, Some("/b.rs"), 0);
        assert_ne!(key_a, key_b, "different file with same geometry must rebuild");
    }

    #[test]
    fn content_generation_bump_forces_rebuild_same_path() {
        // A true owner switch bumps content_generation; even if the path text
        // were identical, the epoch change invalidates the artifact.
        let lines = ["x"];
        let wc = wc_with(&lines);
        let rope = Rope::new("x");
        let mut cache = None;
        let mut lines_hash = 0u64;
        let mut spans_version = 0u64;
        let k0 =
            call(&wc, &rope, &mut cache, &mut lines_hash, &mut spans_version, Some("/a.rs"), 0);
        let k1 =
            call(&wc, &rope, &mut cache, &mut lines_hash, &mut spans_version, Some("/a.rs"), 1);
        assert_ne!(k0, k1, "content_generation epoch must invalidate the cache key");
    }

    #[test]
    fn same_path_same_epoch_reuses() {
        // Case 3: same-path preview->pin does NOT bump the epoch, so the payload
        // is preserved (no needless rebuild / view reset).
        let lines = ["a", "b", "c"];
        let wc = wc_with(&lines);
        let rope = Rope::new(&lines.join("\n"));
        let mut cache = None;
        let mut lines_hash = 0u64;
        let mut spans_version = 0u64;
        let k0 =
            call(&wc, &rope, &mut cache, &mut lines_hash, &mut spans_version, Some("/a.rs"), 3);
        let k1 =
            call(&wc, &rope, &mut cache, &mut lines_hash, &mut spans_version, Some("/a.rs"), 3);
        assert_eq!(k0, k1, "same path + same epoch must reuse the payload");
    }
}
