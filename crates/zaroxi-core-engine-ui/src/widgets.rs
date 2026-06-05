use zaroxi_core_engine_style::{InteractionState, WidgetId};
use zaroxi_kernel_math::{Rect, Vec2};

use crate::primitives::{
    Divider, DividerOrientation, HeaderBar, ShellSurfaceSet, StatusPill, Surface, TabChrome,
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
// ShellWidget — enum over all app-neutral widget variants in the shell tree
// ---------------------------------------------------------------------------

/// A single shell widget. Each variant carries its geometry, label, and state.
/// Widgets are app-neutral: IDE concepts are mapped onto these primitives by
/// the shell builder; the engine owns no business terminology.
#[derive(Debug, Clone)]
pub enum ShellWidget {
    /// Full-window background surface
    AppBackground { rect: Rect, fill_color: [f32; 4] },

    /// Titlebar strip (brand label + window-control buttons)
    Titlebar { rect: Rect, fill_color: [f32; 4], brand_label: String },

    /// An item in a list or rail (selectable, with optional accent indicator)
    ListItem {
        id: WidgetId,
        rect: Rect,
        label: String,
        fill_color: [f32; 4],
        accent_indicator: Option<[f32; 4]>,
        state: InteractionState,
    },

    /// Section header within a list (e.g. "PROJECT", "GIT", "OUTLINE")
    ListSectionHeader { rect: Rect, label: String, fill_color: [f32; 4], text_color: [f32; 4] },

    /// A tab item inside a tab strip
    TabItem {
        id: WidgetId,
        rect: Rect,
        label: String,
        fill_color: [f32; 4],
        text_color: [f32; 4],
        accent_strip: Option<[f32; 4]>,
        state: InteractionState,
    },

    /// Panel header with title label and optional action button slots
    PanelHeader {
        id: WidgetId,
        rect: Rect,
        label: String,
        fill_color: [f32; 4],
        text_color: [f32; 4],
        actions: Vec<PanelHeaderAction>,
    },

    /// Status pill / segment (status bar label, language badge, etc.)
    StatusSegment {
        id: WidgetId,
        rect: Rect,
        label: String,
        fill_color: [f32; 4],
        text_color: [f32; 4],
    },

    /// Scrollbar with track and proportional thumb
    ScrollBar {
        id: WidgetId,
        track_rect: Rect,
        thumb_rect: Rect,
        track_fill: [f32; 4],
        thumb_fill: [f32; 4],
        state: InteractionState,
    },

    /// A generic clickable / interactive button
    Button {
        id: WidgetId,
        rect: Rect,
        label: String,
        fill_color: [f32; 4],
        state: InteractionState,
    },

    /// A filled rectangular surface (panel background, fill region, etc.)
    Surface { rect: Rect, fill_color: [f32; 4], border_color: Option<[f32; 4]>, border_width: f32 },

    /// Thin separator line (horizontal or vertical)
    Divider { rect: Rect, color: [f32; 4], orientation: DividerOrientation, subtle: bool },

    /// Empty content placeholder ("No files open", "No results", etc.)
    EmptyState { rect: Rect, message: String, fill_color: [f32; 4], text_color: [f32; 4] },

    /// Standalone text label (unattached to larger widget chrome)
    TextLabel { rect: Rect, label: String, text_color: [f32; 4] },
}

// ---------------------------------------------------------------------------
// PanelHeaderAction — action button slot in a panel header
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PanelHeaderAction {
    pub id: WidgetId,
    pub rect: Rect,
    pub label: String,
    pub fill_color: [f32; 4],
    pub hover_fill: [f32; 4],
    pub state: InteractionState,
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
            Self::ListItem { rect, .. } => *rect,
            Self::ListSectionHeader { rect, .. } => *rect,
            Self::TabItem { rect, .. } => *rect,
            Self::PanelHeader { rect, .. } => *rect,
            Self::StatusSegment { rect, .. } => *rect,
            Self::ScrollBar { track_rect, .. } => *track_rect,
            Self::Button { rect, .. } => *rect,
            Self::Surface { rect, .. } => *rect,
            Self::Divider { rect, .. } => *rect,
            Self::EmptyState { rect, .. } => *rect,
            Self::TextLabel { rect, .. } => *rect,
        }
    }

    pub fn set_state(&mut self, state: InteractionState) {
        match self {
            Self::ListItem { state: s, .. } => *s = state,
            Self::TabItem { state: s, .. } => *s = state,
            Self::ScrollBar { state: s, .. } => *s = state,
            Self::Button { state: s, .. } => *s = state,
            _ => {}
        }
    }

    pub fn get_state(&self) -> InteractionState {
        match self {
            Self::ListItem { state, .. } => *state,
            Self::TabItem { state, .. } => *state,
            Self::ScrollBar { state, .. } => *state,
            Self::Button { state, .. } => *state,
            _ => InteractionState::Normal,
        }
    }

    /// Returns a hit target if this widget is interactive.
    pub fn hit_target(&self) -> Option<WidgetHitTarget> {
        match self {
            Self::ListItem { id, rect, label, .. } => {
                Some(WidgetHitTarget { id: id.clone(), rect: *rect, label: label.clone() })
            }
            Self::TabItem { id, rect, label, .. } => {
                Some(WidgetHitTarget { id: id.clone(), rect: *rect, label: label.clone() })
            }
            Self::StatusSegment { id, rect, label, .. } => {
                Some(WidgetHitTarget { id: id.clone(), rect: *rect, label: label.clone() })
            }
            Self::PanelHeader { id, rect, label, .. } => {
                Some(WidgetHitTarget { id: id.clone(), rect: *rect, label: label.clone() })
            }
            Self::ScrollBar { id, thumb_rect, .. } => Some(WidgetHitTarget {
                id: id.clone(),
                rect: *thumb_rect,
                label: "scrollbar".into(),
            }),
            Self::Button { id, rect, label, .. } => {
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
            Self::ListItem { rect, fill_color, accent_indicator, .. } => {
                if let Some(accent) = accent_indicator {
                    let accent_rect = Rect::new(rect.x + 2.0, rect.y + 2.0, 3.0, rect.height - 4.0);
                    set.add_surface(Surface::new(accent_rect).with_fill(*accent));
                }
                set.add_surface(Surface::new(*rect).with_fill(*fill_color));
            }
            Self::ListSectionHeader { rect, fill_color, text_color, label } => {
                set.add_header(
                    HeaderBar::new(*rect, label.as_str())
                        .with_fill(*fill_color)
                        .with_text_color(*text_color),
                );
            }
            Self::TabItem { rect, fill_color, text_color, accent_strip, label, .. } => {
                let mut tab = TabChrome::new(*rect, label.as_str())
                    .with_fill(*fill_color)
                    .with_text_color(*text_color);
                if let Some(accent) = accent_strip {
                    tab = tab.active(*accent);
                }
                set.add_tab(tab);
            }
            Self::PanelHeader { rect, fill_color, text_color, label, actions, .. } => {
                set.add_header(
                    HeaderBar::new(*rect, label.as_str())
                        .with_fill(*fill_color)
                        .with_text_color(*text_color),
                );
                for action in actions {
                    set.add_surface(Surface::new(action.rect).with_fill(action.fill_color));
                }
            }
            Self::StatusSegment { rect, fill_color, text_color, label, .. } => {
                set.add_pill(
                    StatusPill::new(*rect, label.as_str())
                        .with_fill(*fill_color)
                        .with_text_color(*text_color),
                );
            }
            Self::ScrollBar { track_rect, thumb_rect, track_fill, thumb_fill, .. } => {
                set.add_surface(Surface::new(*track_rect).with_fill(*track_fill).with_radius(3.0));
                set.add_surface(Surface::new(*thumb_rect).with_fill(*thumb_fill).with_radius(2.0));
            }
            Self::Button { rect, fill_color, .. } => {
                set.add_surface(Surface::new(*rect).with_fill(*fill_color).with_radius(4.0));
            }
            Self::Surface { rect, fill_color, border_color, border_width } => {
                let mut s = Surface::new(*rect).with_fill(*fill_color);
                if let Some(bc) = border_color {
                    s = s.with_border(*bc, *border_width);
                }
                set.add_surface(s);
            }
            Self::Divider { rect, color, orientation, subtle } => {
                let adjusted_color = if *subtle {
                    let mut c = *color;
                    c[3] *= 0.5;
                    c
                } else {
                    *color
                };
                match orientation {
                    DividerOrientation::Horizontal => {
                        set.add_divider(Divider::horizontal(
                            rect.x,
                            rect.y,
                            rect.width,
                            adjusted_color,
                        ));
                    }
                    DividerOrientation::Vertical => {
                        set.add_divider(Divider::vertical(
                            rect.x,
                            rect.y,
                            rect.height,
                            adjusted_color,
                        ));
                    }
                }
            }
            Self::EmptyState { rect, fill_color, .. } => {
                set.add_surface(Surface::new(*rect).with_fill(*fill_color));
            }
            Self::TextLabel { .. } => {}
        }
    }
}
