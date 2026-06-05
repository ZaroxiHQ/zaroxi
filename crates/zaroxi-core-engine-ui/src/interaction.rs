use std::collections::HashMap;

use zaroxi_core_engine_style::{InteractionState, WidgetId};

use crate::widgets::{ShellWidget, ShellWidgetTree};

// ---------------------------------------------------------------------------
// PointerButton — platform-neutral pointer button abstraction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Auxiliary,
}

// ---------------------------------------------------------------------------
// WidgetAction — engine-emitted intents for application reaction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum WidgetAction {
    /// A widget was activated (same-widget press+release on a Button/ListItem/TabItem).
    Activated(WidgetId),
    /// Hover moved to a new widget (or None if cursor left).
    HoverChanged(Option<WidgetId>),
    /// Focus moved to a widget (or None if cleared).
    FocusChanged(Option<WidgetId>),
    /// Widget paint state changed (hover/press/scroll); companion to HoverChanged.
    StateNeedsRedraw,
    /// Scroll offset changed for a scrollbar widget.
    ScrollOffsetChanged(WidgetId, f32),
    /// No meaningful state change.
    Nothing,
}

// ---------------------------------------------------------------------------
// ScrollDragState — internal scrollbar drag tracking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct ScrollDragState {
    widget_idx: usize,
    start_cursor_y: f32,
    start_offset: f32,
    track_height: f32,
    thumb_height: f32,
}

// ---------------------------------------------------------------------------
// WidgetInteractionModel — engine-owned interaction state for a widget tree
// ---------------------------------------------------------------------------

/// Mutable interaction state that survives between frames and drives
/// hit-testing, hover, press, scrollbar drags, and focus traversal from
/// the engine layer. The application feeds it platform events and reacts
/// to the emitted `WidgetAction`s.
#[derive(Debug, Clone)]
pub struct WidgetInteractionModel {
    pub hovered_widget_idx: Option<usize>,
    pub pressed_widget_idx: Option<usize>,
    pub focused_widget_idx: Option<usize>,
    pub cursor_pos: Option<(f32, f32)>,
    scrollbar_drag_state: Option<ScrollDragState>,
    scroll_offsets: HashMap<WidgetId, f32>,
}

impl WidgetInteractionModel {
    pub fn new() -> Self {
        Self {
            hovered_widget_idx: None,
            pressed_widget_idx: None,
            focused_widget_idx: None,
            cursor_pos: None,
            scrollbar_drag_state: None,
            scroll_offsets: HashMap::new(),
        }
    }

    // ── pointer input ──────────────────────────────────────────────────

    /// Process a pointer move event. If a scrollbar drag is active it
    /// computes a new normalized scroll offset; otherwise performs hover
    /// tracking against the widget tree.
    pub fn on_pointer_moved(
        &mut self,
        tree: &mut ShellWidgetTree,
        x: f32,
        y: f32,
    ) -> Vec<WidgetAction> {
        let mut actions = Vec::new();
        self.cursor_pos = Some((x, y));

        if let Some(drag) = self.scrollbar_drag_state {
            let travel = (drag.track_height - drag.thumb_height).max(1.0);
            let raw_offset = drag.start_offset + ((y - drag.start_cursor_y) / travel);
            let clamped = raw_offset.clamp(0.0, 1.0);

            if let Some(w) = tree.widgets.get(drag.widget_idx) {
                if let Some(id) = w.widget_id() {
                    let old_offset = self.scroll_offsets.get(&id).copied().unwrap_or(0.0);
                    if (clamped - old_offset).abs() > 0.001 {
                        self.scroll_offsets.insert(id.clone(), clamped);
                        actions.push(WidgetAction::ScrollOffsetChanged(id, clamped));
                        self.apply_scroll_offsets(tree);
                        actions.push(WidgetAction::StateNeedsRedraw);
                    }
                }
            }
            return actions;
        }

        let new_hover = tree.hit_test(x, y);
        if new_hover != self.hovered_widget_idx {
            self.clear_all_hover(tree);
            if let Some(idx) = new_hover {
                tree.set_state_at(idx, InteractionState::Hover);
            }
            let old_id = self
                .hovered_widget_idx
                .and_then(|i| tree.widgets.get(i).and_then(|w| w.widget_id()));
            let new_id = new_hover.and_then(|i| tree.widgets.get(i).and_then(|w| w.widget_id()));
            self.hovered_widget_idx = new_hover;

            if old_id != new_id {
                actions.push(WidgetAction::HoverChanged(new_id));
            }
            actions.push(WidgetAction::StateNeedsRedraw);
        }

        actions
    }

    /// Process a pointer leave (cursor exited the window).
    pub fn on_pointer_leave(&mut self, tree: &mut ShellWidgetTree) -> Vec<WidgetAction> {
        let mut actions = Vec::new();

        if self.scrollbar_drag_state.is_some() {
            self.scrollbar_drag_state = None;
            self.clear_all_hover(tree);
            actions.push(WidgetAction::StateNeedsRedraw);
        }

        self.hovered_widget_idx = None;
        self.cursor_pos = None;
        self.clear_all_hover(tree);

        actions.push(WidgetAction::HoverChanged(None));
        actions.push(WidgetAction::StateNeedsRedraw);
        actions
    }

