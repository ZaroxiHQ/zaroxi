/// Tiny, semantic scene-description model used by the engine.
///
/// This model is intentionally minimal and semantic-only:
/// - carries text lines and viewport facts,
/// - preserves cursor/selection presence as a semantic signal,
/// - carries small semantic text blocks (status, chrome, last command),
/// - carries boolean flags for presence of AI indicators.
/// It does NOT contain any layout, pixel coordinates, colors, fonts, or GPU
/// resources. It is explicitly convertible from
/// `zaroxi_core_engine_view::EngineShellViewInput`.
use zaroxi_core_engine_view::EngineShellViewInput;
mod chrome;
pub use self::chrome::{ShellChrome, Tab};

/// Semantic, read-only scene model for the engine shell.
///
/// Keep this tiny and stable: it's a descriptive hand-off to later rendering
/// phases without any visual/layout concerns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellSceneModel {
    /// Visible text lines (cloned strings).
    pub text_lines: Vec<String>,

    /// Top-most visible line index (1-based).
    pub viewport_top_line: u32,

    /// Total number of lines in the document.
    pub viewport_total_lines: u32,

    /// Optional compact viewport summary (opaque string).
    pub viewport_summary: Option<String>,

    /// Optional cursor line (1-based) when present.
    pub cursor_line: Option<u32>,

    /// Optional cursor column when present.
    pub cursor_column: Option<u32>,

    /// Whether a selection is present (semantic flag).
    pub selection_present: bool,

    /// Optional status bar text (semantic).
    pub status_text: Option<String>,

    /// Optional shell chrome/header text (semantic).
    pub chrome_text: Option<String>,

    /// Optional last command string (semantic).
    pub last_command: Option<String>,

    /// Whether an AI/status indicator is present (semantic flag).
    pub ai_status_present: bool,
}

impl From<EngineShellViewInput> for ShellSceneModel {
    fn from(src: EngineShellViewInput) -> Self {
        // Minimal semantic mapping from the view model into a scene description.
        // We purposefully do NOT attempt to compute layout or pixel metrics here.
        Self {
            text_lines: src.lines.clone(),
            viewport_top_line: src.top_line,
            viewport_total_lines: src.total_lines,
            viewport_summary: src.viewport_summary.clone(),
            cursor_line: src.cursor_line,
            cursor_column: src.cursor_column,
            // Selection presence is a semantic signal: either an explicit selection
            // or at least a cursor line present.
            selection_present: src.selection.is_some() || src.cursor_line.is_some(),
            status_text: src.status_text.clone(),
            chrome_text: src.shell_chrome.clone(),
            last_command: src.last_command.clone(),
            ai_status_present: src.ai_present,
        }
    }
}

/// Caret primitive describing a thin vertical caret. Coordinates are absolute
/// window-space and sized in integer pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaretItem {
    pub x: u32,
    pub y: u32,
    pub height: u32,
}

/// Selection rectangle primitive covering a contiguous highlighted area.
/// Consumers may emit multiple SelectionRect entries for multi-line selections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

// -----------------------------------------------------------------------------
// Lightweight runtime scene state & minimal input helpers (Phase 4)
// -----------------------------------------------------------------------------
//
// To keep Phase 4 incremental and avoid wide refactors we expose a tiny,
// engine-owned runtime seam that other crates (harness / interface) may use
// to publish the current ShellSceneModel. This is intentionally small:
// - a globally accessible, RwLock-protected ShellSceneModel
// - getters/setters for the whole model
// - simple editing helpers: insert_char, backspace, move_cursor, scroll_by_lines
//
// These helpers are deliberately naive (char-count columns, UTF-8 aware via
// char-index -> byte-index translation) but are sufficient for early input
// wiring and deterministic tests. They will be replaced by the application-
// owned editor buffer / transaction system in later phases.
use std::sync::{OnceLock, RwLock};

fn default_scene() -> ShellSceneModel {
    ShellSceneModel {
        text_lines: vec![
            "fn main() {".to_string(),
            "    println!(\"Hello, Zaroxi!\");".to_string(),
            "}".to_string(),
        ],
        viewport_top_line: 1,
        viewport_total_lines: 3,
        viewport_summary: None,
        cursor_line: Some(1),
        cursor_column: Some(0),
        selection_present: false,
        status_text: None,
        chrome_text: None,
        last_command: None,
        ai_status_present: false,
    }
}

static CURRENT_SCENE: OnceLock<RwLock<ShellSceneModel>> = OnceLock::new();

fn scene_lock<'a>() -> &'a RwLock<ShellSceneModel> {
    CURRENT_SCENE.get_or_init(|| RwLock::new(default_scene()))
}

