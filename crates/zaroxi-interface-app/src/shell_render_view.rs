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
        // Attempt to derive semantic sections from the deterministic transcript.
        // We look for pretty-printed RenderSection entries and map each to a
        // SectionView preserving id, presence and ordered textual lines.
        //
        // Fallback: when no RenderSection entries are detected we preserve the
        // original single "debug" section behaviour to remain compatible.
        let mut sections: Vec<SectionView> = Vec::new();
        let mut idx: usize = 0;

        while idx < t.lines.len() {
            let line = &t.lines[idx];
            // Identify a RenderSection header line that contains an id field.
            if line.contains("RenderSection") && line.contains("id:") {
                // Extract id value between the first pair of quotes after `id: `
                let id = if let Some(start_pos) = line.find("id: \"") {
                    let start = start_pos + 5;
                    if let Some(rel_end) = line[start..].find('"') {
                        line[start..start + rel_end].to_string()
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    "unknown".to_string()
                };

                // Collect the header and any following lines belonging to this section.
                let mut sect_lines: Vec<String> = Vec::new();
                sect_lines.push(line.trim_start().to_string());
                idx += 1;
                while idx < t.lines.len() && !(t.lines[idx].contains("RenderSection") && t.lines[idx].contains("id:")) {
                    sect_lines.push(t.lines[idx].trim_start().to_string());
                    idx += 1;
                }

                let present = sect_lines.iter().any(|l| !l.trim().is_empty());
                sections.push(SectionView { id, present, lines: sect_lines });
                continue;
            }

            idx += 1;
        }

        if sections.is_empty() {
            // No semantic sections found: fall back to legacy debug behaviour.
            let present = !t.lines.is_empty();
            let section = SectionView {
                id: "debug".to_string(),
                present,
                lines: t.lines.clone(),
            };
            ShellRenderViewModel {
                sections: vec![section],
            }
        } else {
            ShellRenderViewModel { sections }
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

        // Model-level assertions: semantic section detected, ordering preserved,
        // presence detected, and section text captured deterministically.
        assert_eq!(vm.sections.len(), 1);
        let s = &vm.sections[0];
        assert_eq!(s.id, "main");
        assert!(s.present);
        assert_eq!(s.lines, vec![transcript.lines[1].trim_start().to_string()]);
    }
}
