/// Minimal engine-facing shell chrome description.
///
/// This type is intentionally small and semantic-only: it carries the tab strip
/// labels, active/focused semantics and a few small extras that the engine can
/// reuse when moving rendering responsibility inward from presenters.
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

    /// Optional focus slot name (observational; may be None).
    pub focus_slot: Option<String>,

    /// Optional status text (semantic).
    pub status_text: Option<String>,

    /// Optional AI indicator text (semantic).
    pub ai_indicator: Option<String>,

    /// Optional content preview (semantic).
    pub content_preview: Option<String>,
}
