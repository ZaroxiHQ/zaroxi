use crate::presenters::paint::{GpuPaintPlan, execute_paint_plan};
use std::cmp::min;

/// Kinds of logical regions present in the shell. Kept intentionally small
/// and explicit so the presenter can deterministically map kinds -> visuals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionKind {
    Chrome,
    Content,
    Status,
}

/// Simple rectangle region (pixel coordinates) augmented with a tiny semantic
/// `kind` field to enable deterministic presentational differences without
/// introducing a styling/theme system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub kind: RegionKind,
}

impl Region {
    /// Construct a region defaulting to `Content` kind for convenience.
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Region { x, y, width, height, kind: RegionKind::Content }
    }

    /// Construct a region with an explicit semantic kind.
    pub fn with_kind(x: u32, y: u32, width: u32, height: u32, kind: RegionKind) -> Self {
        Region { x, y, width, height, kind }
    }
}

/// Collection of named regions for the shell.
///
/// An optional `marker` string is carried with the regions so the presenter
/// can paint a small deterministic visible cue (a colored bar in the chrome)
/// to reflect lightweight shell state (for example: active buffer name).
/// This keeps the visual change primitive and deterministic while avoiding
/// any heavy composition or text rendering logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRegions {
    pub chrome: Region,
    pub content: Region,
    pub status: Region,
    /// Optional marker string rendered into the chrome to reflect visible state
    /// (e.g. active buffer name). Kept optional and crate-local; presenter simply
    /// paints a deterministic colored marker when present.
    pub marker: Option<String>,

    /// Tiny deterministic semantic payloads (kept primitive and optional).
    ///
    /// The intent here is to project lightweight, testable semantic labels and
    /// small telemetry indicators into the presenter/transcript path without
    /// introducing any text rendering. These fields are purely semantic (not
    /// visual) and may be surfaced in the debug transcript or used to drive
    /// small deterministic paint tokens (already present as marker/chrome_label).
    ///
    /// - chrome_label: a short label for the chrome/header (e.g. active buffer name)
    /// - status_text: a short status string for the status bar
    /// - content_preview: an optional single-line preview or hint for the content region
    /// - active_buffer_label: explicitly named active buffer (preferred over ad-hoc marker)
    /// - content_preview_count: optional numeric summary of preview lines (semantic)
    /// - ai_indicator: optional tiny AI status summary (e.g. "ai:available" or "ai:off")
    pub chrome_label: Option<String>,
    pub status_text: Option<String>,
    pub content_preview: Option<String>,

    /// New explicit semantic fields (additive; do not affect painting).
    pub active_buffer_label: Option<String>,
    pub content_preview_count: Option<usize>,
    pub ai_indicator: Option<String>,

    /// Deterministic focus/active semantic: which slot (if any) is currently focused.
    /// This is purely observational and rendered into transcripts; it does not change
    /// painting behavior.
    pub focus_slot: Option<SlotName>,
}

/// Presenter-visible explicit, tiny output contract.
///
/// This struct is intentionally minimal and mirrors the stable concepts the
/// presenter already used: ordering, region kind, bounds, marker, borders and
/// the small semantic payloads. Having this explicit type makes future
/// rendering layers consume a stable model rather than ad-hoc region structs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionView {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub kind: RegionKind,
}

/// Small stable vocabulary of semantic slots inside the shell.
///
/// These are intentionally tiny and deterministic: they describe semantic
/// anchor locations inside the chrome/content/status regions. Kept additive
/// and purely observational (no layout changes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotName {
    ChromeLeft,
    ChromeCenter,
    ChromeRight,
    ContentMain,
    Status,
}

impl SlotName {
    pub fn as_str(&self) -> &'static str {
        match self {
            SlotName::ChromeLeft => "chrome_left",
            SlotName::ChromeCenter => "chrome_center",
            SlotName::ChromeRight => "chrome_right",
            SlotName::ContentMain => "content_main",
            SlotName::Status => "status",
        }
    }
}

/// A small view describing a single semantic slot and its rectangle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotView {
    pub name: SlotName,
    pub rect: RegionView,
}

/// Deterministic, tiny semantic describing content activity inside the main
/// content region. This enum is intentionally minimal and purely observational:
/// it is surfaced in the transcript for debug/observability and does not change
/// painting behavior.
///
/// Rules (deterministic, additive):
/// - Selection: if a non-empty `content_preview` is present (indicates selected text).
/// - Cursor: if focus_slot == Some(SlotName::ContentMain) and no non-empty preview.
/// - Idle: default fallback when neither selection nor cursor heuristics apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentActivity {
    Idle,
    Cursor,
    Selection,
}

impl ContentActivity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentActivity::Idle => "idle",
            ContentActivity::Cursor => "cursor",
            ContentActivity::Selection => "selection",
        }
    }
}

