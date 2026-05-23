use crate::presenters::model::{GpuShellView, TabStrip};
use crate::presenters::paint::GpuPaintPlan;
use zaroxi_core_engine_scene::scene::ShellChrome;
use zaroxi_core_engine_render::intent::ChromePrimitive;

// editor layout / font helpers for caret/selection projection into transcript.
use zaroxi_core_engine_font::load_bundled_monospace;
use zaroxi_core_editor_view::EditorViewLayout;

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
}

impl ShellRenderTranscript {
    /// Construct a transcript from the stable presenter view + paint plan.
    /// The produced plan_lines mirror the exact order of GpuPaintPlan.ops
    /// and contain concise, deterministic descriptions of each op.
    /// This legacy constructor produces a transcript with an empty TabStrip.
    pub fn from_view_and_plan(width: u32, height: u32, view: &GpuShellView, plan: &GpuPaintPlan) -> Self {
        Self::from_view_and_plan_with_tabs(width, height, view, plan, &TabStrip::default(), None)
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
                crate::presenters::paint::GpuPaintOp::Text { x, y, text, color } => {
                    plan_lines.push(format!(
                        "Text x={} y={} text=\"{}\" color={:?}",
                        x, y, text, color
                    ));
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

            for (i, text) in ed_lines.iter().enumerate() {
                let row = (i as u32) + 1; // 1-based visible row index for readability
                let y = base_y.saturating_add((i as u32).saturating_mul(line_height));

                // Gutter label (right-aligned, deterministic width).
                let label = format!("{:>4}", row);
                plan_lines.push(format!("Gutter x={} y={} label=\"{}\"", gutter_x, y, label));

                // Content text entry (slight inset from left content edge for readability).
                let content_text_x = content_x.saturating_add(6);
                plan_lines.push(format!("Text x={} y={} text=\"{}\"", content_text_x, y, text));
            }
        }

        // Build a minimal engine-facing ShellChrome projection from the presenter
        // view + TabStrip and convert it into the engine-render ChromePrimitive.
        // This explicitly reuses the engine-render conversion so downstream engine
        // consumers receive the canonical engine-facing chrome primitive.
        let mut scene_tabs: Vec<zaroxi_core_engine_scene::scene::Tab> = Vec::with_capacity(tabs.tabs.len());
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
            focus_slot: view.focus_slot.as_ref().map(|s| s.as_str().to_string()),
            status_text: view.status_text.clone(),
            ai_indicator: view.ai_indicator.clone(),
            content_preview: view.content_preview.clone(),
        };

        let engine_chrome = ChromePrimitive::from(scene_chrome);

        ShellRenderTranscript {
            width,
            height,
            view: view.clone(),
            plan_lines,
            engine_chrome,
            tabs: tabs.clone(),
        }
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
        lines.push(format!("chrome_label: {}", self.view.chrome_label.as_deref().unwrap_or("<none>")));
        lines.push(format!("status_text: {}", self.view.status_text.as_deref().unwrap_or("<none>")));
        // Additive semantic projection lines for richer observability.
        lines.push(format!("active_buffer: {}", self.view.active_buffer_label.as_deref().unwrap_or("<none>")));
        lines.push(format!("content_preview_count: {}", self.view.content_preview_count.unwrap_or(0)));
        lines.push(format!("ai_indicator: {}", self.view.ai_indicator.as_deref().unwrap_or("<none>")));
        // Additive: emit the small deterministic status emphasis semantic.
        lines.push(format!("status_emphasis: {}", self.view.status_emphasis.as_str()));
        lines.push(format!("shell_tone: {}", self.view.shell_tone.as_str()));
        // Deterministic focus semantic (observational only).
        lines.push(format!("focus_slot: {}", self.view.focus_slot.as_ref().map(|s| s.as_str()).unwrap_or("<none>")));
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
        let focus_slot = self
            .view
            .focus_slot
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("<none>");

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
        lines.join("\n")
    }
}

// Optional minimal wiring: expose a tiny feature-gated adapter that allows
// interface code to derive a deterministic debug-lines view directly from the
// core layout ShellLayout. This is intentionally feature-gated ("layout")
// so default builds remain unchanged unless the dependency and feature are
// explicitly enabled in Cargo.toml.
#[cfg(feature = "layout")]
pub fn layout_debug_lines_from_shell(layout: &zaroxi_core_engine_layout::ShellLayout) -> Vec<String> {
    layout.to_debug_lines()
}
