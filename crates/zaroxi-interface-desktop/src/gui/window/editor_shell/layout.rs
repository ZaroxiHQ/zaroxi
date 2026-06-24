/*!
Editor Phase 1 — Taffy-based shell layout engine.

Replaces the ad-hoc layout math in `ShellFrame::new()` and `ShellLayout::from_window_size()`
with a single deterministic Taffy flexbox tree. All region rects are computed by
Taffy's layout solver, making responsive behavior implicit in flex-box rules
rather than explicit breakpoints.

Responsive rules (encoded via flex-shrink/grow and min/max sizes):
- SidePane:     flex-shrink=3, basis=280, min=0   → collapses first
- Assistant:    flex-shrink=3, basis=300, min=0   → collapses first
- Minimap:      flex-shrink=2, basis=60,  min=0   → collapses when side panels do
- EditorColumn: flex-grow=1,     min=120           → always gets remaining space
- RailPane:     flex-shrink=0,  width=44           → never collapses
*/

use std::collections::HashMap;

use taffy::prelude::*;
use taffy::{NodeId, Style, TaffyTree};

/// All named region rects produced by the Taffy layout.
///
/// Coordinates are in window-space (origin top-left) using f32 pixels.
#[derive(Debug, Clone)]
pub struct EditorShellLayout {
    pub window_size: (f32, f32),

    pub toolbar_rect: (f32, f32, f32, f32),
    pub rail_rect: (f32, f32, f32, f32),
    pub sidebar_rect: (f32, f32, f32, f32),
    pub editor_tabs_rect: (f32, f32, f32, f32),
    pub breadcrumb_rect: (f32, f32, f32, f32),
    pub gutter_rect: (f32, f32, f32, f32),
    pub editor_content_rect: (f32, f32, f32, f32),
    pub terminal_rect: (f32, f32, f32, f32),
    pub assistant_rect: (f32, f32, f32, f32),
    pub assistant_header_rect: (f32, f32, f32, f32),
    pub assistant_content_rect: (f32, f32, f32, f32),
    pub status_bar_rect: (f32, f32, f32, f32),
}

/// Named Taffy nodes in the layout tree.
struct Nodes {
    toolbar: NodeId,
    rail: NodeId,
    sidebar: NodeId,
    tab_strip: NodeId,
    breadcrumb: NodeId,
    gutter: NodeId,
    editor_content: NodeId,
    terminal: NodeId,
    assistant: NodeId,
    status_bar: NodeId,
}

// ── Layout constants (from centralised constants module) ──

use super::constants::{
    AI_HEADER_H, ASSISTANT_BASIS_W, BREADCRUMB_H, EDITOR_MIN_H, EDITOR_MIN_W, GUTTER_W,
    RAIL_STRIP_H, SIDEBAR_BASIS_W, STATUS_H, TAB_STRIP_H, TERMINAL_BASIS_H, TERMINAL_MIN_H,
    TITLEBAR_H,
};

