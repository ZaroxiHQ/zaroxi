/*!
Centralized shell layout constants — shared by shell_builder, editor_shell, frame, and
all panel modules.

Every magic number that governs widget positioning, scrollbar geometry, tab sizing,
button placement, and panel sub-region layout lives here. Changing a value here
updates every consumer that imports from this module.
*/

// ── Window chrome ──

pub const TITLEBAR_H: f32 = 32.0;
pub const STATUS_H: f32 = 26.0;
pub const RAIL_W: f32 = 44.0;
pub const RAIL_STRIP_H: f32 = 32.0;

// ── Toolbar window control buttons ──

pub const TOOLBAR_BTN_W: f32 = 32.0;
pub const TOOLBAR_BTN_GAP: f32 = 2.0;
pub const TOOLBAR_BTN_V_INSET: f32 = 4.0;
pub const TOOLBAR_BTN_RIGHT_MARGIN: f32 = 18.0;

// ── Titlebar brand accent ──

pub const BRAND_ACCENT_LEFT: f32 = 10.0;
pub const BRAND_ACCENT_TOP: f32 = 5.0;
pub const BRAND_ACCENT_W: f32 = 32.0;
pub const BRAND_ACCENT_BOTTOM_INSET: f32 = 10.0;

// ── Rail / activity bar icons ──

pub const RAIL_ICON_W_OFFSET: f32 = 14.0;
pub const RAIL_ICON_H: f32 = 28.0;
pub const RAIL_ICON_GAP: f32 = 4.0;
pub const RAIL_ICON_START_Y: f32 = 10.0;
pub const RAIL_DIVIDER_INSET: f32 = 12.0;
pub const RAIL_BOTTOM_START_OFFSET: f32 = 12.0;

// ── Tabs ──

pub const TAB_STRIP_H: f32 = 28.0;
pub const BREADCRUMB_H: f32 = 20.0;
pub const TAB_W_INACTIVE: f32 = 110.0;
pub const TAB_W_ACTIVE_EXTRA: f32 = 10.0;
pub const TAB_Y_HANG: f32 = 1.0;

// ── Terminal tabs ──

pub const TERMINAL_TAB_W: f32 = 70.0;
pub const TERMINAL_TAB_H: f32 = 22.0;
pub const TERMINAL_TAB_Y_OFFSET: f32 = 2.0;
pub const TERMINAL_TAB_X_OFFSET: f32 = 8.0;
pub const TERMINAL_TAB_GAP: f32 = 4.0;

// ── Gutter ──

pub const GUTTER_W: f32 = 52.0;

// ── Terminal panel ──

pub const TERMINAL_MIN_H: f32 = 24.0;
pub const TERMINAL_BASIS_H: f32 = 150.0;
pub const TERMINAL_HEADER_H: f32 = 26.0;

// ── AI panel ──

pub const AI_HEADER_H: f32 = 28.0;

// ── Flex basis / responsive ──

pub const SIDEBAR_BASIS_W: f32 = 280.0;
pub const ASSISTANT_BASIS_W: f32 = 300.0;
pub const MINIMAP_BASIS_W: f32 = 60.0;
pub const EDITOR_MIN_W: f32 = 120.0;
pub const EDITOR_MIN_H: f32 = 40.0;

// ── Scrollbar metrics ──

pub const SB_EDITOR_WIDTH: f32 = 6.0;
pub const SB_EDITOR_INSET_RIGHT: f32 = 3.0;
pub const SB_EDITOR_TRACK_INSET_Y: f32 = 4.0;
pub const SB_EDITOR_TRACK_H_REDUCTION: f32 = 8.0;
pub const SB_EDITOR_THUMB_RATIO: f32 = 0.25;
pub const SB_EDITOR_THUMB_MIN_H: f32 = 20.0;

pub const SB_SIDEBAR_WIDTH: f32 = 4.0;
pub const SB_SIDEBAR_INSET_RIGHT: f32 = 3.0;
pub const SB_SIDEBAR_TRACK_INSET_Y: f32 = 8.0;
pub const SB_SIDEBAR_TRACK_H_REDUCTION: f32 = 16.0;
pub const SB_SIDEBAR_THUMB_RATIO: f32 = 0.5;
pub const SB_SIDEBAR_THUMB_MIN_H: f32 = 16.0;

pub const SB_BOTTOM_WIDTH: f32 = 6.0;
pub const SB_BOTTOM_INSET_RIGHT: f32 = 2.0;
pub const SB_BOTTOM_TRACK_INSET_Y: f32 = 4.0;
pub const SB_BOTTOM_TRACK_H_REDUCTION: f32 = 8.0;
pub const SB_BOTTOM_THUMB_RATIO: f32 = 0.3;
pub const SB_BOTTOM_THUMB_MIN_H: f32 = 16.0;

