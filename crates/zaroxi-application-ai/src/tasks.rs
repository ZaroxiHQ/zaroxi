//! AI task management.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An AI task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTask {
    /// Unique identifier for the task.
    pub id: Uuid,
    /// The prompt for the task.
    pub prompt: String,
    /// The status of the task.
    pub status: TaskStatus,
}

/// Task status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is pending.
    Pending,
    /// Task is running.
    Running,
    /// Task completed successfully.
    Completed { result: String },
    /// Task failed.
    Failed { error: String },
}

impl AiTask {
    /// Create a new AI task.
    pub fn new(prompt: String) -> Self {
        Self { id: Uuid::new_v4(), prompt, status: TaskStatus::Pending }
    }

    /// Start the task.
    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
    }

    /// Complete the task with a result.
    pub fn complete(&mut self, result: String) {
        self.status = TaskStatus::Completed { result };
    }

    /// Fail the task with an error.
    pub fn fail(&mut self, error: String) {
        self.status = TaskStatus::Failed { error };
    }
}