fn clamp_top_line(scene: &mut ShellSceneModel) {
    let total = scene.text_lines.len() as u32;
    scene.viewport_total_lines = total;
    if scene.viewport_top_line == 0 {
        scene.viewport_top_line = 1;
    }
    if scene.viewport_top_line > total && total > 0 {
        scene.viewport_top_line = total;
    }
    if total == 0 {
        scene.viewport_top_line = 1;
        scene.viewport_total_lines = 0;
    }
}

/// Return a cloned snapshot of the current ShellSceneModel.
pub fn get_current_scene() -> ShellSceneModel {
    scene_lock().read().unwrap().clone()
}

/// Overwrite the current ShellSceneModel with the provided model.
pub fn set_current_scene(mut model: ShellSceneModel) {
    clamp_top_line(&mut model);
    let lock = scene_lock();
    *lock.write().unwrap() = model;
}

/// Convert a char-based column index into a byte index within `s`.
fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }
    s.char_indices()
        .nth(char_idx)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or_else(|| s.len())
}

/// Ensure cursor_line/column invariants (1-based lines, 0-based column)
fn normalize_cursor(scene: &mut ShellSceneModel) {
    let total = scene.text_lines.len() as u32;
    if total == 0 {
        scene.cursor_line = Some(1);
        scene.cursor_column = Some(0);
        scene.viewport_total_lines = 0;
        scene.viewport_top_line = 1;
        return;
    }
    let cl = scene.cursor_line.unwrap_or(1).max(1).min(total);
    scene.cursor_line = Some(cl);
    let line_idx = (cl - 1) as usize;
    let col = scene.cursor_column.unwrap_or(0) as usize;
    let line_len = scene.text_lines.get(line_idx).map(|l| l.chars().count()).unwrap_or(0);
    scene.cursor_column = Some((col.min(line_len)) as u32);
    clamp_top_line(scene);
}

/// Insert a unicode character at the current cursor position (advances column).
pub fn insert_char(ch: char) {
    let lock = scene_lock();
    let mut s = lock.write().unwrap();
    if s.text_lines.is_empty() {
        s.text_lines.push(String::new());
    }
    if s.cursor_line.is_none() {
        s.cursor_line = Some(1);
    }
    let cl = s.cursor_line.unwrap_or(1).max(1);
    let line_idx = (cl - 1) as usize;
    if line_idx >= s.text_lines.len() {
        s.text_lines.resize(line_idx + 1, String::new());
    }
    let col = s.cursor_column.unwrap_or(0) as usize;
    let line = &mut s.text_lines[line_idx];
    let byte_idx = char_to_byte_index(line, col);
    line.insert_str(byte_idx, &ch.to_string());
    s.cursor_column = Some((col + 1) as u32);
    s.viewport_total_lines = s.text_lines.len() as u32;
}

/// Backspace: remove the character before the cursor, merging lines when at col 0.
pub fn backspace() {
    let lock = scene_lock();
    let mut s = lock.write().unwrap();
    if s.text_lines.is_empty() {
        return;
    }
    if s.cursor_line.is_none() {
        s.cursor_line = Some(1);
        s.cursor_column = Some(0);
    }
    let mut cl = s.cursor_line.unwrap_or(1).max(1);
    let col = s.cursor_column.unwrap_or(0) as usize;

    if cl == 1 && col == 0 {
        // nothing to delete
        return;
    }

    if col > 0 {
        let line_idx = (cl - 1) as usize;
        if let Some(line) = s.text_lines.get_mut(line_idx) {
            let start = char_to_byte_index(line, col - 1);
            let end = char_to_byte_index(line, col);
            line.replace_range(start..end, "");
        }
        s.cursor_column = Some((col - 1) as u32);
    } else {
        // at start of line, merge with previous line
        if cl > 1 {
            let cur_idx = (cl - 1) as usize;
            let prev_idx = cur_idx - 1;
            if prev_idx < s.text_lines.len() && cur_idx < s.text_lines.len() {
                let suffix = s.text_lines.remove(cur_idx);
                let prev_len = s.text_lines[prev_idx].chars().count();
                s.text_lines[prev_idx].push_str(&suffix);
                cl -= 1;
                s.cursor_line = Some(cl);
                s.cursor_column = Some(prev_len as u32);
            }
        }
    }
    s.viewport_total_lines = s.text_lines.len() as u32;
    clamp_top_line(&mut s);
}

