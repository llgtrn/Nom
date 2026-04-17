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

/// RGA position: insert ops reference a left-anchor OpId (or None for head).
/// This makes concurrent inserts commutative — order is determined by (counter, peer.id).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RgaPos {
    Head,
    After(OpId),
}

/// The payload of an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpKind {
    /// Insert `text` after the anchor position.
    Insert { pos: RgaPos, text: String },
    /// Tombstone the op with this id (logical delete).
    Delete { target: OpId },
    SetMeta { key: String, value: String },
}

/// A single collaborative operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    pub id: OpId,
    pub kind: OpKind,
}

/// RGA node in the sequence: holds an op id, text content, and a tombstone flag.
#[derive(Debug, Clone)]
struct RgaNode {
    id: OpId,
    text: String,
    tombstoned: bool,
}

/// CRDT document using RGA (Replicated Growable Array) for convergent text editing.
///
/// Every Insert names a left-anchor (`RgaPos`). On apply the node is placed
/// immediately after that anchor, with ties broken by (counter, peer.0) descending
/// so higher-priority ops end up to the left of lower-priority concurrent siblings.
pub struct DocState {
    peer: PeerId,
    counter: u64,
    /// The ordered sequence of RGA nodes (tombstoned nodes stay in place).
    nodes: Vec<RgaNode>,
    /// Full op log — used for merge idempotency checks.
    op_log: Vec<Op>,
}

