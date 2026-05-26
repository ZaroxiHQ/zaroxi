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
pub fn init_cosmic_renderer() -> Result<(), String> {
    if COSMIC_RENDERER.get().is_some() {
        return Ok(());
    }
    let renderer = cosmic_text_renderer::CosmicTextRenderer::new()?;
    COSMIC_RENDERER.set(renderer).map_err(|_| "failed to set global COSMIC_RENDERER".to_string())
}
