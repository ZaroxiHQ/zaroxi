#![allow(dead_code)]
// Tiny semantic scene-description model for Phase 50.
// See ARCHITECTURE.md for rationale and details.
//
// Phase 5 additions: a minimal TextPrimitive used by engine-text to describe
// editor text runs. This type is intentionally small and stable so presenter
// code can map it into GPU paint operations.

pub const CRATE_NAME: &str = "zaroxi-core-engine-scene";

pub mod scene;
pub use scene::{ShellSceneModel, CaretItem, SelectionRect, EditorPrimitiveSet};

/// Primitive describing a single laid-out text run for the scene.
///
/// - x,y are absolute window-space coordinates (top-left of the run baseline/anchor).
/// - text is the raw UTF-8 content for this run (no shaping metadata included).
/// - font_name is an informational identifier (presenter/renderer chooses actual font).
/// - max_width is an optional clamp hint the presenter/renderer should respect.
#[derive(Clone, Debug)]
pub struct TextPrimitive {
    pub x: u32,
    pub y: u32,
    pub text: String,
    pub font_name: String,
    pub max_width: Option<u32>,
}

impl TextPrimitive {
    pub fn to_debug_line(&self) -> String {
        format!("text@({},{}): \"{}\" font={} max_w={:?}", self.x, self.y, self.text, self.font_name, self.max_width)
    }
}

// Editor primitives bundle exported for renderer backends.
//
// This small, stable bundle groups the minimal set of editor-facing primitives
// that renderers/backends need to draw the visible editor surface:
// - texts: text runs (monospace, position is top-left of run)
// - carets: thin vertical caret items
// - selections: highlighted selection rects
// - gutter_labels: textual gutter labels (line numbers) represented as text runs
#[derive(Clone, Debug)]
pub struct EditorPrimitiveSet {
    pub texts: Vec<TextPrimitive>,
    pub carets: Vec<CaretItem>,
    pub selections: Vec<SelectionRect>,
    pub gutter_labels: Vec<TextPrimitive>,
}

impl EditorPrimitiveSet {
    pub fn new() -> Self {
        EditorPrimitiveSet {
            texts: Vec::new(),
            carets: Vec::new(),
            selections: Vec::new(),
            gutter_labels: Vec::new(),
        }
    }
}

pub fn info() -> &'static str {
    CRATE_NAME
}
