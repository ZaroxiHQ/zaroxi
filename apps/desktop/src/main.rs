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
    // Initialize logging early. Default to DEBUG if RUST_LOG is not set.
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

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
    info!(
        "[desktop] initial AppState: title='{}', workspace_items={}, open_docs={}, assistant_visible={}, panels_visible={}, status='{}'",
        app_state.config.title,
        app_state.workspace.items.len(),
        app_state.editor.open_documents.len(),
        app_state.assistant.visible,
        app_state.panels.visible,
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
