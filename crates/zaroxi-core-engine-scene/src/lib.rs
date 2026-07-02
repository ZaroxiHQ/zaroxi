#![allow(dead_code)]
// Tiny semantic scene-description model for Phase 50.
// See ARCHITECTURE.md for rationale and details.
//
// Phase 5 additions: a minimal TextPrimitive used by engine-text to describe
// editor text runs. This type is intentionally small and stable so presenter
// code can map it into GPU paint operations.

pub const CRATE_NAME: &str = "zaroxi-core-engine-scene";

pub mod scene;
pub use scene::{
    CaretItem,
    SelectionRect,
    ShellSceneModel,
    backspace,
    // Phase 4 runtime seam: expose simple getters/setters and input helpers so
    // renderers/harnesses can publish & mutate the current ShellSceneModel.
    get_current_scene,
    insert_char,
    map_click_to_cursor,
    move_cursor,
    // Click-to-cursor helpers published at crate root so interface presenters
    // and render backends can easily invoke them without importing internal
    // `scene` module paths.
    place_cursor_from_click,
    scroll_by_lines,
    set_current_scene,
};

pub mod text_span;
pub use text_span::{SpanKind, SyntaxSpan, TextSpan};
// NOTE:
// EditorPrimitiveSet is defined in this crate root (below) and is NOT provided
// by the `scene` module. Attempting to import it from `scene` caused the
// unresolved import error. Keep the module imports aligned with actual
// definitions to avoid compilation failures.

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
        format!(
            "text@({},{}): \"{}\" font={} max_w={:?}",
            self.x, self.y, self.text, self.font_name, self.max_width
        )
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

impl Default for EditorPrimitiveSet {
    fn default() -> Self {
        Self::new()
    }
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

/// Provide a small, explicit PartialEq implementation for TextPrimitive and
/// EditorPrimitiveSet to allow callers to cheaply detect identity-equivalence
/// of scene outputs without performing a deep, semantic re-layout. This is a
/// conservative optimization: equality compares text runs and gutter labels
/// contents exactly and compares the lengths of caret/selection lists (not their
/// full geometry), keeping the comparison robust and easy to reason about.
impl PartialEq for TextPrimitive {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x
            && self.y == other.y
            && self.text == other.text
            && self.font_name == other.font_name
            && self.max_width == other.max_width
    }
}

impl PartialEq for EditorPrimitiveSet {
    fn eq(&self, other: &Self) -> bool {
        if self.texts.len() != other.texts.len()
            || self.carets.len() != other.carets.len()
            || self.selections.len() != other.selections.len()
            || self.gutter_labels.len() != other.gutter_labels.len()
        {
            return false;
        }

        for (a, b) in self.texts.iter().zip(other.texts.iter()) {
            if a != b {
                return false;
            }
        }

        // Compare gutter labels by content.
        for (a, b) in self.gutter_labels.iter().zip(other.gutter_labels.iter()) {
            if a != b {
                return false;
            }
        }

        // For carets and selections we conservatively compare lengths only.
        true
    }
}

pub fn info() -> &'static str {
    CRATE_NAME
}

// ------- UI-widget scene primitives (moved from zaroxi-core-engine-ui) -------

/// A filled rectangle primitive — the most basic draw command.
///
/// Uses f32 coordinates and an RGBA color array for compatibility with
/// float-based layout systems (taffy) and custom renderers.
#[derive(Clone, Debug)]
pub struct RectPrimitive {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4],
}

impl RectPrimitive {
    pub fn new(x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> Self {
        Self { x, y, width, height, color }
    }
}

/// A positioned text label — the text analogue of `RectPrimitive`.
///
/// Carries the label string, an anchor position, layout bounds,
/// and a caller-supplied color. No fonts, app names, or rendering
/// specifics are baked in.
#[derive(Clone, Debug)]
pub struct LabelPrimitive {
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub max_width: f32,
    pub max_height: f32,
    pub color: [f32; 4],
}

impl LabelPrimitive {
    pub fn new(
        label: impl Into<String>,
        x: f32,
        y: f32,
        max_width: f32,
        max_height: f32,
        color: [f32; 4],
    ) -> Self {
        Self { label: label.into(), x, y, max_width, max_height, color }
    }
}

/// A grouped scene output from widget composition.
///
/// Bundles rectangle and label primitives into a single coherent result.
/// Callers receive one `WidgetScene` instead of managing separate primitive
/// vectors. Intentionally generic — no app concepts, no theme ownership.
#[derive(Clone, Debug, Default)]
pub struct WidgetScene {
    pub rects: Vec<RectPrimitive>,
    pub labels: Vec<LabelPrimitive>,
}

impl WidgetScene {
    pub fn new(rects: Vec<RectPrimitive>, labels: Vec<LabelPrimitive>) -> Self {
        Self { rects, labels }
    }

    pub fn is_empty(&self) -> bool {
        self.rects.is_empty() && self.labels.is_empty()
    }
}
