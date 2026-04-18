/// The kind of CRDT operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpKind {
    Insert,
    Delete,
    Move,
    Annotate,
}

/// A single CRDT operation with metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    pub id: String,
    pub kind: OpKind,
    pub position: u32,
    pub content: String,
    pub author: String,
    pub timestamp: u64,
}

impl Op {
    pub fn new_insert(id: &str, pos: u32, content: &str, author: &str, ts: u64) -> Self {
        Self {
            id: id.to_string(),
            kind: OpKind::Insert,
            position: pos,
            content: content.to_string(),
            author: author.to_string(),
            timestamp: ts,
        }
    }

    pub fn new_delete(id: &str, pos: u32, author: &str, ts: u64) -> Self {
        Self {
            id: id.to_string(),
            kind: OpKind::Delete,
            position: pos,
            content: String::new(),
            author: author.to_string(),
            timestamp: ts,
        }
    }

    pub fn is_insert(&self) -> bool {
        self.kind == OpKind::Insert
    }

    pub fn is_delete(&self) -> bool {
        self.kind == OpKind::Delete
    }
}

/// An append-only log of CRDT operations with a logical clock.
#[derive(Debug, Default)]
pub struct OpLog {
    pub ops: Vec<Op>,
    pub clock: u64,
}

impl OpLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an operation, increment the logical clock, and return the new clock value.
    pub fn push(&mut self, op: Op) -> u64 {
        self.ops.push(op);
        self.clock += 1;
        self.clock
    }

    /// Return all operations authored by `author`.
    pub fn ops_by_author<'a>(&'a self, author: &str) -> Vec<&'a Op> {
        self.ops.iter().filter(|o| o.author == author).collect()
    }

    /// Total number of operations in the log.
    pub fn op_count(&self) -> usize {
        self.ops.len()
    }

    /// Number of Insert operations in the log.
    pub fn insert_count(&self) -> usize {
        self.ops.iter().filter(|o| o.is_insert()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_new_insert() {
        let op = Op::new_insert("i1", 3, "hello", "alice", 100);
        assert_eq!(op.id, "i1");
        assert_eq!(op.kind, OpKind::Insert);
        assert_eq!(op.position, 3);
        assert_eq!(op.content, "hello");
        assert_eq!(op.author, "alice");
        assert_eq!(op.timestamp, 100);
    }

    #[test]
    fn op_new_delete() {
        let op = Op::new_delete("d1", 5, "bob", 200);
        assert_eq!(op.id, "d1");
        assert_eq!(op.kind, OpKind::Delete);
        assert_eq!(op.position, 5);
        assert!(op.content.is_empty());
        assert_eq!(op.author, "bob");
        assert_eq!(op.timestamp, 200);
    }

    #[test]
    fn op_is_insert() {
        let ins = Op::new_insert("i2", 0, "x", "alice", 1);
        let del = Op::new_delete("d2", 0, "alice", 2);
        assert!(ins.is_insert());
        assert!(!ins.is_delete());
        assert!(del.is_delete());
        assert!(!del.is_insert());
    }

    #[test]
    fn op_log_push() {
        let mut log = OpLog::new();
        let op = Op::new_insert("i3", 0, "a", "alice", 10);
        let clock = log.push(op);
        assert_eq!(clock, 1);
        assert_eq!(log.op_count(), 1);
        let op2 = Op::new_delete("d3", 0, "bob", 20);
        let clock2 = log.push(op2);
        assert_eq!(clock2, 2);
        assert_eq!(log.op_count(), 2);
    }

    #[test]
    fn ops_by_author() {
        let mut log = OpLog::new();
        log.push(Op::new_insert("i4", 0, "a", "alice", 1));
        log.push(Op::new_insert("i5", 1, "b", "bob", 2));
        log.push(Op::new_delete("d4", 0, "alice", 3));
        let alice_ops = log.ops_by_author("alice");
        assert_eq!(alice_ops.len(), 2);
        let bob_ops = log.ops_by_author("bob");
        assert_eq!(bob_ops.len(), 1);
    }

    #[test]
    fn insert_count() {
        let mut log = OpLog::new();
        log.push(Op::new_insert("i6", 0, "a", "alice", 1));
        log.push(Op::new_insert("i7", 1, "b", "alice", 2));
        log.push(Op::new_delete("d5", 0, "bob", 3));
        assert_eq!(log.insert_count(), 2);
        assert_eq!(log.op_count(), 3);
    }
}