/// Small deterministic emphasis state for the status bar area.
///
/// This is intentionally tiny and derives purely from existing semantic
/// payloads available on ShellRegions / GpuShellView:
/// - If an `ai_indicator` is present (non-empty) we prefer `Ai`.
/// - Else if `status_text` is present (non-empty) we consider it `Attention`.
/// - Otherwise the fallback is `Normal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusEmphasis {
    Normal,
    Attention,
    Ai,
}

impl StatusEmphasis {
    pub fn as_str(&self) -> &'static str {
        match self {
            StatusEmphasis::Normal => "normal",
            StatusEmphasis::Attention => "attention",
            StatusEmphasis::Ai => "ai",
        }
    }
}

/// Small, deterministic shell-level presentation hint derived from existing
/// GPU-shell semantics (focus_slot, content_activity, status_emphasis).
///
/// Precedence (highest -> lowest):
///  - Ai
///  - Attention
///  - Focused   (derived from content activity selection/cursor or explicit focus_slot)
///  - Neutral   (fallback)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellTone {
    Neutral,
    Focused,
    Attention,
    Ai,
}

impl ShellTone {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShellTone::Neutral => "neutral",
            ShellTone::Focused => "focused",
            ShellTone::Attention => "attention",
            ShellTone::Ai => "ai",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuShellView {
    pub chrome: RegionView,
    pub content: RegionView,
    pub status: RegionView,

    /// Carries the same lightweight marker as ShellRegions (active buffer hint).
    pub marker: Option<String>,

    /// The tiny semantic payloads (kept optional and primitive).
    pub chrome_label: Option<String>,
    pub status_text: Option<String>,
    pub content_preview: Option<String>,

    /// Explicit, additive semantic projection fields:
    /// - active_buffer_label: explicit active buffer name (for transcript/observability)
    /// - content_preview_count: numeric summary of content preview lines
    /// - ai_indicator: tiny AI status summary (semantic only)
    pub active_buffer_label: Option<String>,
    pub content_preview_count: Option<usize>,
    pub ai_indicator: Option<String>,

    /// New explicit per-view status emphasis semantic (additive).
    pub status_emphasis: StatusEmphasis,
    /// Tiny derived shell-level presentation hint (additive; does not affect painting).
    pub shell_tone: ShellTone,

    /// Deterministic focus/active semantic: which slot (if any) is currently focused.
    /// Observational only; does not affect painting.
    pub focus_slot: Option<SlotName>,

    /// Deterministic content activity semantic (additive, default: idle).
    pub content_activity: ContentActivity,

    /// Additive list of semantic slots (stable vocabulary, deterministic ordering).
    pub slots: Vec<SlotView>,

    /// Presenter-facing tab strip (additive, deterministic).
    pub tabs: TabStrip,
}

impl GpuShellView {
    /// Build a stable presenter view from the adapter's ShellRegions.
    pub fn from_shell_regions(s: &ShellRegions) -> Self {
        let chrome = RegionView {
            x: s.chrome.x,
            y: s.chrome.y,
            width: s.chrome.width,
            height: s.chrome.height,
            kind: s.chrome.kind.clone(),
        };
        let content = RegionView {
            x: s.content.x,
            y: s.content.y,
            width: s.content.width,
            height: s.content.height,
            kind: s.content.kind.clone(),
        };
        let status = RegionView {
            x: s.status.x,
            y: s.status.y,
            width: s.status.width,
            height: s.status.height,
            kind: s.status.kind.clone(),
        };

        // Build deterministic slots (left/center/right in chrome, plus content_main and status).
        let mut slots: Vec<SlotView> = Vec::new();

        // Split chrome width into three parts deterministically using integer division.
        let c_w = chrome.width;
        let left_w = c_w / 3;
        let right_w = c_w / 3;
        let center_w = c_w.saturating_sub(left_w).saturating_sub(right_w);

        // chrome_left
        let left_rect = RegionView {
            x: chrome.x,
            y: chrome.y,
            width: left_w,
            height: chrome.height,
            kind: chrome.kind.clone(),
        };
        slots.push(SlotView { name: SlotName::ChromeLeft, rect: left_rect });

        // chrome_center
        let center_x = chrome.x.saturating_add(left_w);
        let center_rect = RegionView {
            x: center_x,
            y: chrome.y,
            width: center_w,
            height: chrome.height,
            kind: chrome.kind.clone(),
        };
        slots.push(SlotView { name: SlotName::ChromeCenter, rect: center_rect });

        // chrome_right
        let right_x = center_x.saturating_add(center_w);
        let right_rect = RegionView {
            x: right_x,
            y: chrome.y,
            width: right_w,
            height: chrome.height,
            kind: chrome.kind.clone(),
        };
        slots.push(SlotView { name: SlotName::ChromeRight, rect: right_rect });

        // content_main
        slots.push(SlotView { name: SlotName::ContentMain, rect: content.clone() });

        // status
        slots.push(SlotView { name: SlotName::Status, rect: status.clone() });

        // Determine content activity semantically:
        // - Selection if a non-empty content_preview is present.
        // - Cursor if focused into the content main slot.
        // - Idle otherwise.
        let content_activity = if let Some(ref p) = s.content_preview {
            if !p.is_empty() {
                ContentActivity::Selection
            } else if s.focus_slot == Some(SlotName::ContentMain) {
                ContentActivity::Cursor
            } else {
                ContentActivity::Idle
            }
        } else if s.focus_slot == Some(SlotName::ContentMain) {
            ContentActivity::Cursor
        } else {
            ContentActivity::Idle
        };

        // Determine a tiny deterministic status emphasis:
        // - Prefer Ai when an AI indicator is present (non-empty).
        // - Else prefer Attention when there is status_text (non-empty).
        // - Otherwise fallback to Normal.
        let status_emphasis = if s.ai_indicator.as_ref().map(|v| !v.is_empty()).unwrap_or(false) {
            StatusEmphasis::Ai
        } else if s.status_text.as_ref().map(|v| !v.is_empty()).unwrap_or(false) {
            StatusEmphasis::Attention
        } else {
            StatusEmphasis::Normal
        };

        // Derive a tiny deterministic shell-level tone from existing semantics.
        // Precedence (highest -> lowest): Ai, Attention, Focused, Neutral.
        // Focused is derived from content activity (selection/cursor) or an explicit focus_slot.
        let shell_tone = if status_emphasis == StatusEmphasis::Ai {
            ShellTone::Ai
        } else if status_emphasis == StatusEmphasis::Attention {
            ShellTone::Attention
        } else if content_activity == ContentActivity::Selection
            || content_activity == ContentActivity::Cursor
            || s.focus_slot == Some(SlotName::ContentMain)
        {
            ShellTone::Focused
        } else {
            ShellTone::Neutral
        };

        GpuShellView {
            chrome,
            content,
            status,
            marker: s.marker.clone(),
            chrome_label: s.chrome_label.clone(),
            status_text: s.status_text.clone(),
            content_preview: s.content_preview.clone(),
            active_buffer_label: s.active_buffer_label.clone(),
            content_preview_count: s.content_preview_count,
            ai_indicator: s.ai_indicator.clone(),
            status_emphasis,
            shell_tone,
            focus_slot: s.focus_slot.clone(),
            content_activity,
            slots,
            tabs: TabStrip::default(),
        }
    }
}

/// Explicit ShellFrame layout model.
///
/// This small, presenter-local helper centralizes shell-frame geometry derived
/// from the high-level GpuShellView. It provides an explicit, stable contract
/// that the paint planner consumes instead of recomputing ad-hoc geometry inline.
/// The intent is to keep layout-policy near the presenter view model while
/// keeping paint execution purely declarative (consumes ShellFrame).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellFrame {
    pub chrome: RegionView,
    pub content: RegionView,
    pub status: RegionView,
    /// Precomputed tab bar height (derived from chrome height).
    pub tab_bar_h: u32,
    /// Tab bar y origin (already vertically centered inside chrome).
    pub tab_bar_y: u32,
}

