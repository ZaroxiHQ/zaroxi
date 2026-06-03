use crate::diagnostics::{Diagnostic as PresentDiagnostic, diagnostics_details_for_uri};
use crate::presenters::model::{GpuShellView, TabStrip};
use crate::presenters::paint::GpuPaintPlan;
use zaroxi_core_engine_render::intent::ChromePrimitive;
use zaroxi_core_engine_scene::scene::ShellChrome;
use zaroxi_core_engine_scene::{CaretItem, EditorPrimitiveSet, SelectionRect, TextPrimitive};

use super::editor_projection::{DEFAULT_CHAR_WIDTH, DEFAULT_LINE_HEIGHT, EditorLayoutSpec};
use super::scene_snapshot;

/// ShellRenderTranscript and its associated presentation assembly logic.
///
/// This file contains the transcript struct and most methods previously
/// located in the large single-file presenter. The editor projection math is
/// delegated to the `editor_projection` submodule to keep file sizes small.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRenderTranscript {
    pub width: u32,
    pub height: u32,
    pub view: GpuShellView,
    pub plan_lines: Vec<String>,
    /// Minimal engine-facing shell chrome projection (engine-render ChromePrimitive) for downstream engine crates.
    pub engine_chrome: ChromePrimitive,
    /// Additive presenter-facing tab strip. Consumers may pass an explicitly
    /// constructed TabStrip when producing a transcript; the default is empty.
    pub tabs: TabStrip,
    /// Optional diagnostics produced for the active/open buffer (presenter-facing).
    /// Diagnostics are intentionally small and deterministic in Phase 10 so tests
    /// and harnesses can assert on their presence easily.
    pub diagnostics: Vec<PresentDiagnostic>,
    /// Whether LSP diagnostics collection is enabled (feature flag + adapter present).
    pub diagnostics_enabled: bool,
}

impl ShellRenderTranscript {
    /// Construct a transcript from the stable presenter view + paint plan.
    /// The produced plan_lines mirror the exact order of GpuPaintPlan.ops
    /// and contain concise, deterministic descriptions of each op.
    pub fn from_view_and_plan(
        width: u32,
        height: u32,
        view: &GpuShellView,
        plan: &GpuPaintPlan,
    ) -> Self {
        // Legacy convenience constructor delegates to the tabbed variant without editor layout.
        Self::from_view_and_plan_with_tabs_and_editor(
            width,
            height,
            view,
            plan,
            &TabStrip::default(),
            None,
            None,
        )
    }

    /// Construct a transcript from the stable presenter view + paint plan and
    /// an explicit presenter-facing TabStrip. This allows callers to project
    /// opened-buffer state into the transcript for immediate desktop consumption.
    pub fn from_view_and_plan_with_tabs(
        width: u32,
        height: u32,
        view: &GpuShellView,
        plan: &GpuPaintPlan,
        tabs: &TabStrip,
        editor_lines: Option<&[String]>,
    ) -> Self {
        // Backwards-compatible: call the extended constructor without editor layout.
        Self::from_view_and_plan_with_tabs_and_editor(
            width,
            height,
            view,
            plan,
            tabs,
            editor_lines,
            None,
        )
    }

