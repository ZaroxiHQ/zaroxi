/// Minimal engine-facing panel chrome description.
///
/// This type is intentionally small and semantic-only: it carries the tab strip
/// labels, active/focused semantics and a few small extras that the engine can
/// reuse when moving rendering responsibility inward from presenters.
///
/// Phase 38: Removed IDE-specific fields (ai_indicator, content_preview).
/// Renamed focus_slot to active_panel_id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
    /// 1-based index of the tab (kept as u32 to match presenter conventions).
    pub index: u32,
    /// Stable identifier for the tab (presenter-provided).
    pub id: String,
    /// Short display label for the tab (already normalized by the presenter).
    pub label: String,
    /// Whether the tab is currently marked active.
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellChrome {
    /// Optional small chrome label (presenter-provided).
    pub chrome_label: Option<String>,

    /// Projected tab strip (deterministic order).
    pub tabs: Vec<Tab>,

    /// Index of the active tab within `tabs`, if any.
    pub active_tab_index: Option<usize>,

    /// Optional active panel identifier (was focus_slot).
    pub active_panel_id: Option<String>,

    /// Optional status text (semantic).
    pub status_text: Option<String>,
}
