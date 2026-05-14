use anyhow::Result;
use std::sync::{Arc, Mutex};
use log::info;
use env_logger::Env;

/// Thin desktop entrypoint for Zaroxi Studio.
///
/// Responsibilities:
/// - initialize logging
/// - build a small app model (AppState)
/// - compose engine config and call into the engine runtime
///
/// The heavy lifting of application logic lives in `crates/zaroxi-app` and
/// the domain crates. This binary acts as a thin composition root.
fn main() -> Result<()> {
    // Initialize logging early.
    // Default policy:
    // - If RUST_LOG is explicitly set by the user, respect it.
    // - Otherwise choose a conservative default:
    //     - global default = warn
    //     - noisy backends forced to warn (naga/wgpu)
    //     - when RENDER_DEBUG=1 enable debug for our crates only
    let env = Env::default();
    if std::env::var("RUST_LOG").is_ok() {
        // If the user explicitly set RUST_LOG, respect it verbatim.
        env_logger::Builder::from_env(env).init();
    } else {
        // Default conservative policy:
        // - global level = info (show startup/important messages)
        // - silence/quiet noisy backends (naga) by setting them to off
        // - keep wgpu-related crates at warn
        // - set our engine/render crates to info by default
        // If RENDER_DEBUG=1 is set in the environment, enable focused debug for our crates.
        let render_debug = std::env::var("RENDER_DEBUG")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let filter = if render_debug {
            // Render debug mode: enable debug for our runtime & renderer, keep noisy deps off/warn
            "info,naga=off,naga::front=off,naga::valid=off,wgpu=warn,wgpu_core=warn,wgpu_hal=warn,zaroxi_engine_render=debug,zaroxi_engine_runtime=debug,zaroxi_app=debug,zaroxi_app::view_model::render_panels=off"
        } else {
            // Normal mode: global info, quiet naga, wgpu at warn, our crates at info
            "info,naga=off,naga::front=off,naga::valid=off,wgpu=warn,wgpu_core=warn,wgpu_hal=warn,zaroxi_engine_render=info,zaroxi_engine_runtime=info,zaroxi_app=info,zaroxi_app::view_model::render_panels=off"
        };

        env_logger::Builder::from_env(env).parse_filters(filter).init();
    }

    // Guaranteed visible startup marker so we can confirm the process started.
    println!("desktop main starting");
    info!("desktop main starting");

    // Build configuration for the app (defaults provided by the config crate).
    let app_config = zaroxi_config::AppConfig::default();

    // Construct a minimal app state from the config. This creates a welcome
    // document and placeholder workspace items. The state is purely in-memory
    // and intentionally small for v1.
    let app_state = zaroxi_app::AppState::new(&app_config);

    // Log initial app state summary for debugging startup path.
    let panels_visible = app_state.app_panels.iter().any(|p| p.visible);
    info!(
        "[desktop] initial AppState: title='{}', workspace_items={}, open_docs={}, assistant_visible={}, panels_visible={}, status='{}'",
        app_state.config.title,
        app_state.workspace.items.len(),
        app_state.editor.open_documents.len(),
        app_state.assistant.visible,
        panels_visible,
        app_state.status.message
    );

    let app_state = Arc::new(Mutex::new(app_state));

    // Compose engine config (engine crate already provides EngineConfig used
    // elsewhere in the workspace). We map the app config into the engine config
    // so existing engine runtime can continue to be used.
    let engine_config = zaroxi_engine::EngineConfig {
        title: app_config.title.clone(),
        width: app_config.window_width,
        height: app_config.window_height,
        clear_color: app_config.clear_color,
    };

    // Run the engine (blocks until window close). Rendering wiring will be
    // integrated later; for v1 this opens the window and keeps the binary
    // consistent with prior expectations.
    zaroxi_engine::run(engine_config, app_state)
}