    /// Extended constructor that accepts an optional EditorViewLayout so the
    /// presenter can append deterministic Caret/Selection plan entries.
    pub fn from_view_and_plan_with_tabs_and_editor(
        width: u32,
        height: u32,
        view: &GpuShellView,
        plan: &GpuPaintPlan,
        tabs: &TabStrip,
        editor_lines: Option<&[String]>,
        editor_layout: Option<&EditorLayoutSpec>,
    ) -> Self {
        let mut plan_lines = Vec::with_capacity(plan.ops.len());
        for op in plan.ops.iter() {
            match op {
                crate::presenters::paint::GpuPaintOp::FillRect(r) => {
                    plan_lines.push(format!(
                        "FillRect x={} y={} w={} h={} color={:?}",
                        r.x, r.y, r.width, r.height, r.color
                    ));
                }
                crate::presenters::paint::GpuPaintOp::BorderRect { rect, thickness } => {
                    plan_lines.push(format!(
                        "BorderRect x={} y={} w={} h={} color={:?} thickness={}",
                        rect.x, rect.y, rect.width, rect.height, rect.color, thickness
                    ));
                }
                crate::presenters::paint::GpuPaintOp::Text {
                    x,
                    y,
                    text,
                    color,
                    max_w: _,
                    max_h: _,
                } => {
                    plan_lines
                        .push(format!("Text x={} y={} text=\"{}\" color={:?}", x, y, text, color));
                }
            }
        }

        // Append editor-visible lines if provided.
        // We produce two deterministic, presenter-facing plan entries per visible row:
        //  - a gutter label ("Gutter ...")
        //  - a content text line ("Text ...")
        // The caller is expected to pass only visible rows in top-to-bottom order.
        if let Some(ed_lines) = editor_lines {
            // Stable layout assumptions for presenter-level transcript:
            // - deterministic line height (pixels)
            // - stable gutter width (pixels)
            // Presenters/renderers should later replace these heuristics with real font metrics.
            let gutter_width: u32 = 48;
            let line_height: u32 = 16;

            // Base positions derived from the shell view content rect.
            // Use conservative casting to u32 for transcript readability.
            let content_x = view.content.x as u32;
            let base_y = view.content.y as u32;
            // Gutter x is placed to the left of the content rect (reserve gutter width).
            let gutter_x = if content_x > gutter_width { content_x - gutter_width } else { 0 };

            // Determine the absolute top-most visible document line (1-based)
            // so that gutter labels and caret/selection math can be expressed in
            // document coordinates and then projected into the visible slice.
            let top_line_val = editor_layout.and_then(|l| l.top_line).unwrap_or(1);

            for (i, text) in ed_lines.iter().enumerate() {
                // Document row (1-based) for this visible entry.
                let doc_row = top_line_val.saturating_add(i as u32);
                let y = base_y.saturating_add((i as u32).saturating_mul(line_height));

                // Gutter label (right-aligned, deterministic width).
                let label = format!("{:>4}", doc_row);
                plan_lines.push(format!("Gutter x={} y={} label=\"{}\"", gutter_x, y, label));

                // Content text entry (slight inset from left content edge for readability).
                let content_text_x = content_x.saturating_add(6);
                plan_lines.push(format!("Text x={} y={} text=\"{}\"", content_text_x, y, text));
            }

            // If an editor layout/state is provided, derive caret & selection plan entries.
            if let Some(layout) = editor_layout {
                // Use conservative, deterministic monospace metrics local to the presenter.
                // Avoids pulling a new crate dependency during this phase.
                let char_w = DEFAULT_CHAR_WIDTH;
                let lh = DEFAULT_LINE_HEIGHT;

                let content_x = view.content.x as u32;
                let base_y = view.content.y as u32;
                let content_text_x = content_x.saturating_add(6);

                // Caret: single vertical thin rect at (col,char_w).
                if let Some(cl) = layout.cursor_line {
                    let col = layout.cursor_column.unwrap_or(0);
                    let top_line_val = layout.top_line.unwrap_or(1);
                    // Compute the caret y relative to the visible slice. Both `cl`
                    // and `top_line_val` are 1-based document lines.
                    let offset_rows = cl.saturating_sub(top_line_val);
                    let caret_x = content_text_x.saturating_add(col.saturating_mul(char_w));
                    let caret_y = base_y.saturating_add(offset_rows.saturating_mul(lh));
                    plan_lines.push(format!("Caret x={} y={} h={}", caret_x, caret_y, lh));
                }

                // Selection: produce one Selection plan entry per visible line that intersects.
                if let Some((sline, scol, eline, ecol)) = layout.selection {
                    let top_line_val = layout.top_line.unwrap_or(1);
                    // Iterate visible rows and emit selection rects for intersections.
                    for (i, _) in ed_lines.iter().enumerate() {
                        // document row for this visible entry (1-based)
                        let row = top_line_val.saturating_add(i as u32);
                        if row < sline || row > eline {
                            continue;
                        }
                        let sel_start_col = if row == sline { scol } else { 0 };
                        let sel_end_col = if row == eline {
                            ecol
                        } else {
                            // rough fallback: full line
                            // fallback width: use length of the provided visible text if available
                            ed_lines.get(i).map(|s| s.chars().count() as u32).unwrap_or(0)
                        };
                        if sel_end_col <= sel_start_col {
                            continue;
                        }
                        let sx =
                            content_text_x.saturating_add(sel_start_col.saturating_mul(char_w));
                        let w = sel_end_col.saturating_sub(sel_start_col).saturating_mul(char_w);
                        // sy is based on the visible row offset (i)
                        let sy = base_y.saturating_add((i as u32).saturating_mul(lh));
                        plan_lines.push(format!("Selection x={} y={} w={} h={}", sx, sy, w, lh));
                    }
                }
            }
        }

        // Build a minimal engine-facing ShellChrome projection from the presenter
        // view + TabStrip and convert it into the engine-render ChromePrimitive.
        // This explicitly reuses the engine-render conversion so downstream engine
        // consumers receive the canonical engine-facing chrome primitive.
        let mut scene_tabs: Vec<zaroxi_core_engine_scene::scene::Tab> =
            Vec::with_capacity(tabs.tabs.len());
        for t in tabs.tabs.iter() {
            scene_tabs.push(zaroxi_core_engine_scene::scene::Tab {
                index: t.index as u32,
                id: t.id.clone(),
                label: t.display.clone(),
                active: t.active,
            });
        }
        let active_index = tabs.tabs.iter().position(|t| t.active);

        let scene_chrome = ShellChrome {
            chrome_label: view.chrome_label.clone(),
            tabs: scene_tabs,
            active_tab_index: active_index,
            active_panel_id: view.focus_slot.as_ref().map(|s| s.as_str().to_string()),
            status_text: view.status_text.clone(),
        };

        let engine_chrome = ChromePrimitive::from(scene_chrome);

        // Publish a lightweight engine-facing ShellSceneModel snapshot so the
        // engine runtime seam (zaroxi-core-engine-scene) reflects the active
        // presenter's visible lines and optional editor layout (caret/selection).
        // This makes renderer backends that query the global scene (get_current_scene)
        // render the real active-buffer text and caret without further plumbing.
        //
        // We intentionally avoid mutating presenter-local state here; this is a
        // best-effort, deterministic snapshot useful for early Phase 4 wiring.
        let scene_model = {
            // prefer explicit editor_lines if provided; otherwise fall back to a
            // single-line content_preview (when present) or an empty vec.
            let text_lines: Vec<String> = editor_lines.map(|s| s.to_vec()).unwrap_or_else(|| {
                view.content_preview.as_ref().map(|p| vec![p.clone()]).unwrap_or_else(|| vec![])
            });

            let top_line = editor_layout.and_then(|l| l.top_line).unwrap_or(1);
            let total_lines = if !text_lines.is_empty() {
                text_lines.len() as u32
            } else {
                view.content_preview.as_ref().map(|_| 1u32).unwrap_or(0u32)
            };

            let cursor_line = editor_layout.and_then(|l| l.cursor_line);
            let cursor_column = editor_layout.and_then(|l| l.cursor_column);
            let selection_present =
                editor_layout.and_then(|l| l.selection).is_some() || cursor_line.is_some();

            zaroxi_core_engine_scene::ShellSceneModel {
                text_lines,
                viewport_top_line: top_line,
                viewport_total_lines: total_lines,
                viewport_summary: None,
                cursor_line,
                cursor_column,
                selection_present,
                status_text: view.status_text.clone(),
                decoration_text: view.chrome_label.clone(),
            }
        };

        // Publish snapshot to the engine seam so backends rendering later in the
        // frame will observe the presenter's active buffer & caret.
        zaroxi_core_engine_scene::set_current_scene(scene_model);

        // Scene published. The renderer consumes the global engine scene (zaroxi_core_engine_scene)
        // during its frame work to layout and rasterize glyphs. Presenters must not attempt to
        // call renderer internals directly to queue glyph uploads; keep this boundary clean.
        let diag_uri = view.active_buffer_label.as_deref().unwrap_or("").trim().to_string();

        let (diagnostics, diagnostics_enabled) = if diag_uri.is_empty() {
            // No active buffer: treat as ready/no-diagnostics for presenter display.
            (Vec::new(), true)
        } else if let Some(v) = diagnostics_details_for_uri(&diag_uri) {
            // Provider present (mock or real adapter) -> ready, may be empty.
            (v, true)
        } else {
            // Provider not available for this uri (feature off / adapter absent).
            (Vec::new(), false)
        };

        ShellRenderTranscript {
            width,
            height,
            view: view.clone(),
            plan_lines,
            engine_chrome,
            tabs: tabs.clone(),
            diagnostics,
            diagnostics_enabled,
        }
    }

