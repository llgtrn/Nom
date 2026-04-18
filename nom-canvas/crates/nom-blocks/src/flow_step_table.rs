/// Flow step execution table — artifact step tracking with status, timing, and cache detection.

/// Execution status of a flow step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    /// Step has not started.
    Pending,
    /// Step is currently executing.
    Running,
    /// Step completed successfully.
    Done,
    /// Step encountered an error.
    Failed,
}

impl StepStatus {
    /// Returns true if the status is a terminal state (Done or Failed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, StepStatus::Done | StepStatus::Failed)
    }

    /// Returns a numeric code for the status: Pending=0, Running=1, Done=2, Failed=3.
    pub fn status_code(&self) -> u8 {
        match self {
            StepStatus::Pending => 0,
            StepStatus::Running => 1,
            StepStatus::Done => 2,
            StepStatus::Failed => 3,
        }
    }
}

/// A single row in the flow step table, recording execution details for one step.
#[derive(Debug, Clone)]
pub struct FlowStepRow {
    /// ID of the artifact this step belongs to.
    pub artifact_id: u64,
    /// Zero-based index of this step within the artifact's flow.
    pub step_index: u32,
    /// Entry ID of the nomtu executed at this step.
    pub entry_id: u64,
    /// Nanosecond timestamp when this step started.
    pub start_ns: u64,
    /// Nanosecond timestamp when this step ended.
    pub end_ns: u64,
    /// Hash of the step's input data.
    pub input_hash: u64,
    /// Hash of the step's output data.
    pub output_hash: u64,
    /// Current execution status.
    pub status: StepStatus,
}

impl FlowStepRow {
    /// Returns elapsed nanoseconds (end_ns - start_ns, saturating at zero).
    pub fn duration_ns(&self) -> u64 {
        self.end_ns.saturating_sub(self.start_ns)
    }

    /// Returns true if this step produced the same output from the same input as `other`.
    pub fn is_cached(&self, other: &FlowStepRow) -> bool {
        self.input_hash == other.input_hash && self.output_hash == other.output_hash
    }
}

/// In-memory table of flow step rows.
#[derive(Debug, Default)]
pub struct FlowStepTable {
    /// All stored rows.
    pub rows: Vec<FlowStepRow>,
}

impl FlowStepTable {
    /// Creates an empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a row into the table.
    pub fn insert(&mut self, row: FlowStepRow) {
        self.rows.push(row);
    }

    /// Returns all rows belonging to the given artifact.
    pub fn for_artifact(&self, artifact_id: u64) -> Vec<&FlowStepRow> {
        self.rows.iter().filter(|r| r.artifact_id == artifact_id).collect()
    }

    /// Returns all rows whose status is Failed.
    pub fn failed_steps(&self) -> Vec<&FlowStepRow> {
        self.rows
            .iter()
            .filter(|r| r.status.is_terminal() && r.status == StepStatus::Failed)
            .collect()
    }
}

/// Query parameters for filtering flow step rows.
#[derive(Debug, Default)]
pub struct FlowStepQuery {
    /// If set, only rows with this artifact_id match.
    pub artifact_id: Option<u64>,
    /// If set, only rows whose duration_ns is >= this value match.
    pub min_duration_ns: Option<u64>,
}

impl FlowStepQuery {
    /// Returns true if the given row satisfies all query constraints.
    pub fn matches(&self, row: &FlowStepRow) -> bool {
        if let Some(id) = self.artifact_id {
            if row.artifact_id != id {
                return false;
            }
        }
        if let Some(min) = self.min_duration_ns {
            if row.duration_ns() < min {
                return false;
            }
        }
        true
    }
}

/// An ordered timeline of steps for a single artifact.
#[derive(Debug, Default)]
pub struct StepTimeline {
    /// Steps sorted ascending by step_index.
    pub steps: Vec<FlowStepRow>,
}

impl StepTimeline {
    /// Builds a timeline from the table for a specific artifact, sorted by step_index ascending.
    pub fn from_table(table: &FlowStepTable, artifact_id: u64) -> StepTimeline {
        let mut steps: Vec<FlowStepRow> = table
            .rows
            .iter()
            .filter(|r| r.artifact_id == artifact_id)
            .cloned()
            .collect();
        steps.sort_by_key(|r| r.step_index);
        StepTimeline { steps }
    }

