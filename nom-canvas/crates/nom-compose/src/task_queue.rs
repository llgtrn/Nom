#![deny(unsafe_code)]

use crate::dispatch::BackendKind;

/// Lifecycle state of a compose task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

/// A queued compose task.
#[derive(Debug, Clone)]
pub struct ComposeTask {
    pub id: u64,
    pub backend: BackendKind,
    pub input: String,
    pub state: TaskState,
    pub progress_pct: u32,
}

/// In-memory task queue for compose dispatch.
pub struct TaskQueue {
    next_id: u64,
    tasks: Vec<ComposeTask>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            tasks: Vec::new(),
        }
    }

    pub fn enqueue(&mut self, backend: BackendKind, input: impl Into<String>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.push(ComposeTask {
            id,
            backend,
            input: input.into(),
            state: TaskState::Pending,
            progress_pct: 0,
        });
        id
    }

    pub fn start(&mut self, id: u64) -> bool {
        if let Some(t) = self
            .tasks
            .iter_mut()
            .find(|t| t.id == id && t.state == TaskState::Pending)
        {
            t.state = TaskState::Running;
            true
        } else {
            false
        }
    }

    pub fn complete(&mut self, id: u64) -> bool {
        if let Some(t) = self
            .tasks
            .iter_mut()
            .find(|t| t.id == id && t.state == TaskState::Running)
        {
            t.state = TaskState::Completed;
            t.progress_pct = 100;
            true
        } else {
            false
        }
    }

    pub fn cancel(&mut self, id: u64) -> bool {
        if let Some(t) = self
            .tasks
            .iter_mut()
            .find(|t| t.id == id && t.state == TaskState::Running)
        {
            t.state = TaskState::Cancelled;
            true
        } else {
            false
        }
    }

    pub fn pending_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.state == TaskState::Pending)
            .count()
    }
    pub fn running_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.state == TaskState::Running)
            .count()
    }
    pub fn get(&self, id: u64) -> Option<&ComposeTask> {
        self.tasks.iter().find(|t| t.id == id)
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn task_queue_enqueue_and_start() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Video, "input");
        assert_eq!(q.pending_count(), 1);
        assert!(q.start(id));
        assert_eq!(q.running_count(), 1);
        assert_eq!(q.pending_count(), 0);
    }
    #[test]
    fn task_queue_complete_transitions_to_done() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Audio, "data");
        q.start(id);
        assert!(q.complete(id));
        assert_eq!(q.get(id).unwrap().state, TaskState::Completed);
    }
    #[test]
    fn task_queue_cancel_running_task() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Image, "src");
        q.start(id);
        assert!(q.cancel(id));
        assert_eq!(q.get(id).unwrap().state, TaskState::Cancelled);
    }
    #[test]
    fn task_queue_ids_are_sequential() {
        let mut q = TaskQueue::new();
        let a = q.enqueue(BackendKind::Video, "a");
        let b = q.enqueue(BackendKind::Audio, "b");
        assert_eq!(b, a + 1);
    }
    #[test]
    fn task_queue_complete_only_from_running_state() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Video, "input");
        // complete() on a Pending task must fail — guard required
        assert!(
            !q.complete(id),
            "complete() must return false when task is not Running"
        );
        assert_eq!(q.get(id).unwrap().state, TaskState::Pending);
        q.start(id);
        // now Running — complete() must succeed
        assert!(q.complete(id));
        assert_eq!(q.get(id).unwrap().state, TaskState::Completed);
        // already Completed — second complete() must fail
        assert!(
            !q.complete(id),
            "complete() must return false on already-Completed task"
        );
    }

    #[test]
    fn task_queue_enqueue() {
        let mut q = TaskQueue::new();
        q.enqueue(BackendKind::Data, "payload");
        assert_eq!(q.pending_count(), 1);
    }

    #[test]
    fn task_queue_dequeue() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Image, "img_src");
        let task = q.get(id).unwrap();
        assert_eq!(task.input, "img_src");
    }

    #[test]
    fn task_queue_complete_marks_done() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Audio, "track");
        q.start(id);
        q.complete(id);
        assert_eq!(q.get(id).unwrap().state, TaskState::Completed);
    }

    #[test]
    fn task_queue_pending_count() {
        let mut q = TaskQueue::new();
        let id1 = q.enqueue(BackendKind::Video, "v1");
        q.enqueue(BackendKind::Audio, "a1");
        q.enqueue(BackendKind::Image, "i1");
        q.start(id1);
        q.complete(id1);
        assert_eq!(q.pending_count(), 2);
    }

    #[test]
    fn task_queue_default_is_empty() {
        let q = TaskQueue::default();
        assert_eq!(q.pending_count(), 0);
        assert_eq!(q.running_count(), 0);
    }

    #[test]
    fn task_queue_cancel_only_from_running() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Render, "scene");
        // Cannot cancel a Pending task.
        assert!(!q.cancel(id), "cancel() must fail on Pending task");
        assert_eq!(q.get(id).unwrap().state, TaskState::Pending);
        q.start(id);
        assert!(q.cancel(id), "cancel() must succeed on Running task");
        assert_eq!(q.get(id).unwrap().state, TaskState::Cancelled);
        // Cannot cancel again once Cancelled.
        assert!(!q.cancel(id), "cancel() must fail on Cancelled task");
    }

    #[test]
    fn task_queue_start_only_from_pending() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Video, "v");
        assert!(q.start(id));
        // Already Running — start() must fail.
        assert!(!q.start(id), "start() must fail on Running task");
    }

    #[test]
    fn task_queue_progress_pct_at_completion() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Audio, "track");
        assert_eq!(q.get(id).unwrap().progress_pct, 0);
        q.start(id);
        q.complete(id);
        assert_eq!(q.get(id).unwrap().progress_pct, 100);
    }

    #[test]
    fn task_queue_many_tasks_running_count() {
        let mut q = TaskQueue::new();
        let ids: Vec<u64> = (0..5)
            .map(|_| q.enqueue(BackendKind::Transform, "t"))
            .collect();
        assert_eq!(q.pending_count(), 5);
        for &id in &ids[..3] {
            q.start(id);
        }
        assert_eq!(q.running_count(), 3);
        assert_eq!(q.pending_count(), 2);
    }

    #[test]
    fn task_queue_get_returns_correct_input() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Render, "my-scene-input");
        let task = q.get(id).unwrap();
        assert_eq!(task.input, "my-scene-input");
        assert_eq!(task.backend, BackendKind::Render);
    }

    #[test]
    fn task_queue_get_unknown_id_returns_none() {
        let q = TaskQueue::new();
        assert!(q.get(9999).is_none());
    }

    #[test]
    fn task_queue_queued_to_running_to_done_state_machine() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Video, "input");
        assert_eq!(q.get(id).unwrap().state, TaskState::Pending);
        q.start(id);
        assert_eq!(q.get(id).unwrap().state, TaskState::Running);
        q.complete(id);
        assert_eq!(q.get(id).unwrap().state, TaskState::Completed);
    }

    #[test]
    fn task_queue_cancel_transition_running_to_cancelled() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Audio, "clip");
        q.start(id);
        assert_eq!(q.get(id).unwrap().state, TaskState::Running);
        assert!(q.cancel(id));
        assert_eq!(q.get(id).unwrap().state, TaskState::Cancelled);
    }

    #[test]
    fn task_queue_depth_limit_via_pending_count() {
        // Simulate a max-depth-5 policy: enqueue 5, each pending.
        let mut q = TaskQueue::new();
        for i in 0..5u64 {
            q.enqueue(BackendKind::Transform, format!("t{i}"));
        }
        assert_eq!(q.pending_count(), 5, "queue depth must be 5");
        // Adding one more makes it 6 — caller is responsible for enforcing limit.
        q.enqueue(BackendKind::Data, "extra");
        assert_eq!(q.pending_count(), 6);
    }

    #[test]
    fn task_queue_failed_state_stores_reason() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Render, "scene");
        // Directly mutate state to Failed for validation.
        if let Some(t) = q.tasks.iter_mut().find(|t| t.id == id) {
            t.state = TaskState::Failed("disk full".to_string());
        }
        assert!(matches!(q.get(id).unwrap().state, TaskState::Failed(_)));
        if let TaskState::Failed(reason) = &q.get(id).unwrap().state {
            assert_eq!(reason, "disk full");
        }
    }

    #[test]
    fn task_queue_running_count_tracks_multiple_concurrent() {
        let mut q = TaskQueue::new();
        let ids: Vec<u64> = (0..8).map(|i| q.enqueue(BackendKind::Transform, format!("t{i}"))).collect();
        for &id in &ids[..6] {
            q.start(id);
        }
        assert_eq!(q.running_count(), 6);
        assert_eq!(q.pending_count(), 2);
    }

    #[test]
    fn task_queue_complete_sets_progress_to_100() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Video, "stream");
        q.start(id);
        assert_eq!(q.get(id).unwrap().progress_pct, 0);
        q.complete(id);
        assert_eq!(q.get(id).unwrap().progress_pct, 100);
    }

    #[test]
    fn task_queue_cancel_does_not_set_progress_to_100() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Audio, "track");
        q.start(id);
        q.cancel(id);
        // Progress must remain 0 for a cancelled task.
        assert_eq!(q.get(id).unwrap().progress_pct, 0, "cancelled task must not have 100% progress");
    }

    #[test]
    fn task_queue_start_nonexistent_id_returns_false() {
        let mut q = TaskQueue::new();
        assert!(!q.start(9999), "start on nonexistent id must return false");
    }

    #[test]
    fn task_queue_complete_nonexistent_id_returns_false() {
        let mut q = TaskQueue::new();
        assert!(!q.complete(9999), "complete on nonexistent id must return false");
    }

    #[test]
    fn task_queue_cancel_nonexistent_id_returns_false() {
        let mut q = TaskQueue::new();
        assert!(!q.cancel(9999), "cancel on nonexistent id must return false");
    }

    #[test]
    fn task_queue_enqueue_stores_backend_kind() {
        let mut q = TaskQueue::new();
        let id = q.enqueue(BackendKind::Export, "data");
        assert_eq!(q.get(id).unwrap().backend, BackendKind::Export);
    }
}
