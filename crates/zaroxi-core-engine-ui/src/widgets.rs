use zaroxi_core_engine_style::{InteractionState, WidgetId};
use zaroxi_kernel_math::{Rect, Vec2};

use crate::primitives::{
    Divider, DividerOrientation, HeaderBar, IconSlot, ShellSurfaceSet, StatusPill, Surface,
    TabChrome,
};

// ---------------------------------------------------------------------------
// WidgetHitTarget — describes a hit-testable widget region for input routing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct WidgetHitTarget {
    pub id: WidgetId,
    pub rect: Rect,
    pub label: String,
}

// ---------------------------------------------------------------------------
// ShellWidget — enum over all widget variants in the shell tree
// ---------------------------------------------------------------------------

/// A single shell widget. Each variant carries its geometry, label, and state.
#[derive(Debug, Clone)]
pub enum ShellWidget {
    /// Full-window background
    AppBackground { rect: Rect, fill_color: [f32; 4] },

    /// Titlebar strip
    Titlebar { rect: Rect, fill_color: [f32; 4], brand_label: String },

    /// Activity rail icon item (explorer, search, git, debug, settings...)
    RailItem {
        id: WidgetId,
        rect: Rect,
        label: String,
        fill_color: [f32; 4],
        accent_indicator: Option<[f32; 4]>,
        state: InteractionState,
    },

    /// Sidebar section header (PROJECT, GIT, OUTLINE...)
    SidebarSection { rect: Rect, label: String, fill_color: [f32; 4], text_color: [f32; 4] },

    /// Tab in the editor tab strip
    Tab {
        id: WidgetId,
        rect: Rect,
        label: String,
        fill_color: [f32; 4],
        text_color: [f32; 4],
        accent_strip: Option<[f32; 4]>,
        state: InteractionState,
    },

    /// Panel header with title (terminal, AI, sidebar, etc.)
    PanelHeader {
        id: WidgetId,
        rect: Rect,
        label: String,
        fill_color: [f32; 4],
        text_color: [f32; 4],
    },

    /// Status pill / segment in the status bar
    StatusSegment {
        id: WidgetId,
        rect: Rect,
        label: String,
        fill_color: [f32; 4],
        text_color: [f32; 4],
    },

    /// A region surface (editor bg, sidebar bg, panel bg, etc.)
    RegionSurface {
        rect: Rect,
        fill_color: [f32; 4],
        border_color: Option<[f32; 4]>,
        border_width: f32,
    },

    /// Thin divider line
    Divider { rect: Rect, color: [f32; 4], orientation: DividerOrientation },
}

// ---------------------------------------------------------------------------
// ShellWidgetTree — ordered widget tree describing the shell
// ---------------------------------------------------------------------------

/// Ordered tree of shell widgets in paint order (background first).
/// Supports hit-testing via `hit_test()` and state mutation via `set_state_of()`.
#[derive(Debug, Clone)]
pub struct ShellWidgetTree {
    pub widgets: Vec<ShellWidget>,
}

impl ShellWidgetTree {
    pub fn new() -> Self {
        Self { widgets: Vec::new() }
    }

    pub fn push(&mut self, w: ShellWidget) {
        self.widgets.push(w);
    }

    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    /// Find the topmost widget containing point (x, y). Returns its index or None.
    /// Searches in reverse paint order so topmost widgets are found first.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<usize> {
        self.widgets.iter().enumerate().rev().find_map(|(i, w)| {
            if w.rect().contains(Vec2::new(x, y)) && w.hit_target().is_some() {
                Some(i)
            } else {
                None
            }
        })
    }

    /// Update the interaction state of the widget at the given index.
    pub fn set_state_at(&mut self, idx: usize, state: InteractionState) {
        if let Some(w) = self.widgets.get_mut(idx) {
            w.set_state(state);
        }
    }

    /// Clear hover state from all widgets.
    pub fn clear_all_hover(&mut self) {
        for w in &mut self.widgets {
            if w.get_state() == InteractionState::Hover {
                w.set_state(InteractionState::Normal);
            }
        }
    }

    /// Collect all hit-targetable widgets for input routing.
    pub fn hit_targets(&self) -> Vec<WidgetHitTarget> {
        self.widgets.iter().filter_map(|w| w.hit_target()).collect()
    }

    /// Convert to the backward-compatible `ShellSurfaceSet` for rendering.
    pub fn to_surface_set(&self) -> ShellSurfaceSet {
        let mut set = ShellSurfaceSet::new();
        for w in &self.widgets {
            w.add_to_surface_set(&mut set);
        }
        set
    }

    /// Get mutable access to all widgets
    pub fn widgets_mut(&mut self) -> &mut Vec<ShellWidget> {
        &mut self.widgets
    }
}