    /// Process a pointer press. Starts a scrollbar drag if a ScrollBar was
    /// hit; otherwise records the pressed widget.
    pub fn on_pointer_down(
        &mut self,
        tree: &mut ShellWidgetTree,
        x: f32,
        y: f32,
        button: PointerButton,
    ) -> Vec<WidgetAction> {
        if button != PointerButton::Primary {
            return Vec::new();
        }

        let mut actions = Vec::new();
        let hit = tree.hit_test(x, y);
        self.pressed_widget_idx = hit;

        if let Some(idx) = hit {
            let is_scrollbar =
                tree.widgets.get(idx).map_or(false, |w| matches!(w, ShellWidget::ScrollBar { .. }));

            if is_scrollbar {
                tree.set_state_at(idx, InteractionState::Active);
                if let Some(w) = tree.widgets.get(idx) {
                    if let ShellWidget::ScrollBar { track_rect, thumb_rect, id, .. } = w {
                        let offset = self.scroll_offsets.get(id).copied().unwrap_or(0.0);
                        self.scrollbar_drag_state = Some(ScrollDragState {
                            widget_idx: idx,
                            start_cursor_y: y,
                            start_offset: offset,
                            track_height: track_rect.height,
                            thumb_height: thumb_rect.height,
                        });
                    }
                }
            } else {
                tree.set_state_at(idx, InteractionState::Active);
            }
            actions.push(WidgetAction::StateNeedsRedraw);
        }

        actions
    }

    /// Process a pointer release. Ends any active scrollbar drag and
    /// detects widget activation (same-widget press+release).
    pub fn on_pointer_up(
        &mut self,
        tree: &mut ShellWidgetTree,
        x: f32,
        y: f32,
        button: PointerButton,
    ) -> Vec<WidgetAction> {
        if button != PointerButton::Primary {
            return Vec::new();
        }

        let mut actions = Vec::new();

        let pressed = self.pressed_widget_idx.take();
        let hit = tree.hit_test(x, y);

        if let Some(idx) = pressed {
            tree.set_state_at(idx, InteractionState::Normal);
        }

        if self.scrollbar_drag_state.take().is_some() {
            self.clear_all_hover(tree);
            actions.push(WidgetAction::StateNeedsRedraw);
        }

        if pressed.is_some() && pressed == hit {
            if let Some(idx) = pressed {
                if let Some(w) = tree.widgets.get(idx) {
                    if let Some(id) = w.widget_id() {
                        actions.push(WidgetAction::Activated(id));
                    }
                }
            }
        }

        self.clear_all_hover(tree);
        actions.push(WidgetAction::StateNeedsRedraw);
        actions
    }

    // ── focus traversal ────────────────────────────────────────────────

    /// Move focus to the next focusable widget in tree order.
    /// Wraps around to the beginning.
    pub fn focus_next(&mut self, tree: &mut ShellWidgetTree) -> Vec<WidgetAction> {
        let focusables = self.focusable_indices(tree);
        if focusables.is_empty() {
            return vec![WidgetAction::FocusChanged(None)];
        }

        let current = self.focused_widget_idx;
        let next_idx = match current {
            Some(c) => {
                let pos = focusables.iter().position(|&i| i == c);
                match pos {
                    Some(p) if p + 1 < focusables.len() => focusables[p + 1],
                    _ => focusables[0],
                }
            }
            None => focusables[0],
        };

        self.set_focus(tree, Some(next_idx));
        let new_id = tree.widgets.get(next_idx).and_then(|w| w.widget_id());
        vec![WidgetAction::FocusChanged(new_id), WidgetAction::StateNeedsRedraw]
    }

    /// Move focus to the previous focusable widget in tree order.
    /// Wraps around to the end.
    pub fn focus_previous(&mut self, tree: &mut ShellWidgetTree) -> Vec<WidgetAction> {
        let focusables = self.focusable_indices(tree);
        if focusables.is_empty() {
            return vec![WidgetAction::FocusChanged(None)];
        }

        let current = self.focused_widget_idx;
        let prev_idx = match current {
            Some(c) => {
                let pos = focusables.iter().position(|&i| i == c);
                match pos {
                    Some(p) if p > 0 => focusables[p - 1],
                    _ => focusables[focusables.len() - 1],
                }
            }
            None => focusables[0],
        };

        self.set_focus(tree, Some(prev_idx));
        let new_id = tree.widgets.get(prev_idx).and_then(|w| w.widget_id());
        vec![WidgetAction::FocusChanged(new_id), WidgetAction::StateNeedsRedraw]
    }

