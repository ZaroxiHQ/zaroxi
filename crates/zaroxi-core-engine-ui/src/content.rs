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
}

impl ContentView {
    pub fn new(title: impl Into<String>, subtitle: impl Into<String>, lines: Vec<String>) -> Self {
        Self { title: title.into(), subtitle: subtitle.into(), lines }
    }
}

impl Default for ContentView {
    fn default() -> Self {
        Self {
            title: "untitled".into(),
            subtitle: String::new(),
            lines: vec!["fn main() {".into(), "    println!(\"hello\");".into(), "}".into()],
        }
    }
}