/// Extra width added to the left of the visual scrollbar for interactive hit area.
/// The visual scrollbar rail is ~6px wide; this expands the clickable gutter to ~30px.
pub const SB_INTERACTIVE_GUTTER_PAD: f32 = 24.0;

// ── Scrollbar spec structs (for compute_scrollbar_geometry) ──

pub struct ScrollbarSpec {
    pub sb_width: f32,
    pub inset_right: f32,
    pub track_inset_y: f32,
    pub track_h_reduction: f32,
    pub thumb_ratio: f32,
    pub thumb_min_h: f32,
}

pub const SB_EDITOR_SPEC: ScrollbarSpec = ScrollbarSpec {
    sb_width: SB_EDITOR_WIDTH,
    inset_right: SB_EDITOR_INSET_RIGHT,
    track_inset_y: SB_EDITOR_TRACK_INSET_Y,
    track_h_reduction: SB_EDITOR_TRACK_H_REDUCTION,
    thumb_ratio: SB_EDITOR_THUMB_RATIO,
    thumb_min_h: SB_EDITOR_THUMB_MIN_H,
};

pub const SB_SIDEBAR_SPEC: ScrollbarSpec = ScrollbarSpec {
    sb_width: SB_SIDEBAR_WIDTH,
    inset_right: SB_SIDEBAR_INSET_RIGHT,
    track_inset_y: SB_SIDEBAR_TRACK_INSET_Y,
    track_h_reduction: SB_SIDEBAR_TRACK_H_REDUCTION,
    thumb_ratio: SB_SIDEBAR_THUMB_RATIO,
    thumb_min_h: SB_SIDEBAR_THUMB_MIN_H,
};

pub const SB_BOTTOM_SPEC: ScrollbarSpec = ScrollbarSpec {
    sb_width: SB_BOTTOM_WIDTH,
    inset_right: SB_BOTTOM_INSET_RIGHT,
    track_inset_y: SB_BOTTOM_TRACK_INSET_Y,
    track_h_reduction: SB_BOTTOM_TRACK_H_REDUCTION,
    thumb_ratio: SB_BOTTOM_THUMB_RATIO,
    thumb_min_h: SB_BOTTOM_THUMB_MIN_H,
};

// ── Text layout / content padding ──

pub const CONTENT_PAD_X: f32 = 8.0;
pub const CONTENT_PAD_Y: f32 = 4.0;
pub const LINE_HEIGHT: f32 = 16.0;
pub const CHAR_WIDTH_STUB: f32 = 8.0;
pub const CONTENT_HEADER_H: f32 = 28.0;

// ── Sidebar / Explorer widget layout ──

pub const SIDEBAR_PAD: f32 = 10.0;
pub const SEARCH_BAR_H: f32 = 26.0;
pub const SEARCH_TO_DIVIDER_GAP: f32 = 8.0;
pub const DIVIDER_SPACE: f32 = 12.0;
pub const EXPLORER_CTA_BTN_W: f32 = 140.0;
pub const EXPLORER_CTA_BTN_H: f32 = 30.0;
pub const EXPLORER_CTA_BTN_Y_OFFSET: f32 = 8.0;
pub const EXPLORER_CTA_BTN_X_EXTRA: f32 = 10.0;

// ── Explorer tree / panel items ──

/// Vertical pitch between explorer rows (row height incl. spacing).
pub const EXPLORER_ROW_H: f32 = 22.0;
/// Visible height of a row's selection/highlight rect (centered in the pitch).
pub const EXPLORER_ROW_VIS_H: f32 = 18.0;
/// Small structural left inset before the row's disclosure/type icon. The
/// chevron now lives in the label glyphs, so this is just breathing room, not a
/// reserved chevron gutter.
pub const EXPLORER_ROW_TEXT_INSET: f32 = 4.0;
/// Total horizontal reduction applied to a row's width so labels never collide
/// with the sidebar scrollbar gutter.
pub const EXPLORER_ROW_W_REDUCTION: f32 = 20.0;
pub const EXPLORER_HEADER_H: f32 = 22.0;
/// Gap between the panel header and the first tree row. Kept at 0 so the tree
/// begins flush at the top of the explorer content area.
pub const EXPLORER_HEADER_TO_ROWS_GAP: f32 = 0.0;
/// Vertical gap between the explorer search box and the first tree row.
pub const EXPLORER_SEARCH_TO_ROWS_GAP: f32 = 8.0;
pub const EXPLORER_INDENT_PX: f32 = 14.0;
pub const EXPLORER_MAX_Y_INSET: f32 = 12.0;

// ── Panel header action button spacing ──

