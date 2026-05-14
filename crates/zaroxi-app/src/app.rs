use zaroxi_config::AppConfig;
use crate::state::AppState;

/// Thin orchestration helpers for the application.
///
/// The heavy logic and state live in `state::AppState`. This module exposes a
/// small helper for building the initial app instance used by the desktop
/// binary.
pub fn build_app(config: &AppConfig) -> AppState {
    AppState::new(config)
}