    /// Small helper that returns a multi-line summary derived directly from the
    /// live engine scene snapshot (zaroxi_core_engine_scene). This is intended
    /// to be a compact, deterministic view of the active buffer and caret for
    /// test assertions and early-phase validation.
    pub fn engine_scene_summary() -> String {
        scene_snapshot::engine_scene_summary()
    }

    /// Produce a compact deterministic multi-line textual snapshot suitable for
    /// test assertions or logging by the native binary. The format is stable
    /// and intentionally small.
    pub fn to_string(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("viewport: {}x{}", self.width, self.height));
        lines.push("regions:".to_string());
        lines.push(format!(
            "  chrome: x={} y={} w={} h={} kind={:?}",
            self.view.chrome.x,
            self.view.chrome.y,
            self.view.chrome.width,
            self.view.chrome.height,
            self.view.chrome.kind
        ));
        lines.push(format!(
            "  content: x={} y={} w={} h={} kind={:?}",
            self.view.content.x,
            self.view.content.y,
            self.view.content.width,
            self.view.content.height,
            self.view.content.kind
        ));
        lines.push(format!(
            "  status: x={} y={} w={} h={} kind={:?}",
            self.view.status.x,
            self.view.status.y,
            self.view.status.width,
            self.view.status.height,
            self.view.status.kind
        ));
        // Stabilized transcript: always emit explicit keys with deterministic fallbacks.
        lines.push(format!("marker: {}", self.view.marker.as_deref().unwrap_or("<none>")));
        lines.push(format!(
            "chrome_label: {}",
            self.view.chrome_label.as_deref().unwrap_or("<none>")
        ));
        lines
            .push(format!("status_text: {}", self.view.status_text.as_deref().unwrap_or("<none>")));
        // Additive semantic projection lines for richer observability.
        lines.push(format!(
            "active_buffer: {}",
            self.view.active_buffer_label.as_deref().unwrap_or("<none>")
        ));
        lines.push(format!(
            "content_preview_count: {}",
            self.view.content_preview_count.unwrap_or(0)
        ));
        lines.push(format!(
            "ai_indicator: {}",
            self.view.ai_indicator.as_deref().unwrap_or("<none>")
        ));
        // Additive: emit the small deterministic status emphasis semantic.
        lines.push(format!("status_emphasis: {}", self.view.status_emphasis.as_str()));
        lines.push(format!("shell_tone: {}", self.view.shell_tone.as_str()));
        // Deterministic focus semantic (observational only).
        lines.push(format!(
            "focus_slot: {}",
            self.view.focus_slot.as_ref().map(|s| s.as_str()).unwrap_or("<none>")
        ));
        // Emit slots (stable, deterministic small vocabulary)
        if self.view.slots.is_empty() {
            lines.push("slots: <none>".to_string());
        } else {
            let mut slot_parts: Vec<String> = Vec::new();
            for slot in &self.view.slots {
                slot_parts.push(format!(
                    "{}: x={} y={} w={} h={}",
                    slot.name.as_str(),
                    slot.rect.x,
                    slot.rect.y,
                    slot.rect.width,
                    slot.rect.height
                ));
            }
            lines.push(format!("slots: {}", slot_parts.join(", ")));
        }
        // Retain content_preview textual hint if present (semantic only; not rendered).
        if let Some(ref preview) = self.view.content_preview {
            // Print the provided preview string (may be empty).
            lines.push(format!("content_preview: {}", preview));
        } else {
            lines.push(format!("content_preview: <none>"));
        }

