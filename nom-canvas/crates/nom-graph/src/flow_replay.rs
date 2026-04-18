/// Replay speed control for flow execution replay.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplaySpeed {
    RealTime,
    FastForward(u8),
    Instant,
}

impl ReplaySpeed {
    pub fn multiplier(&self) -> f64 {
        match self {
            ReplaySpeed::RealTime => 1.0,
            ReplaySpeed::FastForward(n) => *n as f64,
            ReplaySpeed::Instant => f64::INFINITY,
        }
    }
}

/// A single recorded entry in a flow replay.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowReplayEntry {
    pub step_index: usize,
    pub node_id: String,
    pub timestamp_ns: u64,
    pub input_hash: u64,
    pub output_hash: u64,
}

impl FlowReplayEntry {
    /// Returns elapsed nanoseconds from `start_ns` to this entry's timestamp (saturating).
    pub fn duration_from(&self, start_ns: u64) -> u64 {
        self.timestamp_ns.saturating_sub(start_ns)
    }

    /// Returns true when both input and output hashes match the other entry.
    pub fn is_deterministic_with(&self, other: &FlowReplayEntry) -> bool {
        self.input_hash == other.input_hash && self.output_hash == other.output_hash
    }
}

/// An ordered sequence of replay entries for a named flow.
#[derive(Debug, Clone, Default)]
pub struct FlowReplay {
    pub entries: Vec<FlowReplayEntry>,
    pub name: String,
}

impl FlowReplay {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            entries: Vec::new(),
            name: name.into(),
        }
    }

    pub fn add_entry(&mut self, e: FlowReplayEntry) {
        self.entries.push(e);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the last entry whose `node_id` matches.
    pub fn entry_for_node(&self, node_id: &str) -> Option<&FlowReplayEntry> {
        self.entries.iter().rev().find(|e| e.node_id == node_id)
    }

    /// Returns true when both replays have the same length and every corresponding
    /// pair of entries is deterministic with each other.
    pub fn is_deterministic_with(&self, other: &FlowReplay) -> bool {
        self.entries.len() == other.entries.len()
            && self
                .entries
                .iter()
                .zip(other.entries.iter())
                .all(|(a, b)| a.is_deterministic_with(b))
    }
}

/// Cursor-based controller for stepping through a `FlowReplay`.
#[derive(Debug)]
pub struct ReplayController {
    pub replay: FlowReplay,
    pub cursor: usize,
    pub speed: ReplaySpeed,
}

impl ReplayController {
    pub fn new(replay: FlowReplay, speed: ReplaySpeed) -> Self {
        Self { replay, cursor: 0, speed }
    }

    /// Returns the entry at the current cursor position and advances the cursor.
    pub fn advance(&mut self) -> Option<&FlowReplayEntry> {
        if self.cursor < self.replay.entries.len() {
            let entry = &self.replay.entries[self.cursor];
            self.cursor += 1;
            Some(entry)
        } else {
            None
        }
    }

    pub fn is_done(&self) -> bool {
        self.cursor >= self.replay.len()
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
    }
}

/// Lightweight summary snapshot of a `FlowReplay`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaySnapshot {
    pub name: String,
    pub entry_count: usize,
    pub checksum: u64,
}

impl ReplaySnapshot {
    pub fn from_replay(r: &FlowReplay) -> ReplaySnapshot {
        let checksum = r.entries.iter().fold(0u64, |acc, e| acc ^ e.output_hash);
        ReplaySnapshot {
            name: r.name.clone(),
            entry_count: r.len(),
            checksum,
        }
    }
}

#[cfg(test)]
mod flow_replay_tests {
    use super::*;

    fn make_entry(step_index: usize, node_id: &str, timestamp_ns: u64, input_hash: u64, output_hash: u64) -> FlowReplayEntry {
        FlowReplayEntry {
            step_index,
            node_id: node_id.to_string(),
            timestamp_ns,
            input_hash,
            output_hash,
        }
    }

    #[test]
    fn speed_multiplier_fast_forward() {
        assert_eq!(ReplaySpeed::FastForward(4).multiplier(), 4.0);
        assert_eq!(ReplaySpeed::RealTime.multiplier(), 1.0);
        assert!(ReplaySpeed::Instant.multiplier().is_infinite());
    }

    #[test]
    fn entry_duration_from() {
        let e = make_entry(0, "n1", 1000, 0, 0);
        assert_eq!(e.duration_from(600), 400);
        // saturating: start_ns > timestamp_ns returns 0
        assert_eq!(e.duration_from(2000), 0);
    }

    #[test]
    fn entry_is_deterministic_with_true() {
        let a = make_entry(0, "n1", 100, 42, 99);
        let b = make_entry(0, "n2", 200, 42, 99);
        assert!(a.is_deterministic_with(&b));
    }

    #[test]
    fn entry_is_deterministic_with_false() {
        let a = make_entry(0, "n1", 100, 42, 99);
        let b = make_entry(0, "n1", 100, 42, 77);
        assert!(!a.is_deterministic_with(&b));
    }

    #[test]
    fn replay_add_and_len() {
        let mut r = FlowReplay::new("test");
        assert_eq!(r.len(), 0);
        r.add_entry(make_entry(0, "a", 10, 1, 2));
        r.add_entry(make_entry(1, "b", 20, 3, 4));
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn replay_entry_for_node_found() {
        let mut r = FlowReplay::new("test");
        r.add_entry(make_entry(0, "alpha", 10, 1, 2));
        r.add_entry(make_entry(1, "beta", 20, 3, 4));
        r.add_entry(make_entry(2, "alpha", 30, 5, 6));
        // Should return last matching entry (step_index 2)
        let found = r.entry_for_node("alpha").unwrap();
        assert_eq!(found.step_index, 2);
        assert!(r.entry_for_node("gamma").is_none());
    }

    #[test]
    fn controller_advance_and_is_done() {
        let mut r = FlowReplay::new("ctrl");
        r.add_entry(make_entry(0, "x", 0, 0, 0));
        r.add_entry(make_entry(1, "y", 1, 0, 0));
        let mut ctrl = ReplayController::new(r, ReplaySpeed::RealTime);
        assert!(!ctrl.is_done());
        let e0 = ctrl.advance().unwrap();
        assert_eq!(e0.step_index, 0);
        let e1 = ctrl.advance().unwrap();
        assert_eq!(e1.step_index, 1);
        assert!(ctrl.is_done());
        assert!(ctrl.advance().is_none());
    }

    #[test]
    fn controller_reset() {
        let mut r = FlowReplay::new("reset");
        r.add_entry(make_entry(0, "n", 0, 0, 0));
        let mut ctrl = ReplayController::new(r, ReplaySpeed::Instant);
        ctrl.advance();
        assert!(ctrl.is_done());
        ctrl.reset();
        assert!(!ctrl.is_done());
        assert_eq!(ctrl.cursor, 0);
    }

    #[test]
    fn snapshot_checksum_xor() {
        let mut r = FlowReplay::new("snap");
        r.add_entry(make_entry(0, "a", 0, 0, 0b1010));
        r.add_entry(make_entry(1, "b", 0, 0, 0b1100));
        r.add_entry(make_entry(2, "c", 0, 0, 0b0110));
        let snap = ReplaySnapshot::from_replay(&r);
        assert_eq!(snap.name, "snap");
        assert_eq!(snap.entry_count, 3);
        // XOR: 0b1010 ^ 0b1100 ^ 0b0110 = 0b0000
        assert_eq!(snap.checksum, 0b1010 ^ 0b1100 ^ 0b0110);
    }
}
