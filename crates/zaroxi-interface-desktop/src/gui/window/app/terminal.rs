/*!
Integrated terminal controller and desktop wiring.

Owns the interface-side half of the integrated terminal:
- a lazily-spawned [`TerminalSession`] (real PTY + shell),
- keyboard-focus capture so editor shortcuts don't hijack typing,
- per-frame output pumping / resize / scrollback,
- projection of the live emulator grid into the bottom-panel [`UiBlock`].

The heavy lifting (PTY, VT emulation, key encoding, color resolution) lives in
`zaroxi-core-platform-terminal`; this module is the thin, app-aware glue.
*/

use std::path::PathBuf;

use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;
use zaroxi_core_platform_terminal::{
    KeyModifiers, PumpOutcome, TerminalConfig, TerminalKey, TerminalPalette, TerminalSession,
    build_grid, encode_key,
};

use super::{GuiApp, InvalidationFlags};

/// Renderer line height (must match `DEFAULT_FONT_SIZE + EDITOR_LINE_LEADING`
/// in the render core so the emulated grid rows align with drawn rows).
const TERMINAL_LINE_H: f32 = 18.0;
/// Renderer content-area vertical overhead for a panel block: header (28) plus
/// content padding (8) top and bottom. Mirrors `render_frame_inner`.
const PANEL_CONTENT_OVERHEAD: f32 = 44.0;
/// Renderer content-area horizontal overhead: content padding (8) both sides.
const PANEL_CONTENT_PAD_X: f32 = 16.0;

/// Which bottom-panel tab is currently selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BottomTab {
    #[default]
    Terminal,
    Problems,
    Output,
}

impl BottomTab {
    pub fn from_index(idx: usize) -> Self {
        match idx {
            1 => BottomTab::Problems,
            2 => BottomTab::Output,
            _ => BottomTab::Terminal,
        }
    }

    pub fn index(self) -> usize {
        match self {
            BottomTab::Terminal => 0,
            BottomTab::Problems => 1,
            BottomTab::Output => 2,
        }
    }
}

/// Owns the integrated-terminal session and interface state.
#[derive(Default)]
pub struct TerminalController {
    session: Option<TerminalSession>,
    /// True when the terminal panel currently captures keyboard input.
    pub focused: bool,
}