impl DocState {
    /// Create a new empty document owned by `peer`.
    pub fn new(peer: PeerId) -> Self {
        Self {
            peer,
            counter: 0,
            nodes: Vec::new(),
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
        self.apply_rga(&op);
        self.op_log.push(op);
    }

    /// Apply RGA semantics for the operation.
    fn apply_rga(&mut self, op: &Op) {
        match &op.kind {
            OpKind::Insert { pos, text } => {
                // Find the index of the anchor node (or -1 for Head).
                let anchor_idx: Option<usize> = match pos {
                    RgaPos::Head => None,
                    RgaPos::After(anchor_id) => {
                        self.nodes.iter().position(|n| &n.id == anchor_id)
                    }
                };

                // The insertion point starts right after the anchor.
                let start = anchor_idx.map(|i| i + 1).unwrap_or(0);

                // Walk forward past any concurrent siblings that have higher priority.
                // A sibling is a node inserted at the same anchor that must come before
                // this op. We use (counter, peer.0) descending as the tiebreak: higher
                // counter (or same counter + higher peer id) wins and stays to the left.
                let mut insert_at = start;
                for i in start..self.nodes.len() {
                    let sibling = &self.nodes[i];
                    // A sibling shares the same anchor; check by comparing the op that
                    // was inserted after the same position. We detect this by checking
                    // whether the node at `i` has its own anchor equal to our anchor.
                    // We look this up from the op_log.
                    let sibling_anchor = self.node_anchor(&sibling.id);
                    if sibling_anchor.as_ref() != Some(pos) {
                        // This node was not inserted at the same anchor; stop scanning.
                        break;
                    }
                    // Both ops share the same anchor. Higher-priority op goes left.
                    // Priority: descending by (counter, peer.0).
                    if sibling.id > op.id {
                        // Sibling has higher priority — it stays to the left; advance.
                        insert_at = i + 1;
                    } else {
                        // Our op has higher or equal priority — insert here.
                        break;
                    }
                }

                self.nodes.insert(
                    insert_at,
                    RgaNode {
                        id: op.id,
                        text: text.clone(),
                        tombstoned: false,
                    },
                );
            }
            OpKind::Delete { target } => {
                if let Some(node) = self.nodes.iter_mut().find(|n| &n.id == target) {
                    node.tombstoned = true;
                    node.text.clear();
                }
            }
            OpKind::SetMeta { .. } => {
                // Metadata ops do not mutate the text buffer.
            }
        }
    }

    /// Look up the `RgaPos` anchor of a node by its id, using the op_log.
    fn node_anchor(&self, id: &OpId) -> Option<RgaPos> {
        self.op_log.iter().find_map(|op| {
            if &op.id == id {
                if let OpKind::Insert { pos, .. } = &op.kind {
                    return Some(pos.clone());
                }
            }
            None
        })
    }

    /// Idempotently merge all ops from `other` into this document.
    ///
    /// This satisfies the CRDT merge contract: commutativity and idempotency —
    /// merging the same op twice has no additional effect.
    pub fn merge(&mut self, other: &DocState) {
        // Collect ops not yet in our log, sorted by OpId for deterministic replay.
        let mut new_ops: Vec<Op> = other
            .op_log
            .iter()
            .filter(|o| !self.op_log.iter().any(|mine| mine.id == o.id))
            .cloned()
            .collect();
        new_ops.sort_by_key(|o| o.id);
        for op in new_ops {
            self.apply(op);
        }
    }

    /// Return the current document text (tombstoned nodes excluded).
    pub fn text(&self) -> String {
        self.nodes
            .iter()
            .filter(|n| !n.tombstoned)
            .map(|n| n.text.as_str())
            .collect()
    }

    /// Return all operations in the order they were applied.
    pub fn op_log(&self) -> &[Op] {
        &self.op_log
    }

    /// Convenience: author a local insert op, apply it, and return a clone for
    /// broadcasting to remote peers.
    pub fn local_insert(&mut self, pos: RgaPos, text: impl Into<String>) -> Op {
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

    /// Convenience: author a local delete op targeting the given op id,
    /// apply it, and return a clone for broadcasting to remote peers.
    pub fn local_delete(&mut self, target: OpId) -> Op {
        let id = self.next_id();
        let op = Op {
            id,
            kind: OpKind::Delete { target },
        };
        self.apply(op.clone());
        op
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ─────────────────────────────────────────────────────────────

    fn make_insert(peer: u64, counter: u64, pos: RgaPos, text: &str) -> Op {
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

    fn make_delete(peer: u64, counter: u64, target_peer: u64, target_counter: u64) -> Op {
        Op {
            id: OpId {
                peer: PeerId(peer),
                counter,
            },
            kind: OpKind::Delete {
                target: OpId {
                    peer: PeerId(target_peer),
                    counter: target_counter,
                },
            },
        }
    }

    // ── basic ops ───────────────────────────────────────────────────────────

    #[test]
    fn collab_insert_op() {
        let mut doc = DocState::new(PeerId(1));
        let op = doc.local_insert(RgaPos::Head, "hello");
        assert_eq!(doc.text(), "hello");
        assert_eq!(doc.op_log().len(), 1);
        assert_eq!(op.id.peer, PeerId(1));
        assert_eq!(op.id.counter, 1);

        // Insert " world" after the first op.
        doc.local_insert(RgaPos::After(op.id), " world");
        assert_eq!(doc.text(), "hello world");
        assert_eq!(doc.op_log().len(), 2);
    }

    #[test]
    fn collab_delete_op() {
        let mut doc = DocState::new(PeerId(2));
        let op = doc.local_insert(RgaPos::Head, "hello world");
        assert_eq!(doc.text(), "hello world");

        doc.local_delete(op.id);
        // The whole node is tombstoned.
        assert_eq!(doc.text(), "");
        assert_eq!(doc.op_log().len(), 2);
    }

    #[test]
    fn collab_set_meta_does_not_change_text() {
        let mut doc = DocState::new(PeerId(3));
        doc.local_insert(RgaPos::Head, "hello");
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

    // ── merge / convergence ─────────────────────────────────────────────────

    #[test]
    fn collab_merge_two_peers() {
        // Peer A starts with "foo"
        let mut peer_a = DocState::new(PeerId(1));
        let op_a1 = peer_a.local_insert(RgaPos::Head, "foo");

        // Peer B starts empty, receives A's op, then appends " bar"
        let mut peer_b = DocState::new(PeerId(2));
        peer_b.merge(&{
            let mut tmp = DocState::new(PeerId(1));
            tmp.apply(op_a1.clone());
            tmp
        });
        assert_eq!(peer_b.text(), "foo");

        let op_b1 = peer_b.local_insert(RgaPos::After(op_a1.id), " bar");

        // A receives B's op
        peer_a.merge(&{
            let mut tmp = DocState::new(PeerId(2));
            tmp.apply(op_a1.clone());
            tmp.apply(op_b1.clone());
            tmp
        });
        assert_eq!(peer_a.text(), "foo bar");

        // Both peers now have the same text
        assert_eq!(peer_a.text(), peer_b.text());
    }

    #[test]
    fn crdt_convergence_concurrent_inserts() {
        // Peer A and Peer B both insert at Head concurrently (no shared history).
        let mut peer_a = DocState::new(PeerId(1));
        let op_a = peer_a.local_insert(RgaPos::Head, "hello");

        let mut peer_b = DocState::new(PeerId(2));
        let op_b = peer_b.local_insert(RgaPos::Head, "world");

        // Cross-merge: A gets B's op, B gets A's op.
        peer_a.merge(&peer_b);
        peer_b.merge(&peer_a);

        // Both must converge to the same text.
        assert_eq!(peer_a.text(), peer_b.text());

        // Tiebreak: same counter (1), peer 2 > peer 1 → op_b sorts higher → "world"
        // appears to the LEFT of "hello" (higher-priority op wins the left position).
        // i.e. descending (counter, peer.0): peer 2 wins → "worldhello"
        assert_eq!(peer_a.text(), "worldhello");

        // op_a and op_b both have counter=1. op_b.id > op_a.id (peer 2 > peer 1).
        // So op_b has higher priority and is inserted at position 0; op_a follows.
        let _ = (op_a, op_b); // used to author the ops
    }

    #[test]
    fn crdt_merge_idempotent() {
        // Merging the same ops twice must not change the document.
        let mut peer_a = DocState::new(PeerId(1));
        peer_a.local_insert(RgaPos::Head, "hello");

        let mut peer_b = DocState::new(PeerId(2));
        peer_b.merge(&peer_a);
        let text_after_first = peer_b.text();
        let log_len_after_first = peer_b.op_log().len();

        peer_b.merge(&peer_a); // merge again — must be idempotent
        assert_eq!(peer_b.text(), text_after_first);
        assert_eq!(peer_b.op_log().len(), log_len_after_first);
    }

    #[test]
    fn crdt_merge_commutative() {
        // Applying ops in different orders must produce the same result.
        let op1 = make_insert(1, 1, RgaPos::Head, "A");
        let op2 = make_insert(2, 1, RgaPos::Head, "B");

        let mut doc_ab = DocState::new(PeerId(99));
        doc_ab.apply(op1.clone());
        doc_ab.apply(op2.clone());

        let mut doc_ba = DocState::new(PeerId(99));
        doc_ba.apply(op2.clone());
        doc_ba.apply(op1.clone());

        // Both docs must have the same text regardless of arrival order.
        assert_eq!(doc_ab.text(), doc_ba.text());
    }

    #[test]
    fn collab_delete_marks_tombstone() {
        let mut doc = DocState::new(PeerId(4));
        let op = doc.local_insert(RgaPos::Head, "hi");
        let del = make_delete(4, 10, op.id.peer.0, op.id.counter);
        doc.apply(del);
        assert_eq!(doc.text(), "");
    }

    // ── new coverage tests ───────────────────────────────────────────────────

    #[test]
    fn crdt_delete_marks_tombstone() {
        let mut doc = DocState::new(PeerId(10));
        let insert_op = doc.local_insert(RgaPos::Head, "delete_me");
        assert_eq!(doc.text(), "delete_me");

        let _del_op = doc.local_delete(insert_op.id);

        // visible text is empty after deletion
        assert_eq!(doc.text(), "");
        // two ops in log: insert + delete
        assert_eq!(doc.op_log().len(), 2);
        // the delete op targets the insert's id
        match &doc.op_log()[1].kind {
            OpKind::Delete { target } => assert_eq!(*target, insert_op.id),
            other => panic!("expected Delete, got {other:?}"),
        }
    }

    #[test]
    fn crdt_visible_text_excludes_tombstones() {
        let mut doc = DocState::new(PeerId(11));
        let op_a = doc.local_insert(RgaPos::Head, "hello");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), " world");
        assert_eq!(doc.text(), "hello world");

        // Delete the first segment — " world" survives.
        doc.local_delete(op_a.id);
        assert_eq!(doc.text(), " world");

        // Delete the second segment — nothing visible.
        doc.local_delete(op_b.id);
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn crdt_insert_at_head_works() {
        let mut doc = DocState::new(PeerId(12));
        let op = doc.local_insert(RgaPos::Head, "hello");

        assert_eq!(doc.text(), "hello");
        assert_eq!(op.id.peer, PeerId(12));
        assert_eq!(op.id.counter, 1);
        assert_eq!(doc.op_log().len(), 1);
        match &op.kind {
            OpKind::Insert { pos, text } => {
                assert_eq!(*pos, RgaPos::Head);
                assert_eq!(text, "hello");
            }
            other => panic!("expected Insert, got {other:?}"),
        }
    }

    #[test]
    fn crdt_concurrent_delete_and_insert_converge() {
        // Setup: both peers start with a shared node X ("base").
        let mut peer_a = DocState::new(PeerId(1));
        let op_x = peer_a.local_insert(RgaPos::Head, "base");

        let mut peer_b = DocState::new(PeerId(2));
        peer_b.apply(op_x.clone());
        assert_eq!(peer_b.text(), "base");

        // Concurrently: A deletes X, B inserts "extra" after X.
        let op_del = peer_a.local_delete(op_x.id);
        let op_ins = peer_b.local_insert(RgaPos::After(op_x.id), "extra");

        // Merge both ways.
        peer_a.apply(op_ins.clone());
        peer_b.apply(op_del.clone());

        // Both peers must converge to identical text.
        assert_eq!(peer_a.text(), peer_b.text());
        // "base" is deleted; "extra" is alive.
        assert_eq!(peer_a.text(), "extra");
    }

    #[test]
    fn crdt_empty_document_text_is_empty() {
        let doc = DocState::new(PeerId(99));
        assert_eq!(doc.text(), "");
        assert_eq!(doc.op_log().len(), 0);
    }
}
