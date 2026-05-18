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
    // Keep this function intentionally small. It wires:
    // - a minimal winit event loop and window,
    // - pixels framebuffer backed by the window,
    // - a single GpuShellPresenter painting into the pixels frame,
    // - a basic event/render loop (handles resize, close, and escape).
    use std::time::{Duration, Instant};

    use winit::{
        event::{Event, WindowEvent, KeyboardInput, VirtualKeyCode, ElementState},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    };
    use pixels::{Pixels, SurfaceTexture};

    // Reference the presenter from the crate (crate name maps hyphens -> underscores).
    use zaroxi_interface_desktop::presenters::gpu_shell::GpuShellPresenter;

    // Initial window size.
    let initial_width: u32 = 800;
    let initial_height: u32 = 600;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Zaroxi - GPU Shell (minimal)")
        .with_inner_size(winit::dpi::PhysicalSize::new(initial_width, initial_height))
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
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if let KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    } = input
                    {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                WindowEvent::Resized(new_size) => {
                    // Update dimensions and inform pixels of the new surface/buffer sizes.
                    width = new_size.width.max(1);
                    height = new_size.height.max(1);
                    let _ = pixels.resize_surface(width, height);
                    let _ = pixels.resize_buffer(width, height);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    width = new_inner_size.width.max(1);
                    height = new_inner_size.height.max(1);
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

                let frame = pixels.get_frame();
                // Map regions using the presenter and paint into the provided frame buffer.
                let regions = GpuShellPresenter::map_regions(width, height, chrome_height, status_height);
                GpuShellPresenter::paint_to_buffer(width, height, frame, &regions);

                if pixels.render().is_err() {
                    *control_flow = ControlFlow::Exit;
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
