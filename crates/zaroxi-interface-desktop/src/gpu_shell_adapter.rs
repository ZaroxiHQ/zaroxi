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
use zaroxi_interface_desktop::presenters::gpu_shell::{GpuShellPresenter, ShellRegions};

/// Convert a ShellRenderViewModel into presenter ShellRegions for a window of
/// size (width x height).
///
/// Note: the function intentionally accepts the real view model (to prove
/// wiring) but currently uses conservative defaults for chrome/status heights.
#[allow(dead_code)]
pub fn view_model_to_regions(_model: &ShellRenderViewModel, width: u32, height: u32) -> ShellRegions {
    // Default wireframe metrics (kept small and stable).
    let chrome_h: u32 = 60;
    let status_h: u32 = 24;

    GpuShellPresenter::map_regions(width, height, chrome_h, status_h)
}

/// Runtime-friendly helper that does not require a constructed ShellRenderViewModel.
/// This is useful for the binary runtime path where constructing the full model
/// may be handled by the composition pipeline in a later phase. For now this
/// preserves the wiring surface and keeps the runtime loop tiny.
pub fn view_model_to_regions_from_scratch(width: u32, height: u32) -> ShellRegions {
    // Default wireframe metrics (kept small and stable).
    let chrome_h: u32 = 60;
    let status_h: u32 = 24;

    GpuShellPresenter::map_regions(width, height, chrome_h, status_h)
}
