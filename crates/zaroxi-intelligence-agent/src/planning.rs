//! Task planning.

use serde::{Deserialize, Serialize};

/// A plan for executing a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    /// Steps to execute.
    pub steps: Vec<String>,
}

impl TaskPlan {
    /// Create a new task plan.
    pub fn new(steps: Vec<String>) -> Self {
        Self { steps }
    }
}
