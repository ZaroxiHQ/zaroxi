fn main() -> anyhow::Result<()> {
    // Initialize logging early.
    env_logger::init();

    let config = zaroxi_engine::EngineConfig {
        title: "Zaroxi Studio - Desktop".to_string(),
        width: 1400,
        height: 900,
        clear_color: [0.06, 0.07, 0.09, 1.0],
    };

    // Run the engine (blocks until window close).
    zaroxi_engine::run(config)
}