impl TerminalController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a session (running or exited) currently exists.
    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }

    /// Whether the loop should keep polling this terminal for output.
    pub fn needs_poll(&self) -> bool {
        self.session.as_ref().map(|s| s.is_alive()).unwrap_or(false)
    }

    /// The child is running (not exited).
    pub fn is_alive(&self) -> bool {
        self.session.as_ref().map(|s| s.is_alive()).unwrap_or(false)
    }

    /// Spawn a fresh session rooted at `cwd`. Replaces any existing session.
    pub fn start(&mut self, cwd: Option<PathBuf>, rows: u16, cols: u16) {
        let cfg =
            TerminalConfig { cwd, rows: rows.max(1), cols: cols.max(1), ..Default::default() };
        match TerminalSession::spawn(&cfg) {
            Ok(session) => {
                log::info!("terminal: spawned shell '{}' ({}x{})", session.program(), cols, rows);
                self.session = Some(session);
            }
            Err(e) => {
                log::error!("terminal: failed to spawn shell: {e}");
                self.session = None;
            }
        }
    }

    /// Restart a dead session (or start one if none exists).
    pub fn restart(&mut self, cwd: Option<PathBuf>, rows: u16, cols: u16) {
        if let Some(s) = self.session.as_mut() {
            s.shutdown();
        }
        self.session = None;
        self.start(cwd, rows, cols);
    }

    /// Drain PTY output into the emulator. Returns the pump outcome.
    pub fn pump(&mut self) -> PumpOutcome {
        match self.session.as_mut() {
            Some(s) => s.pump(),
            None => PumpOutcome::default(),
        }
    }

    /// Resize the PTY + emulator. No-op when unchanged or when closed.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        if let Some(s) = self.session.as_mut()
            && let Err(e) = s.resize(rows, cols)
        {
            log::warn!("terminal: resize failed: {e}");
        }
    }

    /// Send already-encoded bytes to the shell.
    pub fn send_bytes(&mut self, bytes: &[u8]) {
        if let Some(s) = self.session.as_mut()
            && let Err(e) = s.send_input(bytes)
        {
            log::warn!("terminal: input write failed: {e}");
        }
    }

    /// Paste text, honoring bracketed-paste mode when the program requested it.
    pub fn paste(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let Some(s) = self.session.as_mut() else { return };
        let bytes = if s.bracketed_paste() {
            let mut out = Vec::with_capacity(text.len() + 12);
            out.extend_from_slice(b"\x1b[200~");
            out.extend_from_slice(text.as_bytes());
            out.extend_from_slice(b"\x1b[201~");
            out
        } else {
            text.as_bytes().to_vec()
        };
        if let Err(e) = s.send_input(&bytes) {
            log::warn!("terminal: paste failed: {e}");
        }
    }

    /// Whether the program requested application-cursor-key mode.
    pub fn application_cursor(&self) -> bool {
        self.session.as_ref().map(|s| s.application_cursor()).unwrap_or(false)
    }

    /// Scroll the scrollback view. `delta > 0` moves up (into history).
    pub fn scroll(&mut self, delta: i32) {
        let Some(s) = self.session.as_mut() else { return };
        if delta > 0 {
            s.scroll_up(delta as usize);
        } else if delta < 0 {
            s.scroll_down((-delta) as usize);
        }
    }

    /// Terminate the session (best effort).
    pub fn shutdown(&mut self) {
        if let Some(s) = self.session.as_mut() {
            s.shutdown();
        }
    }

    /// Short status label for the tab strip: the shell name while alive, or an
    /// exit note once the child has terminated. `None` when no session exists.
    pub fn status_label(&self) -> Option<String> {
        let s = self.session.as_ref()?;
        let prog = s.program();
        let name = prog.rsplit(['/', '\\']).next().unwrap_or(prog).to_string();
        match s.exit_status() {
            Some(exit) => Some(format!("exited {}", exit.code)),
            None => Some(name),
        }
    }

    /// Header title for the bottom panel: program name + exit note.
    fn title(&self) -> String {
        match self.session.as_ref() {
            Some(s) => {
                let prog = s.program();
                let name = prog.rsplit(['/', '\\']).next().unwrap_or(prog);
                if let Some(exit) = s.exit_status() {
                    format!("Terminal — {name} [exited: {}]", exit.code)
                } else {
                    format!("Terminal — {name}")
                }
            }
            None => "Terminal \u{2022} Problems \u{2022} Output".to_string(),
        }
    }

    /// Build the render decoration for the current screen, if a session exists.
    pub fn build_decoration(&self, palette: &TerminalPalette) -> Option<TerminalDecoration> {
        let session = self.session.as_ref()?;
        let grid = build_grid(session.screen(), palette);
        Some(decoration_from_grid(&grid, self.title(), palette))
    }

    /// The bottom-panel tab strip label with the active tab marked, plus the
    /// live shell status and focus cue for the Terminal tab.
    fn tab_strip_title(&self, active: BottomTab) -> String {
        let names = ["Terminal", "Problems", "Output"];
        let active_idx = active.index();
        let mut parts: Vec<String> = Vec::with_capacity(3);
        for (i, name) in names.iter().enumerate() {
            if i == active_idx {
                if i == 0 {
                    let label = match self.status_label() {
                        Some(s) => format!("[Terminal: {s}]"),
                        None => "[Terminal]".to_string(),
                    };
                    if self.focused {
                        parts.push(format!("{label} (focused)"));
                    } else {
                        parts.push(label);
                    }
                } else {
                    parts.push(format!("[{name}]"));
                }
            } else {
                parts.push(name.to_string());
            }
        }
        parts.join("    ")
    }

    /// Project the selected bottom tab onto its render block: the live terminal
    /// grid for Terminal, or a clean placeholder for Problems/Output. Replaces
    /// the legacy static "Ready" body entirely.
    ///
    /// Takes `&self` (borrows only the controller field of the app) so it can be
    /// called while the window borrow is held during rendering.
    pub fn apply_bottom_panel(
        &self,
        active: BottomTab,
        block: &mut UiBlock,
        palette: &TerminalPalette,
        accent: [f32; 4],
    ) {
        match active {
            BottomTab::Terminal => {
                if self.is_open() {
                    if let Some(dec) = self.build_decoration(palette) {
                        dec.apply(block);
                    }
                } else {
                    reset_panel_body(block);
                    block.content = "Starting terminal…".to_string();
                }
            }
            BottomTab::Problems => {
                reset_panel_body(block);
                block.content = "No problems have been detected in the workspace.".to_string();
            }
            BottomTab::Output => {
                reset_panel_body(block);
                block.content = "No output.".to_string();
            }
        }
        // The header doubles as the tab strip (active tab marked).
        block.title = self.tab_strip_title(active);
        // A focused terminal gets an accent border so focus is obvious.
        if active == BottomTab::Terminal && self.focused {
            block.border_color = Some(accent);
            block.border_width = 1.0;
        } else {
            block.border_color = None;
            block.border_width = 0.0;
        }
    }
}