        // Additive, deterministic content activity semantic for observability.
        // Stable fallback value is "idle".
        lines.push(format!("content_activity: {}", self.view.content_activity.as_str()));

        // Tabs: deterministic tab strip projection (presenter-facing).
        // Emit a compact, deterministic summary first so consumers can quickly
        // observe counts and the active/focus hints without parsing the full list.
        let tab_count = self.tabs.tabs.len();
        let active_index = self
            .tabs
            .tabs
            .iter()
            .find(|t| t.active)
            .map(|t| t.index.to_string())
            .unwrap_or_else(|| "<none>".to_string());
        let focus_slot = self.view.focus_slot.as_ref().map(|s| s.as_str()).unwrap_or("<none>");

        // Compact, user-facing single-line tab summary. Rules:
        // - no tabs: "tabs_compact: <none>"
        // - 1-3 tabs: "tabs_compact: name1|name2 active=nameX"
        // - >3 tabs: "tabs_compact: <count> active=nameX"
        let active_display = self
            .tabs
            .tabs
            .iter()
            .find(|t| t.active)
            .map(|t| t.display.as_str())
            .unwrap_or("<none>");
        if tab_count == 0 {
            lines.push(format!("tabs_compact: <none>"));
        } else if tab_count <= 3 {
            let names: Vec<String> = self.tabs.tabs.iter().map(|t| t.display.clone()).collect();
            let joined = names.join("|");
            if active_display != "<none>" {
                lines.push(format!("tabs_compact: {} active={}", joined, active_display));
            } else {
                lines.push(format!("tabs_compact: {}", joined));
            }
        } else {
            lines.push(format!("tabs_compact: {} active={}", tab_count, active_display));
        }

