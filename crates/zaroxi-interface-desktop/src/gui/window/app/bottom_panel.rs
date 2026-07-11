/*!
Unified bottom-panel rendering: Terminal / Problems / Output.

The bottom panel is a single surface with a shared header that doubles as a
compact tab strip; the body below renders the currently-selected tab. This
module is the ONE rendering path for all three tabs so their chrome stays
coherent:

- **Terminal** — the live PTY viewport (owned by [`TerminalController`]).
- **Problems** — a real diagnostics list (syntax errors from the parser plus any
  provider diagnostics), with severity, file and line.
- **Output** — the in-app operational log stream captured from `log::*` records.

Rendering takes only borrowed field references (never `&mut GuiApp`) so it can
run while the window borrow is held during a frame.
*/

use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;
use zaroxi_core_platform_terminal::TerminalPalette;

use super::terminal::{BottomTab, TerminalController};
use crate::output_log::OutputLog;

/// Renderer content-area vertical overhead (header + padding), mirroring
/// `render_frame_inner` in the render core.
const PANEL_CONTENT_OVERHEAD: f32 = 44.0;
/// Renderer line height (`DEFAULT_FONT_SIZE + EDITOR_LINE_LEADING`).
const LINE_H: f32 = 18.0;

/// Severity of a [`Problem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl ProblemSeverity {
    fn label(self) -> &'static str {
        match self {
            ProblemSeverity::Error => "error",
            ProblemSeverity::Warning => "warning",
            ProblemSeverity::Info => "info",
            ProblemSeverity::Hint => "hint",
        }
    }

    fn color(self, tokens: &StyleTokens) -> [f32; 4] {
        match self {
            ProblemSeverity::Error => tokens.status_error.to_array(),
            ProblemSeverity::Warning => tokens.status_warning.to_array(),
            ProblemSeverity::Info => tokens.status_info.to_array(),
            ProblemSeverity::Hint => tokens.text_muted.to_array(),
        }
    }

    /// Sort key so errors surface above warnings above info/hints.
    fn rank(self) -> u8 {
        match self {
            ProblemSeverity::Error => 0,
            ProblemSeverity::Warning => 1,
            ProblemSeverity::Info => 2,
            ProblemSeverity::Hint => 3,
        }
    }
}

/// A single problem shown in the Problems tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub severity: ProblemSeverity,
    pub message: String,
    /// Short file label (basename), when known.
    pub file: Option<String>,
    /// 1-based line, when known.
    pub line: Option<u32>,
    /// 1-based column, when known.
    pub column: Option<u32>,
}

impl Problem {
    /// Map a diagnostics-module `Diagnostic` into a panel `Problem`.
    pub fn from_diagnostic(d: crate::diagnostics::Diagnostic) -> Self {
        use crate::diagnostics::DiagnosticSeverity as S;
        let severity = match d.severity {
            S::Error => ProblemSeverity::Error,
            S::Warning => ProblemSeverity::Warning,
            S::Information => ProblemSeverity::Info,
            S::Hint => ProblemSeverity::Hint,
        };
        Self {
            severity,
            message: d.message,
            file: d.uri.map(|u| short_file(&u)),
            // Diagnostics carry 0-based positions; present them 1-based.
            line: d.line.map(|l| l + 1),
            column: d.column.map(|c| c + 1),
        }
    }

    /// Map a real Tree-sitter parse error into a panel `Problem`. Positions are
    /// 0-based in the parse tree; present them 1-based for the UI.
    pub fn from_parse_error(e: &super::background_parse::ParseError, owner: Option<&str>) -> Self {
        let (severity, message) = if e.missing {
            (ProblemSeverity::Warning, "missing expected token".to_string())
        } else {
            (ProblemSeverity::Error, "syntax error".to_string())
        };
        Self {
            severity,
            message,
            file: owner.map(short_file),
            line: Some(e.line + 1),
            column: Some(e.column + 1),
        }
    }

    /// The `file:line:col` location label, when any component is known.
    fn location(&self) -> Option<String> {
        let file = self.file.as_deref()?;
        match (self.line, self.column) {
            (Some(l), Some(c)) => Some(format!("{file}:{l}:{c}")),
            (Some(l), None) => Some(format!("{file}:{l}")),
            _ => Some(file.to_string()),
        }
    }
}