/// Pure projection of a terminal grid into a [`TerminalDecoration`].
///
/// Coalesces per-cell foreground colors into text runs (trailing blanks
/// trimmed) and per-cell backgrounds into run-length rectangles (the default
/// background is skipped since the panel already fills it). Kept free of any
/// session/PTY state so the adapter is deterministically unit-testable.
fn decoration_from_grid(
    grid: &zaroxi_core_platform_terminal::TerminalGrid,
    title: String,
    palette: &TerminalPalette,
) -> TerminalDecoration {
    let mut content = String::new();
    let mut content_spans: Vec<(String, [f32; 4])> = Vec::new();
    let mut cell_bg: Vec<(usize, usize, usize, [f32; 4])> = Vec::new();

    for row in 0..grid.rows {
        // ── Foreground text runs (coalesced by color, trailing blanks trimmed) ──
        let last_non_blank = (0..grid.cols)
            .rev()
            .find(|&c| grid.cell(row, c).map(|cell| cell.ch != ' ').unwrap_or(false));
        if let Some(last) = last_non_blank {
            let mut run = String::new();
            let mut run_color = grid.cell(row, 0).map(|c| c.fg).unwrap_or(palette.foreground);
            let mut col = 0usize;
            while col <= last {
                let cell = match grid.cell(row, col) {
                    Some(c) => c,
                    None => break,
                };
                if cell.wide_continuation {
                    col += 1;
                    continue;
                }
                if cell.fg != run_color && !run.is_empty() {
                    content_spans.push((std::mem::take(&mut run), run_color));
                }
                run_color = cell.fg;
                run.push(cell.ch);
                content.push(cell.ch);
                col += 1;
            }
            if !run.is_empty() {
                content_spans.push((run, run_color));
            }
        }
        content_spans.push(("\n".to_string(), palette.foreground));
        content.push('\n');

        // ── Background runs (coalesced; default background skipped) ──
        let mut c = 0usize;
        while c < grid.cols {
            let Some(cell) = grid.cell(row, c) else { break };
            if cell.bg == palette.background {
                c += 1;
                continue;
            }
            let color = cell.bg;
            let start = c;
            let mut len = 0usize;
            while c < grid.cols && grid.cell(row, c).map(|x| x.bg == color).unwrap_or(false) {
                len += 1;
                c += 1;
            }
            cell_bg.push((row, start, len, color));
        }
    }

    let cursor = if grid.cursor.visible { Some((grid.cursor.row, grid.cursor.col)) } else { None };

    TerminalDecoration { title, content, content_spans, cell_bg, cursor }
}

