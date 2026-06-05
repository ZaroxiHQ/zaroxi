use zaroxi_core_engine_style::{InteractionState, StyleTokens, WidgetId};
use zaroxi_kernel_math::Rect;

use crate::widgets::{ShellWidget, ShellWidgetTree};

/// A row in a command list / command palette view.
#[derive(Debug, Clone)]
pub struct CommandListRow {
    pub label: String,
    pub description: Option<String>,
    pub shortcut: Option<String>,
}

/// A composite widget that produces a list of selectable command rows.
/// Rows are rendered as `ListItem` widgets with sequential WidgetIds.
/// Call `build_widgets()` to append them to a `ShellWidgetTree`.
#[derive(Debug, Clone)]
pub struct CommandListView {
    pub rect: Rect,
    pub rows: Vec<CommandListRow>,
    pub active_index: Option<usize>,
    pub focused_index: Option<usize>,
}

impl CommandListView {
    pub fn new(rect: Rect) -> Self {
        Self { rect, rows: Vec::new(), active_index: None, focused_index: None }
    }

    pub fn with_rows(mut self, rows: Vec<CommandListRow>) -> Self {
        self.rows = rows;
        self
    }

    /// Append widget primitives to `tree` for this command list.
    pub fn build_widgets(&self, tree: &mut ShellWidgetTree, tokens: &StyleTokens) {
        tree.push(ShellWidget::Surface {
            rect: self.rect,
            fill_color: tokens.panel_background.to_array(),
            border_color: None,
            border_width: 0.0,
        });

        let row_h = 24.0;
        let pad = 6.0;
        for (i, row) in self.rows.iter().enumerate() {
            let is_active = self.active_index == Some(i);
            let is_focused = self.focused_index == Some(i);
            let y = self.rect.y + pad + i as f32 * row_h;
            let fill = if is_active || is_focused {
                tokens.rail_item_active.to_array()
            } else {
                tokens.rail_item_inactive.to_array()
            };
            let state = if is_active {
                InteractionState::Selected
            } else if is_focused {
                InteractionState::Focused
            } else {
                InteractionState::Normal
            };

            tree.push(ShellWidget::ListItem {
                id: WidgetId::ListItem { index: i },
                rect: Rect::new(self.rect.x + pad, y, self.rect.width - pad * 2.0, row_h),
                label: row.label.clone(),
                fill_color: fill,
                accent_indicator: if is_active { Some(tokens.accent.to_array()) } else { None },
                state,
            });
        }
    }
}

/// A sectioned list view for explorer/file-tree style content.
#[derive(Debug, Clone)]
pub struct ExplorerListView {
    pub rect: Rect,
    pub sections: Vec<ExplorerListSection>,
}

#[derive(Debug, Clone)]
pub struct ExplorerListSection {
    pub header: String,
    pub items: Vec<String>,
}

impl ExplorerListView {
    pub fn new(rect: Rect) -> Self {
        Self { rect, sections: Vec::new() }
    }

    pub fn with_sections(mut self, sections: Vec<ExplorerListSection>) -> Self {
        self.sections = sections;
        self
    }

    pub fn build_widgets(&self, tree: &mut ShellWidgetTree, tokens: &StyleTokens) {
        let section_h = 20.0;
        let row_h = 16.0;
        let pad = 10.0;
        let mut y_off = self.rect.y + pad;

        for section in &self.sections {
            if y_off + section_h > self.rect.y + self.rect.height - 40.0 {
                break;
            }
            tree.push(ShellWidget::ListSectionHeader {
                rect: Rect::new(self.rect.x, y_off, self.rect.width, section_h),
                label: section.header.clone(),
                fill_color: tokens.panel_header_background.to_array(),
                text_color: tokens.panel_header_text.to_array(),
            });
            y_off += section_h + 2.0;

            for _item_label in &section.items {
                if y_off + row_h > self.rect.y + self.rect.height - 20.0 {
                    break;
                }
                tree.push(ShellWidget::Surface {
                    rect: Rect::new(
                        self.rect.x + pad + 14.0,
                        y_off + 2.0,
                        self.rect.width - pad * 2.0 - 20.0,
                        12.0,
                    ),
                    fill_color: tokens.sidebar_file_item.to_array(),
                    border_color: None,
                    border_width: 0.0,
                });
                y_off += row_h;
            }
            y_off += 6.0;
        }
    }
}
