/// Block event kinds for history tracking.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockEventKind {
    /// A block was inserted into the canvas.
    Insert,
    /// A block was deleted from the canvas.
    Delete,
    /// A block was moved to a new position.
    Move,
    /// A block was resized.
    Resize,
    /// Two blocks were connected.
    Connect,
    /// Two blocks were disconnected.
    Disconnect,
}

impl BlockEventKind {
    /// Returns the event name as a static string.
    pub fn event_name(&self) -> &str {
        match self {
            BlockEventKind::Insert => "insert",
            BlockEventKind::Delete => "delete",
            BlockEventKind::Move => "move",
            BlockEventKind::Resize => "resize",
            BlockEventKind::Connect => "connect",
            BlockEventKind::Disconnect => "disconnect",
        }
    }

    /// Returns true for structural events: Insert, Delete, Connect, Disconnect.
    pub fn is_structural(&self) -> bool {
        matches!(
            self,
            BlockEventKind::Insert
                | BlockEventKind::Delete
                | BlockEventKind::Connect
                | BlockEventKind::Disconnect
        )
    }
}

/// A single recorded block event for undo/redo history.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct BlockEvent {
    /// Unique event identifier.
    pub id: u64,
    /// The kind of event.
    pub kind: BlockEventKind,
    /// The block this event applies to.
    pub block_id: u64,
    /// Serialized payload for reconstructing state.
    pub payload: String,
}

impl BlockEvent {
    /// Constructs a new `BlockEvent`.
    pub fn new(
        id: u64,
        kind: BlockEventKind,
        block_id: u64,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            id,
            kind,
            block_id,
            payload: payload.into(),
        }
    }

    /// Returns the inverse kind for undo operations.
    ///
    /// - Insert ↔ Delete
    /// - Connect ↔ Disconnect
    /// - Move → Move, Resize → Resize (self-inverse)
    pub fn inverse_kind(&self) -> BlockEventKind {
        match &self.kind {
            BlockEventKind::Insert => BlockEventKind::Delete,
            BlockEventKind::Delete => BlockEventKind::Insert,
            BlockEventKind::Connect => BlockEventKind::Disconnect,
            BlockEventKind::Disconnect => BlockEventKind::Connect,
            BlockEventKind::Move => BlockEventKind::Move,
            BlockEventKind::Resize => BlockEventKind::Resize,
        }
    }
}

/// An undo/redo history stack for block events.
#[allow(missing_docs)]
pub struct HistoryStack {
    /// Events available to undo (most recent last).
    pub undo_stack: Vec<BlockEvent>,
    /// Events available to redo (most recent last).
    pub redo_stack: Vec<BlockEvent>,
    /// Maximum number of events to retain in the undo stack.
    pub max_history: usize,
}

impl HistoryStack {
    /// Creates a new `HistoryStack` with the given maximum history size.
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// Pushes an event onto the undo stack, clears redo, and enforces max_history.
    pub fn push(&mut self, event: BlockEvent) {
        self.redo_stack.clear();
        self.undo_stack.push(event);
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Pops the most recent event from the undo stack, pushes it onto redo, and returns it.
    pub fn undo(&mut self) -> Option<BlockEvent> {
        let event = self.undo_stack.pop()?;
        self.redo_stack.push(event.clone());
        Some(event)
    }

    /// Pops the most recent event from the redo stack, pushes it onto undo, and returns it.
    pub fn redo(&mut self) -> Option<BlockEvent> {
        let event = self.redo_stack.pop()?;
        self.undo_stack.push(event.clone());
        Some(event)
    }

    /// Returns the number of events available to undo.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Returns the number of events available to redo.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Returns true if there are events available to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns true if there are events available to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

#[cfg(test)]
mod history_tests {
    use super::*;

    fn make_event(id: u64, kind: BlockEventKind) -> BlockEvent {
        BlockEvent::new(id, kind, 1, "payload")
    }

    #[test]
    fn block_event_kind_is_structural() {
        assert!(BlockEventKind::Insert.is_structural());
        assert!(BlockEventKind::Delete.is_structural());
        assert!(BlockEventKind::Connect.is_structural());
        assert!(BlockEventKind::Disconnect.is_structural());
        assert!(!BlockEventKind::Move.is_structural());
        assert!(!BlockEventKind::Resize.is_structural());
    }

    #[test]
    fn block_event_inverse_kind_insert_to_delete() {
        let event = make_event(1, BlockEventKind::Insert);
        assert_eq!(event.inverse_kind(), BlockEventKind::Delete);
    }

    #[test]
    fn block_event_inverse_kind_connect_to_disconnect() {
        let event = make_event(2, BlockEventKind::Connect);
        assert_eq!(event.inverse_kind(), BlockEventKind::Disconnect);
    }

    #[test]
    fn history_stack_push_increments_undo_count() {
        let mut stack = HistoryStack::new(100);
        assert_eq!(stack.undo_count(), 0);
        stack.push(make_event(1, BlockEventKind::Insert));
        assert_eq!(stack.undo_count(), 1);
        stack.push(make_event(2, BlockEventKind::Move));
        assert_eq!(stack.undo_count(), 2);
    }

    #[test]
    fn history_stack_push_clears_redo() {
        let mut stack = HistoryStack::new(100);
        stack.push(make_event(1, BlockEventKind::Insert));
        stack.undo();
        assert_eq!(stack.redo_count(), 1);
        stack.push(make_event(2, BlockEventKind::Move));
        assert_eq!(stack.redo_count(), 0, "push must clear redo stack");
    }

    #[test]
    fn history_stack_undo_decrements_undo_count() {
        let mut stack = HistoryStack::new(100);
        stack.push(make_event(1, BlockEventKind::Insert));
        stack.push(make_event(2, BlockEventKind::Move));
        assert_eq!(stack.undo_count(), 2);
        stack.undo();
        assert_eq!(stack.undo_count(), 1);
    }

    #[test]
    fn history_stack_undo_increments_redo_count() {
        let mut stack = HistoryStack::new(100);
        stack.push(make_event(1, BlockEventKind::Insert));
        assert_eq!(stack.redo_count(), 0);
        stack.undo();
        assert_eq!(stack.redo_count(), 1);
    }

    #[test]
    fn history_stack_redo_moves_back_to_undo() {
        let mut stack = HistoryStack::new(100);
        stack.push(make_event(1, BlockEventKind::Insert));
        stack.undo();
        assert_eq!(stack.undo_count(), 0);
        assert_eq!(stack.redo_count(), 1);
        let event = stack.redo();
        assert!(event.is_some());
        assert_eq!(stack.undo_count(), 1);
        assert_eq!(stack.redo_count(), 0);
    }

    #[test]
    fn history_stack_can_undo_can_redo_correct_state() {
        let mut stack = HistoryStack::new(100);
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());

        stack.push(make_event(1, BlockEventKind::Insert));
        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        stack.undo();
        assert!(!stack.can_undo());
        assert!(stack.can_redo());

        stack.redo();
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }
}
