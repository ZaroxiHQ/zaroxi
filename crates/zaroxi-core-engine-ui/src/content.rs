/// Engine-owned content model for a scrollable document/text panel.
///
/// `ContentView` holds the structural content of a panel — title, subtitle,
/// and visible lines of text — without owning any geometry, colors, or
/// rendering. The `composer` module handles layout and conversion to
/// `WidgetScene` primitives.
///
/// Intentionally generic: no app names, no editor concepts baked in.
/// Desktop adapts application state into a `ContentView` instance.
#[derive(Clone, Debug)]
pub struct ContentView {
    pub title: String,
    pub subtitle: String,
    pub lines: Vec<String>,
    /// Cursor position (0-based line and column).
    pub cursor_line: usize,
    pub cursor_col: usize,
    /// Selection range as (start_line, start_col) to (end_line, end_col), if any.
    pub selection: Option<(usize, usize, usize, usize)>,
}

impl ContentView {
    pub fn new(title: impl Into<String>, subtitle: impl Into<String>, lines: Vec<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: subtitle.into(),
            lines,
            cursor_line: 0,
            cursor_col: 0,
            selection: None,
        }
    }

    pub fn with_cursor(mut self, line: usize, col: usize) -> Self {
        self.cursor_line = line;
        self.cursor_col = col;
        self
    }

    pub fn with_selection(mut self, sl: usize, sc: usize, el: usize, ec: usize) -> Self {
        self.selection = Some((sl, sc, el, ec));
        self
    }
}

impl Default for ContentView {
    fn default() -> Self {
        Self {
            title: "untitled".into(),
            subtitle: String::new(),
            lines: vec!["fn main() {".into(), "    println!(\"hello\");".into(), "}".into()],
            cursor_line: 0,
            cursor_col: 0,
            selection: None,
        }
    }
}
