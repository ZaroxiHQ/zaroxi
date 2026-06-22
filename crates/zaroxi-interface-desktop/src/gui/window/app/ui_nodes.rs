//! Retained per-element UI-node tracking + `ZAROXI_UI_TRACE` instrumentation.
//!
//! Each major shell element (editor text viewport, gutter / line numbers,
//! status bar, chrome / tab bar, side panel, AI pane, bottom dock) is treated
//! as a retained node with its own content fingerprint. Every frame we
//! recompute each node's fingerprint from the composed render blocks and
//! compare it to the previous frame to decide which nodes are *dirty* (must be
//! re-emitted) and which are *reused*.
//!
//! Crucially the fingerprint mirrors exactly what the text renderer hashes per
//! element bucket (text, position, clip, colors, scroll offset) and
//! deliberately **excludes** cursor line/column and selection range — those are
//! drawn as overlay quads, not glyphs, so a caret blink or selection change
//! must not mark the editor text node dirty. This keeps the app-side dirty view
//! consistent with the renderer's per-element draw-payload reuse.
//!
//! This module is observability only: it never changes what is drawn. It is a
//! no-op unless `ZAROXI_UI_TRACE=1`.

use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_render::renderer::core::RenderPerf;

use super::render_schedule::InvalidationFlags;

/// Number of tracked UI-element classes (see [`element_class`]).
const ELEMENT_COUNT: usize = 8;

const EL_EDITOR: usize = 0;
const EL_GUTTER: usize = 1;
const EL_STATUS: usize = 2;
const EL_CHROME: usize = 3;
const EL_SIDE: usize = 4;
const EL_AI: usize = 5;
const EL_BOTTOM: usize = 6;
const EL_OTHER: usize = 7;

const ELEMENT_LABELS: [&str; ELEMENT_COUNT] =
    ["editor", "gutter", "status", "chrome", "side", "ai", "bottom", "other"];

/// Classify a render block id into a UI-element class index. Mirrors
/// `zaroxi_core_engine_render`'s internal `element_for_block` so the app-side
/// dirty view lines up with the renderer's per-element cache bucketing.
fn element_class(id: &str) -> usize {
    if id == "editor_content" || id.contains("ContentArea") || id.contains("content_area") {
        EL_EDITOR
    } else if id.contains("gutter") {
        EL_GUTTER
    } else if id.contains("status") {
        EL_STATUS
    } else if id.contains("ai_panel") || id.starts_with("ai_") {
        EL_AI
    } else if id.contains("explorer") || id.contains("sidebar") || id.contains("side_panel") {
        EL_SIDE
    } else if id.contains("bottom") || id.contains("terminal") {
        EL_BOTTOM
    } else if id == "toolbar"
        || id.contains("titlebar")
        || id.contains("title_bar")
        || id.contains("title-bar")
        || id.contains("tab")
        || id.contains("header")
        || id.contains("rail")
        || id.contains("chrome")
    {
        EL_CHROME
    } else {
        EL_OTHER
    }
}

#[inline]
fn mix(h: &mut u64, v: u64) {
    *h ^= v;
    *h = h.wrapping_mul(0x100000001b3);
}

/// Fold one block's glyph-affecting fields into a per-element fingerprint.
/// Deliberately excludes `cursor_line/col` and `selection_range` (overlay
/// quads, not glyphs) so caret/selection changes do not mark text nodes dirty.
fn fold_block(h: &mut u64, block: &UiBlock) {
    if !block.visible {
        return;
    }
    for &b in block.id.as_bytes() {
        mix(h, b as u64);
    }
    for &b in block.title.as_bytes() {
        mix(h, b as u64);
    }
    for &b in block.content.as_bytes() {
        mix(h, b as u64);
    }
    mix(h, (block.rect.x * 100.0) as i64 as u64);
    mix(h, (block.rect.y * 100.0) as i64 as u64);
    mix(h, (block.rect.w * 100.0) as i64 as u64);
    mix(h, (block.rect.h * 100.0) as i64 as u64);
    mix(h, (block.content_offset_x * 100.0) as i64 as u64);
    mix(h, (block.content_offset_y * 100.0) as i64 as u64);
    if let Some(off) = block.content_line_offset {
        mix(h, off as u64);
    }
    if let Some(ref spans) = block.content_spans {
        for (t, col) in spans {
            mix(h, t.len() as u64);
            for c in col {
                mix(h, c.to_bits() as u64);
            }
        }
    }
}

/// Count colored span regions in a block (used for `syntax_commit_regions`).
fn span_regions(block: &UiBlock) -> usize {
    block.content_spans.as_ref().map(|s| s.iter().filter(|(t, _)| t != "\n").count()).unwrap_or(0)
}

/// Retained per-element fingerprint state. Lives across frames so each node's
/// dirtiness can be derived by comparing its current content fingerprint to the
/// previous frame's.
#[derive(Default)]
pub struct UiNodeTracker {
    last_fp: [u64; ELEMENT_COUNT],
    present: [bool; ELEMENT_COUNT],
    last_size: (u32, u32),
    last_theme_dark: Option<bool>,
    initialized: bool,
}