impl Default for ShellWidgetTree {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ShellWidget implementation helpers
// ---------------------------------------------------------------------------

impl ShellWidget {
    pub fn rect(&self) -> Rect {
        match self {
            Self::AppBackground { rect, .. } => *rect,
            Self::Titlebar { rect, .. } => *rect,
            Self::RailItem { rect, .. } => *rect,
            Self::SidebarSection { rect, .. } => *rect,
            Self::Tab { rect, .. } => *rect,
            Self::PanelHeader { rect, .. } => *rect,
            Self::StatusSegment { rect, .. } => *rect,
            Self::RegionSurface { rect, .. } => *rect,
            Self::Divider { rect, .. } => *rect,
        }
    }

    pub fn set_state(&mut self, state: InteractionState) {
        match self {
            Self::RailItem { state: s, .. } => *s = state,
            Self::Tab { state: s, .. } => *s = state,
            _ => {}
        }
    }

    pub fn get_state(&self) -> InteractionState {
        match self {
            Self::RailItem { state, .. } => *state,
            Self::Tab { state, .. } => *state,
            _ => InteractionState::Normal,
        }
    }

    /// Returns a hit target if this widget is interactive.
    pub fn hit_target(&self) -> Option<WidgetHitTarget> {
        match self {
            Self::RailItem { id, rect, label, .. } => {
                Some(WidgetHitTarget { id: id.clone(), rect: *rect, label: label.clone() })
            }
            Self::Tab { id, rect, label, .. } => {
                Some(WidgetHitTarget { id: id.clone(), rect: *rect, label: label.clone() })
            }
            Self::StatusSegment { id, rect, label, .. } => {
                Some(WidgetHitTarget { id: id.clone(), rect: *rect, label: label.clone() })
            }
            Self::PanelHeader { id, rect, label, .. } => {
                Some(WidgetHitTarget { id: id.clone(), rect: *rect, label: label.clone() })
            }
            _ => None,
        }
    }

    /// Convert this widget into render primitives within a ShellSurfaceSet.
    fn add_to_surface_set(&self, set: &mut ShellSurfaceSet) {
        match self {
            Self::AppBackground { rect, fill_color } => {
                set.add_surface(Surface::new(*rect).with_fill(*fill_color));
            }
            Self::Titlebar { rect, fill_color, .. } => {
                set.add_surface(Surface::new(*rect).with_fill(*fill_color));
            }
            Self::RailItem { rect, fill_color, accent_indicator, .. } => {
                if let Some(accent) = accent_indicator {
                    set.add_surface(
                        Surface::new(Rect::new(rect.x + 2.0, rect.y + 2.0, 3.0, rect.height - 4.0))
                            .with_fill(*accent),
                    );
                }
                set.add_icon(
                    IconSlot::new(*rect)
                        .with_fill(*fill_color)
                        .with_accent(accent_indicator.unwrap_or([0.0; 4])), // accent_indicator already handled above
                );
                set.add_surface(Surface::new(*rect).with_fill(*fill_color));
            }
            Self::SidebarSection { rect, fill_color, text_color, label } => {
                set.add_header(
                    HeaderBar::new(*rect, label.as_str())
                        .with_fill(*fill_color)
                        .with_text_color(*text_color),
                );
            }
            Self::Tab { rect, fill_color, text_color, accent_strip, label, .. } => {
                let mut tab = TabChrome::new(*rect, label.as_str())
                    .with_fill(*fill_color)
                    .with_text_color(*text_color);
                if let Some(accent) = accent_strip {
                    tab = tab.active(*accent);
                }
                set.add_tab(tab);
            }
            Self::PanelHeader { rect, fill_color, text_color, label, .. } => {
                set.add_header(
                    HeaderBar::new(*rect, label.as_str())
                        .with_fill(*fill_color)
                        .with_text_color(*text_color),
                );
            }
            Self::StatusSegment { rect, fill_color, text_color, label, .. } => {
                set.add_pill(
                    StatusPill::new(*rect, label.as_str())
                        .with_fill(*fill_color)
                        .with_text_color(*text_color),
                );
            }
            Self::RegionSurface { rect, fill_color, border_color, border_width } => {
                let mut s = Surface::new(*rect).with_fill(*fill_color);
                if let Some(bc) = border_color {
                    s = s.with_border(*bc, *border_width);
                }
                set.add_surface(s);
            }
            Self::Divider { rect, color, orientation } => match orientation {
                DividerOrientation::Horizontal => {
                    set.add_divider(Divider::horizontal(rect.x, rect.y, rect.width, *color));
                }
                DividerOrientation::Vertical => {
                    set.add_divider(Divider::vertical(rect.x, rect.y, rect.height, *color));
                }
            },
        }
    }
}