impl ShellFrame {
    /// Build a ShellFrame from a stable GpuShellView. All derived layout
    /// decisions that belong to the frame (tab bar size/position and simple
    /// tab width allocation) are computed here so paint code consumes a clear
    /// contract.
    pub fn from_view(v: &GpuShellView) -> Self {
        let chrome = v.chrome.clone();
        let content = v.content.clone();
        let status = v.status.clone();

        // Small, deterministic tab bar height that fits inside the chrome.
        let tab_bar_h = std::cmp::min(14u32, chrome.height.saturating_sub(4));
        let tab_bar_y = chrome.y + (chrome.height.saturating_sub(tab_bar_h) / 2);

        ShellFrame { chrome, content, status, tab_bar_h, tab_bar_y }
    }

    /// Total viewport width (derived from region widths). This mirrors the
    /// previous ad-hoc computation but centralizes it so tests and paint code
    /// rely on the same contract.
    pub fn total_width(&self) -> u32 {
        self.chrome.width.max(self.content.width).max(self.status.width)
    }

    /// Total viewport height (chrome + content + status).
    pub fn total_height(&self) -> u32 {
        self.chrome.height.saturating_add(self.content.height).saturating_add(self.status.height)
    }

    /// Deterministic base per-tab width for `num` tabs (last tab may take remainder).
    pub fn base_tab_width(&self, num: u32) -> u32 {
        if num > 0 { self.chrome.width / num } else { 0 }
    }
}