/// Reduce a path/URI to a short basename label.
pub fn short_file(uri: &str) -> String {
    let trimmed = uri.strip_prefix("buf:").unwrap_or(uri);
    trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed).to_string()
}

/// Render the selected bottom tab into its block. Single rendering path.
#[allow(clippy::too_many_arguments)]
pub fn render_tab(
    tab: BottomTab,
    terminal: &TerminalController,
    problems: &[Problem],
    output: &OutputLog,
    scroll: usize,
    block: &mut UiBlock,
    palette: &TerminalPalette,
    tokens: &StyleTokens,
) {
    match tab {
        BottomTab::Terminal => render_terminal(terminal, block, palette, tokens),
        BottomTab::Problems => render_problems(problems, scroll, block, tokens),
        BottomTab::Output => render_output(output, scroll, block, tokens),
    }

    // Shared chrome: the header doubles as a compact tab strip.
    block.title = tab_strip_title(tab, terminal, problems.len(), output.len());

    // Focus affordance: only the interactive Terminal gets an accent border, and
    // only when it actually holds keyboard focus.
    if tab == BottomTab::Terminal && terminal.focused {
        block.border_color = Some(tokens.accent.to_array());
        block.border_width = 1.0;
    } else {
        block.border_color = None;
        block.border_width = 0.0;
    }
}

/// Compact tab strip: `Terminal   [Problems 3]   Output`. The active tab is
/// bracketed; Problems carries a live count; Terminal notes an exited shell.
/// Focus is shown by the panel border, not by noisy text.
fn tab_strip_title(
    active: BottomTab,
    terminal: &TerminalController,
    problem_count: usize,
    output_count: usize,
) -> String {
    let terminal_label = match terminal.exited_note() {
        Some(note) => format!("Terminal ({note})"),
        None => "Terminal".to_string(),
    };
    let problems_label = if problem_count > 0 {
        format!("Problems {problem_count}")
    } else {
        "Problems".to_string()
    };
    let output_label =
        if output_count > 0 { format!("Output {output_count}") } else { "Output".to_string() };

    let labels = [
        (BottomTab::Terminal, terminal_label),
        (BottomTab::Problems, problems_label),
        (BottomTab::Output, output_label),
    ];
    labels
        .iter()
        .map(|(tab, label)| if *tab == active { format!("[{label}]") } else { label.clone() })
        .collect::<Vec<_>>()
        .join("   ")
}

fn render_terminal(
    terminal: &TerminalController,
    block: &mut UiBlock,
    palette: &TerminalPalette,
    tokens: &StyleTokens,
) {
    if terminal.is_open() {
        if let Some(dec) = terminal.build_decoration(palette) {
            dec.apply(block);
        }
    } else {
        // The shell is spawned lazily on the next frame; show a calm hint.
        apply_line_runs(
            block,
            vec![vec![("Starting terminal…".to_string(), tokens.text_muted.to_array())]],
        );
    }
}

fn render_problems(problems: &[Problem], scroll: usize, block: &mut UiBlock, tokens: &StyleTokens) {
    if problems.is_empty() {
        apply_line_runs(
            block,
            vec![vec![("No problems detected.".to_string(), tokens.text_muted.to_array())]],
        );
        return;
    }

    // Errors first, then warnings, then the rest — most severe on top.
    let mut ordered: Vec<&Problem> = problems.iter().collect();
    ordered.sort_by_key(|p| p.severity.rank());

    let muted = tokens.text_muted.to_array();
    let message_color = tokens.text_secondary.to_array();
    let start = scroll.min(ordered.len());
    let lines: Vec<Vec<(String, [f32; 4])>> = ordered[start..]
        .iter()
        .map(|p| {
            let mut runs = vec![(format!("{:<8}", p.severity.label()), p.severity.color(tokens))];
            if let Some(loc) = p.location() {
                runs.push((format!("{loc}  "), muted));
            }
            runs.push((p.message.clone(), message_color));
            runs
        })
        .collect();
    apply_line_runs(block, lines);
}

