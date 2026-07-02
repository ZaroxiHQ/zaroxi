/*!
render_debug_text — tiny debug adapter exposing ShellDrawPlan -> String.

Architectural rationale (short):
- Provide a single, minimal, non-visual adapter that turns a ShellDrawPlan
  into a deterministic textual representation for inspection from the
  desktop/harness side.
- Reuse the existing backend `ShellTextRenderer` and `ShellRenderTranscript`
  so we do not introduce rendering behavior or graphics dependencies here.
- Keep the function tiny and stable: it returns a String suitable for logs.

Usage:
- Call `zaroxi_interface_desktop::render_debug_text::render_debug_text(&plan)`
  and log or assert on the returned String from the harness or tests.
*/

use zaroxi_core_engine_render::{ShellDrawPlan, ShellTextRenderer};

/// Render a ShellDrawPlan into a deterministic String suitable for logging.
///
/// The returned string is prefixed with "render debug text:" to make harness
/// log scans and assertions stable and obvious.
pub fn render_debug_text(plan: &ShellDrawPlan) -> String {
    let renderer = ShellTextRenderer::new();
    let transcript = renderer.render(plan);
    format!("render debug text:\n{}", transcript)
}
