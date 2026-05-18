/*!
Phase 53: Tiny semantic draw-plan adapter (ShellDrawPlan)

Architectural rationale (short):
- Adds a tiny, read-only adapter in `zaroxi-core-engine-render` that consumes
  `ShellRenderIntent` and produces a semantic, ordered draw plan.
- This is intentionally not a renderer: no GPU, no coordinates, no colors,
  no glyphs, no pipelines. It preserves only ordered section semantics and
  simple presence flags to allow downstream harnesses/tests to reason about
  "what should be drawn" at a very high level.
- Keeps dependency direction intact: render <- layout <- scene <- view <- interface.

Files added/modified:
- Modified: crates/zaroxi-core-engine-render/src/lib.rs (exports plan)
- Added:    crates/zaroxi-core-engine-render/src/plan.rs
- Added:    crates/zaroxi-core-engine-render/tests/plan_from_intent.rs

Public types and names:
- ShellDrawPlan (pub struct)
- DrawSection (pub enum)
- From<ShellRenderIntent> for ShellDrawPlan

Fields / sections preserved from ShellRenderIntent:
- Ordered sections converted into DrawSection variants:
  - Text -> DrawSection::Content
  - Selection -> DrawSection::Selection
  - Status -> DrawSection::Status
  - Chrome -> DrawSection::Chrome
- Convenience booleans:
  - selection_present
  - status_present
  - content_present
  - chrome_present

Test added:
- crates/zaroxi-core-engine-render/tests/plan_from_intent.rs
  - Verifies order preservation and presence flags when converting from
    a populated ShellLayoutInput -> ShellRenderIntent -> ShellDrawPlan.

Validation commands (to run from workspace root):
- cargo test -p zaroxi-core-engine-render
- cargo test -p zaroxi-core-engine-layout
- cargo test -p zaroxi-core-engine-scene
- cargo test -p zaroxi-core-engine-view
- cargo test -p zaroxi-interface-app
- cargo test -p zaroxi-interface-desktop
- cargo run -p zaroxi-desktop-harness
- bash scripts/architecture_check.sh

Notes:
- This adapter is intentionally minimal and semantic-only to keep Phase 53
  focused on "draw-plan" bookkeeping without introducing rendering logic.
*/

use crate::intent::{RenderSection, ShellRenderIntent};

/// High-level, ordered draw plan for a shell view.
///
/// Semantic-only: contains an ordered list of DrawSection and a few presence flags.
/// No geometry, metrics, colors, or rendering resources are present here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellDrawPlan {
    /// Ordered semantic sections derived from the render intent.
    pub sections: Vec<DrawSection>,

    /// Convenience flags for quick tests.
    pub selection_present: bool,
    pub status_present: bool,
    pub content_present: bool,
    pub chrome_present: bool,
}

/// Semantic section kinds in the draw plan (no payloads, only high-level kind).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawSection {
    Content,
    Selection,
    Status,
    Chrome,
}

impl From<ShellRenderIntent> for ShellDrawPlan {
    fn from(intent: ShellRenderIntent) -> Self {
        let mut sections: Vec<DrawSection> = Vec::new();

        for s in intent.sections.into_iter() {
            match s {
                RenderSection::Text { .. } => sections.push(DrawSection::Content),
                RenderSection::Selection { .. } => sections.push(DrawSection::Selection),
                RenderSection::Status { .. } => sections.push(DrawSection::Status),
                RenderSection::Chrome => sections.push(DrawSection::Chrome),
            }
        }

        let selection_present = sections.iter().any(|s| matches!(s, DrawSection::Selection));
        let status_present = sections.iter().any(|s| matches!(s, DrawSection::Status));
        let content_present = sections.iter().any(|s| matches!(s, DrawSection::Content));
        let chrome_present = sections.iter().any(|s| matches!(s, DrawSection::Chrome));

        ShellDrawPlan {
            sections,
            selection_present,
            status_present,
            content_present,
            chrome_present,
        }
    }
}

/// Provide a deterministic, minimal default ShellDrawPlan for tests and
/// simple consumers that need an empty plan instance.
///
/// Default is intentionally empty and deterministic (no sections, all flags false).
impl Default for ShellDrawPlan {
    fn default() -> Self {
        ShellDrawPlan {
            sections: Vec::new(),
            selection_present: false,
            status_present: false,
            content_present: false,
            chrome_present: false,
        }
    }
}
