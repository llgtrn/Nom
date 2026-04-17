//! Lightweight task queue with per-kind concurrency caps.

use std::collections::HashMap;

use crate::kind::NomKind;

pub type TaskId = u64;

#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct TaskRecord {
    pub id: TaskId,
    pub state: TaskState,
    pub kind: NomKind,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
}

pub struct TaskQueue {
    tasks: Vec<TaskRecord>,
    per_kind_concurrency: HashMap<NomKind, usize>,
    per_kind_cap: HashMap<NomKind, usize>,
    next_id: TaskId,
}

impl TaskQueue {
    pub fn new() -> Self {
        let mut per_kind_cap = HashMap::new();
        per_kind_cap.insert(NomKind::MediaVideo, 2);
        per_kind_cap.insert(NomKind::MediaImage, 4);
        per_kind_cap.insert(NomKind::DataTransform, 8);

        TaskQueue {
            tasks: Vec::new(),
            per_kind_concurrency: HashMap::new(),
            per_kind_cap,
            next_id: 1,
        }
    }

    /// Override or set the concurrency cap for a kind.
    pub fn set_cap(&mut self, kind: NomKind, cap: usize) {
        self.per_kind_cap.insert(kind, cap);
    }

    /// Add a new task in `Pending` state; returns its id.
    pub fn enqueue(&mut self, kind: NomKind) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.push(TaskRecord {
            id,
            state: TaskState::Pending,
            kind,
            started_at_ms: None,
            finished_at_ms: None,
        });
        id
    }

    /// Move the oldest pending task for this kind to Running, if under cap.
    pub fn start_next_for(&mut self, kind: NomKind) -> Option<TaskId> {
        let cap = self.per_kind_cap.get(&kind).copied().unwrap_or(usize::MAX);
        let running = self.running_count(kind);
        if running >= cap {
            return None;
        }
        // Find oldest pending task for this kind.
        let idx = self
            .tasks
            .iter()
            .position(|t| t.kind == kind && t.state == TaskState::Pending)?;

        let id = self.tasks[idx].id;
        self.tasks[idx].state = TaskState::Running;
        *self.per_kind_concurrency.entry(kind).or_insert(0) += 1;
        Some(id)
    }

    pub fn mark_completed(&mut self, id: TaskId) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
            if t.state == TaskState::Running {
                *self.per_kind_concurrency.entry(t.kind).or_insert(1) -= 1;
            }
            t.state = TaskState::Completed;
        }
    }

    pub fn mark_failed(&mut self, id: TaskId, reason: String) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
            if t.state == TaskState::Running {
                *self.per_kind_concurrency.entry(t.kind).or_insert(1) -= 1;
            }
            t.state = TaskState::Failed(reason);
        }
    }

    pub fn mark_cancelled(&mut self, id: TaskId) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
            if t.state == TaskState::Running {
                *self.per_kind_concurrency.entry(t.kind).or_insert(1) -= 1;
            }
            t.state = TaskState::Cancelled;
        }
    }

    pub fn running_count(&self, kind: NomKind) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.kind == kind && t.state == TaskState::Running)
            .count()
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_returns_sequential_ids() {
        let mut q = TaskQueue::new();
        let a = q.enqueue(NomKind::MediaImage);
        let b = q.enqueue(NomKind::MediaImage);
        assert_eq!(b, a + 1);
    }

    #[test]
    fn start_next_for_moves_to_running() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(NomKind::DataTransform);
        let started = q.start_next_for(NomKind::DataTransform).unwrap();
        assert_eq!(started, id);
        assert_eq!(q.running_count(NomKind::DataTransform), 1);
    }

    #[test]
    fn cap_respected() {
        let mut q = TaskQueue::new();
        q.set_cap(NomKind::MediaVideo, 1);
        q.enqueue(NomKind::MediaVideo);
        q.enqueue(NomKind::MediaVideo);
        q.start_next_for(NomKind::MediaVideo);
        // Cap is 1, second should not start.
        let result = q.start_next_for(NomKind::MediaVideo);
        assert!(result.is_none());
    }

    #[test]
    fn mark_completed_decrements_running() {
        let mut q = TaskQueue::new();
        q.enqueue(NomKind::MediaImage);
        let id = q.start_next_for(NomKind::MediaImage).unwrap();
        q.mark_completed(id);
        assert_eq!(q.running_count(NomKind::MediaImage), 0);
    }

    #[test]
    fn mark_failed_sets_failed_state() {
        let mut q = TaskQueue::new();
        q.enqueue(NomKind::DataQuery);
        let id = q.start_next_for(NomKind::DataQuery).unwrap();
        q.mark_failed(id, "timeout".into());
        let t = q.tasks.iter().find(|t| t.id == id).unwrap();
        assert!(matches!(t.state, TaskState::Failed(_)));
    }

    #[test]
    fn mark_cancelled_sets_cancelled_state() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(NomKind::ScreenWeb);
        q.mark_cancelled(id);
        let t = q.tasks.iter().find(|t| t.id == id).unwrap();
        assert_eq!(t.state, TaskState::Cancelled);
    }

    #[test]
    fn default_caps_present() {
        let q = TaskQueue::new();
        assert_eq!(q.per_kind_cap[&NomKind::MediaVideo], 2);
        assert_eq!(q.per_kind_cap[&NomKind::MediaImage], 4);
        assert_eq!(q.per_kind_cap[&NomKind::DataTransform], 8);
    }

    #[test]
    fn no_pending_returns_none() {
        let mut q = TaskQueue::new();
        assert!(q.start_next_for(NomKind::Media3D).is_none());
    }
}
