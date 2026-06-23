/*!
Text subsystem

Provides the Cosmic Text–backed text rendering pipeline used by renderer core.

Design summary:
- TextCommand: small command model emitted by the renderer core for each text
  item (title/body). Commands are queued and consumed by the CosmicText renderer.
- TextRenderer trait: minimal interface used by core:
    - queue_text(cmd)
    - prepare(queue) -> perform glyph rasterization / GPU atlas uploads
    - render_pass(rpass, pipeline, panel_indices_len, total_indices_len)
    - resize_viewport(w,h)
- CosmicTextRenderer: concrete implementation that owns cosmic-text native
  state (FontSystem, SwashCache, SharedAtlas) and the wgpu instance buffer.

CosmicTextRenderer is the single authoritative text renderer for GUI text.
*/

use crate::error::RenderError;
use log::info;
use std::sync::Mutex;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass, RenderPipeline};

pub mod cosmic;
pub mod desktop_shim;
pub use cosmic::CosmicTextRenderer;

/// Stable UI-element class codes used to bucket queued [`TextCommand`]s so the
/// text renderer can retain a per-element prepared draw payload (glyph instance
/// samples) and skip re-emitting elements whose content did not change between
/// frames. These mirror the major shell elements the GUI splits its scene into
/// (editor text viewport, gutter/line numbers, status bar, chrome/tab bar,
/// side/auxiliary panels). `OTHER` is the default for untagged commands and is
/// always treated conservatively (still cached, but as a single catch-all
/// bucket).
pub mod element {
    /// Default bucket for any command not explicitly tagged.
    pub const OTHER: u32 = 0;
    /// Editor text content viewport (the document body).
    pub const EDITOR_CONTENT: u32 = 1;
    /// Gutter / line-number lane.
    pub const GUTTER: u32 = 2;
    /// Status bar.
    pub const STATUS_BAR: u32 = 3;
    /// Window chrome: titlebar, tab bar / header, toolbar.
    pub const CHROME: u32 = 4;
    /// Side panel / explorer / sidebar.
    pub const SIDE_PANEL: u32 = 5;
    /// Right-hand AI / auxiliary pane.
    pub const AI_PANEL: u32 = 6;
    /// Bottom panel / terminal pane.
    pub const BOTTOM_PANEL: u32 = 7;

    /// Human-readable short label for a class code (for tracing).
    pub fn label(code: u32) -> &'static str {
        match code {
            EDITOR_CONTENT => "editor",
            GUTTER => "gutter",
            STATUS_BAR => "status",
            CHROME => "chrome",
            SIDE_PANEL => "side",
            AI_PANEL => "ai",
            BOTTOM_PANEL => "bottom",
            _ => "other",
        }
    }
}

/// Small in-process command representing text to be rendered.
///
/// The renderer core emits these commands per panel title/content. The native
/// Cosmic renderer consumes them, performs shaping/rasterization in `prepare`
/// and draws them in `render_pass`.
#[derive(Debug, Clone)]
pub struct TextCommand {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub color: [f32; 4],
    pub size: f32,
    pub clip_x: f32,
    pub clip_y: f32,
    pub clip_w: f32,
    pub clip_h: f32,
    pub is_title: bool,
    /// Optional per-run colors for a single logical line. When present the line
    /// is shaped as ONE continuous cosmic buffer (natural advances, stable
    /// baseline) with per-range color attributes, instead of one independently
    /// positioned buffer per colored segment. This keeps syntax colors while
    /// preserving normal continuous editor-text layout.
    pub color_runs: Option<Vec<(String, [f32; 4])>>,
    /// UI-element class this command belongs to (see [`element`]). Drives the
    /// per-element retained draw-payload cache in the text renderer so that
    /// unchanged elements are not re-shaped/re-emitted every frame. Defaults to
    /// [`element::OTHER`] for commands built without an explicit tag.
    pub element: u32,
}