/// Draw payload injected into the bottom-panel [`UiBlock`].
pub struct TerminalDecoration {
    title: String,
    content: String,
    content_spans: Vec<(String, [f32; 4])>,
    cell_bg: Vec<(usize, usize, usize, [f32; 4])>,
    cursor: Option<(usize, usize)>,
}

impl TerminalDecoration {
    /// Overlay this terminal frame onto the bottom-panel block.
    pub fn apply(self, block: &mut UiBlock) {
        block.title = self.title;
        block.content = self.content;
        block.content_spans = Some(self.content_spans);
        block.terminal_cell_bg = Some(self.cell_bg);
        block.block_cursor = true;
        block.highlight_active_line = false;
        block.content_offset_x = 0.0;
        block.content_offset_y = 0.0;
        block.content_line_offset = None;
        block.selection_range = None;
        match self.cursor {
            Some((row, col)) => {
                block.cursor_line = Some(row);
                block.cursor_col = Some(col);
            }
            None => {
                block.cursor_line = None;
                block.cursor_col = None;
            }
        }
    }
}

/// Build a terminal palette from the active theme tokens so the terminal feels
/// native to the current editor theme.
pub fn palette_from_tokens(tokens: &StyleTokens) -> TerminalPalette {
    TerminalPalette::from_surface(
        tokens.text_primary.to_array(),
        tokens.bottom_panel_background.to_array(),
        tokens.editor_cursor.to_array(),
    )
}

/// Compute `(rows, cols)` for a bottom-panel region rect, matching the
/// renderer's content-area insets and cell metrics.
pub fn grid_dims(region_w: f32, region_h: f32, char_w: f32) -> (u16, u16) {
    let content_w = (region_w - PANEL_CONTENT_PAD_X).max(0.0);
    let content_h = (region_h - PANEL_CONTENT_OVERHEAD).max(0.0);
    let cols = (content_w / char_w.max(1.0)).floor().max(1.0) as u16;
    let rows = (content_h / TERMINAL_LINE_H).floor().max(1.0) as u16;
    (rows, cols)
}

impl GuiApp {
    /// Default working directory for a new terminal: the workspace root.
    fn terminal_default_cwd(&self) -> Option<PathBuf> {
        self.composition.as_ref().and_then(|c| c.workspace_root_path.clone())
    }

    /// Current `(rows, cols)` for the bottom-panel region, from the last layout.
    fn terminal_layout_dims(&self) -> (u16, u16) {
        let char_w = self.monospace_advance_x().unwrap_or(8.0).max(1.0);
        let regions = self.layout_controller.shell_regions();
        let dims = crate::gui::region_dispatch::find_region_by_role(
            regions,
            zaroxi_core_engine_style::PanelRole::BottomPanel,
        )
        .map(|r| grid_dims(r.rect.width as f32, r.rect.height as f32, char_w));
        dims.unwrap_or((24, 80))
    }

    /// Toggle the integrated terminal: select+open+focus the Terminal tab on
    /// first use, then flip keyboard focus on subsequent invocations. A dead
    /// session is restarted.
    pub(crate) fn toggle_terminal(&mut self) {
        let (rows, cols) = self.terminal_layout_dims();
        let cwd = self.terminal_default_cwd();
        self.bottom_tab = BottomTab::Terminal;
        if !self.terminal.is_open() {
            self.terminal.start(cwd, rows, cols);
            self.terminal.focused = true;
        } else if !self.terminal.is_alive() {
            self.terminal.restart(cwd, rows, cols);
            self.terminal.focused = true;
        } else {
            self.terminal.focused = !self.terminal.focused;
        }
        self.invalidate(InvalidationFlags::content());
    }

