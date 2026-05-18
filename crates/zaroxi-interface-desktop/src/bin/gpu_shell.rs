/*!
Minimal GPU shell entry point for the desktop harness.

This binary creates a winit window and uses the existing GpuShellPresenter
to paint three simple regions (chrome, content, status) into a Pixels
RGBA8 framebuffer. The implementation is intentionally thin and UI-only.

Enable by building/running with the feature flag:
  cargo run -p zaroxi-interface-desktop --bin gpu_shell --features gpu_shell_bin

The feature gate prevents this binary from being compiled during normal test
builds (avoiding winit/pixels platform-specific concerns). The presenter logic
remains in the crate and is fully testable without the native window.
*/

use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use zaroxi_interface_desktop::presenters::GpuShellPresenter;

fn main() {
    // Basic window setup - EventLoop::new may return a Result on some platforms,
    // so unwrap/expect to obtain the EventLoop value needed to call .run().
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let window = WindowBuilder::new()
        .with_title("Zaroxi - GPU Shell (minimal)")
        .with_inner_size(LogicalSize::new(960.0, 640.0))
        .build(&event_loop)
        .expect("failed to create window");

    let mut size = window.inner_size();
    let mut pixels = {
        let surface_texture = SurfaceTexture::new(size.width, size.height, &window);
        Pixels::new(size.width, size.height, surface_texture).expect("failed to create pixels")
    };

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
                size = new_size;
                // Resize both surface and internal buffer
                let _ = pixels.resize_surface(size.width, size.height);
                let _ = pixels.resize_buffer(size.width, size.height);
                window.request_redraw();
            }

            Event::RedrawRequested(_) => {
                // Paint into the pixel frame using the existing presenter
                let frame = pixels.frame_mut();
                let regions = GpuShellPresenter::map_regions(size.width, size.height, chrome_h, status_h);
                GpuShellPresenter::paint_to_buffer(size.width, size.height, frame, &regions);

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
