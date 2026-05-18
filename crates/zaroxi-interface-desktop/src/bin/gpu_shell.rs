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
    // Minimal, robust native path using minifb instead of winit/pixels.
    // This avoids fragile winit API differences while still providing a
    // real native window and reusing the presenter's paint_to_buffer logic.
    use std::{thread::sleep, time::Duration};

    use minifb::{Key, Window, WindowOptions};

    use zaroxi_interface_desktop::presenters::gpu_shell::GpuShellPresenter;

    let width: u32 = 800;
    let height: u32 = 600;

    // RGBA8 backing buffer for the presenter (mutated in place).
    let mut rgba_buf = vec![0u8; (width as usize) * (height as usize) * 4];
    // minifb expects a u32 per pixel buffer (format: 0x00RRGGBB little-endian).
    let mut pixel_buf: Vec<u32> = vec![0u32; (width as usize) * (height as usize)];

    let mut window = Window::new(
        "Zaroxi - GPU Shell (minimal)",
        width as usize,
        height as usize,
        WindowOptions::default(),
    )
    .expect("Failed to create window");

    // Fixed chrome/status sizes (presenter will clamp).
    let chrome_height = 60u32;
    let status_height = 24u32;

    // Simple render loop.
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Ask the presenter for regions and paint into the RGBA buffer.
        let regions = GpuShellPresenter::map_regions(width, height, chrome_height, status_height);
        GpuShellPresenter::paint_to_buffer(width, height, &mut rgba_buf, &regions);

        // Convert RGBA8 -> 0x00RRGGBB u32 pixels (drop alpha).
        for i in 0..(width as usize * height as usize) {
            let base = i * 4;
            let r = rgba_buf[base] as u32;
            let g = rgba_buf[base + 1] as u32;
            let b = rgba_buf[base + 2] as u32;
            pixel_buf[i] = (r << 16) | (g << 8) | b;
        }

        // Update window; ignore occasional update errors for robustness.
        let _ = window.update_with_buffer(&pixel_buf, width as usize, height as usize);

        // Throttle to ~60fps.
        sleep(Duration::from_millis(16));
    }
}