    /// Select a bottom-panel tab (0=Terminal, 1=Problems, 2=Output). Selecting
    /// Terminal opens (lazily) and focuses the shell; other tabs release focus.
    pub(crate) fn select_bottom_tab(&mut self, idx: usize) {
        let tab = BottomTab::from_index(idx);
        self.bottom_tab = tab;
        if tab == BottomTab::Terminal {
            let (rows, cols) = self.terminal_layout_dims();
            let cwd = self.terminal_default_cwd();
            if !self.terminal.is_open() {
                self.terminal.start(cwd, rows, cols);
            }
            self.terminal.focused = true;
        } else {
            self.terminal.focused = false;
        }
        self.invalidate(InvalidationFlags::content());
    }

    /// Focus the terminal from a click in the panel body (opens it if needed).
    pub(crate) fn focus_terminal_from_click(&mut self) {
        self.bottom_tab = BottomTab::Terminal;
        let (rows, cols) = self.terminal_layout_dims();
        let cwd = self.terminal_default_cwd();
        if !self.terminal.is_open() {
            self.terminal.start(cwd, rows, cols);
        }
        self.terminal.focused = true;
        self.invalidate(InvalidationFlags::content());
    }

    /// The Terminal panel's close/restart action: kill the running shell (it
    /// then shows an exited state; Enter or Ctrl+` restarts it).
    pub(crate) fn close_terminal_action(&mut self) {
        self.terminal.shutdown();
        self.invalidate(InvalidationFlags::content());
    }

    /// Give up terminal keyboard focus (Escape from a live prompt keeps focus;
    /// this is the explicit "return to editor" path used by `Ctrl+\``).
    pub(crate) fn blur_terminal(&mut self) {
        if self.terminal.focused {
            self.terminal.focused = false;
            self.invalidate(InvalidationFlags::content());
        }
    }

    /// Route a decoded key to the terminal, restarting a dead session on Enter.
    pub(crate) fn terminal_send_key(&mut self, key: TerminalKey) {
        if !self.terminal.is_alive() {
            if matches!(key, TerminalKey::Enter) {
                let (rows, cols) = self.terminal_layout_dims();
                let cwd = self.terminal_default_cwd();
                self.terminal.restart(cwd, rows, cols);
                self.invalidate(InvalidationFlags::content());
            }
            return;
        }
        let mods =
            KeyModifiers { ctrl: self.ctrl_held, alt: self.alt_held, shift: self.shift_held };
        let app_cursor = self.terminal.application_cursor();
        if let Some(bytes) = encode_key(key, mods, app_cursor) {
            self.terminal.send_bytes(&bytes);
            self.invalidate(InvalidationFlags::input());
        }
    }

    /// Paste clipboard text into the terminal.
    pub(crate) fn terminal_paste_clipboard(&mut self) {
        match zaroxi_core_engine_clipboard::get_text() {
            Ok(text) => {
                self.terminal.paste(&text);
                self.invalidate(InvalidationFlags::input());
            }
            Err(e) => log::warn!("terminal: clipboard paste failed: {e}"),
        }
    }

    /// Per-frame terminal maintenance: lazily start the shell when the Terminal
    /// tab is active, resize the PTY to the current layout, and drain output.
    /// Runs before the dirty-check so live output keeps painting.
    pub(crate) fn maintain_terminal(&mut self) {
        // Lazily bring the shell up the first time the Terminal tab is shown.
        if self.bottom_tab == BottomTab::Terminal && !self.terminal.is_open() {
            let (rows, cols) = self.terminal_layout_dims();
            let cwd = self.terminal_default_cwd();
            self.terminal.start(cwd, rows, cols);
        }
        if !self.terminal.is_open() {
            return;
        }
        let (rows, cols) = self.terminal_layout_dims();
        self.terminal.resize(rows, cols);
        let outcome = self.terminal.pump();
        if outcome.dirty {
            self.invalidate(InvalidationFlags::content());
        }
    }

    /// Pump terminal output (idle path). Invalidates when output arrived.
    pub(crate) fn poll_terminal(&mut self) {
        if !self.terminal.is_open() {
            return;
        }
        let outcome = self.terminal.pump();
        if outcome.dirty {
            self.invalidate(InvalidationFlags::content());
        }
    }
}

