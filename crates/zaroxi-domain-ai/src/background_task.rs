//! Background task queue — domain types for long-running AI operations
//! that execute asynchronously without blocking the UI thread.
//!
//! Phase 4: background AI tasks with progress tracking, cancellation,
//! priority ordering, and result collection.

use serde::{Deserialize, Serialize};

use crate::actions::ActionKind;

/// Possible statuses of a background task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled)
    }
}

/// Priority level for task scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// A background AI task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundTask {
    pub task_id: String,
    pub label: String,
    pub description: String,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub action_kind: ActionKind,
    pub target_buffer: Option<String>,
    pub progress_frac: f32,
    pub progress_message: String,
    pub result: Option<String>,
    pub error_message: Option<String>,
    pub created_at_ms: u64,
}

impl BackgroundTask {
    pub fn new(
        label: impl Into<String>,
        description: impl Into<String>,
        priority: TaskPriority,
        action_kind: ActionKind,
    ) -> Self {
        Self {
            task_id: uuid::Uuid::new_v4().to_string(),
            label: label.into(),
            description: description.into(),
            priority,
            status: TaskStatus::Queued,
            action_kind,
            target_buffer: None,
            progress_frac: 0.0,
            progress_message: String::new(),
            result: None,
            error_message: None,
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target_buffer = Some(target.into());
        self
    }

    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
        self.progress_frac = 0.0;
    }

    pub fn set_progress(&mut self, frac: f32, message: &str) {
        self.progress_frac = frac.clamp(0.0, 1.0);
        self.progress_message = message.to_string();
    }

    pub fn complete(&mut self, result: &str) {
        self.status = TaskStatus::Completed;
        self.progress_frac = 1.0;
        self.result = Some(result.to_string());
    }

    pub fn fail(&mut self, error: &str) {
        self.status = TaskStatus::Failed;
        self.error_message = Some(error.to_string());
    }

    pub fn cancel(&mut self) {
        self.status = TaskStatus::Cancelled;
    }
}

/// A queue of background AI tasks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TaskQueue {
    pub tasks: Vec<BackgroundTask>,
}

impl TaskQueue {
    pub fn enqueue(&mut self, task: BackgroundTask) {
        self.tasks.push(task);
    }

    pub fn dequeue(&mut self) -> Option<BackgroundTask> {
        let best = self
            .tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.status == TaskStatus::Queued)
            .max_by_key(|(_, t)| (t.priority, -(t.created_at_ms as i64)))
            .map(|(i, _)| i);

        best.map(|i| self.tasks.remove(i))
    }

    pub fn active_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.status == TaskStatus::Running).count()
    }

    pub fn pending_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.status == TaskStatus::Queued).count()
    }

    pub fn completed_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count()
    }

    pub fn find(&self, task_id: &str) -> Option<&BackgroundTask> {
        self.tasks.iter().find(|t| t.task_id == task_id)
    }

    pub fn find_mut(&mut self, task_id: &str) -> Option<&mut BackgroundTask> {
        self.tasks.iter_mut().find(|t| t.task_id == task_id)
    }

    pub fn cancel_all_pending(&mut self) {
        for t in &mut self.tasks {
            if t.status == TaskStatus::Queued {
                t.cancel();
            }
        }
    }

    pub fn prune_completed(&mut self, max_age_ms: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.tasks.retain(|t| !t.status.is_terminal() || (now - t.created_at_ms) < max_age_ms);
    }
}

/// Aggregated background task status for UI display.
#[derive(Debug, Clone, Default)]
pub struct TaskQueueSummary {
    pub queued: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
}

impl From<&TaskQueue> for TaskQueueSummary {
    fn from(q: &TaskQueue) -> Self {
        let mut summary = TaskQueueSummary::default();
        for t in &q.tasks {
            match t.status {
                TaskStatus::Queued => summary.queued += 1,
                TaskStatus::Running => summary.running += 1,
                TaskStatus::Completed => summary.completed += 1,
                TaskStatus::Failed => summary.failed += 1,
                _ => {}
            }
        }
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_lifecycle() {
        let mut t =
            BackgroundTask::new("format", "Format main.rs", TaskPriority::Normal, ActionKind::Edit);
        assert_eq!(t.status, TaskStatus::Queued);

        t.start();
        assert_eq!(t.status, TaskStatus::Running);

        t.set_progress(0.5, "half done");
        assert!((t.progress_frac - 0.5).abs() < 0.001);

        t.complete("formatted 3 lines");
        assert_eq!(t.status, TaskStatus::Completed);
        assert_eq!(t.result, Some("formatted 3 lines".into()));
    }

    #[test]
    fn task_queue_priority_ordering() {
        let mut q = TaskQueue::default();
        q.enqueue(BackgroundTask::new(
            "low",
            "low priority",
            TaskPriority::Low,
            ActionKind::Explain,
        ));
        q.enqueue(BackgroundTask::new(
            "high",
            "high priority",
            TaskPriority::High,
            ActionKind::Edit,
        ));

        let first = q.dequeue().unwrap();
        assert_eq!(first.priority, TaskPriority::High);
    }

    #[test]
    fn task_cancel_and_prune() {
        let mut q = TaskQueue::default();
        q.enqueue(BackgroundTask::new("t1", "d1", TaskPriority::Normal, ActionKind::Explain));

        q.cancel_all_pending();
        assert_eq!(q.pending_count(), 0);
        assert!(q.tasks.iter().all(|t| t.status.is_terminal()));
    }

    #[test]
    fn task_queue_summary_counts() {
        let mut q = TaskQueue::default();
        q.enqueue(
            BackgroundTask::new("a", "running", TaskPriority::Normal, ActionKind::Edit)
                .with_target("main.rs"),
        );
        q.enqueue(BackgroundTask::new("b", "queued", TaskPriority::Normal, ActionKind::Explain));

        let mut running = q.dequeue().unwrap();
        running.start();
        q.tasks.push(running);

        let summary: TaskQueueSummary = (&q).into();
        assert_eq!(summary.running, 1);
        assert_eq!(summary.queued, 1);
    }
}
