use std::collections::HashMap;

use crate::gui::ShellWorkContent;
use crate::gui::window::editor::EditorContentData;
use zaroxi_core_editor_rope::Rope;
use zaroxi_core_platform_syntax::parser::ParserPool;
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

pub(crate) fn compute_lines_hash_fast(
    work_content: &Option<ShellWorkContent>,
    rope: Option<&Rope>,
) -> u64 {
    let total = rope.map(|r| r.line_count()).unwrap_or_else(|| {
        work_content
            .as_ref()
            .and_then(|wc| wc.editor_body.as_ref())
            .map(|cv| cv.lines.len())
            .unwrap_or(0)
    });
    if total == 0 {
        return 0;
    }
    let n = total;
    let idx = |frac: f32| -> usize { ((n - 1) as f32 * frac).round() as usize };
    let sample = |i: usize| -> u64 {
        if let Some(r) = rope {
            r.line(i).map(|l| l.len() as u64).unwrap_or(0)
        } else {
            work_content
                .as_ref()
                .and_then(|wc| wc.editor_body.as_ref())
                .and_then(|cv| cv.lines.get(i).map(|l| l.len() as u64))
                .unwrap_or(0)
        }
    };
    let mut h: u64 = n as u64;
    h = h.wrapping_mul(31).wrapping_add(sample(idx(0.00)));
    h = h.wrapping_mul(31).wrapping_add(sample(idx(0.25)));
    h = h.wrapping_mul(31).wrapping_add(sample(idx(0.50)));
    h = h.wrapping_mul(31).wrapping_add(sample(idx(0.75)));
    h = h.wrapping_mul(31).wrapping_add(sample(idx(1.00)));
    h
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

pub(crate) fn prepare_editor_data(
    work_content: &Option<ShellWorkContent>,
    cached_editor_data: &mut Option<EditorContentData>,
    cached_editor_lines_hash: &mut u64,
    parser_pool: &ParserPool,
    sem: &SemanticColors,
    line_syntax_cache: &mut HashMap<(usize, u64), Vec<(String, [f32; 4])>>,
    cached_line_hashes: &mut Vec<u64>,
    large_file_mode: bool,
    visible_line_range: Option<(usize, usize)>,
    rope: Option<&Rope>,
) -> EditorContentData {
    if large_file_mode {
        let lines_hash = compute_lines_hash_fast(work_content, rope);

        if should_use_editor_cache(lines_hash, *cached_editor_lines_hash) {
            if cached_editor_data.is_some() {
                return cached_editor_data.clone().unwrap();
            }
        }

        cached_line_hashes.clear();
        line_syntax_cache.clear();

        let data = super::super::presenters::shape_editor_content_plain(
            work_content,
            sem,
            visible_line_range,
            rope,
        );

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
        return data;
    }

    let lines_hash = compute_lines_hash(work_content);
    let per_line_hashes = compute_per_line_hashes(work_content);

    if should_use_editor_cache(lines_hash, *cached_editor_lines_hash) {
        return cached_editor_data.clone().unwrap_or_else(|| {
            super::super::presenters::shape_editor_content(
                work_content,
                sem,
                parser_pool,
                visible_line_range,
                rope,
            )
        });
    }

    let data = super::super::presenters::shape_editor_content_incremental(
        work_content,
        sem,
        parser_pool,
        line_syntax_cache,
        &per_line_hashes,
        cached_line_hashes,
        visible_line_range,
        rope,
    );

    *cached_editor_data = Some(data.clone());
    *cached_editor_lines_hash = lines_hash;
    *cached_line_hashes = per_line_hashes;

    data
}