impl TextCommand {
    /// Tag this command with a UI-element class (builder style). See [`element`].
    pub fn with_element(mut self, element: u32) -> Self {
        self.element = element;
        self
    }

    pub fn new(
        text: impl Into<String>,
        x: f32,
        y: f32,
        color: [f32; 4],
        size: f32,
        clip_x: f32,
        clip_y: f32,
        clip_w: f32,
        clip_h: f32,
        is_title: bool,
    ) -> Self {
        Self {
            text: text.into(),
            x,
            y,
            color,
            size,
            clip_x,
            clip_y,
            clip_w,
            clip_h,
            is_title,
            color_runs: None,
            element: element::OTHER,
        }
    }

    pub fn new_title(
        text: &str,
        x: f32,
        y: f32,
        color: [f32; 4],
        size: f32,
        clip_x: f32,
        clip_y: f32,
        clip_w: f32,
        clip_h: f32,
    ) -> Self {
        Self::new(text, x, y, color, size, clip_x, clip_y, clip_w, clip_h, true)
    }

    pub fn new_body(
        text: &str,
        x: f32,
        y: f32,
        color: [f32; 4],
        size: f32,
        clip_x: f32,
        clip_y: f32,
        clip_w: f32,
        clip_h: f32,
    ) -> Self {
        Self::new(text, x, y, color, size, clip_x, clip_y, clip_w, clip_h, false)
    }

    /// Build a body command for a single logical line composed of colored runs.
    /// The runs are shaped as one continuous buffer with per-range colors.
    #[allow(clippy::too_many_arguments)]
    pub fn new_body_runs(
        runs: Vec<(String, [f32; 4])>,
        x: f32,
        y: f32,
        size: f32,
        clip_x: f32,
        clip_y: f32,
        clip_w: f32,
        clip_h: f32,
    ) -> Self {
        let text: String = runs.iter().map(|(t, _)| t.as_str()).collect();
        let fallback_color = runs.first().map(|(_, c)| *c).unwrap_or([1.0; 4]);
        Self {
            text,
            x,
            y,
            color: fallback_color,
            size,
            clip_x,
            clip_y,
            clip_w,
            clip_h,
            is_title: false,
            color_runs: Some(runs),
            element: element::OTHER,
        }
    }
}

/// Minimal internal trait used by renderer core to plan/prepare/render text.
///
/// The goal is to keep the rest of the renderer backend-agnostic while giving
/// the CosmicText backend ownership of the native prepare/render lifecycle.
pub trait TextRenderer: Send + Sync {
    /// Queue a text command for the upcoming frame.
    fn queue_text(&self, cmd: TextCommand);

    /// Return number of queued text commands waiting to be prepared.
    ///
    /// This allows renderer core to decide to invoke native prepare/render even
    /// when legacy vertex/index counters are zero.
    fn queued_len(&self) -> usize;

    /// Prepare glyphs for queued commands: shape, rasterize and upload any GPU resources.
    ///
    /// The prepare step needs access to the Device to create or update GPU
    /// resources (textures, buffers) during prepare. Device is
    /// passed in along with the Queue.
    fn prepare(&self, device: &Device, queue: &mut Queue) -> Result<(), RenderError>;