/// Move the cursor by a signed delta in lines and columns. This is a small
/// helper to support arrow-key movements in early wiring.
pub fn move_cursor(d_line: i32, d_col: i32) {
    let lock = scene_lock();
    let mut s = lock.write().unwrap();
    if s.text_lines.is_empty() {
        s.text_lines.push(String::new());
    }
    let mut cl = s.cursor_line.unwrap_or(1) as i32;
    let mut col = s.cursor_column.unwrap_or(0) as i32;

    cl = (cl + d_line).max(1).min(s.text_lines.len() as i32);
    let line_idx = (cl - 1) as usize;
    let line_len = s.text_lines.get(line_idx).map(|l| l.chars().count() as i32).unwrap_or(0);
    col = (col + d_col).max(0).min(line_len);

    s.cursor_line = Some(cl as u32);
    s.cursor_column = Some(col as u32);

    clamp_top_line(&mut s);
}

/// Scroll viewport by signed number of lines. Positive scrolls down.
pub fn scroll_by_lines(delta: i32) {
    let lock = scene_lock();
    let mut s = lock.write().unwrap();
    let total = s.text_lines.len() as i32;
    if total == 0 {
        s.viewport_top_line = 1;
        return;
    }
    let mut top = s.viewport_top_line as i32;
    top = (top + delta).max(1).min(total);
    s.viewport_top_line = top as u32;
    clamp_top_line(&mut s);
}

/// Map a window-space click (x,y) into a document cursor (1-based line, 0-based column).
///
/// - click_x/click_y: absolute window-space click coordinates (pixels).
/// - content_x/base_y: absolute window-space origin of the editor content area
///   (content_x refers to the left edge of the content rect).
/// - top_line: document 1-based top-of-viewport line.
/// - char_w/line_h: monospace character width & line height in pixels.
/// - content_inset: content inset (pixels) applied by presenters (default 6).
///
/// Returns (line, column) where `line` is 1-based and `column` is 0-based.
pub fn map_click_to_cursor(
    click_x: u32,
    click_y: u32,
    content_x: u32,
    base_y: u32,
    top_line: u32,
    char_w: u32,
    line_h: u32,
    content_inset: u32,
) -> (u32, u32) {
    // compute the content text origin (presenter applies a small inset)
    let content_text_x = content_x.saturating_add(content_inset);

    // row offset within the visible slice (0-based)
    let row_offset = if click_y < base_y {
        0u32
    } else if line_h == 0 {
        0u32
    } else {
        (click_y.saturating_sub(base_y)) / line_h
    };

    let line = top_line.saturating_add(row_offset).max(1);

    let col = if click_x < content_text_x || char_w == 0 {
        0u32
    } else {
        (click_x.saturating_sub(content_text_x)) / char_w
    };

    (line, col)
}

/// Place the global scene cursor based on a window-space click. The function
/// updates the engine-owned ShellSceneModel (protected by RwLock) so callers
/// can simply invoke this from input handlers.
///
/// Parameters mirror those of `map_click_to_cursor` except `top_line` is taken
/// from the current scene so callers need not provide it.
pub fn place_cursor_from_click(
    click_x: u32,
    click_y: u32,
    content_x: u32,
    base_y: u32,
    char_w: u32,
    line_h: u32,
    content_inset: u32,
) {
    let lock = scene_lock();
    let mut s = lock.write().unwrap();

    // derive top_line from live scene state
    let top_line = s.viewport_top_line.max(1);

    let (line, col) = map_click_to_cursor(
        click_x,
        click_y,
        content_x,
        base_y,
        top_line,
        char_w,
        line_h,
        content_inset,
    );

    s.cursor_line = Some(line);
    s.cursor_column = Some(col);
    // enforce invariants after mutation
    normalize_cursor(&mut s);
    clamp_top_line(&mut s);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_to_cursor_basic() {
        // content_x = 100, base_y = 50, content_inset = 6, char_w = 8, line_h = 16
        // click at x=106 => relative to content_text_x(106) => column 0
        // click at x=130 => column = (130 - 106)/8 = 3
        // click at y=66 => row_offset = (66 - 50)/16 = 1 => line = top_line + 1
        let (line, col) = map_click_to_cursor(130, 66, 100, 50, 1, 8, 16, 6);
        assert_eq!((line, col), (2, 3));
    }

    #[test]
    fn place_cursor_updates_scene() {
        // prepare a small scene with two lines
        let model = ShellSceneModel {
            text_lines: vec!["alpha".to_string(), "beta".to_string()],
            viewport_top_line: 1,
            viewport_total_lines: 2,
            viewport_summary: None,
            cursor_line: Some(1),
            cursor_column: Some(0),
            selection_present: false,
            status_text: None,
            chrome_text: None,
            last_command: None,
            ai_status_present: false,
        };
        set_current_scene(model);

        // perform click that should map to line 2, column 3
        place_cursor_from_click(130, 66, 100, 50, 8, 16, 6);

        let s = get_current_scene();
        assert_eq!(s.cursor_line.unwrap(), 2);
        assert_eq!(s.cursor_column.unwrap(), 3);
    }
}
