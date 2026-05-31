/*!
GUI-2 widgets and chrome transcripts (Phase 2).

Deterministic textual descriptions of chrome and navigation widgets
matching the refined IDE shell layout.
*/

use crate::gui::shell::ShellRegion;

pub fn render_chrome(regions: &[ShellRegion]) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    if let Some(tb) = regions.iter().find(|r| r.id == "toolbar") {
        lines.push(format!("toolbar.brand: Zaroxi rect={}", tb.rect));
        lines.push(format!("toolbar.actions: [minimize,maximize,close] rect={}", tb.rect));
    }

    if let Some(ar) = regions.iter().find(|r| r.id == "app_rail") {
        lines.push(format!("app_rail.icons: [explorer,search,git,extensions] rect={}", ar.rect));
        lines.push(format!("app_rail.bottom_icons: [settings,account] rect={}", ar.rect));
        lines.push("app_rail.active: explorer".to_string());
    }

    if let Some(sb) = regions.iter().find(|r| r.id == "sidebar") {
        lines.push(format!("sidebar.header: Zaroxi Studio rect={}", sb.rect));
        lines.push(format!("sidebar.search_field: placeholder='Filter files...' rect={}", sb.rect));
        lines.push("sidebar.section: PROJECT".to_string());
        lines.push("sidebar.tree: src/".to_string());
        lines.push("sidebar.tree:   main.rs".to_string());
        lines.push("sidebar.tree:   lib.rs".to_string());
        lines.push("sidebar.tree: Cargo.toml".to_string());
        lines.push("sidebar.tree:   mod.rs".to_string());
        lines.push("sidebar.tree: tests/".to_string());
        lines.push("sidebar.tree:   integration.rs".to_string());
        lines.push("sidebar.section: GIT".to_string());
        lines.push("sidebar.git.status: clean".to_string());
        lines.push("sidebar.section: OUTLINE".to_string());
        lines.push("sidebar.outline.symbol: fn main".to_string());
        lines.push("sidebar.outline.symbol: struct App".to_string());
        lines.push("sidebar.outline.symbol: impl App".to_string());
        lines.push("sidebar.outline.symbol: fn run".to_string());
        lines.push("sidebar.tools_dock: [terminal,build,debug,docker]".to_string());
        lines.push(format!("sidebar.rect: {}", sb.rect));
    }

    if let Some(et) = regions.iter().find(|r| r.id == "editor_tabs") {
        lines.push(format!("editor.tabs: [main.rs,lib.rs,mod.rs,config.rs] rect={}", et.rect));
    }

    if let Some(bc) = regions.iter().find(|r| r.id == "breadcrumb") {
        lines.push(format!("editor.breadcrumb: src > app > desktop > main.rs rect={}", bc.rect));
    }

    if let Some(ce) = regions.iter().find(|r| r.id == "center_editor") {
        lines.push(format!("editor.content: syntax-highlighted code rect={}", ce.rect));
    }

    if let Some(ml) = regions.iter().find(|r| r.id == "minimap_lane") {
        lines.push(format!("editor.minimap: code-outline rect={}", ml.rect));
    }

    if let Some(cb) = regions.iter().find(|r| r.id == "center_bottom_panel") {
        lines.push(format!("editor.terminal: [Terminal,Problems,Output] rect={}", cb.rect));
    }

    if let Some(dk) = regions.iter().find(|r| r.id == "bottom_dock") {
        lines.push(format!("bottom_dock.tabs: [Terminal,Problems,Output,Debug] rect={}", dk.rect));
        lines.push("bottom_dock.problems_count: 0".to_string());
    }

    if let Some(st) = regions.iter().find(|r| r.id == "status_bar") {
        lines.push("status.line_col: 22:14".to_string());
        lines.push("status.encoding: UTF-8".to_string());
        lines.push("status.line_ending: LF".to_string());
        lines.push("status.lang: Rust".to_string());
        lines.push("status.formatter: rust-analyzer".to_string());
        lines.push(format!("status.rect: {}", st.rect));
    }

    if let Some(ai) = regions.iter().find(|r| r.id == "ai_panel_header") {
        lines.push(format!("ai.header.title: Assistant rect={}", ai.rect));
        lines.push("ai.header.actions: [pin,close]".to_string());
    }

    if let Some(aic) = regions.iter().find(|r| r.id == "ai_panel_content") {
        lines.push(format!(
            "ai.content.cards: [explanation,bullet-list,code-snippet] rect={}",
            aic.rect
        ));
        lines.push("ai.content.actions: [Accept,Reject,Edit]".to_string());
        lines.push(
            "ai.content.input: placeholder='Rewrite this...' ".to_string()
                + "model='Claude 3.5 Sonnet'",
        );
    }

    lines
}