        // Existing deterministic summary retained for downstream engine consumers.
        lines.push(format!(
            "tabs_summary: count={} active_index={} focus_slot={}",
            tab_count, active_index, focus_slot
        ));

        // Detailed presenter-facing tab list (kept unchanged to preserve tests).
        lines.push("tabs:".to_string());
        if self.tabs.tabs.is_empty() {
            lines.push("  <none>".to_string());
        } else {
            for t in &self.tabs.tabs {
                lines.push(format!(
                    "  {}: id={} display=\"{}\" active={}",
                    t.index, t.id, t.display, t.active
                ));
            }
        }

        lines.push("plan:".to_string());
        for l in &self.plan_lines {
            lines.push(format!("  {}", l));
        }

        // Append a concise engine-scene snapshot summary so the transcript reflects
        // the active buffer and caret coming from the engine runtime seam.
        let scene_summary = ShellRenderTranscript::engine_scene_summary();
        for line in scene_summary.lines() {
            lines.push(format!("  {}", line));
        }

        // Diagnostics: deterministic presenter-facing projection for Phase 10.
        lines.push("diagnostics:".to_string());
        if !self.diagnostics_enabled {
            lines.push("  <lsp:disabled>".to_string());
        } else if self.diagnostics.is_empty() {
            lines.push("  <none>".to_string());
        } else {
            for d in &self.diagnostics {
                let uri_part = d.uri.as_deref().unwrap_or("<unknown>");
                lines.push(format!("  {}: {} ({})", d.severity.as_str(), d.message, uri_part));
            }
        }

