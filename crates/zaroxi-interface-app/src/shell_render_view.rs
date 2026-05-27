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
    /// Stable section identifier (semantic).
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
    /// transcript.
    pub sections: Vec<SectionView>,
}

impl ShellRenderViewModel {
    /// Try to extract an id value from a transcript line using a few tolerant
    /// patterns. Returns Some(id) when found.
    fn extract_id_from_line(line: &str) -> Option<String> {
        // Common pretty-printed Debug form: id: "name"
        if let Some(start_pos) = line.find("id: \"") {
            let start = start_pos + 5;
            if let Some(rel_end) = line[start..].find('"') {
                return Some(line[start..start + rel_end].to_string());
            }
        }
        // Accept single-quoted variant: id: 'name'
        if let Some(start_pos) = line.find("id: '") {
            let start = start_pos + 5;
            if let Some(rel_end) = line[start..].find('\'') {
                return Some(line[start..start + rel_end].to_string());
            }
        }
        None
    }
}

impl From<&ShellRenderTranscript> for ShellRenderViewModel {
    fn from(t: &ShellRenderTranscript) -> Self {
        // Attempt to derive semantic sections from the deterministic transcript.
        // We look for pretty-printed RenderSection entries (or any line that
        // carries an `id: "..."`) and map each to a SectionView preserving id,
        // presence and ordered textual lines. This favors fidelity to the
        // ShellDrawPlan's explicit section ids (e.g. "content", "status", etc.).
        let mut sections: Vec<SectionView> = Vec::new();
        let mut idx: usize = 0;

        while idx < t.lines.len() {
            let line = &t.lines[idx];

            // Detect a RenderSection header or any line carrying an id field.
            if line.contains("RenderSection") || line.contains("id:") {
                if let Some(id) = ShellRenderViewModel::extract_id_from_line(line) {
                    // Collect the header and any following lines belonging to this section.
                    let mut sect_lines: Vec<String> = Vec::new();
                    sect_lines.push(line.trim_start().to_string());
                    idx += 1;
                    while idx < t.lines.len()
                        && !t.lines[idx].contains("RenderSection")
                        && ShellRenderViewModel::extract_id_from_line(&t.lines[idx]).is_none()
                    {
                        sect_lines.push(t.lines[idx].trim_start().to_string());
                        idx += 1;
                    }

                    let present = sect_lines.iter().any(|l| !l.trim().is_empty());
                    sections.push(SectionView { id, present, lines: sect_lines });
                    continue;
                }
            }

            idx += 1;
        }

        if sections.is_empty() {
            // No semantic sections found: fall back to a single "content" section
            // that carries the whole transcript. Prefer "content" over the older
            // generic "debug" id so harness output reads more like the real shell.
            let present = !t.lines.is_empty();
            let section =
                SectionView { id: "content".to_string(), present, lines: t.lines.clone() };
            ShellRenderViewModel { sections: vec![section] }
        } else {
            ShellRenderViewModel { sections }
        }
    }
}

impl From<&ShellDrawPlan> for ShellRenderViewModel {
    fn from(plan: &ShellDrawPlan) -> Self {
        // Reuse the existing, stable debug-backed transcript conversion. This
        // preserves the deterministic textual representation while offering a
        // tiny semantic model for UI code to consume in later phases.
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

    #[test]
    fn detects_multiple_sections_and_preserves_ordering() {
        let transcript = ShellRenderTranscript {
            lines: vec![
                "ShellDrawPlan:".to_string(),
                "  RenderSection { id: \"content\", ... }".to_string(),
                "    content-line-1".to_string(),
                "    content-line-2".to_string(),
                "  RenderSection { id: \"status\", ... }".to_string(),
                "    Ready".to_string(),
                "  RenderSection { id: \"ai_indicator\", ... }".to_string(),
                "    AI: suggested".to_string(),
            ],
        };

        let vm = ShellRenderViewModel::from(&transcript);

        assert_eq!(vm.sections.len(), 3);
        assert_eq!(vm.sections[0].id, "content");
        assert_eq!(vm.sections[1].id, "status");
        assert_eq!(vm.sections[2].id, "ai_indicator");

        assert!(vm.sections[0].present);
        assert!(vm.sections[1].present);
        assert!(vm.sections[2].present);

        // Verify the lines for the first section were captured in order.
        assert_eq!(
            vm.sections[0].lines,
            vec![
                transcript.lines[1].trim_start().to_string(),
                transcript.lines[2].trim_start().to_string(),
                transcript.lines[3].trim_start().to_string(),
            ]
        );
    }
}
