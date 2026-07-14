//! Background task service — manages a queue of background AI tasks,
//! spawning them asynchronously, tracking progress, and collecting results.
//!
//! Phase 4: non-blocking AI operations with cancellation and progress.
//! Integrates with the agent orchestrator and action service for execution.

use std::sync::Mutex;

use zaroxi_domain_ai::actions::ActionKind;
use zaroxi_domain_ai::background_task::{
    BackgroundTask, TaskPriority, TaskQueue, TaskQueueSummary, TaskStatus,
};

/// Manages a priority queue of background AI tasks.
pub struct BackgroundTaskService {
    queue: Mutex<TaskQueue>,
    max_concurrent: usize,
}

impl BackgroundTaskService {
    pub fn new(max_concurrent: usize) -> Self {
        Self { queue: Mutex::new(TaskQueue::default()), max_concurrent }
    }

    /// Enqueue a new task.
    pub fn enqueue(
        &self,
        label: &str,
        description: &str,
        priority: TaskPriority,
        kind: ActionKind,
        target: Option<&str>,
    ) -> BackgroundTask {
        let mut task = BackgroundTask::new(label, description, priority, kind);
        if let Some(t) = target {
            task = task.with_target(t);
        }
        let result = task.clone();
        self.queue.lock().unwrap().enqueue(task);
        result
    }

    /// Dequeue one task to start executing.
    pub fn dequeue(&self) -> Option<BackgroundTask> {
        let mut queue = self.queue.lock().unwrap();
        if queue.active_count() >= self.max_concurrent {
            return None;
        }
        let mut task = queue.dequeue()?;
        task.start();
        let result = task.clone();
        queue.tasks.push(task);
        Some(result)
    }

    /// Update progress for a running task.
    pub fn update_progress(&self, task_id: &str, frac: f32, message: &str) {
        if let Some(task) = self.queue.lock().unwrap().find_mut(task_id) {
            task.set_progress(frac, message);
        }
    }

    /// Mark a task as completed with result.
    pub fn complete_task(&self, task_id: &str, result: &str) {
        if let Some(task) = self.queue.lock().unwrap().find_mut(task_id) {
            task.complete(result);
        }
    }

    /// Mark a task as failed.
    pub fn fail_task(&self, task_id: &str, error: &str) {
        if let Some(task) = self.queue.lock().unwrap().find_mut(task_id) {
            task.fail(error);
        }
    }

    /// Cancel a task by id.
    pub fn cancel_task(&self, task_id: &str) {
        if let Some(task) = self.queue.lock().unwrap().find_mut(task_id) {
            task.cancel();
        }
    }

    /// Cancel all queued (not yet started) tasks.
    pub fn cancel_all_pending(&self) {
        self.queue.lock().unwrap().cancel_all_pending();
    }

    /// Get a task by id.
    pub fn get_task(&self, task_id: &str) -> Option<BackgroundTask> {
        self.queue.lock().unwrap().find(task_id).cloned()
    }

    /// Get all tasks.
    pub fn all_tasks(&self) -> Vec<BackgroundTask> {
        self.queue.lock().unwrap().tasks.clone()
    }

    /// Get active (running) tasks.
    pub fn active_tasks(&self) -> Vec<BackgroundTask> {
        self.queue
            .lock()
            .unwrap()
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .cloned()
            .collect()
    }

    /// Get queued (pending) tasks.
    pub fn pending_tasks(&self) -> Vec<BackgroundTask> {
        self.queue
            .lock()
            .unwrap()
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Queued)
            .cloned()
            .collect()
    }

    /// Generate a summary of the queue.
    pub fn summary(&self) -> TaskQueueSummary {
        let queue = self.queue.lock().unwrap();
        (&*queue).into()
    }

    /// Prune old completed tasks (older than max_age_ms).
    pub fn prune(&self, max_age_ms: u64) {
        self.queue.lock().unwrap().prune_completed(max_age_ms);
    }

    /// Check if any task can be dequeued (slots available and tasks queued).
    pub fn can_dequeue(&self) -> bool {
        let queue = self.queue.lock().unwrap();
        queue.active_count() < self.max_concurrent && queue.pending_count() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_dequeue_lifecycle() {
        let svc = BackgroundTaskService::new(3);
        let task = svc.enqueue("test", "desc", TaskPriority::Normal, ActionKind::Explain, None);
        assert_eq!(task.status, TaskStatus::Queued);

        assert!(svc.can_dequeue());
        let running = svc.dequeue().unwrap();
        assert_eq!(running.status, TaskStatus::Running);

        svc.update_progress(&running.task_id, 0.5, "half done");
        let updated = svc.get_task(&running.task_id).unwrap();
        assert!((updated.progress_frac - 0.5).abs() < 0.001);

        svc.complete_task(&running.task_id, "done");
        let completed = svc.get_task(&running.task_id).unwrap();
        assert_eq!(completed.status, TaskStatus::Completed);
    }

    #[test]
    fn max_concurrent_limit() {
        let svc = BackgroundTaskService::new(1);
        svc.enqueue("low", "d", TaskPriority::Low, ActionKind::Edit, None);
        svc.enqueue("high", "d", TaskPriority::High, ActionKind::Explain, None);

        let first = svc.dequeue().unwrap();
        assert_eq!(first.label, "high");

        // Second dequeue should return None (at capacity)
        let second = svc.dequeue();
        assert!(second.is_none());

        // Complete first, then second can be dequeued
        svc.complete_task(&first.task_id, "done");
        let second = svc.dequeue().unwrap();
        assert_eq!(second.label, "low");
    }

    #[test]
    fn cancel_all_pending() {
        let svc = BackgroundTaskService::new(3);
        svc.enqueue("t1", "d", TaskPriority::Normal, ActionKind::Explain, None);
        svc.enqueue("t2", "d", TaskPriority::Normal, ActionKind::Edit, None);
        svc.cancel_all_pending();

        assert_eq!(svc.summary().queued, 0);
        assert_eq!(svc.summary().failed, 0); // cancelled != failed
    }

    #[test]
    fn fail_task_propagates_error() {
        let svc = BackgroundTaskService::new(1);
        let task = svc.enqueue("failing", "desc", TaskPriority::Normal, ActionKind::Edit, None);
        svc.dequeue();
        svc.fail_task(&task.task_id, "connection lost");

        let failed = svc.get_task(&task.task_id).unwrap();
        assert_eq!(failed.status, TaskStatus::Failed);
        assert_eq!(failed.error_message, Some("connection lost".into()));
    }
}
