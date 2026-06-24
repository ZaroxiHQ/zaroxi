/*!
Editor Phase 2 — Centralized shell layout constants.

All magic numbers for shell regions, scrollbars, text layout, and
sub-region positioning live here. This is the single authority for
dimensions used by:
- layout.rs    (Taffy tree)
- frame.rs     (UiBlock scrollbar blocks)
- app.rs       (visible-lines math)
- rail.rs      (explorer CTA button)
- shell_builder.rs (widget tree positioning)
- editor.rs    (gutter block)

Whenever a dimension needs adjusting, it should be changed HERE.
*/

// ── Main shell region heights / widths ──

pub const TITLEBAR_H: f32 = 32.0;
pub const STATUS_H: f32 = 26.0;
pub const RAIL_W: f32 = 44.0;
pub const RAIL_STRIP_H: f32 = 40.0;
pub const TAB_STRIP_H: f32 = 28.0;
pub const BREADCRUMB_H: f32 = 20.0;
pub const GUTTER_W: f32 = 52.0;
pub const GUTTER_COLLAPSE_THRESHOLD: f32 = 200.0;

pub const TERMINAL_MIN_H: f32 = 24.0;
pub const TERMINAL_BASIS_H: f32 = 150.0;
pub const TERMINAL_HEADER_H: f32 = 26.0;

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

// ── Rail / activity bar ──

pub const RAIL_ICON_W_OFFSET: f32 = 14.0;
pub const RAIL_ICON_H: f32 = 28.0;
pub const RAIL_ICON_GAP: f32 = 4.0;
pub const RAIL_ICON_START_Y: f32 = 10.0;
pub const RAIL_DIVIDER_INSET: f32 = 12.0;
pub const RAIL_BOTTOM_START_OFFSET: f32 = 12.0;

// ── Tab strip ──

pub const TAB_W_INACTIVE: f32 = 110.0;
pub const TAB_W_ACTIVE_EXTRA: f32 = 10.0;
pub const TAB_Y_OFFSET: f32 = 1.0;

// ── Terminal tabs ──

pub const TERMINAL_TAB_W: f32 = 70.0;
pub const TERMINAL_TAB_H: f32 = 22.0;
pub const TERMINAL_TAB_Y_OFFSET: f32 = 2.0;
pub const TERMINAL_TAB_X_OFFSET: f32 = 8.0;
pub const TERMINAL_TAB_GAP: f32 = 4.0;

// ── AI panel actions ──

pub const AI_CLOSE_BTN_W: f32 = 20.0;
pub const AI_ACTION_BTN_W: f32 = 64.0;
pub const AI_ACTION_BTN_H: f32 = 22.0;
pub const AI_ACTION_BTN_GAP: f32 = 8.0;
pub const AI_INPUT_H: f32 = 28.0;

// ── Status bar ──

pub const STATUSBAR_PILL_H_INSET: f32 = 6.0;
pub const STATUSBAR_PILL_Y: f32 = 3.0;
pub const STATUSBAR_BADGE_W: f32 = 48.0;

// ── Explorer tree / panel items ──

/// Vertical pitch between explorer rows (row height incl. spacing).
pub const EXPLORER_ROW_H: f32 = 22.0;
/// Visible height of a row's selection/highlight rect (centered in the pitch).
pub const EXPLORER_ROW_VIS_H: f32 = 18.0;
/// Small structural left inset before the row's disclosure/type icon. The
/// chevron now lives in the label glyphs, so this is just breathing room, not a
/// reserved chevron gutter.
pub const EXPLORER_ROW_TEXT_INSET: f32 = 4.0;
/// Width of the fixed disclosure+icon column (chevron + space + type icon).
/// The filename is drawn in its own column at `row_x + EXPLORER_GLYPH_COL_W`,
/// so a double-width Nerd Font icon can never shift the name column.
pub const EXPLORER_GLYPH_COL_W: f32 = 34.0;
/// Left inset the renderer applies to a block's title text (mirrors the
/// hardcoded title pad in `core.rs`). Used to place glyph/name columns at exact
/// x positions by pre-subtracting it from the block origin.
pub const EXPLORER_TITLE_PAD: f32 = 8.0;
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

// ── Content empty state ──

pub const EMPTY_STATE_X_OFFSET: f32 = 40.0;
pub const EMPTY_STATE_Y_OFFSET: f32 = 60.0;
pub const EMPTY_STATE_W: f32 = 200.0;
pub const EMPTY_STATE_H: f32 = 40.0;

// ── Helper: compute explorer CTA button rect from sidebar region rect ──

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

// ── Helper: compute scrollbar track/thumb geometry ──

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

/// Compute scrollbar track and thumb geometry from a panel region rect.
/// Returns (track_x, track_y, track_w, track_h, thumb_h).
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

/// Compute visible lines from region height using the SAME content-rect insets
/// the renderer applies, so the line count equals the number of full rows the
/// renderer can actually draw (no underfill, no bottom-row drop).
///
/// Renderer (`render_frame_inner`) content rect:
///   content_h = region_h - header(28) - content_padding(8) * 2 = region_h - 44.
/// Rows that fully fit = floor(content_h / LINE_HEIGHT).
pub fn visible_lines_from_region(region_h: f32) -> usize {
    // Mirror the renderer: header height + content_padding*2.
    const RENDER_CONTENT_PADDING: f32 = 8.0;
    let usable_h = region_h - CONTENT_HEADER_H - RENDER_CONTENT_PADDING * 2.0;
    (usable_h / LINE_HEIGHT).floor().max(1.0) as usize
}