        lines.join("\n")
    }

    /// Build a minimal engine-facing EditorPrimitiveSet directly from the
    /// presenter's deterministic visible-line inputs (visible rows and an optional
    /// EditorLayoutSpec). This re-implements the same projection math the
    /// presenter uses to emit "Gutter"/"Text"/"Caret"/"Selection" plan lines so
    /// tests and harnesses can validate the exact primitives without pulling
    /// presenter internals into engine backends.
    ///
    /// This function is intentionally stable and deterministic: it mirrors
    /// the presenter's metrics constants (DEFAULT_CHAR_WIDTH / DEFAULT_LINE_HEIGHT)
    /// and uses the same content inset heuristics.
    pub fn build_editor_primitives_from_lines(
        content_x: u32,
        base_y: u32,
        editor_lines: &[String],
        editor_layout: Option<&EditorLayoutSpec>,
    ) -> EditorPrimitiveSet {
        super::editor_projection::build_editor_primitives_from_lines(
            content_x,
            base_y,
            editor_lines,
            editor_layout,
        )
    }

    /// Produce a minimal engine-facing EditorPrimitiveSet by parsing the
    /// deterministic plan_lines emitted by this presenter.
    ///
    /// The presenter intentionally emits stable "plan" lines such as:
    ///  - "Gutter x={} y={} label=\"...\""
    ///  - "Text x={} y={} text=\"...\""
    ///  - "Caret x={} y={} h={}"
    ///  - "Selection x={} y={} w={} h={}"
    ///
    /// This helper parses those lines and constructs the small EditorPrimitiveSet
    /// that renderer backends can consume to draw the editor surface without
    /// pulling presenter internals into engine backends.
    pub fn to_editor_primitives(&self) -> EditorPrimitiveSet {
        let mut set = EditorPrimitiveSet::new();

        for line in &self.plan_lines {
            let s = line.trim();
            if s.starts_with("Text ") {
                // tokens: Text x={} y={} text="..." color={:?}
                let mut x: u32 = 0;
                let mut y: u32 = 0;
                let mut text = String::new();

                // Extract quoted text payload if present.
                if let Some(start) = s.find("text=\"") {
                    let after = &s[start + 6..];
                    if let Some(end) = after.find('"') {
                        text = after[..end].to_string();
                    }
                }

                for token in s.split_whitespace() {
                    if let Some(v) = token.strip_prefix("x=") {
                        x = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("y=") {
                        y = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    }
                }

                set.texts.push(TextPrimitive {
                    x,
                    y,
                    text,
                    font_name: "ZaroxiMono".to_string(),
                    max_width: None,
                });
            } else if s.starts_with("Gutter ") {
                // Gutter x={} y={} label="..."
                let mut x: u32 = 0;
                let mut y: u32 = 0;
                let mut label = String::new();

                if let Some(start) = s.find("label=\"") {
                    let after = &s[start + 7..];
                    if let Some(end) = after.find('"') {
                        label = after[..end].to_string();
                    }
                }

                for token in s.split_whitespace() {
                    if let Some(v) = token.strip_prefix("x=") {
                        x = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("y=") {
                        y = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    }
                }

                set.gutter_labels.push(TextPrimitive {
                    x,
                    y,
                    text: label,
                    font_name: "ZaroxiMono".to_string(),
                    max_width: None,
                });
            } else if s.starts_with("Caret ") {
                // Caret x={} y={} h={}
                let mut x: u32 = 0;
                let mut y: u32 = 0;
                let mut h: u32 = 0;
                for token in s.split_whitespace() {
                    if let Some(v) = token.strip_prefix("x=") {
                        x = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("y=") {
                        y = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("h=") {
                        h = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    }
                }
                set.carets.push(CaretItem { x, y, height: h });
            } else if s.starts_with("Selection ") {
                // Selection x={} y={} w={} h={}
                let mut x: u32 = 0;
                let mut y: u32 = 0;
                let mut w: u32 = 0;
                let mut h: u32 = 0;
                for token in s.split_whitespace() {
                    if let Some(v) = token.strip_prefix("x=") {
                        x = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("y=") {
                        y = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("w=") {
                        w = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("h=") {
                        h = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    }
                }
                if w > 0 && h > 0 {
                    set.selections.push(SelectionRect { x, y, width: w, height: h });
                }
            }
        }

        set
    }

    /// Parse plan_lines slice and return EditorPrimitiveSet without needing a full ShellRenderTranscript.
    ///
    /// This helper mirrors the parsing logic used in `to_editor_primitives` but
    /// operates on an explicit slice of plan lines. It allows tests and harnesses
    /// to validate the presenter's deterministic plan format without constructing
    /// a full transcript (view/engine chrome/etc).
    pub fn parse_plan_lines(plan_lines: &[String]) -> EditorPrimitiveSet {
        let mut set = EditorPrimitiveSet::new();

        for line in plan_lines {
            let s = line.trim();
            if s.starts_with("Text ") {
                // tokens: Text x={} y={} text="..." color={:?}
                let mut x: u32 = 0;
                let mut y: u32 = 0;
                let mut text = String::new();

                // Extract quoted text payload if present.
                if let Some(start) = s.find("text=\"") {
                    let after = &s[start + 6..];
                    if let Some(end) = after.find('"') {
                        text = after[..end].to_string();
                    }
                }

                for token in s.split_whitespace() {
                    if let Some(v) = token.strip_prefix("x=") {
                        x = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("y=") {
                        y = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    }
                }

                set.texts.push(TextPrimitive {
                    x,
                    y,
                    text,
                    font_name: "ZaroxiMono".to_string(),
                    max_width: None,
                });
            } else if s.starts_with("Gutter ") {
                // Gutter x={} y={} label="..."
                let mut x: u32 = 0;
                let mut y: u32 = 0;
                let mut label = String::new();

                if let Some(start) = s.find("label=\"") {
                    let after = &s[start + 7..];
                    if let Some(end) = after.find('"') {
                        label = after[..end].to_string();
                    }
                }

                for token in s.split_whitespace() {
                    if let Some(v) = token.strip_prefix("x=") {
                        x = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("y=") {
                        y = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    }
                }

                set.gutter_labels.push(TextPrimitive {
                    x,
                    y,
                    text: label,
                    font_name: "ZaroxiMono".to_string(),
                    max_width: None,
                });
            } else if s.starts_with("Caret ") {
                // Caret x={} y={} h={}
                let mut x: u32 = 0;
                let mut y: u32 = 0;
                let mut h: u32 = 0;
                for token in s.split_whitespace() {
                    if let Some(v) = token.strip_prefix("x=") {
                        x = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("y=") {
                        y = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("h=") {
                        h = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    }
                }
                set.carets.push(CaretItem { x, y, height: h });
            } else if s.starts_with("Selection ") {
                // Selection x={} y={} w={} h={}
                let mut x: u32 = 0;
                let mut y: u32 = 0;
                let mut w: u32 = 0;
                let mut h: u32 = 0;
                for token in s.split_whitespace() {
                    if let Some(v) = token.strip_prefix("x=") {
                        x = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("y=") {
                        y = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("w=") {
                        w = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("h=") {
                        h = v.trim_end_matches(|c| c == ',' || c == ' ').parse().unwrap_or(0);
                    }
                }
                if w > 0 && h > 0 {
                    set.selections.push(SelectionRect { x, y, width: w, height: h });
                }
            }
        }

        set
    }
}