/// Build a Taffy tree for the IDE shell layout and compute the final rects.
pub fn compute_layout(window_w: f32, window_h: f32) -> EditorShellLayout {
    let mut taffy = TaffyTree::new();

    // ── Column leaves (fixed heights) ──
    let titlebar_style = leaf_fixed(TITLEBAR_H);
    let status_style = leaf_fixed(STATUS_H);
    let tab_strip_style = leaf_fixed(TAB_STRIP_H);
    let breadcrumb_style = leaf_fixed(BREADCRUMB_H);

    let titlebar = taffy.new_leaf(titlebar_style).unwrap();
    let status = taffy.new_leaf(status_style).unwrap();
    let tab_strip = taffy.new_leaf(tab_strip_style).unwrap();
    let breadcrumb = taffy.new_leaf(breadcrumb_style).unwrap();

    // ── Rail (bottom strip within the left column, fixed height) ──
    let rail_style = Style {
        size: Size { width: auto(), height: length(RAIL_STRIP_H) },
        flex_shrink: 0.0,
        ..Default::default()
    };
    let rail = taffy.new_leaf(rail_style).unwrap();

    // ── Sidebar (fills remaining space above the rail in the left column) ──
    let sidebar_style = Style {
        flex_grow: 1.0,
        min_size: Size { width: auto(), height: length(0.0) },
        ..Default::default()
    };
    let sidebar = taffy.new_leaf(sidebar_style).unwrap();

    // ── Left column: sidebar on top, rail strip at the bottom ──
    let left_column = taffy
        .new_with_children(
            Style {
                flex_basis: length(SIDEBAR_BASIS_W),
                min_size: Size { width: length(0.0), height: auto() },
                max_size: Size { width: length(SIDEBAR_BASIS_W), height: auto() },
                flex_shrink: 3.0,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &[sidebar, rail],
        )
        .unwrap();

    // ── Gutter (fixed width, collapsible via min 0) ──
    let gutter_style = Style {
        size: Size { width: length(GUTTER_W), height: auto() },
        flex_shrink: 1.0,
        min_size: Size { width: length(0.0), height: auto() },
        ..Default::default()
    };
    let gutter = taffy.new_leaf(gutter_style).unwrap();

    // ── Editor content (flex-grow 1 in the editor body row) ──
    let editor_body_style = Style {
        flex_grow: 1.0,
        min_size: Size { width: length(0.0), height: auto() },
        ..Default::default()
    };
    let editor_content = taffy.new_leaf(editor_body_style).unwrap();

    // ── Editor body row: gutter + content ──
    let editor_body_row = taffy
        .new_with_children(
            Style {
                flex_grow: 1.0,
                min_size: Size { width: auto(), height: length(EDITOR_MIN_H) },
                ..Default::default()
            },
            &[gutter, editor_content],
        )
        .unwrap();

    // ── Terminal panel (shrinks relative to editor body) ──
    let terminal_style = Style {
        flex_basis: length(TERMINAL_BASIS_H),
        min_size: Size { width: auto(), height: length(TERMINAL_MIN_H) },
        flex_shrink: 1.0,
        ..Default::default()
    };
    let terminal = taffy.new_leaf(terminal_style).unwrap();

    // ── Editor column (tab strip → breadcrumb → body-row → terminal) ──
    let editor_col = taffy
        .new_with_children(
            Style {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_direction: FlexDirection::Column,
                min_size: Size { width: length(EDITOR_MIN_W), height: auto() },
                ..Default::default()
            },
            &[tab_strip, breadcrumb, editor_body_row, terminal],
        )
        .unwrap();

    // ── Assistant pane (shrinks before editor) ──
    let assistant_style = Style {
        flex_basis: length(ASSISTANT_BASIS_W),
        min_size: Size { width: length(0.0), height: auto() },
        max_size: Size { width: length(ASSISTANT_BASIS_W), height: auto() },
        flex_shrink: 3.0,
        ..Default::default()
    };
    let assistant = taffy.new_leaf(assistant_style).unwrap();

    // ── Main content row: left_column | editor-col | assistant ──
    let main_row = taffy
        .new_with_children(
            Style {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                min_size: Size { width: auto(), height: length(80.0) },
                ..Default::default()
            },
            &[left_column, editor_col, assistant],
        )
        .unwrap();

    // ── Root column: titlebar | main-row | status ──
    let root = taffy
        .new_with_children(
            Style {
                size: Size { width: length(window_w), height: length(window_h) },
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &[titlebar, main_row, status],
        )
        .unwrap();

    // Compute the full layout tree.
    let _ = taffy.compute_layout(
        root,
        Size {
            width: AvailableSpace::Definite(window_w),
            height: AvailableSpace::Definite(window_h),
        },
    );

    // Collect absolute coordinates for every named node.
    let positions = collect_abs_positions(&taffy, root);

    let nodes = Nodes {
        toolbar: titlebar,
        rail,
        sidebar,
        tab_strip,
        breadcrumb,
        gutter,
        editor_content,
        terminal,
        assistant,
        status_bar: status,
    };

    // ── Compute sub-regions ──

    let pos = |n: NodeId| -> (f32, f32, f32, f32) {
        let p = positions.get(&n).copied().unwrap_or((0.0, 0.0, 0.0, 0.0));
        (p.0, p.1, p.2, p.3)
    };

    let tool = pos(nodes.toolbar);
    let rail_p = pos(nodes.rail);
    let side = pos(nodes.sidebar);
    let tabs = pos(nodes.tab_strip);
    let crumb = pos(nodes.breadcrumb);
    let gut = pos(nodes.gutter);
    let content = pos(nodes.editor_content);
    let term = pos(nodes.terminal);
    let asst = pos(nodes.assistant);
    let stat = pos(nodes.status_bar);

    // AI panel header/content split derived from the assistant rect.
    let asst_header = (asst.0, asst.1, asst.2, AI_HEADER_H.min(asst.3));
    let asst_content = (asst.0, asst.1 + AI_HEADER_H, asst.2, (asst.3 - AI_HEADER_H).max(0.0));

    EditorShellLayout {
        window_size: (window_w, window_h),
        toolbar_rect: tool,
        rail_rect: rail_p,
        sidebar_rect: side,
        editor_tabs_rect: tabs,
        breadcrumb_rect: crumb,
        gutter_rect: gut,
        editor_content_rect: content,
        terminal_rect: term,
        assistant_rect: asst,
        assistant_header_rect: asst_header,
        assistant_content_rect: asst_content,
        status_bar_rect: stat,
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

fn leaf_fixed(height: f32) -> Style {
    Style { size: Size { width: auto(), height: length(height) }, ..Default::default() }
}

/// Collect absolute (x, y, w, h) positions for all nodes by accumulating
/// parent offsets. Returns a map of NodeId → (x, y, w, h).
fn collect_abs_positions(taffy: &TaffyTree, root: NodeId) -> HashMap<NodeId, (f32, f32, f32, f32)> {
    let mut map = HashMap::new();
    accumulate_positions(taffy, root, 0.0, 0.0, &mut map);
    map
}

fn accumulate_positions(
    taffy: &TaffyTree,
    node: NodeId,
    parent_x: f32,
    parent_y: f32,
    out: &mut HashMap<NodeId, (f32, f32, f32, f32)>,
) {
    let layout = taffy.layout(node).unwrap();
    let abs_x = parent_x + layout.location.x;
    let abs_y = parent_y + layout.location.y;
    let w = layout.size.width;
    let h = layout.size.height;
    out.insert(node, (abs_x, abs_y, w, h));

    for child in taffy.children(node).unwrap() {
        accumulate_positions(taffy, child, abs_x, abs_y, out);
    }
}