    /// Activate the currently focused widget (if any).
    pub fn activate_focused(&mut self, tree: &mut ShellWidgetTree) -> Vec<WidgetAction> {
        let idx = match self.focused_widget_idx {
            Some(i) => i,
            None => return Vec::new(),
        };

        if let Some(w) = tree.widgets.get(idx) {
            if let Some(id) = w.widget_id() {
                tree.set_state_at(idx, InteractionState::Active);
                return vec![WidgetAction::Activated(id), WidgetAction::StateNeedsRedraw];
            }
        }

        Vec::new()
    }

    // ── scroll offset management ───────────────────────────────────────

    /// Get the stored scroll offset for a scrollbar id (defaults to 0.0).
    pub fn get_scroll_offset(&self, id: &WidgetId) -> f32 {
        self.scroll_offsets.get(id).copied().unwrap_or(0.0)
    }

    /// Set the scroll offset for a scrollbar id.
    pub fn set_scroll_offset(&mut self, id: &WidgetId, offset: f32) {
        self.scroll_offsets.insert(id.clone(), offset.clamp(0.0, 1.0));
    }

    /// Apply stored interaction state (hover, pressed, focused) to a
    /// freshly-built widget tree before rendering.
    pub fn apply_to_tree(&self, tree: &mut ShellWidgetTree) {
        if let Some(idx) = self.hovered_widget_idx {
            tree.set_state_at(idx, InteractionState::Hover);
        }
        if let Some(idx) = self.pressed_widget_idx {
            tree.set_state_at(idx, InteractionState::Active);
        }
        if let Some(idx) = self.focused_widget_idx {
            tree.set_state_at(idx, InteractionState::Focused);
        }
    }

    /// Update all scrollbar thumb positions in the tree from stored offsets.
    pub fn apply_scroll_offsets(&self, tree: &mut ShellWidgetTree) {
        for i in 0..tree.widgets.len() {
            let maybe_updated = match &tree.widgets[i] {
                ShellWidget::ScrollBar {
                    id,
                    track_rect,
                    thumb_rect,
                    track_fill,
                    thumb_fill,
                    state,
                } => {
                    if let Some(&offset) = self.scroll_offsets.get(id) {
                        let travel = (track_rect.height - thumb_rect.height).max(1.0);
                        let new_y = track_rect.y + offset * travel;
                        let mut new_thumb = *thumb_rect;
                        new_thumb.y = new_y;
                        Some(ShellWidget::ScrollBar {
                            id: id.clone(),
                            track_rect: *track_rect,
                            thumb_rect: new_thumb,
                            track_fill: *track_fill,
                            thumb_fill: *thumb_fill,
                            state: *state,
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(w) = maybe_updated {
                tree.widgets[i] = w;
            }
        }
    }

    // ── private helpers ────────────────────────────────────────────────

    fn clear_all_hover(&self, tree: &mut ShellWidgetTree) {
        for w in &mut tree.widgets {
            if w.get_state() == InteractionState::Hover {
                w.set_state(InteractionState::Normal);
            }
            if w.get_state() == InteractionState::Active
                && self.pressed_widget_idx.is_none()
                && self.scrollbar_drag_state.is_none()
            {
                w.set_state(InteractionState::Normal);
            }
        }
    }

    fn focusable_indices(&self, tree: &ShellWidgetTree) -> Vec<usize> {
        tree.widgets.iter().enumerate().filter(|(_, w)| w.is_focusable()).map(|(i, _)| i).collect()
    }

    fn set_focus(&mut self, tree: &mut ShellWidgetTree, new_idx: Option<usize>) {
        if let Some(old) = self.focused_widget_idx {
            tree.set_state_at(old, InteractionState::Normal);
        }
        if let Some(new) = new_idx {
            tree.set_state_at(new, InteractionState::Focused);
        }
        self.focused_widget_idx = new_idx;
    }

    /// Return the cursor position as logical (x, y) floats.
    pub fn cursor_pos_f32(&self) -> Option<(f32, f32)> {
        self.cursor_pos
    }
}

impl Default for WidgetInteractionModel {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ShellWidget helpers added for the interaction model
// ---------------------------------------------------------------------------

impl ShellWidget {
    /// Whether this widget can receive keyboard focus.
    pub fn is_focusable(&self) -> bool {
        matches!(
            self,
            Self::TabItem { .. }
                | Self::ListItem { .. }
                | Self::Button { .. }
                | Self::ScrollBar { .. }
        )
    }

    /// Return the `WidgetId` if this widget carries one.
    pub fn widget_id(&self) -> Option<WidgetId> {
        match self {
            Self::ListItem { id, .. } => Some(id.clone()),
            Self::TabItem { id, .. } => Some(id.clone()),
            Self::PanelHeader { id, .. } => Some(id.clone()),
            Self::StatusSegment { id, .. } => Some(id.clone()),
            Self::ScrollBar { id, .. } => Some(id.clone()),
            Self::Button { id, .. } => Some(id.clone()),
            _ => None,
        }
    }
}
