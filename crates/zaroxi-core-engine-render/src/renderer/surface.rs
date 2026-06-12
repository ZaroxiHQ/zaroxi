use crate::error::RenderError;
use wgpu::{
    Adapter, Device, PresentMode, Surface, SurfaceConfiguration, TextureFormat, TextureUsages,
};
use winit::dpi::PhysicalSize;

/// Surface and frame lifecycle helpers moved out of core.rs.
/// Move-only refactor: these functions encapsulate surface configuration,
/// resize/reconfigure logic, current-texture acquisition and submit/present.
///
/// They preserve behavior (same choices for formats/alpha_mode/present_mode)
/// and surface configuration as the original implementation.

/// Configure the provided surface for the given adapter/device and window size.
///
/// This mirrors the logic previously in core.rs to pick a format and construct
/// a SurfaceConfiguration, then configures the surface. Returns the created
/// SurfaceConfiguration on success.
pub(crate) fn configure_surface(
    surface: &Surface,
    adapter: &Adapter,
    device: &Device,
    size: PhysicalSize<u32>,
) -> Result<SurfaceConfiguration, RenderError> {
    let caps = surface.get_capabilities(adapter);

    let format = caps
        .formats
        .iter()
        .copied()
        .find(|f| matches!(f, TextureFormat::Bgra8UnormSrgb | TextureFormat::Rgba8UnormSrgb))
        .or_else(|| caps.formats.get(0).copied())
        .unwrap_or(TextureFormat::Bgra8UnormSrgb);

    let config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: PresentMode::Fifo,
        alpha_mode: caps.alpha_modes[0],
        view_formats: Vec::new(),
        desired_maximum_frame_latency: 2u32,
    };

    surface.configure(device, &config);
    Ok(config)
}

/// Resize the surface configuration in-place and reconfigure the surface.
///
/// This updates the provided `config` and calls `surface.configure(...)`.
pub(crate) fn resize_surface(
    surface: &Surface,
    device: &Device,
    config: &mut SurfaceConfiguration,
    new_size: PhysicalSize<u32>,
) -> Result<(), RenderError> {
    if new_size.width == 0 || new_size.height == 0 {
        return Ok(());
    }
    config.width = new_size.width.max(1);
    config.height = new_size.height.max(1);
    surface.configure(device, config);
    Ok(())
}

/// Reconfigure the surface using the supplied configuration (no mutation).
pub(crate) fn reconfigure_surface(
    surface: &Surface,
    device: &Device,
    config: &SurfaceConfiguration,
) -> Result<(), RenderError> {
    surface.configure(device, config);
    Ok(())
}

/// Acquire the current surface texture (thin wrapper).
pub(crate) fn acquire_current_surface_texture(surface: &Surface) -> wgpu::CurrentSurfaceTexture {
    surface.get_current_texture()
}

/// Submit the encoder to the queue and present the provided frame.
///
/// This mirrors the previous pattern in core.rs where the queue was submitted
/// and the surface frame was presented. Kept as a small helper to isolate
/// lifecycle interactions.
pub(crate) fn submit_and_present(
    queue: &wgpu::Queue,
    encoder: wgpu::CommandEncoder,
    frame: wgpu::SurfaceTexture,
) {
    queue.submit(Some(encoder.finish()));
    frame.present();
}
