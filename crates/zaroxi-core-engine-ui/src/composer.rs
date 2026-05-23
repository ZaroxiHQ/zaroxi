use crate::scene::RectPrimitive;
use taffy::prelude::*;

/// Build a simple, deterministic shell UI composed of:
/// - background (full window)
/// - top bar (fixed height)
/// - middle row: sidebar (fixed width) + editor (flex)
/// - status bar (fixed height)
///
/// Returns a stable vector of RectPrimitive in paint order (background first).
pub fn build_shell_ui(window_w: u32, window_h: u32) -> Vec<RectPrimitive> {
    // Defensive clamps to convert to f32 viewport
    let w = window_w.max(1) as f32;
    let h = window_h.max(1) as f32;

    // design constants (kept small and deterministic)
    const TITLE_H: f32 = 28.0;
    const STATUS_H: f32 = 22.0;
    const SIDEBAR_W: f32 = 240.0;

    // Build a taffy layout tree
    let mut taffy = taffy::Taffy::new();

    // Root: column, size = viewport
    let root_style = Style {
        size: Size {
            width: Dimension::Points(w),
            height: Dimension::Points(h),
        },
        flex_direction: FlexDirection::Column,
        ..Default::default()
    };
    let root = taffy.new_with_children(root_style, &[]).expect("taffy new node");

    // topbar
    let top_style = Style {
        size: Size {
            width: Dimension::Percent(1.0),
            height: Dimension::Points(TITLE_H),
        },
        ..Default::default()
    };
    let top = taffy.new_leaf(top_style).expect("taffy top");

    // middle row (sidebar + editor)
    let middle_style = Style {
        size: Size {
            width: Dimension::Percent(1.0),
            height: Dimension::Auto,
        },
        flex_grow: 1.0,
        flex_direction: FlexDirection::Row,
        ..Default::default()
    };
    let middle = taffy.new_with_children(middle_style, &[]).expect("taffy middle");

    // sidebar
    let side_style = Style {
        size: Size {
            width: Dimension::Points(SIDEBAR_W),
            height: Dimension::Percent(1.0),
        },
        ..Default::default()
    };
    let sidebar = taffy.new_leaf(side_style).expect("taffy sidebar");

    // editor area
    let editor_style = Style {
        size: Size {
            width: Dimension::Auto,
            height: Dimension::Percent(1.0),
        },
        flex_grow: 1.0,
        ..Default::default()
    };
    let editor = taffy.new_leaf(editor_style).expect("taffy editor");

    // attach children: middle -> [sidebar, editor]
    taffy.add_child(middle, sidebar).unwrap();
    taffy.add_child(middle, editor).unwrap();

    // status bar
    let status_style = Style {
        size: Size {
            width: Dimension::Percent(1.0),
            height: Dimension::Points(STATUS_H),
        },
        ..Default::default()
    };
    let status = taffy.new_leaf(status_style).expect("taffy status");

    // attach root children: top, middle, status
    taffy.add_child(root, top).unwrap();
    taffy.add_child(root, middle).unwrap();
    taffy.add_child(root, status).unwrap();

    // compute layout
    taffy
        .compute_layout(root, taffy::geometry::Size::undefined())
        .expect("taffy compute layout");

    // gather computed rects
    let mut rects: Vec<RectPrimitive> = Vec::new();

    // background (full window) — paint first
    rects.push(RectPrimitive::new(0.0, 0.0, w, h, [13.0 / 255.0, 14.0 / 255.0, 17.0 / 255.0, 1.0]));

    // helper to push rect for a node
    let mut push_node_rect = |node, color: [f32; 4]| {
        if let Ok(layout) = taffy.layout(node) {
            rects.push(RectPrimitive::new(
                layout.location.x,
                layout.location.y,
                layout.size.width,
                layout.size.height,
                color,
            ));
        }
    };

    // top bar color
    push_node_rect(top, [0.18, 0.18, 0.22, 1.0]);
    // sidebar color
    push_node_rect(sidebar, [0.12, 0.12, 0.14, 1.0]);
    // editor area color
    push_node_rect(editor, [0.08, 0.09, 0.11, 1.0]);
    // status bar color
    push_node_rect(status, [0.15, 0.15, 0.17, 1.0]);

    rects
}