fn render_output(output: &OutputLog, scroll: usize, block: &mut UiBlock, tokens: &StyleTokens) {
    if output.is_empty() {
        apply_line_runs(
            block,
            vec![vec![("No output yet.".to_string(), tokens.text_muted.to_array())]],
        );
        return;
    }

    let muted = tokens.text_muted.to_array();
    let message_color = tokens.text_secondary.to_array();
    let visible = visible_rows(block);

    let lines = output.with_entries(|entries| {
        let total = entries.len();
        // Tail-anchored: the newest entries stay visible; scroll walks back.
        let end = total.saturating_sub(scroll);
        let start = end.saturating_sub(visible.max(1));
        entries
            .iter()
            .take(end)
            .skip(start)
            .map(|e| {
                let level_color = match e.level {
                    log::Level::Error => tokens.status_error.to_array(),
                    log::Level::Warn => tokens.status_warning.to_array(),
                    _ => message_color,
                };
                let secs = e.millis as f64 / 1000.0;
                vec![
                    (format!("{secs:>7.2}s "), muted),
                    (format!("{:<5} ", level_str(e.level)), level_color),
                    (format!("{}  ", e.target), muted),
                    (e.message.clone(), message_color),
                ]
            })
            .collect::<Vec<_>>()
    });
    apply_line_runs(block, lines);
}

fn level_str(level: log::Level) -> &'static str {
    match level {
        log::Level::Error => "ERROR",
        log::Level::Warn => "WARN",
        log::Level::Info => "INFO",
        log::Level::Debug => "DEBUG",
        log::Level::Trace => "TRACE",
    }
}

/// Whole rows that fit in the panel body for the current block geometry.
fn visible_rows(block: &UiBlock) -> usize {
    (((block.rect.h - PANEL_CONTENT_OVERHEAD) / LINE_H).floor()).max(1.0) as usize
}

/// Set a block's body from per-line colored runs, clearing any terminal payload.
fn apply_line_runs(block: &mut UiBlock, lines: Vec<Vec<(String, [f32; 4])>>) {
    reset_panel_body(block);
    let mut content = String::new();
    let mut spans: Vec<(String, [f32; 4])> = Vec::new();
    for line in &lines {
        for (text, color) in line {
            if !text.is_empty() {
                spans.push((text.clone(), *color));
                content.push_str(text);
            }
        }
        spans.push(("\n".to_string(), [0.0, 0.0, 0.0, 0.0]));
        content.push('\n');
    }
    block.content = content;
    block.content_spans = Some(spans);
}

/// Clear any terminal-grid payload / cursor from a panel block.
pub fn reset_panel_body(block: &mut UiBlock) {
    block.content_spans = None;
    block.terminal_cell_bg = None;
    block.block_cursor = false;
    block.cursor_line = None;
    block.cursor_col = None;
    block.highlight_active_line = false;
    block.content = String::new();
}

impl super::GuiApp {
    /// The active buffer URI used to key diagnostics, if any.
    fn active_problem_uri(&self) -> Option<String> {
        let comp = self.composition.as_ref()?;
        let abd = comp.latest_active_buffer_details()?;
        Some(abd.display.clone().unwrap_or_else(|| abd.buffer_id.to_string()))
    }

    /// Rebuild the merged Problems list from real sources: parser syntax errors
    /// for the active buffer plus any provider (LSP/ingested) diagnostics.
    /// Cheap; called once per frame before the bottom panel is rendered.
    pub(crate) fn refresh_problems(&mut self) {
        let mut list: Vec<Problem> = Vec::new();

        // Real syntax problems from the parser, only while they still belong to
        // the active file (dropped after a file switch until the next parse).
        let owner_matches =
            self.parse_problems_owner.as_deref() == self.committed_active_file.as_deref();
        if owner_matches {
            list.extend(self.parse_problems.iter().cloned());
        }

        // Provider diagnostics for the active buffer, when a provider is ready.
        if let Some(uri) = self.active_problem_uri()
            && let Some(diags) = crate::diagnostics::diagnostics_details_for_uri(&uri)
        {
            list.extend(diags.into_iter().map(Problem::from_diagnostic));
        }

        self.problems = list;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Diagnostic, DiagnosticSeverity};

