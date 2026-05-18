/*!
Feature-gated native GPU shell bootstrap.

This binary provides two builds:
- Non-feature (default): prints a compatibility message and exits.
- With feature "gpu_shell_bin": runs a minimal native window using winit + pixels,
  reusing the existing GpuShellPresenter for region mapping and painting.

Design constraints:
- Tiny, minimal event loop with one window and one presenter.
- All native deps are optional and enabled only when the feature is requested.
- No changes to presenter implementation; we reuse paint_to_buffer.
*/

#[cfg(not(feature = "gpu_shell_bin"))]
fn main() {
    eprintln!("gpu_shell: native GPU shell is not started in this build.");
    eprintln!("If you intended to run the native windowed demo, enable the feature:");
    eprintln!("  cargo run -p zaroxi-interface-desktop --bin gpu_shell --features=\"gpu_shell_bin\"");
}

#[cfg(feature = "gpu_shell_bin")]
fn main() {
    // Delegate the native run to the presenter implementation which is
    // feature-gated itself. Keeping the binary tiny avoids cross-version
    // duplication of event-loop code here.
    zaroxi_interface_desktop::presenters::gpu_shell::GpuShellPresenter::run_native(800, 600);
}