/// Whether `ZAROXI_UI_TRACE=1` is set.
pub fn ui_trace_enabled() -> bool {
    std::env::var("ZAROXI_UI_TRACE").as_deref() == Ok("1")
}

impl UiNodeTracker {
    /// Recompute per-element fingerprints from this frame's composed blocks,
    /// diff against the retained state, and emit one `ZAROXI_UI_TRACE` line
    /// describing exactly which UI nodes rebuilt vs. reused and why. No-op
    /// unless `ZAROXI_UI_TRACE=1`.
    ///
    /// `perf` (when present) carries the renderer's own per-element bucket reuse
    /// and GPU-upload accounting so the line cross-references the app-side dirty
    /// view with the renderer's actual draw-payload reuse.
    #[allow(clippy::too_many_arguments)]
    pub fn record_frame(
        &mut self,
        frame_id: u64,
        blocks: &[UiBlock],
        size: (u32, u32),
        theme_dark: bool,
        flags: InvalidationFlags,
        editor_visible_lines: usize,
        perf: Option<&RenderPerf>,
    ) {
        if !ui_trace_enabled() {
            // Keep retained state cheap-but-current so the first traced frame
            // after toggling does not report a spurious whole-shell rebuild.
            self.last_size = size;
            self.last_theme_dark = Some(theme_dark);
            return;
        }

        // Compute current fingerprints + region counts per element class.
        let mut fp = [0u64; ELEMENT_COUNT];
        let mut present = [false; ELEMENT_COUNT];
        let mut editor_regions = 0usize;
        for block in blocks {
            if !block.visible {
                continue;
            }
            let class = element_class(&block.id);
            present[class] = true;
            fold_block(&mut fp[class], block);
            if class == EL_EDITOR {
                editor_regions += span_regions(block);
            }
        }

        // Geometry / style changes invalidate every node regardless of content.
        let geometry_changed = self.initialized && self.last_size != size;
        let style_changed = self.initialized && self.last_theme_dark != Some(theme_dark);
        let force_all = !self.initialized || geometry_changed || style_changed;

        let mut rebuilt: Vec<&str> = Vec::new();
        let mut reused: Vec<&str> = Vec::new();
        let mut dirty = [false; ELEMENT_COUNT];
        for class in 0..ELEMENT_COUNT {
            if !present[class] {
                continue;
            }
            let changed = force_all || !self.present[class] || self.last_fp[class] != fp[class];
            dirty[class] = changed;
            if changed {
                rebuilt.push(ELEMENT_LABELS[class]);
            } else {
                reused.push(ELEMENT_LABELS[class]);
            }
        }

        let nodes_total = rebuilt.len() + reused.len();
        let layout_reused = !geometry_changed && !style_changed && self.initialized;

        // Syntax commit accounting: only meaningful on frames where a fresh
        // highlight result was applied (syntax flag) AND the editor text node
        // actually changed.
        let (syntax_commit_lines, syntax_commit_regions) = if flags.syntax && dirty[EL_EDITOR] {
            (editor_visible_lines, editor_regions)
        } else {
            (0, 0)
        };

        let (gpu_bytes, gpu_reason, draw_payload_reused, text_prepare_reused) = match perf {
            Some(p) => (
                p.gpu_upload_bytes,
                p.gpu_upload_reason,
                p.elements_reused,
                (p.gpu_upload_reason == "reused") as usize,
            ),
            None => (0, "none", 0, 0),
        };

        eprintln!(
            "ZAROXI_UI_TRACE: frame={} ui_nodes_total={} ui_nodes_rebuilt={} ui_nodes_reused={} rebuilt=[{}] reused=[{}] dirty_reasons=[{}] editor_content_dirty={} gutter_dirty={} status_bar_dirty={} chrome_dirty={} side_dirty={} ai_dirty={} bottom_dirty={} syntax_commit_lines={} syntax_commit_regions={} text_prepare_reused={} draw_payload_reused={} layout_reused={} gpu_upload_bytes={} gpu_upload_reason={}",
            frame_id,
            nodes_total,
            rebuilt.len(),
            reused.len(),
            rebuilt.join(","),
            reused.join(","),
            flags.summary(),
            dirty[EL_EDITOR] as u8,
            dirty[EL_GUTTER] as u8,
            dirty[EL_STATUS] as u8,
            dirty[EL_CHROME] as u8,
            dirty[EL_SIDE] as u8,
            dirty[EL_AI] as u8,
            dirty[EL_BOTTOM] as u8,
            syntax_commit_lines,
            syntax_commit_regions,
            text_prepare_reused,
            draw_payload_reused,
            layout_reused as u8,
            gpu_bytes,
            gpu_reason,
        );

        self.last_fp = fp;
        self.present = present;
        self.last_size = size;
        self.last_theme_dark = Some(theme_dark);
        self.initialized = true;
    }
}
