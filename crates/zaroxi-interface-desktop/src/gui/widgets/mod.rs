/*!
GUI-2 widgets and chrome transcripts (Phase 3).

Deterministic textual descriptions of chrome and navigation widgets
matching the refined IDE shell layout. When DesktopComposition is
available, real data from the session/workspace is wired into all
panels; otherwise placeholder content is used.
*/

use crate::desktop::DesktopComposition;
use crate::gui::shell::ShellRegion;
use zaroxi_application_ai::panel::idle_content_view;
use zaroxi_application_navigation::view_model::AppRailState;
use zaroxi_core_engine_ui::ContentView;
use zaroxi_core_platform_terminal::view_model::TerminalPanelState;

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
        lines.push(format!("app_rail.avatar_slot: user@zaroxi rect={}", ar.rect));
    }

    if let Some(sb) = regions.iter().find(|r| r.id == "sidebar") {
        lines.push(format!("sidebar.header: Zaroxi Studio rect={}", sb.rect));
        lines.push(format!("sidebar.search_field: placeholder='Filter files...' rect={}", sb.rect));
        lines.push("sidebar.section: PROJECT".to_string());

        // Prefer real opened buffers as explorer tree entries.
        if let Some(d) = comp {
            let opened = d.latest_opened_buffers_summary();
            let active_id = opened.active.clone();
            if !opened.items.is_empty() {
                for item in &opened.items {
                    let marker = if Some(&item.buffer_id) == active_id.as_ref() {
                        " *" // active file marker
                    } else {
                        ""
                    };
                    let display = item.display.as_deref().unwrap_or("untitled");
                    lines.push(format!("sidebar.tree: {}{}", display, marker));
                }
                lines.push(format!("sidebar.tree.count: {} opened", opened.items.len()));
            } else {
                // No live buffers — show placeholder tree.
                lines.push("sidebar.tree: src/".to_string());
                lines.push("sidebar.tree:   main.rs".to_string());
                lines.push("sidebar.tree:   lib.rs".to_string());
                lines.push("sidebar.tree: Cargo.toml".to_string());
            }
        } else {
            lines.push("sidebar.tree: src/".to_string());
            lines.push("sidebar.tree:   main.rs".to_string());
            lines.push("sidebar.tree:   lib.rs".to_string());
            lines.push("sidebar.tree: Cargo.toml".to_string());
        }
        lines.push("sidebar.section: GIT".to_string());
        lines.push("sidebar.git.status: clean".to_string());
        lines.push("sidebar.section: OUTLINE".to_string());
        lines.push("sidebar.outline.symbol: fn main".to_string());
        lines.push("sidebar.outline.symbol: struct App".to_string());
        lines.push("sidebar.tools_dock: [terminal,build,debug,docker]".to_string());
        lines.push(format!("sidebar.rect: {}", sb.rect));
    }

    if let Some(et) = regions.iter().find(|r| r.id == "editor_tabs") {
        // Prefer opened-buffer list from DesktopComposition when available.
        if let Some(d) = comp {
            let opened = d.latest_opened_buffers_summary();
            let active_id = opened.active.clone();
            if !opened.items.is_empty() {
                let names: Vec<String> = opened
                    .items
                    .iter()
                    .map(|it| {
                        let disp = it.display.clone().unwrap_or_else(|| "untitled".to_string());
                        if Some(&it.buffer_id) == active_id.as_ref() {
                            format!("{}*", disp)
                        } else {
                            disp
                        }
                    })
                    .collect();
                lines.push(format!("editor.tabs: [{}] rect={}", names.join(","), et.rect));
            } else {
                lines.push(format!("editor.tabs: [main.rs,lib.rs,mod.rs] rect={}", et.rect));
            }
        } else {
            lines.push(format!("editor.tabs: [main.rs,lib.rs,mod.rs] rect={}", et.rect));
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
        // Prefer active document summary + visible window from composition.
        let content = if let Some(d) = comp {
            if let Some(doc) = d.latest_active_document_summary() {
                let title = doc.display.unwrap_or_else(|| "untitled".to_string());
                let subtitle = doc.buffer_id.map(|b| b.to_string()).unwrap_or_default();
                let lines: Vec<String> = if let Some(md) = d.latest_metadata() {
                    md.visible_window.map(|vw| vw.lines.clone()).unwrap_or_else(|| {
                        doc.current_line_snippet.into_iter().map(|s| s.to_string()).collect()
                    })
                } else {
                    doc.current_line_snippet.into_iter().map(|s| s.to_string()).collect()
                };
                let mut cv = ContentView::new(&title, &subtitle, lines.clone());
                if cv.lines.is_empty() {
                    cv = ContentView::default();
                }
                cv
            } else {
                ContentView::default()
            }
        } else {
            ContentView::default()
        };
        lines.push(format!("editor.title: {} rect={}", content.title, ce.rect));
        if !content.subtitle.is_empty() {
            lines.push(format!("editor.subtitle: {} rect={}", content.subtitle, ce.rect));
        }
        lines.push(format!("editor.lines: count={} rect={}", content.lines.len(), ce.rect));
        if !content.lines.is_empty() {
            for (i, line) in content.lines.iter().take(4).enumerate() {
                lines.push(format!("editor.line[{}]: {}", i + 1, line));
            }
        }
    }

    if let Some(ml) = regions.iter().find(|r| r.id == "minimap_lane") {
        lines.push(format!("editor.minimap: code-outline rect={}", ml.rect));
    }

    if let Some(cb) = regions.iter().find(|r| r.id == "center_bottom_panel") {
        let terminal = TerminalPanelState::default();
        let tabs = if terminal.tabs.is_empty() {
            vec!["Terminal".to_string(), "Problems".to_string(), "Output".to_string()]
        } else {
            terminal.tabs.clone()
        };
        lines.push(format!("editor.terminal: [{}] rect={}", tabs.join(","), cb.rect));
        if !terminal.lines.is_empty() {
            for tl in terminal.lines.iter().take(3) {
                lines.push(format!("editor.terminal.line: {}", tl.text));
            }
        }
    }

    if let Some(dk) = regions.iter().find(|r| r.id == "bottom_dock") {
        lines.push(format!("bottom_dock.tabs: [Terminal,Problems,Output,Debug] rect={}", dk.rect));
        lines.push("bottom_dock.problems_count: 0".to_string());
    }

    if let Some(st) = regions.iter().find(|r| r.id == "status_bar") {
        let line_col = if let Some(d) = comp {
            d.latest_active_document_summary()
                .map(|doc| {
                    format!(
                        "{}:{}",
                        doc.cursor_line.map(|l| l + 1).unwrap_or(1),
                        doc.cursor_column.map(|c| c + 1).unwrap_or(1)
                    )
                })
                .unwrap_or_else(|| "1:1".to_string())
        } else {
            "1:1".to_string()
        };
        lines.push(format!("status.line_col: {}", line_col));
        lines.push("status.encoding: UTF-8".to_string());
        lines.push("status.line_ending: LF".to_string());
        lines.push("status.lang: Rust".to_string());
        lines.push("status.formatter: rust-analyzer".to_string());
        lines.push(format!("status.rect: {}", st.rect));
    }

    if let Some(ai) = regions.iter().find(|r| r.id == "ai_panel_header") {
        let content = comp
            .and_then(|c| c.build_work_content().ai_panel_content)
            .unwrap_or_else(idle_content_view);
        lines.push(format!("ai.header.title: {} rect={}", content.title, ai.rect));
        lines.push("ai.header.actions: [pin,close]".to_string());
    }

    if let Some(aic) = regions.iter().find(|r| r.id == "ai_panel_content") {
        let content = comp
            .and_then(|c| c.build_work_content().ai_panel_content)
            .unwrap_or_else(idle_content_view);
        lines.push(format!("ai.content.title: {} rect={}", content.title, aic.rect));
        if !content.subtitle.is_empty() {
            lines.push(format!("ai.content.subtitle: {} rect={}", content.subtitle, aic.rect));
        }
        let body_lines: Vec<&str> = content
            .lines
            .iter()
            .filter(|l| !l.starts_with('[') || !l.ends_with(']'))
            .map(|s| s.as_str())
            .collect();
        let action_lines: Vec<&str> = content
            .lines
            .iter()
            .filter(|l| l.starts_with('[') && l.ends_with(']'))
            .map(|s| s.as_str())
            .collect();
        lines.push(format!("ai.content.lines: count={} rect={}", content.lines.len(), aic.rect));
        if !body_lines.is_empty() {
            for bl in body_lines.iter().take(3) {
                lines.push(format!("ai.content.body: {}", bl));
            }
        }
        if !action_lines.is_empty() {
            lines.push(format!("ai.content.actions: {}", action_lines.join(" ")));
        } else {
            lines.push("ai.content.actions: []".to_string());
        }
        lines.push("ai.content.input: placeholder='Ask anything...'".to_string());
    }

    lines
}