/// Thin GPU-backed presenter. It does not own any heavy application state;
/// it provides pure functions for region mapping and buffer painting.
///
/// The presenter is intentionally small so the core mapping logic is easily tested.
pub struct GpuShellPresenter;

impl GpuShellPresenter {
    /// Compute the chrome/content/status regions for a window of size (width x height).
    /// - chrome_height: default top chrome height in pixels (suggested: 60).
    /// - status_height: default bottom status bar height in pixels (suggested: 24).
    pub fn map_regions(
        width: u32,
        height: u32,
        chrome_height: u32,
        status_height: u32,
    ) -> ShellRegions {
        // Compute raw band sizes first (reserve chrome/top and status/bottom as requested).
        let chrome_h = min(chrome_height, height);
        let status_h = min(status_height, height.saturating_sub(chrome_h));
        let content_h = height.saturating_sub(chrome_h).saturating_sub(status_h);

        // Chrome stays at the top as before.
        let chrome = Region::with_kind(0, 0, width, chrome_h, RegionKind::Chrome);

        // Introduce conservative vertical insets for the content region so it reads
        // as an intentionally inset area between chrome and status. Keep horizontal
        // full-width for deterministic tooling/tests that rely on width equality.
        //
        // Insets are clamped to the available content height to avoid negative sizes.
        let top_inset: u32 = std::cmp::min(6u32, content_h);
        let bottom_inset: u32 = std::cmp::min(6u32, content_h.saturating_sub(top_inset));
        let content_y = chrome_h.saturating_add(top_inset);
        let content_h_inset = content_h.saturating_sub(top_inset.saturating_add(bottom_inset));

        let content = Region::with_kind(0, content_y, width, content_h_inset, RegionKind::Content);

        // Status remains pinned to the bottom of the frame (same origin as previous calculation).
        // This preserves the overall banding (chrome / content-area / status) while making the
        // content area feel inset between the two chrome bands.
        let status =
            Region::with_kind(0, chrome_h + content_h, width, status_h, RegionKind::Status);

        ShellRegions {
            chrome,
            content,
            status,
            marker: None,
            chrome_label: None,
            status_text: None,
            content_preview: None,
            active_buffer_label: None,
            content_preview_count: None,
            ai_indicator: None,
            focus_slot: None,
        }
    }

    /// Paint the three regions into the provided RGBA8 buffer.
    ///
    /// - `buffer` must be exactly width * height * 4 bytes long (RGBA8).
    /// - Colors are simple flat fills (no text rendering).
    ///
    /// Color choices (RGBA):
    /// - chrome: dark gray [32, 32, 40, 255]
    /// - content: light gray [220, 220, 225, 255]
    /// - status: medium gray [48, 48, 56, 255]
    pub fn paint_to_buffer(width: u32, height: u32, buffer: &mut [u8], regions: &ShellRegions) {
        // Backwards-compatible wrapper that renders without tabs (empty TabStrip).
        GpuShellPresenter::paint_to_buffer_with_tabs(
            width,
            height,
            buffer,
            regions,
            &TabStrip::default(),
        );
    }

    /// Paint the three regions into the provided RGBA8 buffer with an explicit TabStrip.
    /// This additive variant allows callers to project presenter-facing tab state
    /// (opened buffers + active id) into the visible frame.
    pub fn paint_to_buffer_with_tabs(
        width: u32,
        height: u32,
        buffer: &mut [u8],
        regions: &ShellRegions,
        tabs: &TabStrip,
    ) {
        let expected = (width as usize) * (height as usize) * 4;
        if buffer.len() != expected {
            // Silence: do nothing if buffer size mismatches.
            return;
        }

        // Clear to a baseline (transparent black) first.
        buffer.fill(0);

        // Build the explicit presenter output contract and convert into a paint plan.
        let mut view = GpuShellView::from_shell_regions(regions);
        // Inject the presenter-facing TabStrip (additive; preserves existing API).
        view.tabs = tabs.clone();
        let plan = GpuPaintPlan::from_view(&view);

        // Delegate execution to the explicit paint-plan executor (pure, dumb).
        execute_paint_plan(&plan, buffer, width, height);
    }

    /// Native window runner (no-op in the presenter).
    ///
    /// We intentionally avoid embedding winit/pixels usage in the presenter to
    /// keep the presenter free of platform API churn. The binary (src/bin/gpu_shell.rs)
    /// owns the native event loop and uses the presenter's pure functions
    /// (map_regions + paint_to_buffer) to render into a framebuffer.
    pub fn run_native(_initial_width: u32, _initial_height: u32) {
        // No-op: the native bootstrap lives in the gpu_shell binary to avoid
        // version/API coupling inside this presenter module.
    }

