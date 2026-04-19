//! Durable workflow execution inspired by Temporal.
//!
//! Provides deterministic replay of workflows via an event-sourced history log.
//! MVP: in-memory history with activity stubs. Production: persistent history
//! store + async activity worker pool.

use std::collections::HashMap;

/// Kinds of events recorded in a workflow execution history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryEvent {
    /// Workflow execution started.
    WorkflowStarted { workflow_id: String, input: String },
    /// An activity was scheduled.
    ActivityScheduled { activity_id: String, activity_name: String, input: String },
    /// An activity completed with a result.
    ActivityCompleted { activity_id: String, output: String },
    /// An activity failed.
    ActivityFailed { activity_id: String, reason: String },
    /// Workflow completed.
    WorkflowCompleted { output: String },
    /// Workflow failed.
    WorkflowFailed { reason: String },
}

/// A single durable activity definition.
pub struct Activity {
    pub name: String,
    pub handler: Box<dyn Fn(&str) -> Result<String, String> + Send + Sync>,
}

impl Activity {
    pub fn new(
        name: impl Into<String>,
        handler: impl Fn(&str) -> Result<String, String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            handler: Box::new(handler),
        }
    }
}

/// In-memory history store for workflow execution events.
#[derive(Debug, Clone, Default)]
pub struct HistoryStore {
    events: Vec<HistoryEvent>,
}

impl HistoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, event: HistoryEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[HistoryEvent] {
        &self.events
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}

/// A durable workflow definition composed of activities.
pub struct Workflow {
    pub id: String,
    pub activities: HashMap<String, Activity>,
}

impl Workflow {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            activities: HashMap::new(),
        }
    }

    pub fn register_activity(mut self, activity: Activity) -> Self {
        self.activities.insert(activity.name.clone(), activity);
        self
    }
}

/// Executes a [`Workflow`] durably: replays history first, then runs new activities.
pub struct WorkflowExecutor;

impl WorkflowExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Run `workflow` with `input`, using `history` for deterministic replay.
    ///
    /// * If the history contains a prior execution, activities that already
    ///   have a `ActivityCompleted` event are skipped and their recorded output
    ///   is reused.
    /// * New activities are executed and their results appended to `history`.
    pub fn execute(
        &self,
        workflow: &Workflow,
        input: &str,
        history: &mut HistoryStore,
    ) -> Result<String, String> {
        if history.is_empty() {
            history.push(HistoryEvent::WorkflowStarted {
                workflow_id: workflow.id.clone(),
                input: input.to_string(),
            });
        }

        // Build a lookup of already-completed activities from history.
        let mut completed: HashMap<String, String> = HashMap::new();
        for event in history.events() {
            if let HistoryEvent::ActivityCompleted { activity_id, output } = event {
                completed.insert(activity_id.clone(), output.clone());
            }
        }

        // Execute each activity that has not yet been recorded.
        let mut final_output = input.to_string();
        for (act_name, activity) in &workflow.activities {
            let act_id = format!("{}:{}", workflow.id, act_name);
            if let Some(output) = completed.get(&act_id) {
                final_output = output.clone();
                continue;
            }

            history.push(HistoryEvent::ActivityScheduled {
                activity_id: act_id.clone(),
                activity_name: act_name.clone(),
                input: final_output.clone(),
            });

            match (activity.handler)(&final_output) {
                Ok(output) => {
                    history.push(HistoryEvent::ActivityCompleted {
                        activity_id: act_id,
                        output: output.clone(),
                    });
                    final_output = output;
                }
                Err(reason) => {
                    history.push(HistoryEvent::ActivityFailed {
                        activity_id: act_id.clone(),
                        reason: reason.clone(),
                    });
                    return Err(reason);
                }
            }
        }

        // Only record completion if the workflow hasn't already terminated.
        let already_terminated = history.events().iter().any(|e| {
            matches!(e, HistoryEvent::WorkflowCompleted { .. } | HistoryEvent::WorkflowFailed { .. })
        });
        if !already_terminated {
            history.push(HistoryEvent::WorkflowCompleted {
                output: final_output.clone(),
            });
        }
        Ok(final_output)
    }
}

impl Default for WorkflowExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_store_push_and_len() {
        let mut store = HistoryStore::new();
        assert!(store.is_empty());
        store.push(HistoryEvent::WorkflowStarted {
            workflow_id: "w1".into(),
            input: "hello".into(),
        });
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn workflow_executor_runs_activities() {
        let wf = Workflow::new("wf1").register_activity(Activity::new("upper", |s| {
            Ok(s.to_uppercase())
        }));
        let mut history = HistoryStore::new();
        let result = WorkflowExecutor::new().execute(&wf, "hello", &mut history);
        assert_eq!(result.unwrap(), "HELLO");
        assert!(history.len() >= 3); // started + scheduled + completed + completed-event
    }

    #[test]
    fn workflow_executor_replays_from_history() {
        let wf = Workflow::new("wf1").register_activity(Activity::new("upper", |s| {
            Ok(s.to_uppercase())
        }));
        let mut history = HistoryStore::new();
        let exec = WorkflowExecutor::new();
        let _ = exec.execute(&wf, "hello", &mut history);
        let first_len = history.len();

        // Replay with the same history — should not add new events.
        let mut history2 = history.clone();
        let result = exec.execute(&wf, "hello", &mut history2);
        assert_eq!(result.unwrap(), "HELLO");
        assert_eq!(history2.len(), first_len);
    }

    #[test]
    fn workflow_executor_propagates_activity_failure() {
        let wf = Workflow::new("wf1").register_activity(Activity::new("fail", |_s| {
            Err("boom".into())
        }));
        let mut history = HistoryStore::new();
        let result = WorkflowExecutor::new().execute(&wf, "hello", &mut history);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("boom"));
    }

    #[test]
    fn activity_new_and_name() {
        let act = Activity::new("test", |s| Ok(s.to_string()));
        assert_eq!(act.name, "test");
    }
}