/// Clear any terminal-grid payload from a panel block (used for the non-terminal
/// tabs and the pre-startup state).
fn reset_panel_body(block: &mut UiBlock) {
    block.content_spans = None;
    block.terminal_cell_bg = None;
    block.block_cursor = false;
    block.cursor_line = None;
    block.cursor_col = None;
    block.highlight_active_line = false;
    block.content = String::new();
}

#[cfg(test)]
mod tests {
    use super::*;
    use zaroxi_core_platform_terminal::{TerminalCell, TerminalCursor, TerminalGrid};

    fn cell(ch: char, fg: [f32; 4], bg: [f32; 4]) -> TerminalCell {
        TerminalCell {
            ch,
            fg,
            bg,
            bold: false,
            italic: false,
            underline: false,
            wide: false,
            wide_continuation: false,
        }
    }

    #[test]
    fn grid_dims_match_renderer_metrics() {
        // char_w 8, content overhead 44 vertical, 16 horizontal:
        // 200 wide -> (200-16)/8 = 23 cols; 150 tall -> (150-44)/18 = 5 rows.
        assert_eq!(grid_dims(200.0, 150.0, 8.0), (5, 23));
        // Degenerate sizes clamp to at least 1x1.
        assert_eq!(grid_dims(0.0, 0.0, 8.0), (1, 1));
    }

    #[test]
    fn decoration_projects_text_bg_and_cursor() {
        let palette = TerminalPalette::default();
        let fg = [1.0, 0.0, 0.0, 1.0];
        let bg = [0.0, 0.5, 0.0, 1.0];
        // 1 row × 4 cols: "hi" red fg; col 2 carries a colored bg; trailing blank.
        let mut cells = vec![cell(' ', palette.foreground, palette.background); 4];
        cells[0] = cell('h', fg, palette.background);
        cells[1] = cell('i', fg, palette.background);
        cells[2] = cell(' ', palette.foreground, bg);
        let grid = TerminalGrid {
            rows: 1,
            cols: 4,
            cells,
            cursor: TerminalCursor { row: 0, col: 2, visible: true },
            alternate_screen: false,
        };
        let dec = decoration_from_grid(&grid, "T".to_string(), &palette);
        assert!(
            dec.content_spans.iter().any(|(t, c)| t == "hi" && *c == fg),
            "expected coalesced red 'hi' run"
        );
        assert!(
            dec.cell_bg.iter().any(|&(r, s, l, col)| r == 0 && s == 2 && l == 1 && col == bg),
            "expected a colored background run at col 2"
        );
        assert_eq!(dec.cursor, Some((0, 2)));
    }

    #[test]
    fn hidden_cursor_is_not_emitted() {
        let palette = TerminalPalette::default();
        let cells = vec![cell(' ', palette.foreground, palette.background); 2];
        let grid = TerminalGrid {
            rows: 1,
            cols: 2,
            cells,
            cursor: TerminalCursor { row: 0, col: 0, visible: false },
            alternate_screen: false,
        };
        let dec = decoration_from_grid(&grid, "T".to_string(), &palette);
        assert_eq!(dec.cursor, None);
    }

    #[test]
    fn apply_sets_terminal_block_fields() {
        let palette = TerminalPalette::default();
        let cells = vec![cell('x', [1.0, 1.0, 1.0, 1.0], palette.background); 2];
        let grid = TerminalGrid {
            rows: 1,
            cols: 2,
            cells,
            cursor: TerminalCursor { row: 0, col: 1, visible: true },
            alternate_screen: false,
        };
        let dec = decoration_from_grid(&grid, "Terminal".to_string(), &palette);
        let mut block = UiBlock::default();
        dec.apply(&mut block);
        assert!(block.block_cursor);
        assert_eq!(block.cursor_line, Some(0));
        assert_eq!(block.cursor_col, Some(1));
        assert!(block.content_spans.is_some());
        assert!(block.terminal_cell_bg.is_some());
        assert!(!block.highlight_active_line);
        assert_eq!(block.title, "Terminal");
    }

