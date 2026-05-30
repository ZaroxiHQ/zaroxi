/*!
GUI-2 widgets and chrome transcripts.

This file holds small, interface-facing widget definitions and a single
rendering helper used by the GUI-1 ShellFrame. The module is intentionally
lightweight and keeps UI widget semantics inside the `interface` crate.

Purpose (short):
- Provide deterministic, stable textual descriptions of chrome and navigation
  widgets for smoke tests and harness verification.
- Avoid introducing app/domain dependencies: widgets operate purely on the
  ShellRegion layout produced by ShellFrame.

Small step plan implemented here:
1. Expose a single function `render_chrome(regions: &[ShellRegion]) -> Vec<String>`
   that returns ordered widget/transcript lines.
2. Surface toolbar, app rail, sidebar, bottom dock tabs, status bar and AI header
   placeholders matching GUI-2 requirements.
3. Keep placeholders stable and free of runtime app state; wire to real state
   later when the application layer is required.

Note: The top docblock above serves as the short step-by-step explanation required
by the change process and is safe to keep in the interface crate.
*/

use crate::gui::shell::ShellRegion;

/// Render deterministic chrome/widget placeholder lines derived from the shell regions.
///
/// The output is intentionally compact and stable so tests and harnesses can
/// assert on strings rather than complex binary art.
pub fn render_chrome(regions: &[ShellRegion]) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    // Toolbar chrome
    if let Some(tb) = regions.iter().find(|r| r.id == "toolbar") {
        lines.push(format!("toolbar.brand: Zaroxi rect={}", tb.rect));
        lines.push(format!("toolbar.tabs_placeholder: rect={}", tb.rect));
        lines.push(format!("toolbar.actions: [search,settings,profile] rect={}", tb.rect));
    }

    // App rail: stacked icons, bottom avatar, active item
    if let Some(ar) = regions.iter().find(|r| r.id == "app_rail") {
        lines.push(format!(
            "app_rail.icons: [explorer,search,source_control,extensions] rect={}",
            ar.rect
        ));
        lines.push(format!("app_rail.avatar_slot: bottom rect={}", ar.rect));
        lines.push("app_rail.active: explorer".to_string());
    }

    // Sidebar: search/filter, PROJECT section, file rows, GIT, OUTLINE
    if let Some(sb) = regions.iter().find(|r| r.id == "sidebar") {
        lines.push(format!("sidebar.search_field: placeholder='Filter files' rect={}", sb.rect));
        lines.push("sidebar.section: PROJECT".to_string());
        lines.push("sidebar.row: src/main.rs".to_string());
        lines.push("sidebar.row: src/lib.rs (active)".to_string());
        lines.push("sidebar.section: GIT".to_string());
        lines.push("sidebar.git.status: clean".to_string());
        lines.push("sidebar.section: OUTLINE".to_string());
        lines.push("sidebar.outline.symbol: fn main".to_string());
        lines.push(format!("sidebar.rect: {}", sb.rect));
    }

    // Bottom dock chrome: tabs and problems badge placeholder
    if let Some(dk) = regions.iter().find(|r| r.id == "bottom_dock") {
        lines.push(format!(
            "bottom_dock.tabs: [Terminal,Problems,Output,Debug Console] rect={}",
            dk.rect
        ));
        lines.push("bottom_dock.problems_count: 0".to_string());
    }

    // Status bar placeholders
    if let Some(st) = regions.iter().find(|r| r.id == "status_bar") {
        lines.push("status.line_col: 1:1".to_string());
        lines.push("status.encoding: UTF-8".to_string());
        lines.push("status.line_ending: LF".to_string());
        lines.push("status.lang: Rust".to_string());
        lines.push("status.formatter: none".to_string());
        lines.push(format!("status.rect: {}", st.rect));
    }

    // AI panel header chrome
    if let Some(ai) = regions.iter().find(|r| r.id == "ai_panel_header") {
        lines.push(format!("ai.header.title: AI Assistant rect={}", ai.rect));
        lines.push("ai.header.actions: [pin,close]".to_string());
    }

    lines
}