    /// Deterministic, additive debug summary that consumes the derived `shell_tone`.
    /// Returns a single-line, read-only string: `shell_tone=<value>`.
    pub fn debug_summary(view: &GpuShellView) -> String {
        format!("shell_tone={}", view.shell_tone.as_str())
    }
}

// ---------------------------------------------------------------------
// Small, explicit tab navigation action seam
//
// Architectural notes (concise):
// - This tiny, presenter-local API exposes deterministic tab navigation
//   intents as a minimal action shape that callers (desktop input bridge /
//   harness / application) can use to compute which buffer id should be
//   activated. The function below re-uses the existing TabStrip navigation
//   rules (next_active_id / prev_active_id) and returns the id to be fed
//   into the existing active-buffer flow. No state is mutated here and
//   no new crates are introduced.
//
// Usage:
// - Call compute_tab_action_target(...) with the desired action, the
//   current opened buffers list and the current active id (if any).
// - If Some(id) is returned, pass that id into the application's existing
//   active-buffer setter (unchanged wiring).
//
// This keeps the source-of-truth (opened buffers + active id) in the same
// place and makes the navigation reachable as an explicit action/intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabAction {
    /// Activate the next tab. `wrap` controls whether we wrap at the end.
    ActivateNext { wrap: bool },
    /// Activate the previous tab. `wrap` controls whether we wrap at the start.
    ActivatePrevious { wrap: bool },
    /// Activate a specific tab by its string id.
    ///
    /// Semantics:
    /// - If `id` is not present in `opened` -> no-op (returns None).
    /// - If `id` equals the current active id -> no-op (returns None).
    /// - Otherwise returns Some(id) so the outer layer can perform activation.
    ActivateById { id: String },
}

/// Compute the id that should become active when applying `action`.
///
/// - `opened`: ordered slice of (id, display) pairs (source-of-truth).
/// - `current_active`: optional currently-active id.
///
/// Returns: Option<String> — the id that should be passed to the
/// existing active-buffer flow. The caller is responsible for performing
/// the actual activation (this keeps wiring/side-effects outside the
/// presenter and preserves dependency inversion).
pub fn compute_tab_action_target(
    action: TabAction,
    opened: &[(String, String)],
    current_active: Option<&str>,
) -> Option<String> {
    match action {
        TabAction::ActivateNext { wrap } => {
            let ts = TabStrip::from_opened_and_active(opened, current_active);
            ts.next_active_id(wrap)
        }
        TabAction::ActivatePrevious { wrap } => {
            let ts = TabStrip::from_opened_and_active(opened, current_active);
            ts.prev_active_id(wrap)
        }
        TabAction::ActivateById { id } => {
            // If the requested id is not among opened buffers -> no-op.
            if !opened.iter().any(|(oid, _)| oid == &id) {
                return None;
            }
            // If it's already active -> no-op.
            if current_active.map(|a| a == id.as_str()).unwrap_or(false) {
                return None;
            }
            // Otherwise return the requested id for outer-layer activation.
            Some(id)
        }
    }
}

/// Apply a `TabAction` through the deterministic resolution path and invoke
/// the provided side-effecting setter when a target id is computed.
///
/// This function centralizes the presenter-facing action -> target resolution
/// by reusing `compute_tab_action_target` and ensures desktop callers can
/// perform the mutation (activate the buffer) in their usual outer-layer flow.
///
/// - `apply` is invoked only when a target id is produced and receives the
///    chosen id (string slice) to pass into the application's activation flow.
/// - Returns the chosen id (Some) or None when no target exists (no buffers).
pub fn apply_tab_action<F>(
    action: TabAction,
    opened: &[(String, String)],
    current_active: Option<&str>,
    mut _apply: F,
) -> Option<String>
where
    F: FnMut(&str),
{
    if let Some(id) = compute_tab_action_target(action, opened, current_active) {
        _apply(&id);
        Some(id)
    } else {
        None
    }
}

/// New minimal focus action seam for tab-strip selection/focus semantics.
///
/// Focus is independent of activation. The outer-layer may hold focused state
/// in presenter-facing TabStrip, or callers may compute and apply focus via
/// these helpers. Focus changes do not mutate opened/active state; they are
/// pure and return the chosen id for the caller to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusAction {
    FocusNext { wrap: bool },
    FocusPrevious { wrap: bool },
    FocusById { id: String },
}