    /// Returns the sum of duration_ns across all steps in the timeline.
    pub fn total_duration_ns(&self) -> u64 {
        self.steps.iter().map(|r| r.duration_ns()).sum()
    }
}

#[cfg(test)]
mod flow_step_table_tests {
    use super::*;

    fn make_row(artifact_id: u64, step_index: u32, start_ns: u64, end_ns: u64, status: StepStatus) -> FlowStepRow {
        FlowStepRow {
            artifact_id,
            step_index,
            entry_id: 1,
            start_ns,
            end_ns,
            input_hash: 0xAA,
            output_hash: 0xBB,
            status,
        }
    }

    #[test]
    fn status_is_terminal_done_true() {
        assert!(StepStatus::Done.is_terminal());
        assert!(!StepStatus::Pending.is_terminal());
        assert!(!StepStatus::Running.is_terminal());
    }

    #[test]
    fn status_code_failed_is_3() {
        assert_eq!(StepStatus::Failed.status_code(), 3);
        assert_eq!(StepStatus::Pending.status_code(), 0);
        assert_eq!(StepStatus::Running.status_code(), 1);
        assert_eq!(StepStatus::Done.status_code(), 2);
    }

    #[test]
    fn row_duration_ns() {
        let row = make_row(1, 0, 100, 250, StepStatus::Done);
        assert_eq!(row.duration_ns(), 150);
    }

    #[test]
    fn row_is_cached_true() {
        let a = FlowStepRow {
            artifact_id: 1,
            step_index: 0,
            entry_id: 1,
            start_ns: 0,
            end_ns: 10,
            input_hash: 0xDEAD,
            output_hash: 0xBEEF,
            status: StepStatus::Done,
        };
        let b = FlowStepRow {
            artifact_id: 2,
            step_index: 5,
            entry_id: 9,
            start_ns: 100,
            end_ns: 200,
            input_hash: 0xDEAD,
            output_hash: 0xBEEF,
            status: StepStatus::Done,
        };
        assert!(a.is_cached(&b));
    }

    #[test]
    fn table_for_artifact_count() {
        let mut table = FlowStepTable::new();
        table.insert(make_row(1, 0, 0, 10, StepStatus::Done));
        table.insert(make_row(1, 1, 10, 20, StepStatus::Done));
        table.insert(make_row(2, 0, 0, 5, StepStatus::Done));
        assert_eq!(table.for_artifact(1).len(), 2);
        assert_eq!(table.for_artifact(2).len(), 1);
        assert_eq!(table.for_artifact(99).len(), 0);
    }

    #[test]
    fn table_failed_steps() {
        let mut table = FlowStepTable::new();
        table.insert(make_row(1, 0, 0, 10, StepStatus::Done));
        table.insert(make_row(1, 1, 10, 20, StepStatus::Failed));
        table.insert(make_row(2, 0, 0, 5, StepStatus::Pending));
        let failed = table.failed_steps();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].step_index, 1);
    }

    #[test]
    fn query_matches_artifact_filter() {
        let row_1 = make_row(42, 0, 0, 100, StepStatus::Done);
        let row_2 = make_row(99, 0, 0, 100, StepStatus::Done);
        let query = FlowStepQuery { artifact_id: Some(42), min_duration_ns: None };
        assert!(query.matches(&row_1));
        assert!(!query.matches(&row_2));
    }

    #[test]
    fn timeline_sorted_by_step_index() {
        let mut table = FlowStepTable::new();
        table.insert(make_row(5, 2, 20, 30, StepStatus::Done));
        table.insert(make_row(5, 0, 0, 10, StepStatus::Done));
        table.insert(make_row(5, 1, 10, 20, StepStatus::Done));
        let timeline = StepTimeline::from_table(&table, 5);
        let indices: Vec<u32> = timeline.steps.iter().map(|r| r.step_index).collect();
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn timeline_total_duration() {
        let mut table = FlowStepTable::new();
        table.insert(make_row(7, 0, 0, 50, StepStatus::Done));
        table.insert(make_row(7, 1, 50, 120, StepStatus::Done));
        table.insert(make_row(7, 2, 120, 200, StepStatus::Done));
        let timeline = StepTimeline::from_table(&table, 7);
        // durations: 50 + 70 + 80 = 200
        assert_eq!(timeline.total_duration_ns(), 200);
    }
}