pub const PANEL_ACTION_W: f32 = 20.0;
pub const PANEL_ACTION_X_INSET: f32 = 8.0;
pub const PANEL_ACTION_Y_INSET: f32 = 4.0;
pub const PANEL_ACTION_V_REDUCTION: f32 = 8.0;

// ── AI panel actions ──

pub const AI_CLOSE_BTN_W: f32 = 20.0;
pub const AI_ACTION_BTN_W: f32 = 64.0;
pub const AI_ACTION_BTN_H: f32 = 22.0;
pub const AI_ACTION_BTN_GAP: f32 = 8.0;
pub const AI_INPUT_H: f32 = 28.0;
pub const AI_ACTION_X_INSET: f32 = 10.0;

// ── Status bar ──

pub const STATUSBAR_PILL_H_INSET: f32 = 6.0;
pub const STATUSBAR_PILL_Y: f32 = 3.0;
pub const STATUSBAR_BADGE_W: f32 = 48.0;

// ── Editor content empty state ──

pub const EMPTY_STATE_X_OFFSET: f32 = 40.0;
pub const EMPTY_STATE_Y_OFFSET: f32 = 60.0;
pub const EMPTY_STATE_W: f32 = 200.0;
pub const EMPTY_STATE_H: f32 = 40.0;

// ── WidgetId button indices (must match shell_builder.rs and app.rs dispatch_activation) ──

pub const BTN_ID_MINIMIZE: usize = 0;
pub const BTN_ID_MAXIMIZE: usize = 1;
pub const BTN_ID_CLOSE_WINDOW: usize = 2;
pub const BTN_ID_TERMINAL_CLOSE: usize = 10;
pub const BTN_ID_AI_CLOSE: usize = 11;
pub const BTN_ID_AI_EXPLAIN: usize = 20;
pub const BTN_ID_AI_REVIEW: usize = 21;
pub const BTN_ID_AI_APPLY: usize = 22;
pub const BTN_ID_AI_REJECT: usize = 23;
pub const BTN_ID_EXPLORER_CTA: usize = 30;

// ── WidgetId scrollbar indices ──

pub const SCROLLBAR_ID_BOTTOM: usize = 0;
pub const SCROLLBAR_ID_EDITOR: usize = 1;
pub const SCROLLBAR_ID_SIDEBAR: usize = 2;

// ── Helper: compute scrollbar track + thumb geometry ──

pub fn compute_scrollbar_geometry(
    region: (f32, f32, f32, f32),
    spec: &ScrollbarSpec,
    top_extra: f32,
) -> (f32, f32, f32, f32, f32) {
    let x = region.0 + region.2 - spec.sb_width - spec.inset_right;
    let y = region.1 + spec.track_inset_y + top_extra;
    let w = spec.sb_width;
    let h = (region.3 - spec.track_h_reduction - top_extra).max(0.0);
    let thumb_h = (h * spec.thumb_ratio).max(spec.thumb_min_h).min(h);
    (x, y, w, h, thumb_h)
}

// ── Helper: compute explorer CTA button rect ──

pub fn explorer_cta_button_rect(sidebar_rect: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let x = sidebar_rect.0 + SIDEBAR_PAD + EXPLORER_CTA_BTN_X_EXTRA;
    let y = sidebar_rect.1
        + SIDEBAR_PAD
        + SEARCH_BAR_H
        + SEARCH_TO_DIVIDER_GAP
        + DIVIDER_SPACE
        + EXPLORER_CTA_BTN_Y_OFFSET;
    (x, y, EXPLORER_CTA_BTN_W, EXPLORER_CTA_BTN_H)
}

/// Vertical offset (relative to the sidebar panel top) of the first explorer
/// tree row. The rows sit below the rendered search box, so capacity
/// computation must match the row builders (`rail.rs` / `shell_builder.rs`).
pub fn explorer_first_row_offset(_has_title: bool) -> f32 {
    SIDEBAR_PAD + SEARCH_BAR_H + EXPLORER_SEARCH_TO_ROWS_GAP
}

/// Number of explorer rows that fully fit in the sidebar viewport for a given
/// panel height. Used to clamp the explorer scroll offset and to size the
/// sidebar scrollbar thumb.
pub fn explorer_visible_rows(panel_height: f32, has_title: bool) -> usize {
    let first = explorer_first_row_offset(has_title);
    let usable = panel_height - EXPLORER_MAX_Y_INSET - first;
    (usable / EXPLORER_ROW_H).floor().max(0.0) as usize
}

pub fn visible_lines_from_region(region_h: f32) -> usize {
    let usable_h = region_h - CONTENT_HEADER_H - CONTENT_PAD_Y * 2.0;
    (usable_h / LINE_HEIGHT).max(1.0) as usize
}
