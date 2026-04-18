/// Undo/redo command stack for canvas operations.

/// Describes what a command did.
pub enum CommandKind {
    /// Move an element by a delta.
    MoveElement {
        /// Element identifier.
        id: String,
        /// X displacement in canvas units.
        dx: f32,
        /// Y displacement in canvas units.
        dy: f32,
    },
    /// Delete an element from the canvas.
    DeleteElement {
        /// Element identifier.
        id: String,
    },
    /// Add an element to the canvas.
    AddElement {
        /// Element identifier.
        id: String,
    },
}

/// A single reversible canvas operation.
pub struct Command {
    /// Human-readable description for debug/display.
    pub description: String,
    /// The operation kind and its parameters.
    pub kind: CommandKind,
}

/// Finite-depth undo/redo stack.
///
/// `push` appends a command to the history and clears the redo stack.
/// `undo` pops from history and pushes to the redo stack.
/// `redo` pops from the redo stack and pushes back to history.
pub struct CommandStack {
    history: Vec<Command>,
    future: Vec<Command>,
}

impl CommandStack {
    /// Creates an empty command stack.
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            future: Vec::new(),
        }
    }

    /// Pushes a new command onto the history stack and clears the redo stack.
    pub fn push(&mut self, cmd: Command) {
        self.history.push(cmd);
        self.future.clear();
    }

    /// Undoes the most recent command, returning it.
    ///
    /// Returns `None` when there is nothing to undo.
    pub fn undo(&mut self) -> Option<Command> {
        let cmd = self.history.pop()?;
        self.future.push(Command {
            description: cmd.description.clone(),
            kind: match &cmd.kind {
                CommandKind::MoveElement { id, dx, dy } => CommandKind::MoveElement {
                    id: id.clone(),
                    dx: *dx,
                    dy: *dy,
                },
                CommandKind::DeleteElement { id } => CommandKind::DeleteElement { id: id.clone() },
                CommandKind::AddElement { id } => CommandKind::AddElement { id: id.clone() },
            },
        });
        Some(cmd)
    }

    /// Redoes the most recently undone command, returning it.
    ///
    /// Returns `None` when there is nothing to redo.
    pub fn redo(&mut self) -> Option<Command> {
        let cmd = self.future.pop()?;
        self.history.push(Command {
            description: cmd.description.clone(),
            kind: match &cmd.kind {
                CommandKind::MoveElement { id, dx, dy } => CommandKind::MoveElement {
                    id: id.clone(),
                    dx: *dx,
                    dy: *dy,
                },
                CommandKind::DeleteElement { id } => CommandKind::DeleteElement { id: id.clone() },
                CommandKind::AddElement { id } => CommandKind::AddElement { id: id.clone() },
            },
        });
        Some(cmd)
    }

    /// Returns `true` when there is at least one command that can be undone.
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    /// Returns `true` when there is at least one command that can be redone.
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Returns the number of commands in the history stack.
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Returns `true` when the history stack is empty.
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Clears both the history and future stacks.
    pub fn clear(&mut self) {
        self.history.clear();
        self.future.clear();
    }
}