    /// Render queued/ prepared text into the provided render pass. This method
    /// must bind any atlas bind groups and issue draw calls. It is called after
    /// shape/background drawing to preserve draw ordering.
    fn render_pass<'a>(
        &self,
        rpass: &mut RenderPass<'a>,
        pipeline: &RenderPipeline,
        panel_indices_len: u32,
        total_indices_len: u32,
    ) -> Result<(), RenderError>;

    /// Return an optional atlas bind group to be used by the renderer if it
    /// needs access to it for compatibility with existing submit paths.
    fn atlas_bind_group(&self) -> Option<&BindGroup> {
        None
    }

    /// Update viewport/resolution information.
    fn resize_viewport(&self, width: u32, height: u32) -> Result<(), RenderError> {
        info!("TextRenderer: viewport resize requested ({}x{})", width, height);
        Ok(())
    }

    /// Return the monospace character advance width in physical pixels,
    /// as computed from the actual loaded font metrics multiplied by the
    /// surface device scale. Used for cursor and selection positioning.
    /// Returns None if the backend cannot determine a reliable advance value.
    fn monospace_advance_x(&self) -> Option<f32> {
        None
    }

    /// Wall time (ms) spent shaping + rasterizing glyphs in the most recent
    /// `prepare` call. Gated reporting: backends only populate this when
    /// `ZAROXI_PERF_TRACE=1`. Default `0.0` for backends without instrumentation.
    fn perf_shape_ms(&self) -> f32 {
        0.0
    }

    /// Wall time (ms) spent on GPU atlas + instance-buffer upload in the most
    /// recent `prepare` call. Default `0.0` for uninstrumented backends.
    fn perf_prepare_ms(&self) -> f32 {
        0.0
    }

    /// Number of glyph instances emitted in the most recent `prepare` call.
    /// Default `0` for uninstrumented backends.
    fn perf_glyph_count(&self) -> usize {
        0
    }

    /// Number of lines whose shaping was deferred by the per-frame shaping
    /// budget in the most recent `prepare` (staged first paint). >0 means the
    /// caller should request another frame to finish shaping. Default `0`.
    fn perf_pending_lines(&self) -> usize {
        0
    }

    /// Number of UI-element buckets whose prepared glyph instances were reused
    /// from the per-element retained draw-payload cache (no re-shaping /
    /// re-emission) in the most recent `prepare`. Default `0`.
    fn perf_elements_reused(&self) -> usize {
        0
    }

    /// Number of UI-element buckets that had to be re-emitted (cache miss /
    /// content changed) in the most recent `prepare`. Default `0`.
    fn perf_elements_rebuilt(&self) -> usize {
        0
    }

    /// Bytes written to the GPU text instance buffer in the most recent
    /// `prepare` (0 when the buffer upload was skipped because nothing changed).
    /// Default `0`.
    fn perf_gpu_upload_bytes(&self) -> usize {
        0
    }

    /// Short reason describing why the instance buffer was (or was not)
    /// re-uploaded this frame: `"reused"`, `"rebuilt"`, `"partial"`, or
    /// `"none"`. Default `"none"`.
    fn perf_gpu_upload_reason(&self) -> &'static str {
        "none"
    }

    /// Lines actually shaped (cache miss) in the most recent `prepare`. Default `0`.
    fn perf_lines_shaped(&self) -> usize {
        0
    }

    /// Lines considered (visible window queued) in the most recent `prepare`.
    /// Default `0`.
    fn perf_lines_considered(&self) -> usize {
        0
    }

    /// Override the per-frame shaping budget (ms) for upcoming `prepare` calls.
    /// `Some(ms)` forces that budget (used for the open-completion burst);
    /// `None` restores the env / default steady-state budget. Default no-op.
    fn set_shape_budget_ms(&self, _ms: Option<f32>) {}

    /// Estimated resident footprint of the shaped-glyph (line) cache, in bytes.
    /// Used by the memory-pressure monitor (`ZAROXI_MEM_TRACE`). Default `0`.
    fn mem_shape_cache_bytes(&self) -> u64 {
        0
    }

    /// Estimated resident footprint of the GPU buffers this backend manages
    /// (glyph atlas + instance buffer), in bytes. Default `0`.
    fn mem_gpu_bytes(&self) -> u64 {
        0
    }

    /// Pressure response (Elevated): evict the coldest shaped-line cache entries
    /// until at most `target_entries` remain. Returns the number evicted.
    /// Default no-op returning `0`.
    fn evict_shaped_cold(&self, _target_entries: usize) -> usize {
        0
    }

    /// Pressure response (Critical): emergency flush of the shaped-glyph and
    /// per-element draw-payload caches. Default no-op.
    fn flush_glyph_cache(&self) {}
}
