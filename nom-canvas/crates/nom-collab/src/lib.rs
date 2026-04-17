#![deny(unsafe_code)]

/// Unique identifier for a peer in the collaborative session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerId(pub u64);

/// Lamport-style operation identifier: (counter, peer) pairs sort deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpId {
    pub peer: PeerId,
    pub counter: u64,
}

impl PartialOrd for OpId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OpId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.counter
            .cmp(&other.counter)
            .then_with(|| self.peer.0.cmp(&other.peer.0))
    }
}

/// The payload of an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpKind {
    Insert { pos: usize, text: String },
    Delete { pos: usize, len: usize },
    SetMeta { key: String, value: String },
}

/// A single collaborative operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    pub id: OpId,
    pub kind: OpKind,
}

/// Simple CRDT document that maintains a text buffer and an ordered op log.
pub struct DocState {
    peer: PeerId,
    counter: u64,
    text: String,
    op_log: Vec<Op>,
}

impl DocState {
    /// Create a new empty document owned by `peer`.
    pub fn new(peer: PeerId) -> Self {
        Self {
            peer,
            counter: 0,
            text: String::new(),
            op_log: Vec::new(),
        }
    }

    /// Generate the next [`OpId`] for a locally-authored operation.
    fn next_id(&mut self) -> OpId {
        self.counter += 1;
        OpId {
            peer: self.peer,
            counter: self.counter,
        }
    }

    /// Apply a single operation to the document, advancing the Lamport clock if
    /// the incoming counter is ahead of the local one.
    pub fn apply(&mut self, op: Op) {
        // Advance local counter to stay ahead of incoming ops.
        if op.id.counter > self.counter {
            self.counter = op.id.counter;
        }
        self.apply_to_text(&op);
        self.op_log.push(op);
    }

    /// Apply the text mutation described by `op` to the internal buffer.
    fn apply_to_text(&mut self, op: &Op) {
        match &op.kind {
            OpKind::Insert { pos, text } => {
                let pos = (*pos).min(self.text.len());
                self.text.insert_str(pos, text);
            }
            OpKind::Delete { pos, len } => {
                let start = (*pos).min(self.text.len());
                let end = (start + len).min(self.text.len());
                self.text.drain(start..end);
            }
            OpKind::SetMeta { .. } => {
                // Metadata ops do not mutate the text buffer.
            }
        }
    }

    /// Merge a batch of remote operations.
    ///
    /// Operations are sorted by `(counter, peer.0)` before application so that
    /// concurrent ops from different peers always converge to the same order.
    pub fn merge(&mut self, mut ops: Vec<Op>) {
        ops.sort_by_key(|op| op.id);
        for op in ops {
            self.apply(op);
        }
    }

    /// Return the current document text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Return all operations in the order they were applied.
    pub fn op_log(&self) -> &[Op] {
        &self.op_log
    }

    /// Convenience: author a local insert op, apply it, and return a clone for
    /// broadcasting to remote peers.
    pub fn local_insert(&mut self, pos: usize, text: impl Into<String>) -> Op {
        let id = self.next_id();
        let op = Op {
            id,
            kind: OpKind::Insert {
                pos,
                text: text.into(),
            },
        };
        self.apply(op.clone());
        op
    }

    /// Convenience: author a local delete op, apply it, and return a clone for
    /// broadcasting to remote peers.
    pub fn local_delete(&mut self, pos: usize, len: usize) -> Op {
        let id = self.next_id();
        let op = Op {
            id,
            kind: OpKind::Delete { pos, len },
        };
        self.apply(op.clone());
        op
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_insert(peer: u64, counter: u64, pos: usize, text: &str) -> Op {
        Op {
            id: OpId {
                peer: PeerId(peer),
                counter,
            },
            kind: OpKind::Insert {
                pos,
                text: text.to_string(),
            },
        }
    }

    fn make_delete(peer: u64, counter: u64, pos: usize, len: usize) -> Op {
        Op {
            id: OpId {
                peer: PeerId(peer),
                counter,
            },
            kind: OpKind::Delete { pos, len },
        }
    }

    #[test]
    fn collab_insert_op() {
        let mut doc = DocState::new(PeerId(1));
        let op = doc.local_insert(0, "hello");
        assert_eq!(doc.text(), "hello");
        assert_eq!(doc.op_log().len(), 1);
        assert_eq!(op.id.peer, PeerId(1));
        assert_eq!(op.id.counter, 1);

        doc.local_insert(5, " world");
        assert_eq!(doc.text(), "hello world");
        assert_eq!(doc.op_log().len(), 2);
    }

    #[test]
    fn collab_delete_op() {
        let mut doc = DocState::new(PeerId(2));
        doc.local_insert(0, "hello world");
        assert_eq!(doc.text(), "hello world");

        doc.local_delete(5, 6); // remove " world"
        assert_eq!(doc.text(), "hello");
        assert_eq!(doc.op_log().len(), 2);
    }

    #[test]
    fn collab_merge_two_peers() {
        // Peer A starts with "foo"
        let mut peer_a = DocState::new(PeerId(1));
        let op_a1 = peer_a.local_insert(0, "foo");

        // Peer B starts empty, receives A's op, then appends " bar"
        let mut peer_b = DocState::new(PeerId(2));
        peer_b.merge(vec![op_a1]);
        assert_eq!(peer_b.text(), "foo");

        let op_b1 = peer_b.local_insert(3, " bar");

        // A receives B's op
        peer_a.merge(vec![op_b1]);
        assert_eq!(peer_a.text(), "foo bar");

        // Both peers now have the same text
        assert_eq!(peer_a.text(), peer_b.text());
    }

    #[test]
    fn collab_op_order_deterministic() {
        // Two peers concurrently insert at position 0 with the same counter.
        // Sort key: (counter=1, peer.0) → peer 1 comes before peer 2.
        let op_peer2 = make_insert(2, 1, 0, "B");
        let op_peer1 = make_insert(1, 1, 0, "A");

        let mut doc = DocState::new(PeerId(99));

        // Deliver in reverse order — merge should sort them first.
        doc.merge(vec![op_peer2.clone(), op_peer1.clone()]);

        // After sort: peer1(counter=1) < peer2(counter=1 but peer.0=2)
        // op_peer1 inserts "A" at 0 → "A"
        // op_peer2 inserts "B" at 0 → "BA"
        assert_eq!(doc.text(), "BA");

        // A fresh doc applying in the other order should produce the same result.
        let mut doc2 = DocState::new(PeerId(99));
        doc2.merge(vec![op_peer1, op_peer2]);
        assert_eq!(doc2.text(), doc.text());
    }

    #[test]
    fn collab_set_meta_does_not_change_text() {
        let mut doc = DocState::new(PeerId(3));
        doc.local_insert(0, "hello");
        let meta_op = Op {
            id: OpId {
                peer: PeerId(3),
                counter: 99,
            },
            kind: OpKind::SetMeta {
                key: "title".to_string(),
                value: "My Doc".to_string(),
            },
        };
        doc.apply(meta_op);
        assert_eq!(doc.text(), "hello");
        assert_eq!(doc.op_log().len(), 2);
    }

    #[test]
    fn collab_delete_clamps_to_buffer_end() {
        let mut doc = DocState::new(PeerId(4));
        doc.local_insert(0, "hi");
        // Delete beyond the end — should not panic, just remove to end.
        doc.apply(make_delete(4, 10, 1, 100));
        assert_eq!(doc.text(), "h");
    }
}
