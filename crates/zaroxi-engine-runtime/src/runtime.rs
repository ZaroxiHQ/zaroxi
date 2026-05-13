use anyhow::Result;
use log::{error, info};
use std::sync::Arc;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::window_state::WindowState;
use zaroxi_engine_input::event::Event as InputEvent;
use zaroxi_engine_render::renderer::Renderer;

/// Start the engine runtime: create a window, initialize the renderer, and run the winit event loop.
///
/// This function blocks the current thread and returns when the window closes.
pub fn run(config: crate::super::EngineConfig) -> Result<()> {
    // Initialize logging for the desktop binary path (safe to call multiple times).
    let _ = env_logger::try_init();

    info!("Starting runtime with title '{}'", config.title);

    let event_loop = EventLoop::new();
    // Build the window and wrap it in an Arc so the renderer can own a handle safely.
    let window = Arc::new(
        WindowBuilder::new()
            .with_title(config.title)
            .with_inner_size(PhysicalSize::new(config.width, config.height))
            .build(&event_loop)?,
    );

    // Block on async GPU initialization.
    let mut renderer = pollster::block_on(Renderer::new(window.clone(), config.clear_color))?;

    let mut window_state = WindowState::new(window.inner_size());

    // Run event loop. This uses winit's `run` which does not return until exit.
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent { event, window_id } => match event {
                WindowEvent::CloseRequested => {
                    info!("Close requested");
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(size) => {
                    info!("Window resized to {:?}", size);
                    if size.width > 0 && size.height > 0 {
                        window_state.size = size;
                        if let Err(e) = renderer.resize(size) {
                            error!("Renderer resize error: {:?}", e);
                        }
                    }
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    // On some platforms a scale factor change delivers a new size.
                    if new_inner_size.width > 0 && new_inner_size.height > 0 {
                        window_state.size = *new_inner_size;
                        if let Err(e) = renderer.resize(*new_inner_size) {
                            error!("Renderer resize error: {:?}", e);
                        }
                    }
                }
                other => {
                    // Convert to normalized input events for future dispatch.
                    if let Some(_ie) = InputEvent::from_winit(&other) {
                        // For v1 we do not dispatch; this is a placeholder seam.
                    }
                }
            },
            Event::RedrawRequested(_) => {
                match renderer.render() {
                    Ok(_) => {
                        // Request continuous redraw (simple behaviour for v1).
                        // If you prefer event-driven redraws, remove this.
                        renderer.request_redraw();
                    }
                    Err(wgpu::SurfaceError::Lost) | Err(wgpu::SurfaceError::Outdated) => {
                        log::warn!("Surface lost/outdated, reconfiguring surface.");
                        if let Err(e) = renderer.reconfigure() {
                            error!("Failed to reconfigure surface after lost/outdated: {:?}", e);
                        }
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        error!("Out of memory while rendering; exiting.");
                        *control_flow = ControlFlow::Exit;
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        log::warn!("Surface timeout; skipping frame.");
                    }
                }
            }
            Event::MainEventsCleared => {
                // Trigger redraws at will for v1 (continuous redraw).
                // Request a redraw of the main window via the renderer's internal window handle.
                renderer.request_redraw();
            }
            _ => {}
        }
    });
}