    #[test]
    fn controller_starts_closed_and_unfocused() {
        let c = TerminalController::new();
        assert!(!c.is_open());
        assert!(!c.focused);
        assert!(!c.needs_poll());
    }

    // ── GUI-level wiring tests (drive the real app state) ──
    //
    // These spawn a real shell via the controller and exercise the end-to-end
    // wiring: bottom-tab selection, viewport projection (no "Ready"
    // placeholder), focus routing, and session persistence across tab switches.

    use super::super::test_support::make_headless_app;
    use zaroxi_core_engine_render::UiBlock;
    use zaroxi_core_engine_style::test_utils::test_tokens_dark;

    fn palette() -> TerminalPalette {
        let t = test_tokens_dark();
        palette_from_tokens(&t)
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

        // Start from the legacy placeholder block and project the terminal tab.
        let mut block = UiBlock { content: "Ready".to_string(), ..Default::default() };
        app.terminal.apply_bottom_panel(app.bottom_tab, &mut block, &palette(), [0.0; 4]);

        assert_ne!(block.content, "Ready", "the Ready placeholder must be gone");
        assert!(block.block_cursor, "terminal viewport draws a block cursor");
        assert!(block.content_spans.is_some(), "terminal viewport emits a cell grid");
        assert!(block.title.contains("Terminal"), "header shows the Terminal tab: {}", block.title);
        assert!(block.title.contains("focused"), "focused terminal is marked: {}", block.title);
    }

    #[test]
    fn problems_and_output_tabs_show_placeholders_and_release_focus() {
        let mut app = make_headless_app();
        app.select_bottom_tab(0); // open + focus terminal first
        app.select_bottom_tab(1); // Problems
        assert!(!app.terminal.focused, "switching to Problems releases terminal focus");
        assert!(app.terminal.is_open(), "the terminal session persists across tab switches");

        let mut block = UiBlock::default();
        app.terminal.apply_bottom_panel(app.bottom_tab, &mut block, &palette(), [0.0; 4]);
        assert!(block.content.contains("No problems"), "problems placeholder: {}", block.content);
        assert!(!block.block_cursor, "non-terminal tab must not draw a terminal cursor");
        assert!(block.content_spans.is_none());
        assert!(block.title.contains("[Problems]"), "active tab marked: {}", block.title);

        app.select_bottom_tab(2); // Output
        let mut block = UiBlock::default();
        app.terminal.apply_bottom_panel(app.bottom_tab, &mut block, &palette(), [0.0; 4]);
        assert!(block.content.contains("No output"));
        assert!(!block.block_cursor);
    }

    #[test]
    fn click_to_focus_and_blur_are_the_focus_source_of_truth() {
        let mut app = make_headless_app();
        app.select_bottom_tab(1); // Problems: terminal not focused
        assert!(!app.terminal.focused);
        app.focus_terminal_from_click();
        assert_eq!(app.bottom_tab, BottomTab::Terminal);
        assert!(app.terminal.focused, "clicking the body focuses the terminal");
        app.blur_terminal();
        assert!(!app.terminal.focused, "blur hands focus back to the editor");
    }

    #[test]
    fn session_persists_and_survives_return_to_terminal_tab() {
        let mut app = make_headless_app();
        app.select_bottom_tab(0);
        assert!(app.terminal.is_open());
        // Leave and come back: the same session must remain (no respawn/kill).
        app.select_bottom_tab(2);
        assert!(app.terminal.is_open());
        app.select_bottom_tab(0);
        assert!(app.terminal.is_open());
        assert!(app.terminal.focused);
    }

    #[test]
    fn close_action_shuts_down_but_leaves_a_restartable_pane() {
        let mut app = make_headless_app();
        app.select_bottom_tab(0);
        app.close_terminal_action();
        // Session object remains (so the pane shows an exited/dead state), but
        // it is no longer alive.
        assert!(app.terminal.is_open());
        assert!(!app.terminal.is_alive());
    }
}