    fn tokens() -> StyleTokens {
        zaroxi_core_engine_style::test_utils::test_tokens_dark()
    }

    fn problem(sev: ProblemSeverity, msg: &str, line: Option<u32>) -> Problem {
        Problem {
            severity: sev,
            message: msg.to_string(),
            file: Some("main.rs".into()),
            line,
            column: None,
        }
    }

    #[test]
    fn problems_empty_state_is_clean_not_a_stub() {
        let t = tokens();
        let mut block = UiBlock { rect: rect(400.0, 160.0), ..Default::default() };
        render_problems(&[], 0, &mut block, &t);
        assert!(block.content.contains("No problems detected"));
        assert!(!block.block_cursor);
        assert!(block.content_spans.is_some());
    }

    #[test]
    fn problems_render_severity_location_and_message() {
        let t = tokens();
        let problems = vec![
            problem(ProblemSeverity::Warning, "unused import", Some(3)),
            problem(ProblemSeverity::Error, "expected `;`", Some(12)),
        ];
        let mut block = UiBlock { rect: rect(400.0, 160.0), ..Default::default() };
        render_problems(&problems, 0, &mut block, &t);
        // Error is sorted above warning.
        let error_pos = block.content.find("expected `;`").unwrap();
        let warn_pos = block.content.find("unused import").unwrap();
        assert!(error_pos < warn_pos, "errors sort above warnings");
        assert!(block.content.contains("main.rs:12"));
        assert!(block.content.contains("error"));
    }

    #[test]
    fn output_empty_state_is_clean() {
        let t = tokens();
        let log = OutputLog::new(10);
        let mut block = UiBlock { rect: rect(400.0, 160.0), ..Default::default() };
        render_output(&log, 0, &mut block, &t);
        assert!(block.content.contains("No output yet"));
    }

    #[test]
    fn output_is_tail_anchored_newest_visible() {
        let t = tokens();
        let log = OutputLog::new(100);
        for i in 0..50 {
            log.push(log::Level::Info, "app", format!("event{i}"));
        }
        // Small panel: only a few rows fit; newest must be present.
        let mut block = UiBlock { rect: rect(600.0, 120.0), ..Default::default() };
        render_output(&log, 0, &mut block, &t);
        assert!(block.content.contains("event49"), "newest entry is visible");
        assert!(!block.content.contains("event0"), "oldest entry scrolled off");
    }

    #[test]
    fn tab_strip_marks_active_and_counts() {
        let term = TerminalController::new();
        let title = tab_strip_title(BottomTab::Problems, &term, 3, 0);
        assert!(title.contains("[Problems 3]"), "active problems tab with count: {title}");
        assert!(title.contains("Terminal"));
        assert!(title.contains("Output"));
        assert!(!title.contains("[Terminal]"));
    }

    #[test]
    fn problem_from_diagnostic_maps_fields_1_based() {
        let d = Diagnostic {
            message: "boom".into(),
            severity: DiagnosticSeverity::Error,
            uri: Some("buf:/x/main.rs".into()),
            line: Some(4),
            column: Some(2),
        };
        let p = Problem::from_diagnostic(d);
        assert_eq!(p.severity, ProblemSeverity::Error);
        assert_eq!(p.file.as_deref(), Some("main.rs"));
        assert_eq!(p.line, Some(5));
        assert_eq!(p.column, Some(3));
    }

    fn rect(w: f32, h: f32) -> zaroxi_core_engine_render::Rect {
        zaroxi_core_engine_render::Rect { x: 0.0, y: 0.0, w, h }
    }

    // ── GUI-level wiring tests (drive the real app state) ──
    //
    // These spawn a real shell via the controller and exercise the end-to-end
    // bottom-panel wiring through the unified `render_tab` path: viewport
    // projection (no "Ready" placeholder), tab switching, focus routing, and
    // session persistence.

    use super::super::test_support::make_headless_app;

    fn palette() -> TerminalPalette {
        super::super::terminal::palette_from_tokens(&tokens())
    }

