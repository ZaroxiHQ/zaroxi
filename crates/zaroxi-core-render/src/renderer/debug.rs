use std::sync::atomic::AtomicBool;

/// Whether to enable verbose render-time debug logging by default.
///
/// This flag can be overridden at runtime by the environment variable
/// `RENDER_DEBUG=1` or `RENDER_DEBUG=true`.
pub(crate) const RENDER_DEBUG: bool = false;
/// If true, use nearest sampling for the font atlas (diagnostic).
pub(crate) const TEXT_SAMPLER_NEAREST: bool = false;

/// Global single-shot flag to ensure we only emit the "first glyph" sample line once.
pub(crate) static FIRST_GLYPH_LOGGED: AtomicBool = AtomicBool::new(false);

/// One-shot flags to log CPU-side panel quad colors only once per panel at startup.
pub(crate) static LOGGED_TITLEBAR: AtomicBool = AtomicBool::new(false);
pub(crate) static LOGGED_SIDEBAR: AtomicBool = AtomicBool::new(false);
pub(crate) static LOGGED_EDITOR: AtomicBool = AtomicBool::new(false);
/// One-shot flag to dump packed panel vertex values for the sidebar (packed GPU upload values).
pub(crate) static LOGGED_SIDEBAR_PACKED: AtomicBool = AtomicBool::new(false);

/// Experiment flags:
/// When true, force the sidebar content quad to magenta for quick visual verification.
pub(crate) const FORCE_MAGENTA_SIDEBAR: bool = false;
/// When true, skip the text pass entirely (draw shapes only).
pub(crate) const DISABLE_TEXT_PASS: bool = false;

/// Validation scene toggle (disabled by default to avoid contaminating normal runs).
/// Re-enable validation scene by default so GPU/shape/text validation geometry
/// runs during development and surfaces regressions in the renderer.
pub(crate) const VALIDATION_SCENE: bool = true;

/// Helper used to decide whether to show render-time diagnostics.
/// Default is controlled by the compile-time `RENDER_DEBUG` constant, but
/// an environment variable `RENDER_DEBUG=1` or `RENDER_DEBUG=true` can also
/// enable debug logging at runtime without rebuilding.
pub(crate) fn render_debug_enabled() -> bool {
    if RENDER_DEBUG {
        return true;
    }
    std::env::var("RENDER_DEBUG")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}
