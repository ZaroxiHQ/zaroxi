use crate::presenters::model::{GpuShellView, TabStrip};
use crate::presenters::paint::GpuPaintPlan;
use zaroxi_core_engine_scene::{ShellChrome, Tab as EngineTab};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRenderTranscript {
    pub width: u32,
    pub height: u32,
    pub view: GpuShellView,
    pub plan_lines: Vec<String>,
    /// Minimal engine-facing shell chrome projection for downstream engine crates.
    pub engine_chrome: ShellChrome,
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
        Self::from_view_and_plan_with_tabs(width, height, view, plan, &TabStrip::default())
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

        // Build a minimal engine-facing ShellChrome projection from the presenter
        // view + TabStrip. This keeps rendering semantics in interface-desktop
        // minimal (what tabs exist, active/focused semantics, short labels)
        // while allowing engine crates to own the rendering primitives later.
        let mut engine_tabs: Vec<EngineTab> = Vec::with_capacity(tabs.tabs.len());
        for t in tabs.tabs.iter() {
            engine_tabs.push(EngineTab {
                index: t.index,
                id: t.id.clone(),
                label: t.display.clone(),
                active: t.active,
            });
        }
        let active_index = tabs.tabs.iter().position(|t| t.active);

        let engine_chrome = ShellChrome {
            chrome_label: view.chrome_label.clone(),
            tabs: engine_tabs,
            active_tab_index: active_index,
            focus_slot: view.focus_slot.clone(),
            status_text: view.status_text.clone(),
            ai_indicator: view.ai_indicator.clone(),
            content_preview: view.content_preview.clone(),
        };

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
