//! AI service implementation.

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// AI service for handling AI-related operations.
pub struct AiService {
    /// Internal state.
    state: Arc<Mutex<AiServiceState>>,
}

struct AiServiceState {
    /// Whether the service is running.
    running: bool,
}

impl AiService {
    /// Create a new AI service.
    pub fn new() -> Self {
        Self { state: Arc::new(Mutex::new(AiServiceState { running: false })) }
    }

    /// Start the AI service.
    pub async fn start(&self) -> Result<(), anyhow::Error> {
        let mut state = self.state.lock().await;
        if state.running {
            return Err(anyhow::anyhow!("AI service is already running"));
        }
        state.running = true;
        info!("AI service started");
        Ok(())
    }

    /// Stop the AI service.
    pub async fn stop(&self) -> Result<(), anyhow::Error> {
        let mut state = self.state.lock().await;
        if !state.running {
            return Err(anyhow::anyhow!("AI service is not running"));
        }
        state.running = false;
        info!("AI service stopped");
        Ok(())
    }

    /// Check if the service is running.
    pub async fn is_running(&self) -> bool {
        let state = self.state.lock().await;
        state.running
    }
}
