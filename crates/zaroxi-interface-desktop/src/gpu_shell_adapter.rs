/*!
Tiny adapter to convert the high-level ShellRenderViewModel (provided by
zaroxi-interface-app) into the presenter's ShellRegions.

This adapter intentionally keeps mapping decisions minimal and wireframe-like:
- It accepts a real ShellRenderViewModel so the GPU shell is driven by the same
  shell-facing model used by the harness and terminal shell.
- For this phase the adapter uses conservative default chrome/status heights
  and delegates region math to the pure GpuShellPresenter::map_regions function.
- Future phases may extract precise layout metrics from the model; keeping the
  adapter separate preserves separation of concerns (no presenter duplication).
*/

use zaroxi_interface_app::shell_render_view::ShellRenderViewModel;
use crate::presenters::gpu_shell::{GpuShellPresenter, ShellRegions};

use crate::events::{UiEvent, Key as UiKey};

/// Minimal native-key abstraction kept crate-local so translation logic is
/// testable without pulling in optional native deps (minifb).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeKey {
    Up,
    Down,
    Enter,
    Char(char),
}

/// Map a crate-local NativeKey into the existing UiEvent model.
/// This function keeps the translation surface small and reuses the canonical
/// UiEvent/Key types defined in the events module.
pub fn map_native_to_ui_event(k: NativeKey) -> Option<UiEvent> {
    match k {
        NativeKey::Up => Some(UiEvent::Key(UiKey::ArrowUp)),
        NativeKey::Down => Some(UiEvent::Key(UiKey::ArrowDown)),
        NativeKey::Enter => Some(UiEvent::Key(UiKey::Enter)),
        NativeKey::Char(c) => Some(UiEvent::Key(UiKey::Char(c))),
    }
}

/// Convert a ShellRenderViewModel into presenter ShellRegions for a window of
/// size (width x height).
///
/// Note: the function intentionally accepts the real view model (to prove
/// wiring) but currently uses conservative defaults for chrome/status heights.
#[allow(dead_code)]
pub fn view_model_to_regions(_model: &ShellRenderViewModel, width: u32, height: u32, active_buffer: Option<&str>) -> ShellRegions {
    // Default wireframe metrics (kept small and stable).
    let chrome_h: u32 = 60;
    let status_h: u32 = 24;

    let mut regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    regions.marker = active_buffer.map(|s| s.to_string());
    // Expose a tiny deterministic semantic payload derived from the active buffer.
    // - chrome_label: short visible label for chrome (active buffer)
    // - status_text: lightweight status derived from active buffer
    regions.chrome_label = active_buffer.map(|s| s.to_string());
    regions.status_text = active_buffer.map(|s| format!("status: {}", s));
    regions.content_preview = None;
    regions
}

/// Runtime-friendly helper that does not require a constructed ShellRenderViewModel.
/// This is useful for the binary runtime path where constructing the full model
/// may be handled by the composition pipeline in a later phase. For now this
/// preserves the wiring surface and keeps the runtime loop tiny.
pub fn view_model_to_regions_from_scratch(width: u32, height: u32, active_buffer: Option<&str>) -> ShellRegions {
    // Default wireframe metrics (kept small and stable).
    let chrome_h: u32 = 60;
    let status_h: u32 = 24;

    let mut regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    regions.marker = active_buffer.map(|s| s.to_string());
    // Expose a tiny deterministic semantic payload derived from the active buffer.
    regions.chrome_label = active_buffer.map(|s| s.to_string());
    regions.status_text = active_buffer.map(|s| format!("status: {}", s));
    regions.content_preview = None;
    regions
}