/// Compute the id that should become focused when applying `action`.
///
/// - `opened`: ordered slice of (id, display) pairs (source-of-truth).
/// - `current_focused`: optional currently-focused id.
///
/// Returns: Option<String> — the id that should become focused.
pub fn compute_focus_action_target(
    action: FocusAction,
    opened: &[(String, String)],
    current_focused: Option<&str>,
) -> Option<String> {
    match action {
        FocusAction::FocusNext { wrap } => {
            let ts = TabStrip::from_opened_and_active(opened, None);
            // derive focused id deterministically from TabStrip semantics
            let focused_idx = ts.focused_index();
            let len = ts.tabs.len();
            if len == 0 {
                None
            } else if len == 1 {
                Some(ts.tabs[0].id.clone())
            } else if let Some(idx) = focused_idx {
                if idx + 1 < len {
                    Some(ts.tabs[idx + 1].id.clone())
                } else if wrap {
                    Some(ts.tabs[0].id.clone())
                } else {
                    Some(ts.tabs[idx].id.clone())
                }
            } else {
                // no focused -> choose first
                Some(ts.tabs[0].id.clone())
            }
        }
        FocusAction::FocusPrevious { wrap } => {
            let ts = TabStrip::from_opened_and_active(opened, None);
            let focused_idx = ts.focused_index();
            let len = ts.tabs.len();
            if len == 0 {
                None
            } else if len == 1 {
                Some(ts.tabs[0].id.clone())
            } else if let Some(idx) = focused_idx {
                if idx > 0 {
                    Some(ts.tabs[idx - 1].id.clone())
                } else if wrap {
                    Some(ts.tabs[len - 1].id.clone())
                } else {
                    Some(ts.tabs[idx].id.clone())
                }
            } else {
                // no focused -> choose last deterministically
                Some(ts.tabs[len - 1].id.clone())
            }
        }
        FocusAction::FocusById { id } => {
            if !opened.iter().any(|(oid, _)| oid == &id) {
                None
            } else if current_focused.map(|a| a == id.as_str()).unwrap_or(false) {
                None
            } else {
                Some(id)
            }
        }
    }
}

/// Apply a focus action through the deterministic resolution path and invoke
/// the provided setter when a focus target id is computed.
///
/// - `apply` is invoked only when a target id is produced and receives the
///    chosen id (string slice) so outer layer can set presenter-facing focus.
pub fn apply_focus_action<F>(
    action: FocusAction,
    opened: &[(String, String)],
    current_focused: Option<&str>,
    mut _apply: F,
) -> Option<String>
where
    F: FnMut(&str),
{
    if let Some(id) = compute_focus_action_target(action, opened, current_focused) {
        _apply(&id);
        Some(id)
    } else {
        None
    }
}

/// Activate the currently-focused tab by delegating to the existing ActivateById
/// semantics. This reuses `apply_tab_action` to ensure activation behavior is
/// identical to direct ActivateById usage (no-op when focused id equals active,
/// or when focused id missing).
pub fn activate_focused<F>(
    opened: &[(String, String)],
    current_active: Option<&str>,
    current_focused: Option<&str>,
    mut _apply: F,
) -> Option<String>
where
    F: FnMut(&str),
{
    if let Some(fid) = current_focused {
        // If the focused id is not present -> no-op
        if !opened.iter().any(|(oid, _)| oid == fid) {
            return None;
        }
        // If already active -> no-op
        if current_active.map(|a| a == fid).unwrap_or(false) {
            return None;
        }
        // Delegate to apply_tab_action to reuse ActivateById semantics.
        apply_tab_action(
            TabAction::ActivateById { id: fid.to_string() },
            opened,
            current_active,
            _apply,
        )
    } else {
        None
    }
}

/// Execute a paint plan into an RGBA8 buffer.
///
/// This executor is intentionally dumb: it follows the GpuPaintPlan operations
/// exactly and writes pixels into the provided buffer. It performs a size check
/// and returns early when the buffer size does not match width*height*4.
/// Small, public keyboard event representation for the desktop input bridge.
///
/// Kept intentionally minimal so presenter code and unit tests can construct
/// deterministic input events without depending on platform types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub ctrl: bool,
    pub shift: bool,
    /// Simple key name (e.g. "Tab", "A").
    pub key: String,
}

impl KeyEvent {
    fn is_ctrl_tab(&self) -> bool {
        self.ctrl && self.key == "Tab"
    }

    fn is_plain_tab(&self) -> bool {
        !self.ctrl && self.key == "Tab"
    }

    fn is_enter(&self) -> bool {
        self.key == "Enter"
    }
}

