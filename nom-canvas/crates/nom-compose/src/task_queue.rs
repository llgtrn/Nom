#![deny(unsafe_code)]

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
    pub backend: String,
    pub input: String,
    pub state: TaskState,
    pub progress_pct: u32,
}

/// In-memory task queue for compose dispatch.
pub struct TaskQueue {
    next_id: u64,
    tasks: Vec<ComposeTask>,
    /// Optional hard cap on pending tasks (0 = unlimited).
    pub max_size: usize,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            tasks: Vec::new(),
            max_size: 0,
        }
    }

    /// Create a queue with a hard cap on pending tasks.
    pub fn with_max_size(max: usize) -> Self {
        Self {
            next_id: 1,
            tasks: Vec::new(),
            max_size: max,
        }
    }

    /// Enqueue a task. Returns `None` if `max_size > 0` and the pending count
    /// has reached the cap; returns `Some(id)` on success.
    pub fn try_enqueue(&mut self, backend: impl Into<String>, input: impl Into<String>) -> Option<u64> {
        if self.max_size > 0 && self.pending_count() >= self.max_size {
            return None;
        }
        Some(self.enqueue(backend, input))
    }

    pub fn enqueue(&mut self, backend: impl Into<String>, input: impl Into<String>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.push(ComposeTask {
            id,
            backend: backend.into(),
            input: input.into(),
            state: TaskState::Pending,
            progress_pct: 0,
        });
        id
    }

    /// Return and remove the oldest pending task (FIFO dequeue). Returns `None` when empty.
    pub fn dequeue_pending(&mut self) -> Option<ComposeTask> {
        if let Some(pos) = self
            .tasks
            .iter()
            .position(|t| t.state == TaskState::Pending)
        {
            Some(self.tasks.remove(pos))
        } else {
            None
        }
    }

    /// Remove and return all tasks in insertion order.
    pub fn drain_all(&mut self) -> Vec<ComposeTask> {
        self.tasks.drain(..).collect()
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
        let id = q.enqueue("video", "input");
        assert_eq!(q.pending_count(), 1);
        assert!(q.start(id));
        assert_eq!(q.running_count(), 1);
        assert_eq!(q.pending_count(), 0);
    }
    #[test]
    fn task_queue_complete_transitions_to_done() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("audio", "data");
        q.start(id);
        assert!(q.complete(id));
        assert_eq!(q.get(id).unwrap().state, TaskState::Completed);
    }
    #[test]
    fn task_queue_cancel_running_task() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("image", "src");
        q.start(id);
        assert!(q.cancel(id));
        assert_eq!(q.get(id).unwrap().state, TaskState::Cancelled);
    }
    #[test]
    fn task_queue_ids_are_sequential() {
        let mut q = TaskQueue::new();
        let a = q.enqueue("video", "a");
        let b = q.enqueue("audio", "b");
        assert_eq!(b, a + 1);
    }
    #[test]
    fn task_queue_complete_only_from_running_state() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("video", "input");
        assert!(
            !q.complete(id),
            "complete() must return false when task is not Running"
        );
        assert_eq!(q.get(id).unwrap().state, TaskState::Pending);
        q.start(id);
        assert!(q.complete(id));
        assert_eq!(q.get(id).unwrap().state, TaskState::Completed);
        assert!(
            !q.complete(id),
            "complete() must return false on already-Completed task"
        );
    }

    #[test]
    fn task_queue_enqueue() {
        let mut q = TaskQueue::new();
        q.enqueue("data", "payload");
        assert_eq!(q.pending_count(), 1);
    }

    #[test]
    fn task_queue_dequeue() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("image", "img_src");
        let task = q.get(id).unwrap();
        assert_eq!(task.input, "img_src");
    }

    #[test]
    fn task_queue_complete_marks_done() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("audio", "track");
        q.start(id);
        q.complete(id);
        assert_eq!(q.get(id).unwrap().state, TaskState::Completed);
    }

    #[test]
    fn task_queue_pending_count() {
        let mut q = TaskQueue::new();
        let id1 = q.enqueue("video", "v1");
        q.enqueue("audio", "a1");
        q.enqueue("image", "i1");
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
        let id = q.enqueue("render", "scene");
        assert!(!q.cancel(id), "cancel() must fail on Pending task");
        assert_eq!(q.get(id).unwrap().state, TaskState::Pending);
        q.start(id);
        assert!(q.cancel(id), "cancel() must succeed on Running task");
        assert_eq!(q.get(id).unwrap().state, TaskState::Cancelled);
        assert!(!q.cancel(id), "cancel() must fail on Cancelled task");
    }

    #[test]
    fn task_queue_start_only_from_pending() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("video", "v");
        assert!(q.start(id));
        assert!(!q.start(id), "start() must fail on Running task");
    }

    #[test]
    fn task_queue_progress_pct_at_completion() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("audio", "track");
        assert_eq!(q.get(id).unwrap().progress_pct, 0);
        q.start(id);
        q.complete(id);
        assert_eq!(q.get(id).unwrap().progress_pct, 100);
    }

    #[test]
    fn task_queue_many_tasks_running_count() {
        let mut q = TaskQueue::new();
        let ids: Vec<u64> = (0..5)
            .map(|_| q.enqueue("transform", "t"))
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
        let id = q.enqueue("render", "my-scene-input");
        let task = q.get(id).unwrap();
        assert_eq!(task.input, "my-scene-input");
        assert_eq!(task.backend, "render");
    }

    #[test]
    fn task_queue_get_unknown_id_returns_none() {
        let q = TaskQueue::new();
        assert!(q.get(9999).is_none());
    }

    #[test]
    fn task_queue_queued_to_running_to_done_state_machine() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("video", "input");
        assert_eq!(q.get(id).unwrap().state, TaskState::Pending);
        q.start(id);
        assert_eq!(q.get(id).unwrap().state, TaskState::Running);
        q.complete(id);
        assert_eq!(q.get(id).unwrap().state, TaskState::Completed);
    }

    #[test]
    fn task_queue_cancel_transition_running_to_cancelled() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("audio", "clip");
        q.start(id);
        assert_eq!(q.get(id).unwrap().state, TaskState::Running);
        assert!(q.cancel(id));
        assert_eq!(q.get(id).unwrap().state, TaskState::Cancelled);
    }

    #[test]
    fn task_queue_depth_limit_via_pending_count() {
        let mut q = TaskQueue::new();
        for i in 0..5u64 {
            q.enqueue("transform", format!("t{i}"));
        }
        assert_eq!(q.pending_count(), 5, "queue depth must be 5");
        q.enqueue("data", "extra");
        assert_eq!(q.pending_count(), 6);
    }

    #[test]
    fn task_queue_failed_state_stores_reason() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("render", "scene");
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
        let ids: Vec<u64> = (0..8)
            .map(|i| q.enqueue("transform", format!("t{i}")))
            .collect();
        for &id in &ids[..6] {
            q.start(id);
        }
        assert_eq!(q.running_count(), 6);
        assert_eq!(q.pending_count(), 2);
    }

    #[test]
    fn task_queue_complete_sets_progress_to_100() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("video", "stream");
        q.start(id);
        assert_eq!(q.get(id).unwrap().progress_pct, 0);
        q.complete(id);
        assert_eq!(q.get(id).unwrap().progress_pct, 100);
    }

    #[test]
    fn task_queue_cancel_does_not_set_progress_to_100() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("audio", "track");
        q.start(id);
        q.cancel(id);
        assert_eq!(
            q.get(id).unwrap().progress_pct,
            0,
            "cancelled task must not have 100% progress"
        );
    }

    #[test]
    fn task_queue_start_nonexistent_id_returns_false() {
        let mut q = TaskQueue::new();
        assert!(!q.start(9999), "start on nonexistent id must return false");
    }

    #[test]
    fn task_queue_complete_nonexistent_id_returns_false() {
        let mut q = TaskQueue::new();
        assert!(
            !q.complete(9999),
            "complete on nonexistent id must return false"
        );
    }

    #[test]
    fn task_queue_cancel_nonexistent_id_returns_false() {
        let mut q = TaskQueue::new();
        assert!(
            !q.cancel(9999),
            "cancel on nonexistent id must return false"
        );
    }

    #[test]
    fn task_queue_enqueue_stores_backend_kind() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("export", "data");
        assert_eq!(q.get(id).unwrap().backend, "export");
    }

    // ── Wave AK new tests ────────────────────────────────────────────────────

    #[test]
    fn task_queue_with_max_size_rejects_over_limit() {
        let mut q = TaskQueue::with_max_size(3);
        assert!(q.try_enqueue("video", "a").is_some());
        assert!(q.try_enqueue("audio", "b").is_some());
        assert!(q.try_enqueue("image", "c").is_some());
        assert!(
            q.try_enqueue("data", "d").is_none(),
            "4th enqueue must be rejected"
        );
        assert_eq!(q.pending_count(), 3, "pending count must stay at 3");
    }

    #[test]
    fn task_queue_completed_task_not_counted_as_pending() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("video", "v");
        q.start(id);
        q.complete(id);
        assert_eq!(
            q.pending_count(),
            0,
            "completed task must not be counted as pending"
        );
        assert_eq!(
            q.running_count(),
            0,
            "completed task must not be counted as running"
        );
    }

    #[test]
    fn task_queue_dequeue_pending_fifo_order() {
        let mut q = TaskQueue::new();
        q.enqueue("video", "first");
        q.enqueue("audio", "second");
        let t1 = q.dequeue_pending().unwrap();
        let t2 = q.dequeue_pending().unwrap();
        assert_eq!(t1.input, "first", "first dequeued must be first inserted");
        assert_eq!(
            t2.input, "second",
            "second dequeued must be second inserted"
        );
    }

    #[test]
    fn task_queue_dequeue_pending_empty_returns_none() {
        let mut q = TaskQueue::new();
        assert!(
            q.dequeue_pending().is_none(),
            "empty queue dequeue must return None"
        );
    }

    #[test]
    fn task_queue_drain_returns_all_tasks_in_order() {
        let mut q = TaskQueue::new();
        q.enqueue("video", "t1");
        q.enqueue("audio", "t2");
        q.enqueue("image", "t3");
        let drained = q.drain_all();
        assert_eq!(drained.len(), 3, "drain must return all 3 tasks");
        assert_eq!(drained[0].input, "t1");
        assert_eq!(drained[1].input, "t2");
        assert_eq!(drained[2].input, "t3");
        assert_eq!(q.pending_count(), 0, "queue must be empty after drain");
    }

    #[test]
    fn task_queue_max_size_zero_is_unlimited() {
        let mut q = TaskQueue::with_max_size(0);
        for i in 0..20u64 {
            assert!(
                q.try_enqueue("transform", format!("t{i}"))
                    .is_some(),
                "unlimited queue must accept task {i}"
            );
        }
        assert_eq!(q.pending_count(), 20);
    }

    #[test]
    fn task_queue_try_enqueue_allows_after_task_started() {
        let mut q = TaskQueue::with_max_size(2);
        let id1 = q.try_enqueue("video", "v1").unwrap();
        q.try_enqueue("audio", "a1").unwrap();
        assert!(q.try_enqueue("image", "i1").is_none());
        q.start(id1);
        assert!(
            q.try_enqueue("data", "d1").is_some(),
            "after starting a task, new enqueue must succeed"
        );
    }

    #[test]
    fn task_queue_dequeue_pending_skips_running_tasks() {
        let mut q = TaskQueue::new();
        let id = q.enqueue("video", "run-me");
        q.start(id);
        q.enqueue("audio", "pending-one");
        let t = q.dequeue_pending().unwrap();
        assert_eq!(t.input, "pending-one", "dequeue must skip running tasks");
    }

    #[test]
    fn task_queue_drain_empty_returns_empty_vec() {
        let mut q = TaskQueue::new();
        let drained = q.drain_all();
        assert!(
            drained.is_empty(),
            "drain on empty queue must return empty vec"
        );
    }

    #[test]
    fn task_queue_with_max_size_1_accepts_one_rejects_second() {
        let mut q = TaskQueue::with_max_size(1);
        assert!(q.try_enqueue("video", "only").is_some());
        assert!(q.try_enqueue("audio", "overflow").is_none());
    }

    #[test]
    fn task_queue_drain_mixed_states() {
        let mut q = TaskQueue::new();
        let id1 = q.enqueue("video", "pending");
        let id2 = q.enqueue("audio", "running");
        let id3 = q.enqueue("image", "done");
        q.start(id2);
        q.start(id3);
        q.complete(id3);
        let drained = q.drain_all();
        assert_eq!(drained.len(), 3);
        assert_eq!(drained[0].state, TaskState::Pending);
        assert_eq!(drained[1].state, TaskState::Running);
        assert_eq!(drained[2].state, TaskState::Completed);
        let _ = (id1, id2, id3);
    }
}
