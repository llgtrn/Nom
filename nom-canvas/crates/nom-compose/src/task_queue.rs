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
    pub fn new() -> Self { Self { next_id: 1, tasks: Vec::new() } }

    pub fn enqueue(&mut self, backend: BackendKind, input: impl Into<String>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.push(ComposeTask { id, backend, input: input.into(), state: TaskState::Pending, progress_pct: 0 });
        id
    }

    pub fn start(&mut self, id: u64) -> bool {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id && t.state == TaskState::Pending) {
            t.state = TaskState::Running; true
        } else { false }
    }

    pub fn complete(&mut self, id: u64) -> bool {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id && t.state == TaskState::Running) {
            t.state = TaskState::Completed; t.progress_pct = 100; true
        } else { false }
    }

    pub fn cancel(&mut self, id: u64) -> bool {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id && t.state == TaskState::Running) {
            t.state = TaskState::Cancelled; true
        } else { false }
    }

    pub fn pending_count(&self) -> usize { self.tasks.iter().filter(|t| t.state == TaskState::Pending).count() }
    pub fn running_count(&self) -> usize { self.tasks.iter().filter(|t| t.state == TaskState::Running).count() }
    pub fn get(&self, id: u64) -> Option<&ComposeTask> { self.tasks.iter().find(|t| t.id == id) }
}

impl Default for TaskQueue { fn default() -> Self { Self::new() } }

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
        assert!(!q.complete(id), "complete() must return false when task is not Running");
        assert_eq!(q.get(id).unwrap().state, TaskState::Pending);
        q.start(id);
        // now Running — complete() must succeed
        assert!(q.complete(id));
        assert_eq!(q.get(id).unwrap().state, TaskState::Completed);
        // already Completed — second complete() must fail
        assert!(!q.complete(id), "complete() must return false on already-Completed task");
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
}
