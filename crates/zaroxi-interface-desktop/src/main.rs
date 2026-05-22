/*!
Desktop entrypoint for Phase 2 bootstrap:
- create the winit event loop and window
- initialize the render backend (wgpu)
- on redraw, build a trivial vello::Scene and ask backend to render it

This binary supports a runtime feature `gpu_shell_bin`. When the feature is
enabled, the real GUI binary is compiled and run. When disabled, a small
no-op stub main is provided so CI/test builds do not pull GUI deps.
*/

#[cfg(feature = "gpu_shell_bin")]
use std::time::{Duration, Instant};

#[cfg(feature = "gpu_shell_bin")]
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

#[cfg(feature = "gpu_shell_bin")]
use vello;
#[cfg(feature = "gpu_shell_bin")]
use zaroxi_core_engine_render_backend::RenderBackend;
#[cfg(feature = "gpu_shell_bin")]
use zaroxi_core_engine_window::ZaroxiWindow;

/// No-op stub main when the gpu_shell_bin feature is not enabled.
/// Keeps the crate usable as a library during CI/test builds.
#[cfg(not(feature = "gpu_shell_bin"))]
fn main() {
    // Intentionally empty when GUI binary is disabled.
}

/// Full GUI binary entrypoint when feature is enabled.
#[cfg(feature = "gpu_shell_bin")]
#[tokio::main]
async fn main() {
    // Create the event loop and window
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut zwin = ZaroxiWindow::new(&event_loop);

    // Initialize the render backend
    let mut backend: RenderBackend<'_> = RenderBackend::new(&zwin).await;

    // Request an initial redraw to start rendering
    zwin.window().request_redraw();

    // Basic frame pacing: continuously request redraw on MainEventsCleared to drive 60fps-ish.
    // This keeps the simple bootstrap rendering active without a complex scheduler.
    let mut last_frame = Instant::now();
    let frame_duration = Duration::from_micros(16_666); // ~60 FPS target

    // event_loop.run takes an ActiveEventLoop reference in this winit version.
    // Use the active loop handle to control flow / exit.
    event_loop
        .run(move |event, active_loop| {
            active_loop.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, window_id } => {
                    if window_id == zwin.window().id() {
                        match event {
                            WindowEvent::CloseRequested => {
                                // use the ActiveEventLoop exit helper
                                active_loop.exit();
                            }
                            WindowEvent::Resized(physical) => {
                                let w = physical.width.max(1);
                                let h = physical.height.max(1);
                                zwin.update_size(w, h);
                                backend.resize(w, h);
                            }
                            WindowEvent::ScaleFactorChanged { scale_factor, new_inner_size } => {
                                // winit's ScaleFactorChanged provides both scale_factor and
                                // a mutable reference to a new_inner_size value in this version.
                                // Accept the new_inner_size by value for our backend use.
                                let size: PhysicalSize<u32> = *new_inner_size;
                                let w = size.width.max(1);
                                let h = size.height.max(1);
                                zwin.update_size(w, h);
                                backend.resize(w, h);
                            }
                            _ => {}
                        }
                    }
                }
                Event::RedrawRequested(window_id) => {
                    if window_id == zwin.window().id() {
                        // Build an empty vello::Scene for now; backend clears the background.
                        let scene = vello::Scene::new();
                        backend.render_frame(&scene);
                    }
                }
                Event::MainEventsCleared => {
                    // Simple pacing to avoid burning CPU at full tilt. We still poll to remain responsive.
                    let now = Instant::now();
                    if now.duration_since(last_frame) >= frame_duration {
                        zwin.window().request_redraw();
                        last_frame = now;
                    }
                }
                _ => {}
            }
        })
        .expect("event loop failed");
}
