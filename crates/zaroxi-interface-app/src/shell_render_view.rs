/*!
ShellRenderViewModel — tiny UI‑facing, non‑visual view model.

Architectural rationale (short):
- This tiny view model is a UI-facing semantic seam: it carries ordering,
  section identifiers, presence markers, and simple textual content derived
  from a ShellDrawPlan without introducing any geometry, colors, fonts,
  layout, or graphics API bindings.
- The conversion from ShellDrawPlan is implemented via the existing
  ShellRenderTranscript (a stable, deterministic, debug-backed textual
  representation). This preserves the "debug-only" contract of the existing
  render_debug_text adapter while providing a minimal, stable UI-facing
  model that can be extended in future phases.

Public API:
- ShellRenderViewModel: top-level view model carrying ordered SectionView items.
- SectionView: tiny descriptor for a single logical section.

Conversion:
- impl From<&ShellRenderTranscript> for ShellRenderViewModel
- impl From<&zaroxi_core_engine_render::ShellDrawPlan> for ShellRenderViewModel
  (delegates to ShellRenderTranscript::from(plan))

This module intentionally keeps the model semantic and tiny.
*/

use zaroxi_core_engine_render::{ShellDrawPlan, ShellRenderTranscript};

/// Semantic, non-visual representation of a logical section produced by the renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionView {
    /// Stable section identifier (semantic). For this phase we use "debug" when we
    /// only have transcript text; future phases can map real section ids here.
    pub id: String,

    /// Presence marker: true when the section carries any textual content.
    pub present: bool,

    /// Simple textual content or markers for the section. Intentionally not a
    /// layout/geometry structure — only ordered lines of text.
    pub lines: Vec<String>,
}

/// Tiny UI-facing view model produced from a ShellDrawPlan (via the transcript).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRenderViewModel {
    /// Ordered semantic sections. Ordering is preserved from the underlying
    /// transcript; in this phase the transcript is treated as a single "debug"
    /// section to keep the model intentionally minimal.
    pub sections: Vec<SectionView>,
}

impl From<&ShellRenderTranscript> for ShellRenderViewModel {
    fn from(t: &ShellRenderTranscript) -> Self {
        let present = !t.lines.is_empty();
        let section = SectionView {
            id: "debug".to_string(),
            present,
            lines: t.lines.clone(),
        };
        ShellRenderViewModel {
            sections: vec![section],
        }
    }
}

impl From<&ShellDrawPlan> for ShellRenderViewModel {
    fn from(plan: &ShellDrawPlan) -> Self {
        // Reuse the existing, stable debug-backed transcript conversion. This
        // preserves the "debug-only" contract while offering a tiny semantic
        // model for UI code to consume in later phases.
        let transcript = ShellRenderTranscript::from(plan);
        ShellRenderViewModel::from(&transcript)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_render_view_from_transcript_is_stable() {
        // Build a deterministic transcript (this mirrors what ShellRenderTranscript
        // produces from a ShellDrawPlan). We avoid constructing ShellDrawPlan here
        // to keep the test narrow and focused on the view-model behavior.
        let transcript = ShellRenderTranscript {
            lines: vec![
                "ShellDrawPlan:".to_string(),
                "  RenderSection { id: \"main\", ... }".to_string(),
            ],
        };

        let vm = ShellRenderViewModel::from(&transcript);

        // Model-level assertions: ordering preserved, presence detected, text preserved.
        assert_eq!(vm.sections.len(), 1);
        let s = &vm.sections[0];
        assert_eq!(s.id, "debug");
        assert!(s.present);
        assert_eq!(s.lines, transcript.lines);
    }
}
