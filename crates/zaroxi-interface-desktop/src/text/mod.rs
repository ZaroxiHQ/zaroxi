use std::sync::Arc;
use once_cell::sync::OnceCell;

pub mod cosmic_text_renderer;

/// Global, shared cosmic renderer holder.
///
/// The native binary or an initialization path should call `init_cosmic_renderer()` once
/// before the first frame to ensure the renderer is loaded with the project font.
/// Consumers may read `COSMIC_RENDERER.get()` to obtain an Arc reference.
pub static COSMIC_RENDERER: OnceCell<Arc<cosmic_text_renderer::CosmicTextRenderer>> = OnceCell::new();

/// Initialize the global cosmic renderer (idempotent).
///
/// Returns Ok(()) on success or if already initialized. Returns Err with a descriptive
/// message on failure to load fonts or initialize the renderer.
///
/// This implementation uses OnceCell::get_or_try_init to avoid a race where
/// multiple test threads attempt initialization concurrently. Returning an
/// error will surface the underlying initialization failure (for example,
/// inability to construct the renderer).
pub fn init_cosmic_renderer() -> Result<(), String> {
    COSMIC_RENDERER
        .get_or_try_init(|| cosmic_text_renderer::CosmicTextRenderer::new())
        .map(|_| ())
        .map_err(|e| format!("failed to initialize COSMIC_RENDERER: {}", e))
}
