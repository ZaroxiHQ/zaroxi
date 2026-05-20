/// Small text label primitive intended for engine-facing seams.
///
/// This type is deliberately minimal: a small wrapper around a String so the
/// engine-side code can evolve richer label handling later (styling,
/// elision, accessibility metadata) without touching the presenter code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextLabel {
    pub text: String,
}

impl From<String> for TextLabel {
    fn from(s: String) -> Self {
        Self { text: s }
    }
}

impl From<&str> for TextLabel {
    fn from(s: &str) -> Self {
        Self { text: s.to_string() }
    }
}