    fn render(app: &crate::gui::window::GuiApp, tab: BottomTab) -> UiBlock {
        let mut block =
            UiBlock { rect: rect(700.0, 200.0), content: "Ready".into(), ..Default::default() };
        render_tab(
            tab,
            &app.terminal,
            &app.problems,
            &app.output_log,
            app.bottom_scroll,
            &mut block,
            &palette(),
            &tokens(),
        );
        block
    }

    #[test]
    fn default_bottom_tab_is_terminal() {
        let app = make_headless_app();
        assert_eq!(app.bottom_tab, BottomTab::Terminal);
    }

    #[test]
    fn selecting_terminal_tab_builds_viewport_not_ready_placeholder() {
        let mut app = make_headless_app();
        app.select_bottom_tab(0);
        assert!(app.terminal.is_open(), "selecting Terminal must start the shell");
        assert!(app.terminal.focused, "selecting Terminal focuses it");

        let block = render(&app, app.bottom_tab);
        assert_ne!(block.content, "Ready", "the Ready placeholder must be gone");
        assert!(block.block_cursor, "terminal viewport draws a block cursor");
        assert!(block.content_spans.is_some(), "terminal viewport emits a cell grid");
        assert!(block.title.contains("[Terminal]"), "active terminal tab marked: {}", block.title);
        // Focus is shown by the accent border, not by noisy header text.
        assert!(block.border_color.is_some(), "focused terminal shows an accent border");
        assert!(!block.title.contains("focused"), "no noisy focus text in the header");
    }

    #[test]
    fn switching_to_problems_releases_focus_and_persists_session() {
        let mut app = make_headless_app();
        app.select_bottom_tab(0);
        app.select_bottom_tab(1);
        assert!(!app.terminal.focused, "switching to Problems releases terminal focus");
        assert!(app.terminal.is_open(), "the terminal session persists across tab switches");

        let block = render(&app, app.bottom_tab);
        assert!(
            block.content.contains("No problems detected"),
            "clean empty state: {}",
            block.content
        );
        assert!(!block.block_cursor, "non-terminal tab must not draw a terminal cursor");
        assert!(block.border_color.is_none(), "non-terminal tab has no focus border");
        assert!(block.title.contains("[Problems]"), "active tab marked: {}", block.title);
    }

    #[test]
    fn output_tab_renders_real_captured_log_lines() {
        let mut app = make_headless_app();
        app.output_log.push(log::Level::Info, "terminal", "spawned shell 'bash'".into());
        app.output_log.push(log::Level::Warn, "workspace", "slow open".into());
        app.select_bottom_tab(2);

        let block = render(&app, app.bottom_tab);
        assert!(block.content.contains("spawned shell"), "real log line shown: {}", block.content);
        assert!(block.content.contains("WARN"), "level rendered: {}", block.content);
        assert!(block.title.contains("[Output 2]"), "output count in tab: {}", block.title);
    }

    #[test]
    fn problems_tab_shows_real_diagnostics_when_present() {
        let mut app = make_headless_app();
        // A real (ingested) diagnostic for the merged Problems list.
        app.problems = vec![Problem {
            severity: ProblemSeverity::Error,
            message: "expected `;`".into(),
            file: Some("main.rs".into()),
            line: Some(12),
            column: Some(5),
        }];
        app.select_bottom_tab(1);
        let block = render(&app, app.bottom_tab);
        assert!(block.content.contains("main.rs:12:5"));
        assert!(block.content.contains("expected `;`"));
        assert!(block.title.contains("[Problems 1]"));
    }

    #[test]
    fn click_to_focus_and_blur_are_the_focus_source_of_truth() {
        let mut app = make_headless_app();
        app.select_bottom_tab(1);
        assert!(!app.terminal.focused);
        app.focus_terminal_from_click();
        assert_eq!(app.bottom_tab, BottomTab::Terminal);
        assert!(app.terminal.focused, "clicking the body focuses the terminal");
        app.blur_terminal();
        assert!(!app.terminal.focused, "blur hands focus back to the editor");
    }

    #[test]
    fn close_action_leaves_a_restartable_dead_session() {
        let mut app = make_headless_app();
        app.select_bottom_tab(0);
        app.close_terminal_action();
        assert!(app.terminal.is_open(), "pane keeps the session to show an exited state");
        assert!(!app.terminal.is_alive());
    }
}
