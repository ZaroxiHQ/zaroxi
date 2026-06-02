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

    /// Engine-owned AI assistant panel content model.
    ///
    /// Returns a `ContentView` preset for the AI panel: a panel title,
    /// a short status line, plus body/snippet/action text lines.
    /// Desktop layers structural chrome (cards, buttons) on top; the
    /// content model owns the text.
    pub fn ai_panel() -> Self {
        Self {
            title: "Assistant".into(),
            subtitle: "Ready".into(),
            lines: vec![
                "Here are the changes needed to refactor the module:".into(),
                "Extract validation logic".into(),
                "Add error handling".into(),
                "Update tests".into(),
                "fn validate(input: &str) -> Result<()> {".into(),
                "Accept".into(),
                "Reject".into(),
                "Edit".into(),
                "Ask anything...".into(),
                "Claude 3.5 Sonnet".into(),
                "Send".into(),
            ],
        }
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
