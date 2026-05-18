/*!
Minimal GPU shell entry point for the desktop harness.

This binary creates a winit window and uses the existing GpuShellPresenter
to paint three simple regions (chrome, content, status) into a Pixels
RGBA8 framebuffer. The implementation is intentionally thin and UI-only.

Run with:
  cargo run -p zaroxi-desktop-harness --bin gpu_shell
(or build the package to include this binary)

This file stays inside `zaroxi-interface-desktop` and reuses the presenter
logic already defined in presenters/gpu_shell.rs.
*/

use std::error::Error;

use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use zaroxi_interface_desktop::presenters::GpuShellPresenter;

fn main() -> Result<(), Box<dyn Error>> {
    // Basic window setup
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Zaroxi - GPU Shell (minimal)")
        .with_inner_size(LogicalSize::new(960.0, 640.0))
        .build(&event_loop)?;

    let size = window.inner_size();
    let mut width = size.width;
    let mut height = size.height;

    // Create the pixels surface (RGBA8)
    let surface_texture = SurfaceTexture::new(width, height, &window);
    let mut pixels = Pixels::new(width, height, surface_texture)?;

    // Simple chrome/status heights
    let chrome_h = 60u32;
    let status_h = 24u32;

    // Run the event loop and request redraws continuously (simple demo)
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent { event: WindowEvent::Resized(new_size), .. } => {
                width = new_size.width;
                height = new_size.height;
                // Resize both surface and internal buffer
                let _ = pixels.resize_surface(width, height);
                let _ = pixels.resize_buffer(width, height);
                window.request_redraw();
            }

            Event::RedrawRequested(_) => {
                // Paint into the pixel frame using the existing presenter
                let frame = pixels.get_frame();
                let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
                GpuShellPresenter::paint_to_buffer(width, height, frame, &regions);

                // Render the frame to the window surface
                if let Err(err) = pixels.render() {
                    eprintln!("pixels.render error: {err}");
                    *control_flow = ControlFlow::Exit;
                }
            }

            Event::MainEventsCleared => {
                // Continuously render for the demo; harnesses can adopt smarter scheduling.
                window.request_redraw();
            }

            _ => {}
        }
    });
}