/// Map a keyboard event into a TabAction and apply it through the existing
/// deterministic resolution path (apply_tab_action). This keeps selection
/// logic centralized in compute_tab_action_target and apply_tab_action.
///
/// Mapping rules:
/// - Ctrl+Tab -> ActivateNext { wrap: true }
/// - Ctrl+Shift+Tab -> ActivatePrevious { wrap: true }
/// - Any other key -> no-op (returns None)
pub fn handle_key_event<F>(
    ev: &KeyEvent,
    opened: &[(String, String)],
    current_active: Option<&str>,
    apply: F,
) -> Option<String>
where
    F: FnMut(&str),
{
    if !ev.is_ctrl_tab() {
        return None;
    }

    if ev.shift {
        apply_tab_action(TabAction::ActivatePrevious { wrap: true }, opened, current_active, apply)
    } else {
        apply_tab_action(TabAction::ActivateNext { wrap: true }, opened, current_active, apply)
    }
}

/// Map keyboard events into focus/activation UI events and route them through
/// the existing deterministic presenter-level focus/activation helpers.
///
/// Mapping rules:
/// - Tab (no ctrl) -> FocusNext { wrap: true }
/// - Shift+Tab (no ctrl) -> FocusPrevious { wrap: true }
/// - Enter -> ActivateFocused (delegates to activate_focused)
/// - Any other key -> no-op
///
/// This function intentionally reuses existing FocusAction / apply_focus_action
/// and activate_focused helpers so resolution and no-op semantics remain stable.
pub fn handle_focus_key_event<F, G>(
    ev: &KeyEvent,
    opened: &[(String, String)],
    current_active: Option<&str>,
    current_focused: Option<&str>,
    apply_focus: &mut F,
    apply_activate: &mut G,
) -> Option<String>
where
    F: FnMut(&str),
    G: FnMut(&str),
{
    // Tab navigation (focus only) — ignore Ctrl+Tab (reserved for activation cycling).
    if ev.is_plain_tab() {
        if ev.shift {
            apply_focus_action(
                FocusAction::FocusPrevious { wrap: true },
                opened,
                current_focused,
                apply_focus,
            )
        } else {
            apply_focus_action(
                FocusAction::FocusNext { wrap: true },
                opened,
                current_focused,
                apply_focus,
            )
        }
    } else if ev.is_enter() {
        // Confirm/activate the currently focused tab (delegates to existing activation path).
        activate_focused(opened, current_active, current_focused, apply_activate)
    } else {
        None
    }
}

/// A single tab entry in the presenter-facing tab strip projection.
///
/// - `id` is the buffer identity (string form, stable across the projection).
/// - `display` is the deterministic label shown to the UI (e.g. buffer name).
/// - `active` marks whether this tab is the active buffer.
/// - `index` preserves the original ordering from the opened-buffers source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabEntry {
    pub id: String,
    pub display: String,
    pub active: bool,
    /// Focused state is distinct from `active`. It represents the UI-level
    /// selection/focus that can be moved independently of activation and then
    /// confirmed to perform activation via `ActivateById`.
    pub focused: bool,
    pub index: usize,
}

/// Simple presenter-facing tab strip projection.
///
/// The presenter constructs this from an ordered list of opened buffers and
/// an optional active buffer id. It is intentionally small and additive to the
/// existing presenter model so desktop/harness code can immediately consume it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TabStrip {
    pub tabs: Vec<TabEntry>,
}

impl TabStrip {
    /// Build a TabStrip from an ordered slice of (id, display) pairs and an optional active id.
    /// Ordering is preserved; exactly one tab will be marked active when the `active` id matches.
    pub fn from_opened_and_active(opened: &[(String, String)], active: Option<&str>) -> Self {
        let mut tabs: Vec<TabEntry> = Vec::with_capacity(opened.len());
        for (i, pair) in opened.iter().enumerate() {
            let id = pair.0.clone();
            let display = pair.1.clone();
            let active_flag = active.map(|a| a == id).unwrap_or(false);
            tabs.push(TabEntry { id, display, active: active_flag, focused: false, index: i });
        }
        TabStrip { tabs }
    }

    /// Return the index of the first active tab, if any.
    pub fn active_index(&self) -> Option<usize> {
        self.tabs.iter().position(|t| t.active)
    }

    /// Return the index of the first focused tab, if any.
    pub fn focused_index(&self) -> Option<usize> {
        self.tabs.iter().position(|t| t.focused)
    }

    /// Compute the id that should become active for the "next tab" intent.
    ///
    /// Deterministic rules:
    /// - If there are no tabs -> None.
    /// - If there is exactly one tab -> that tab's id.
    /// - If an active tab exists:
    ///     - If it's not the last tab -> select the next tab in order.
    ///     - If it's the last tab:
    ///         - If wrap == true -> wrap to the first tab.
    ///         - If wrap == false -> remain on the last tab (no-op).
    /// - If no active tab exists -> deterministically select the first tab.
    pub fn next_active_id(&self, wrap: bool) -> Option<String> {
        let len = self.tabs.len();
        if len == 0 {
            return None;
        }
        if len == 1 {
            return Some(self.tabs[0].id.clone());
        }

        if let Some(idx) = self.active_index() {
            if idx + 1 < len {
                return Some(self.tabs[idx + 1].id.clone());
            } else if wrap {
                return Some(self.tabs[0].id.clone());
            } else {
                // no-op: remain on current active
                return Some(self.tabs[idx].id.clone());
            }
        }

        // No active tab: choose first deterministically.
        Some(self.tabs[0].id.clone())
    }

