/*!
GUI-2 widgets and chrome transcripts (Phase 2).

Deterministic textual descriptions of chrome and navigation widgets
matching the refined IDE shell layout.
*/

use crate::desktop::DesktopComposition;
use crate::gui::shell::ShellRegion;
use zaroxi_application_ai::view_model::AiPanelState;
use zaroxi_application_navigation::view_model::AppRailState;

pub fn render_chrome(regions: &[ShellRegion], comp: Option<&DesktopComposition>) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    if let Some(tb) = regions.iter().find(|r| r.id == "toolbar") {
        lines.push(format!("toolbar.brand: Zaroxi rect={}", tb.rect));
        lines.push(format!("toolbar.actions: [minimize,maximize,close] rect={}", tb.rect));
    }

    if let Some(ar) = regions.iter().find(|r| r.id == "app_rail") {
        // Prefer the application-owned AppRailState for the icon list and active item.
        let rail = AppRailState::default();
        let icons = if rail.icons.is_empty() {
            vec![
                "explorer".to_string(),
                "search".to_string(),
                "git".to_string(),
                "extensions".to_string(),
            ]
        } else {
            rail.icons.clone()
        };
        let icons_joined = icons.join(",");
        lines.push(format!("app_rail.icons: [{}] rect={}", icons_joined, ar.rect));
        let bottom = vec!["settings".to_string(), "account".to_string()].join(",");
        lines.push(format!("app_rail.bottom_icons: [{}] rect={}", bottom, ar.rect));
        let active = rail.active.unwrap_or_else(|| "explorer".to_string());
        lines.push(format!("app_rail.active: {}", active));
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
        // Prefer opened-buffer list from DesktopComposition when available.
        if let Some(d) = comp {
            let opened = d.latest_opened_buffers_summary();
            if !opened.items.is_empty() {
                let names: Vec<String> = opened
                    .items
                    .iter()
                    .map(|it| it.display.clone().unwrap_or_else(|| "untitled".to_string()))
                    .collect();
                lines.push(format!("editor.tabs: [{}] rect={}", names.join(","), et.rect));
            } else {
                lines.push(format!(
                    "editor.tabs: [main.rs,lib.rs,mod.rs,config.rs] rect={}",
                    et.rect
                ));
            }
        } else {
            // No composition available: fall back to an empty tabs placeholder.
            lines.push(format!("editor.tabs: [main.rs,lib.rs,mod.rs,config.rs] rect={}", et.rect));
        }
    }

    if let Some(bc) = regions.iter().find(|r| r.id == "breadcrumb") {
        // Prefer the shell context active_display when available
        let breadcrumb = if let Some(d) = comp {
            if let Some(ctx) = crate::desktop::composition::projections::latest_shell_context(d) {
                ctx.active_display.unwrap_or_else(|| "src > app > desktop > main.rs".to_string())
            } else {
                "src > app > desktop > main.rs".to_string()
            }
        } else {
            "src > app > desktop > main.rs".to_string()
        };
        lines.push(format!("editor.breadcrumb: {} rect={}", breadcrumb, bc.rect));
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
        let ai = AiPanelState::default();
        if ai.cards.is_empty() {
            lines.push(format!(
                "ai.content.cards: [explanation,bullet-list,code-snippet] rect={}",
                aic.rect
            ));
        } else {
            let titles: Vec<String> = ai.cards.iter().map(|c| c.title.clone()).collect();
            lines.push(format!("ai.content.cards: [{}] rect={}", titles.join(","), aic.rect));
        }
        lines.push("ai.content.actions: [Accept,Reject,Edit]".to_string());
        lines.push(format!(
            "ai.content.input: placeholder='{}' model='{}'",
            if ai.composer_text.is_empty() { "Rewrite this..." } else { &ai.composer_text },
            ai.header
        ));
    }

    lines
}