impl Default for CommandStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn move_cmd(id: &str, dx: f32, dy: f32) -> Command {
        Command {
            description: format!("move {id}"),
            kind: CommandKind::MoveElement {
                id: id.to_string(),
                dx,
                dy,
            },
        }
    }

    fn add_cmd(id: &str) -> Command {
        Command {
            description: format!("add {id}"),
            kind: CommandKind::AddElement { id: id.to_string() },
        }
    }

    fn delete_cmd(id: &str) -> Command {
        Command {
            description: format!("delete {id}"),
            kind: CommandKind::DeleteElement { id: id.to_string() },
        }
    }

    // ── basic stack invariants ────────────────────────────────────────────────

    #[test]
    fn new_stack_is_empty() {
        let stack = CommandStack::new();
        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn push_increments_len() {
        let mut stack = CommandStack::new();
        stack.push(move_cmd("a", 10.0, 0.0));
        assert_eq!(stack.len(), 1);
        assert!(stack.can_undo());
    }

    #[test]
    fn push_multiple_increments_len() {
        let mut stack = CommandStack::new();
        stack.push(move_cmd("a", 1.0, 0.0));
        stack.push(add_cmd("b"));
        stack.push(delete_cmd("c"));
        assert_eq!(stack.len(), 3);
    }

    // ── undo behaviour ────────────────────────────────────────────────────────

    #[test]
    fn undo_returns_most_recent_command() {
        let mut stack = CommandStack::new();
        stack.push(move_cmd("x", 5.0, 0.0));
        stack.push(add_cmd("y"));
        let cmd = stack.undo().unwrap();
        assert_eq!(cmd.description, "add y");
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn undo_empty_stack_returns_none() {
        let mut stack = CommandStack::new();
        assert!(stack.undo().is_none());
    }

    #[test]
    fn undo_decrements_len() {
        let mut stack = CommandStack::new();
        stack.push(move_cmd("a", 0.0, 5.0));
        stack.push(move_cmd("b", 1.0, 1.0));
        stack.undo();
        assert_eq!(stack.len(), 1);
        stack.undo();
        assert_eq!(stack.len(), 0);
        assert!(!stack.can_undo());
    }

    #[test]
    fn undo_enables_redo() {
        let mut stack = CommandStack::new();
        stack.push(add_cmd("elem"));
        assert!(!stack.can_redo());
        stack.undo();
        assert!(stack.can_redo());
    }

    // ── redo behaviour ────────────────────────────────────────────────────────

    #[test]
    fn redo_restores_undone_command() {
        let mut stack = CommandStack::new();
        stack.push(move_cmd("z", 3.0, -1.0));
        stack.undo();
        let cmd = stack.redo().unwrap();
        assert_eq!(cmd.description, "move z");
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn redo_empty_future_returns_none() {
        let mut stack = CommandStack::new();
        assert!(stack.redo().is_none());
    }

    #[test]
    fn redo_cleared_after_new_push() {
        let mut stack = CommandStack::new();
        stack.push(add_cmd("a"));
        stack.push(add_cmd("b"));
        stack.undo(); // b goes to future
        assert!(stack.can_redo());
        stack.push(add_cmd("c")); // future must be cleared
        assert!(!stack.can_redo(), "redo must be cleared after a new push");
    }

    #[test]
    fn undo_redo_round_trip_preserves_description() {
        let mut stack = CommandStack::new();
        stack.push(delete_cmd("node42"));
        let description_before = "delete node42";
        stack.undo();
        let cmd = stack.redo().unwrap();
        assert_eq!(cmd.description, description_before);
    }

    // ── clear behaviour ───────────────────────────────────────────────────────

    #[test]
    fn clear_empties_history_and_future() {
        let mut stack = CommandStack::new();
        stack.push(add_cmd("a"));
        stack.push(add_cmd("b"));
        stack.undo(); // moves b to future
        assert!(stack.can_undo());
        assert!(stack.can_redo());
        stack.clear();
        assert_eq!(stack.len(), 0);
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn clear_on_empty_stack_is_safe() {
        let mut stack = CommandStack::new();
        stack.clear();
        assert_eq!(stack.len(), 0);
    }

    // ── can_undo / can_redo correctness ───────────────────────────────────────

    #[test]
    fn can_undo_false_when_empty() {
        let stack = CommandStack::new();
        assert!(!stack.can_undo());
    }

    #[test]
    fn can_redo_false_when_nothing_undone() {
        let mut stack = CommandStack::new();
        stack.push(add_cmd("x"));
        assert!(!stack.can_redo());
    }

    #[test]
    fn multiple_undos_then_redo_sequence() {
        let mut stack = CommandStack::new();
        stack.push(add_cmd("a"));
        stack.push(add_cmd("b"));
        stack.push(add_cmd("c"));
        stack.undo(); // undo c
        stack.undo(); // undo b
        assert_eq!(stack.len(), 1);
        assert!(stack.can_redo());
        // redo b
        let r1 = stack.redo().unwrap();
        assert_eq!(r1.description, "add b");
        assert_eq!(stack.len(), 2);
        // redo c
        let r2 = stack.redo().unwrap();
        assert_eq!(r2.description, "add c");
        assert_eq!(stack.len(), 3);
        assert!(!stack.can_redo());
    }

    // ── MoveElement kind ─────────────────────────────────────────────────────

    #[test]
    fn move_command_kind_preserves_delta() {
        let mut stack = CommandStack::new();
        stack.push(move_cmd("elem1", 12.5, -3.0));
        let cmd = stack.undo().unwrap();
        if let CommandKind::MoveElement { id, dx, dy } = cmd.kind {
            assert_eq!(id, "elem1");
            assert!((dx - 12.5).abs() < 1e-6);
            assert!((dy - (-3.0)).abs() < 1e-6);
        } else {
            panic!("expected MoveElement kind");
        }
    }
}
