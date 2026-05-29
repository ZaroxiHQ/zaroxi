/*!
Redraw / invalidation helpers for the window module.

Currently minimal: contains helpers for future redraw coordination.
Kept as a separate file to provide an obvious extension point before GUI-6.
*/

/// Helper to request a redraw on the engine window if present.
#[allow(dead_code)]
pub fn request_redraw_if_present(w: &Option<zaroxi_core_engine_window::ZaroxiWindow>) {
    if let Some(z) = w.as_ref() {
        let _ = z.window().request_redraw();
    }
}
