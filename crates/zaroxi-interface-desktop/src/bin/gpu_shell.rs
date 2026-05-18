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
    use std::time::{Duration, Instant};

    use winit::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        dpi::PhysicalSize,
    };
    use pixels::{Pixels, SurfaceTexture};

    use zaroxi_interface_desktop::presenters::gpu_shell::GpuShellPresenter;

    // Initial window size.
    let initial_width: u32 = 800;
    let initial_height: u32 = 600;

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = winit::window::WindowBuilder::new()
        .with_title("Zaroxi - GPU Shell (minimal)")
        .with_inner_size(PhysicalSize::new(initial_width, initial_height))
        .build(&event_loop)
        .expect("Failed to create window");

    let mut physical_size = window.inner_size();
    let mut width = physical_size.width.max(1);
    let mut height = physical_size.height.max(1);

    let surface_texture = SurfaceTexture::new(width, height, &window);
    let mut pixels = Pixels::new(width, height, surface_texture)
        .expect("Failed to create pixel buffer");

    // Use a simple fixed chrome/status size for mapping; presenter handles clamping.
    let chrome_height = 60u32;
    let status_height = 24u32;

    // Simple frame rate limiter so the window is not a tight busy loop.
    let frame_duration = Duration::from_millis(16); // ~60fps
    let mut last_frame = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        // Default to waiting for events to reduce CPU usage.
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    // Exit in a robust way that avoids depending on specific
                    // ControlFlow variant names across winit releases.
                    std::process::exit(0);
                }
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    // Re-query the window size to avoid depending on specific
                    // variant field shapes across winit versions.
                    let size = window.inner_size();
                    width = size.width.max(1);
                    height = size.height.max(1);
                    let _ = pixels.resize_surface(width, height);
                    let _ = pixels.resize_buffer(width, height);
                }
                _ => {}
            },
            Event::RedrawRequested(_) => {
                // Basic framerate cap
                if last_frame.elapsed() < frame_duration {
                    // Skip painting if too soon; request another redraw later.
                    window.request_redraw();
                    return;
                }
                last_frame = Instant::now();

                let frame = pixels.frame_mut();
                // Map regions using the presenter and paint into the provided frame buffer.
                let regions = GpuShellPresenter::map_regions(width, height, chrome_height, status_height);
                GpuShellPresenter::paint_to_buffer(width, height, frame, &regions);

                if pixels.render().is_err() {
                    // Fail fast in a way that is version-agnostic.
                    std::process::exit(1);
                }
            }
            Event::MainEventsCleared => {
                // Request a redraw to run our simple render loop.
                window.request_redraw();
            }
            _ => {}
        }
    });
}