    /// Compute the id that should become active for the "previous tab" intent.
    ///
    /// Deterministic rules:
    /// - If there are no tabs -> None.
    /// - If there is exactly one tab -> that tab's id.
    /// - If an active tab exists:
    ///     - If it's not the first tab -> select the previous tab in order.
    ///     - If it's the first tab:
    ///         - If wrap == true -> wrap to the last tab.
    ///         - If wrap == false -> remain on the first tab (no-op).
    /// - If no active tab exists -> deterministically select the last tab.
    pub fn prev_active_id(&self, wrap: bool) -> Option<String> {
        let len = self.tabs.len();
        if len == 0 {
            return None;
        }
        if len == 1 {
            return Some(self.tabs[0].id.clone());
        }

        if let Some(idx) = self.active_index() {
            if idx > 0 {
                return Some(self.tabs[idx - 1].id.clone());
            } else if wrap {
                return Some(self.tabs[len - 1].id.clone());
            } else {
                // no-op: remain on current active
                return Some(self.tabs[idx].id.clone());
            }
        }

        // No active tab: choose last deterministically.
        Some(self.tabs[len - 1].id.clone())
    }

    /// Return a new TabStrip with the given id marked active.
    /// If the id is not found, returns self.clone() unchanged.
    pub fn with_active_id(&self, id: &str) -> Self {
        let mut new = self.clone();
        let mut found = false;
        for t in new.tabs.iter_mut() {
            if t.id == id {
                t.active = true;
                found = true;
            } else {
                t.active = false;
            }
        }
        if found {
            new
        } else {
            // id not found: return original (no change)
            self.clone()
        }
    }

    /// Return a new TabStrip with the given id marked focused.
    /// If the id is not found, returns self.clone() unchanged.
    pub fn with_focused_id(&self, id: &str) -> Self {
        let mut new = self.clone();
        let mut found = false;
        for t in new.tabs.iter_mut() {
            if t.id == id {
                t.focused = true;
                found = true;
            } else {
                t.focused = false;
            }
        }
        if found { new } else { self.clone() }
    }

    /// Move focus to the next tab deterministically and return a new TabStrip.
    /// - If there are no tabs -> return clone unchanged.
    /// - If there is one tab -> it becomes focused.
    /// - If a focused tab exists -> move focus to next (wrap if requested).
    /// - If no focused tab exists -> focus the first tab.
    pub fn focus_next(&self, wrap: bool) -> Self {
        let len = self.tabs.len();
        if len == 0 {
            return self.clone();
        }
        if len == 1 {
            let mut n = self.clone();
            n.tabs[0].focused = true;
            return n;
        }

        if let Some(idx) = self.focused_index() {
            let next_idx = if idx + 1 < len {
                idx + 1
            } else if wrap {
                0
            } else {
                idx
            };
            let mut n = self.clone();
            for (i, t) in n.tabs.iter_mut().enumerate() {
                t.focused = i == next_idx;
            }
            return n;
        }

        // No focused tab: focus first deterministically.
        let mut n = self.clone();
        for (i, t) in n.tabs.iter_mut().enumerate() {
            t.focused = i == 0;
        }
        n
    }

    /// Move focus to the previous tab deterministically and return a new TabStrip.
    /// - If there are no tabs -> return clone unchanged.
    /// - If there is one tab -> it becomes focused.
    /// - If a focused tab exists -> move focus to previous (wrap if requested).
    /// - If no focused tab exists -> focus the last tab.
    pub fn focus_prev(&self, wrap: bool) -> Self {
        let len = self.tabs.len();
        if len == 0 {
            return self.clone();
        }
        if len == 1 {
            let mut n = self.clone();
            n.tabs[0].focused = true;
            return n;
        }

        if let Some(idx) = self.focused_index() {
            let prev_idx = if idx > 0 {
                idx - 1
            } else if wrap {
                len - 1
            } else {
                idx
            };
            let mut n = self.clone();
            for (i, t) in n.tabs.iter_mut().enumerate() {
                t.focused = i == prev_idx;
            }
            return n;
        }

        // No focused tab: focus last deterministically.
        let mut n = self.clone();
        for (i, t) in n.tabs.iter_mut().enumerate() {
            t.focused = i + 1 == len;
        }
        n
    }
}
