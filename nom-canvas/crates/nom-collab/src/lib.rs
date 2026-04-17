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

    // ── new coverage tests (Wave T) ──────────────────────────────────────────

    #[test]
    fn crdt_multiple_peers_converge_after_3_ops() {
        // Three peers each insert one char; all merge to the same text.
        let mut peer_a = DocState::new(PeerId(1));
        let op_a = peer_a.local_insert(RgaPos::Head, "A");

        let mut peer_b = DocState::new(PeerId(2));
        let op_b = peer_b.local_insert(RgaPos::Head, "B");

        let mut peer_c = DocState::new(PeerId(3));
        let op_c = peer_c.local_insert(RgaPos::Head, "C");

        // Full cross-merge: every peer gets the other two ops.
        for op in [op_b.clone(), op_c.clone()] {
            peer_a.apply(op);
        }
        for op in [op_a.clone(), op_c.clone()] {
            peer_b.apply(op);
        }
        for op in [op_a.clone(), op_b.clone()] {
            peer_c.apply(op);
        }

        // All three peers converge to identical text.
        assert_eq!(peer_a.text(), peer_b.text());
        assert_eq!(peer_b.text(), peer_c.text());
        // Three characters must be present (order deterministic but not asserted here).
        assert_eq!(peer_a.text().len(), 3);
        assert!(peer_a.text().contains('A'));
        assert!(peer_a.text().contains('B'));
        assert!(peer_a.text().contains('C'));
    }

    #[test]
    fn crdt_op_id_ordering_deterministic() {
        // Two ops with same counter, different peer ids — lower peer id sorts lower.
        let op_low = OpId { peer: PeerId(1), counter: 5 };
        let op_high = OpId { peer: PeerId(2), counter: 5 };

        // OpId Ord: counter first, then peer.0 ascending.
        assert!(op_low < op_high, "lower peer id must sort before higher peer id at equal counter");

        // Different counters: counter dominates.
        let op_counter_low = OpId { peer: PeerId(99), counter: 1 };
        let op_counter_high = OpId { peer: PeerId(1), counter: 10 };
        assert!(op_counter_low < op_counter_high, "lower counter must sort before higher counter");
    }

    #[test]
    fn crdt_text_preserves_insertion_order() {
        // Insert "A" at Head, then "B" After A → expected text "AB".
        let mut doc = DocState::new(PeerId(1));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        doc.local_insert(RgaPos::After(op_a.id), "B");

        assert_eq!(doc.text(), "AB", "sequential inserts must preserve left-to-right order");
    }

    #[test]
    fn crdt_merge_self_is_idempotent() {
        // Merging a doc with itself must not grow the op log or change text.
        let mut doc = DocState::new(PeerId(7));
        doc.local_insert(RgaPos::Head, "hello");
        doc.local_insert(RgaPos::Head, "world");

        let text_before = doc.text();
        let log_len_before = doc.op_log().len();

        // Clone the doc so we can pass a reference to merge() without borrowing issues.
        let snapshot_ops: Vec<Op> = doc.op_log().to_vec();
        let mut mirror = DocState::new(PeerId(7));
        for op in snapshot_ops {
            mirror.apply(op);
        }

        doc.merge(&mirror);

        assert_eq!(doc.text(), text_before, "merge-self must not change text");
        assert_eq!(doc.op_log().len(), log_len_before, "merge-self must not grow op log");
    }

    #[test]
    fn crdt_local_insert_increments_counter() {
        // Consecutive local_insert calls must produce strictly increasing OpId counters.
        let mut doc = DocState::new(PeerId(5));
        let op1 = doc.local_insert(RgaPos::Head, "x");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "y");
        let op3 = doc.local_insert(RgaPos::After(op2.id), "z");

        assert!(op1.id.counter < op2.id.counter, "op2 counter must be > op1");
        assert!(op2.id.counter < op3.id.counter, "op3 counter must be > op2");
        // Counters must be 1, 2, 3 for a fresh doc.
        assert_eq!(op1.id.counter, 1);
        assert_eq!(op2.id.counter, 2);
        assert_eq!(op3.id.counter, 3);
        assert_eq!(doc.text(), "xyz");
    }

    // ── additional coverage (wave T+1) ──────────────────────────────────────

    #[test]
    fn crdt_insert_and_lookup_via_text() {
        // Insert a token and confirm it appears in the document text.
        let mut doc = DocState::new(PeerId(20));
        doc.local_insert(RgaPos::Head, "canvas");
        assert_eq!(doc.text(), "canvas");
        assert!(doc.text().contains("canvas"));
    }

    #[test]
    fn crdt_delete_tombstones_node() {
        // Insert then delete; node must be tombstoned (text disappears).
        let mut doc = DocState::new(PeerId(21));
        let op = doc.local_insert(RgaPos::Head, "tombstone_me");
        assert_eq!(doc.text(), "tombstone_me");

        doc.local_delete(op.id);

        assert_eq!(doc.text(), "", "tombstoned text must not appear in doc.text()");
        // The delete op is recorded in the log.
        assert_eq!(doc.op_log().len(), 2);
        match &doc.op_log()[1].kind {
            OpKind::Delete { target } => assert_eq!(*target, op.id),
            other => panic!("second op must be Delete, got {other:?}"),
        }
    }

    #[test]
    fn crdt_merge_two_concurrent_inserts_converge() {
        // Peers A and B both insert at Head without knowing each other's op.
        let mut peer_a = DocState::new(PeerId(30));
        let op_a = peer_a.local_insert(RgaPos::Head, "X");

        let mut peer_b = DocState::new(PeerId(31));
        let op_b = peer_b.local_insert(RgaPos::Head, "Y");

        // Cross-merge.
        peer_a.apply(op_b.clone());
        peer_b.apply(op_a.clone());

        // Both docs must converge to the same text.
        assert_eq!(peer_a.text(), peer_b.text());
        // Both characters must be present.
        assert!(peer_a.text().contains('X'));
        assert!(peer_a.text().contains('Y'));
    }

    #[test]
    fn crdt_deleted_position_is_tombstoned_in_op_log() {
        // After local_delete the op_log carries a Delete op whose target matches
        // the insert's OpId — this is the tombstone record.
        let mut doc = DocState::new(PeerId(40));
        let insert_op = doc.local_insert(RgaPos::Head, "will_be_deleted");
        let del_op = doc.local_delete(insert_op.id);

        assert_eq!(del_op.id.peer, PeerId(40));
        // Delete op counter > insert op counter.
        assert!(del_op.id.counter > insert_op.id.counter);
        match del_op.kind {
            OpKind::Delete { target } => assert_eq!(target, insert_op.id),
            other => panic!("expected Delete kind, got {other:?}"),
        }
    }

    #[test]
    fn crdt_insert_then_delete_sequence_leaves_correct_content() {
        // Insert two segments; delete the first; verify only the second survives.
        let mut doc = DocState::new(PeerId(50));
        let op_first = doc.local_insert(RgaPos::Head, "first");
        let op_second = doc.local_insert(RgaPos::After(op_first.id), "_second");

        assert_eq!(doc.text(), "first_second");

        doc.local_delete(op_first.id);
        assert_eq!(doc.text(), "_second");

        // Deleting the second as well leaves empty string.
        doc.local_delete(op_second.id);
        assert_eq!(doc.text(), "");
    }

    // ── extended coverage (wave expand) ─────────────────────────────────────

    #[test]
    fn crdt_insert_at_head() {
        // Insert at RgaPos::Head on a fresh doc; verify id, text, pos.
        let mut doc = DocState::new(PeerId(100));
        let op = doc.local_insert(RgaPos::Head, "first");
        assert_eq!(doc.text(), "first");
        assert_eq!(op.id.peer, PeerId(100));
        assert_eq!(op.id.counter, 1);
        match &op.kind {
            OpKind::Insert { pos, text } => {
                assert_eq!(*pos, RgaPos::Head);
                assert_eq!(text, "first");
            }
            other => panic!("expected Insert, got {other:?}"),
        }
    }

    #[test]
    fn crdt_insert_multiple_ordering() {
        // Three sequential inserts; combined text must match insertion order.
        let mut doc = DocState::new(PeerId(101));
        let op1 = doc.local_insert(RgaPos::Head, "A");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "B");
        doc.local_insert(RgaPos::After(op2.id), "C");
        assert_eq!(doc.text(), "ABC");
        assert_eq!(doc.op_log().len(), 3);
    }

    #[test]
    fn crdt_delete_first_char() {
        // Delete the head node; only the remaining nodes survive.
        let mut doc = DocState::new(PeerId(102));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        doc.local_insert(RgaPos::After(op_b.id), "C");
        assert_eq!(doc.text(), "ABC");

        doc.local_delete(op_a.id);
        assert_eq!(doc.text(), "BC");
    }

    #[test]
    fn crdt_delete_last_char() {
        // Delete the tail node; leading nodes survive.
        let mut doc = DocState::new(PeerId(103));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        let op_c = doc.local_insert(RgaPos::After(op_b.id), "C");
        assert_eq!(doc.text(), "ABC");

        doc.local_delete(op_c.id);
        assert_eq!(doc.text(), "AB");
    }

    #[test]
    fn crdt_delete_middle_char() {
        // Delete the middle node; surrounding nodes are preserved and adjacent.
        let mut doc = DocState::new(PeerId(104));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        let op_c = doc.local_insert(RgaPos::After(op_b.id), "C");
        assert_eq!(doc.text(), "ABC");

        doc.local_delete(op_b.id);
        assert_eq!(doc.text(), "AC");
        // op_c is still live
        let _ = op_c;
    }

    #[test]
    fn crdt_insert_after_deleted_op() {
        // Even after a node is tombstoned, a subsequent insert anchored After it
        // must still appear in the text (the anchor position is logical, not visual).
        let mut doc = DocState::new(PeerId(105));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        doc.local_delete(op_a.id);
        assert_eq!(doc.text(), "");

        // Insert after the deleted op — the new node is live.
        doc.local_insert(RgaPos::After(op_a.id), "B");
        assert_eq!(doc.text(), "B");
    }

    #[test]
    fn crdt_concurrent_insert_same_pos_deterministic() {
        // Two peers concurrently insert at Head. Regardless of merge order the
        // result must be identical (CRDT commutativity).
        let mut peer_a = DocState::new(PeerId(110));
        let op_a = peer_a.local_insert(RgaPos::Head, "P");

        let mut peer_b = DocState::new(PeerId(111));
        let op_b = peer_b.local_insert(RgaPos::Head, "Q");

        // Merge A→B then B→A.
        let mut doc_ab = DocState::new(PeerId(200));
        doc_ab.apply(op_a.clone());
        doc_ab.apply(op_b.clone());

        let mut doc_ba = DocState::new(PeerId(200));
        doc_ba.apply(op_b.clone());
        doc_ba.apply(op_a.clone());

        assert_eq!(doc_ab.text(), doc_ba.text(),
            "concurrent inserts at same pos must converge regardless of apply order");
        assert_eq!(doc_ab.text().len(), 2);
    }

    #[test]
    fn crdt_len_excludes_tombstones() {
        // op_log length counts all ops (including deletes), but text() length
        // only counts live characters.
        let mut doc = DocState::new(PeerId(120));
        let op_a = doc.local_insert(RgaPos::Head, "X");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "Y");
        doc.local_insert(RgaPos::After(op_b.id), "Z");
        assert_eq!(doc.text().len(), 3);

        doc.local_delete(op_a.id);
        // Visible length drops by one.
        assert_eq!(doc.text().len(), 2);

        doc.local_delete(op_b.id);
        assert_eq!(doc.text().len(), 1);
        // op_log has 5 entries: 3 inserts + 2 deletes.
        assert_eq!(doc.op_log().len(), 5);
    }

    #[test]
    fn crdt_to_text_skips_tombstones() {
        // text() must return only live nodes and skip tombstoned ones.
        let mut doc = DocState::new(PeerId(121));
        let op_hello = doc.local_insert(RgaPos::Head, "hello");
        let op_space = doc.local_insert(RgaPos::After(op_hello.id), " ");
        let op_world = doc.local_insert(RgaPos::After(op_space.id), "world");
        assert_eq!(doc.text(), "hello world");

        doc.local_delete(op_space.id);
        assert_eq!(doc.text(), "helloworld", "tombstoned space must not appear");

        doc.local_delete(op_hello.id);
        assert_eq!(doc.text(), "world");

        doc.local_delete(op_world.id);
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn crdt_op_id_monotonic() {
        // Each successive local_insert must yield a strictly higher counter.
        let mut doc = DocState::new(PeerId(130));
        let op1 = doc.local_insert(RgaPos::Head, "a");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "b");
        let op3 = doc.local_insert(RgaPos::After(op2.id), "c");
        let op4 = doc.local_insert(RgaPos::After(op3.id), "d");

        assert!(op1.id.counter < op2.id.counter);
        assert!(op2.id.counter < op3.id.counter);
        assert!(op3.id.counter < op4.id.counter);
        // All belong to the same peer.
        assert_eq!(op1.id.peer, PeerId(130));
        assert_eq!(op4.id.peer, PeerId(130));
    }

    // ── session / peer-management simulation ────────────────────────────────

    #[test]
    fn session_new_has_no_ops() {
        // A freshly created DocState has an empty op log (no peers have contributed).
        let doc = DocState::new(PeerId(200));
        assert_eq!(doc.op_log().len(), 0);
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn session_add_peer_contributes_ops() {
        // Simulating a peer join: after merge the receiving doc has the peer's ops.
        let mut doc_host = DocState::new(PeerId(201));
        doc_host.local_insert(RgaPos::Head, "from_host");

        let mut doc_peer = DocState::new(PeerId(202));
        // Peer joins by merging the host's state.
        doc_peer.merge(&doc_host);
        assert_eq!(doc_peer.text(), "from_host");
        // Peer doc now has the host's op.
        assert_eq!(doc_peer.op_log().len(), 1);
    }

    #[test]
    fn session_remove_peer_leaves_text_intact() {
        // Peer "leaving" is modelled by stop merging; existing text is unaffected.
        let mut doc_host = DocState::new(PeerId(203));
        let op = doc_host.local_insert(RgaPos::Head, "shared");

        let mut doc_peer = DocState::new(PeerId(204));
        doc_peer.merge(&doc_host);
        assert_eq!(doc_peer.text(), "shared");

        // Peer inserts one more op then stops syncing.
        doc_peer.local_insert(RgaPos::After(op.id), "_peer");
        assert_eq!(doc_peer.text(), "shared_peer");

        // Host has not received the peer's op — host text is unchanged.
        assert_eq!(doc_host.text(), "shared");
    }

    #[test]
    fn session_peer_id_preserved_in_ops() {
        // All ops authored on a doc carry the doc's PeerId.
        let mut doc = DocState::new(PeerId(205));
        let op1 = doc.local_insert(RgaPos::Head, "x");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "y");

        assert_eq!(op1.id.peer, PeerId(205));
        assert_eq!(op2.id.peer, PeerId(205));
    }

    // ── vector-clock properties (modelled via Lamport counter) ───────────────

    #[test]
    fn vector_clock_counter_increments_on_each_op() {
        // The Lamport counter advances by 1 for each local operation.
        let mut doc = DocState::new(PeerId(300));
        let op1 = doc.local_insert(RgaPos::Head, "a");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "b");
        let op3 = doc.local_insert(RgaPos::After(op2.id), "c");
        assert_eq!(op1.id.counter, 1);
        assert_eq!(op2.id.counter, 2);
        assert_eq!(op3.id.counter, 3);
    }

    #[test]
    fn vector_clock_merge_advances_to_max() {
        // Applying a remote op with a higher counter must advance the local counter
        // so future local ops are strictly greater.
        let mut doc_a = DocState::new(PeerId(301));
        // Create a remote op with counter = 50.
        let remote_op = make_insert(302, 50, RgaPos::Head, "remote");
        doc_a.apply(remote_op);

        // Next local op counter must be > 50.
        let local_op = doc_a.local_insert(RgaPos::Head, "local");
        assert!(local_op.id.counter > 50,
            "local counter must exceed the remote op's counter after merge");
    }

    #[test]
    fn vector_clock_happens_before_ordering() {
        // Op authored later on the same peer has a strictly greater counter
        // — classic happens-before for a single peer.
        let mut doc = DocState::new(PeerId(303));
        let earlier = doc.local_insert(RgaPos::Head, "e");
        let later = doc.local_insert(RgaPos::After(earlier.id), "l");

        assert!(earlier.id < later.id,
            "earlier op must sort before later op (happens-before)");
        assert!(earlier.id.counter < later.id.counter);
    }

    #[test]
    fn vector_clock_concurrent_ops_on_different_peers() {
        // Two ops on different peers with the same counter are concurrent
        // (neither happens-before the other in the causal sense); the Ord
        // tie-break is by peer.0.
        let op_p1 = OpId { peer: PeerId(1), counter: 7 };
        let op_p2 = OpId { peer: PeerId(2), counter: 7 };

        // They are not equal.
        assert_ne!(op_p1, op_p2);
        // The sort order is deterministic (peer 1 < peer 2 at same counter).
        assert!(op_p1 < op_p2);
        assert!(op_p2 > op_p1);
    }

    #[test]
    fn vector_clock_empty_doc_counter_starts_at_zero() {
        // A fresh doc has no ops, so the internal counter has never fired.
        // The first op must carry counter = 1.
        let mut doc = DocState::new(PeerId(304));
        let first = doc.local_insert(RgaPos::Head, "z");
        assert_eq!(first.id.counter, 1);
    }

    // ── awareness / cursor simulation ────────────────────────────────────────

    #[test]
    fn awareness_cursor_encoded_in_op_meta() {
        // Cursor positions can be communicated via SetMeta ops keyed by user id.
        let mut doc = DocState::new(PeerId(400));
        let cursor_op = Op {
            id: OpId { peer: PeerId(400), counter: 99 },
            kind: OpKind::SetMeta {
                key: "cursor:400".to_string(),
                value: "5".to_string(),
            },
        };
        doc.apply(cursor_op);
        // SetMeta does not change text.
        assert_eq!(doc.text(), "");
        // But it is recorded in the op log.
        assert_eq!(doc.op_log().len(), 1);
        match &doc.op_log()[0].kind {
            OpKind::SetMeta { key, value } => {
                assert_eq!(key, "cursor:400");
                assert_eq!(value, "5");
            }
            other => panic!("expected SetMeta, got {other:?}"),
        }
    }

    #[test]
    fn awareness_two_users_cursors_coexist() {
        // Two SetMeta cursor ops from different peers both land in the op log.
        let mut doc = DocState::new(PeerId(401));
        let cursor_a = Op {
            id: OpId { peer: PeerId(401), counter: 1 },
            kind: OpKind::SetMeta {
                key: "cursor:401".to_string(),
                value: "3".to_string(),
            },
        };
        let cursor_b = Op {
            id: OpId { peer: PeerId(402), counter: 1 },
            kind: OpKind::SetMeta {
                key: "cursor:402".to_string(),
                value: "7".to_string(),
            },
        };
        doc.apply(cursor_a);
        doc.apply(cursor_b);
        assert_eq!(doc.op_log().len(), 2);
        // Text unaffected.
        assert_eq!(doc.text(), "");

        // Both cursors are retrievable from the op log.
        let cursors: Vec<&Op> = doc.op_log().iter()
            .filter(|op| matches!(&op.kind, OpKind::SetMeta { key, .. } if key.starts_with("cursor:")))
            .collect();
        assert_eq!(cursors.len(), 2);
    }

    // ── new 21 tests (wave expand-2) ─────────────────────────────────────────

    #[test]
    fn rga_insert_empty_string() {
        // Insert "" at Head still creates an op entry in the op_log.
        let mut doc = DocState::new(PeerId(500));
        let op = doc.local_insert(RgaPos::Head, "");
        assert_eq!(doc.op_log().len(), 1);
        assert_eq!(op.id.peer, PeerId(500));
        assert_eq!(op.id.counter, 1);
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn rga_insert_unicode() {
        // Insert a multi-byte unicode string; text() must return it intact.
        let mut doc = DocState::new(PeerId(501));
        doc.local_insert(RgaPos::Head, "你好");
        assert_eq!(doc.text(), "你好");
        // Rust String is UTF-8; len() is bytes, chars().count() is code points.
        assert_eq!(doc.text().chars().count(), 2);
    }

    #[test]
    fn rga_tombstone_count_after_delete() {
        // Delete 3 chars; count tombstoned ops in op_log == 3.
        let mut doc = DocState::new(PeerId(502));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        let op_c = doc.local_insert(RgaPos::After(op_b.id), "C");
        doc.local_delete(op_a.id);
        doc.local_delete(op_b.id);
        doc.local_delete(op_c.id);
        // Count Delete ops in the log.
        let tombstone_count = doc.op_log().iter()
            .filter(|op| matches!(op.kind, OpKind::Delete { .. }))
            .count();
        assert_eq!(tombstone_count, 3);
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn rga_live_count_after_deletes() {
        // 5 inserts then 2 deletes → 3 live characters in text().
        let mut doc = DocState::new(PeerId(503));
        let op1 = doc.local_insert(RgaPos::Head, "1");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "2");
        let op3 = doc.local_insert(RgaPos::After(op2.id), "3");
        let op4 = doc.local_insert(RgaPos::After(op3.id), "4");
        doc.local_insert(RgaPos::After(op4.id), "5");
        assert_eq!(doc.text(), "12345");
        doc.local_delete(op1.id);
        doc.local_delete(op2.id);
        // 3 live chars remain.
        assert_eq!(doc.text().chars().count(), 3);
        assert_eq!(doc.text(), "345");
    }

    #[test]
    fn rga_text_length_matches_live_chars() {
        // text().chars().count() matches the number of non-tombstoned insert ops.
        let mut doc = DocState::new(PeerId(504));
        let op_a = doc.local_insert(RgaPos::Head, "X");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "Y");
        doc.local_insert(RgaPos::After(op_b.id), "Z");
        // Delete one.
        doc.local_delete(op_a.id);
        // Live inserts: B and C.
        let live_inserts = doc.op_log().iter()
            .filter(|op| matches!(&op.kind, OpKind::Insert { .. }))
            .count();
        let deleted = doc.op_log().iter()
            .filter(|op| matches!(&op.kind, OpKind::Delete { .. }))
            .count();
        let expected_live = live_inserts - deleted;
        assert_eq!(doc.text().chars().count(), expected_live);
    }

    #[test]
    fn rga_sequential_inserts_in_order() {
        // Insert A, B, C sequentially → "ABC".
        let mut doc = DocState::new(PeerId(505));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        doc.local_insert(RgaPos::After(op_b.id), "C");
        assert_eq!(doc.text(), "ABC");
    }

    #[test]
    fn rga_delete_all_chars() {
        // Delete every inserted char → to_text() == "".
        let mut doc = DocState::new(PeerId(506));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        doc.local_delete(op_a.id);
        doc.local_delete(op_b.id);
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn rga_re_insert_after_all_deleted() {
        // Delete all, then insert "X" → text() == "X".
        let mut doc = DocState::new(PeerId(507));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        doc.local_delete(op_a.id);
        assert_eq!(doc.text(), "");
        doc.local_insert(RgaPos::Head, "X");
        assert_eq!(doc.text(), "X");
    }

    #[test]
    fn rga_large_document() {
        // Insert 100 single-char strings → text.chars().count() == 100.
        let mut doc = DocState::new(PeerId(508));
        let mut prev_id = doc.local_insert(RgaPos::Head, "a").id;
        for _ in 1..100 {
            let op = doc.local_insert(RgaPos::After(prev_id), "a");
            prev_id = op.id;
        }
        assert_eq!(doc.text().chars().count(), 100);
    }

    #[test]
    fn op_id_peer_preserved() {
        // OpId.peer field carries the PeerId used at creation.
        let mut doc = DocState::new(PeerId(600));
        let op = doc.local_insert(RgaPos::Head, "test");
        assert_eq!(op.id.peer, PeerId(600));
        assert_eq!(op.id.peer.0, 600u64);
    }

    #[test]
    fn op_id_counter_preserved() {
        // OpId.counter starts at 1 and increments each local op.
        let mut doc = DocState::new(PeerId(601));
        let op1 = doc.local_insert(RgaPos::Head, "a");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "b");
        assert_eq!(op1.id.counter, 1);
        assert_eq!(op2.id.counter, 2);
    }

    #[test]
    fn op_id_ordering_by_counter() {
        // OpId with lower counter is "earlier" regardless of peer id.
        let id_early = OpId { peer: PeerId(999), counter: 1 };
        let id_late  = OpId { peer: PeerId(1),   counter: 2 };
        assert!(id_early < id_late);
    }

    #[test]
    fn op_id_two_different_peers_same_counter() {
        // Same counter, different peers → different OpIds that compare by peer.0.
        let id_p1 = OpId { peer: PeerId(1), counter: 5 };
        let id_p2 = OpId { peer: PeerId(2), counter: 5 };
        assert_ne!(id_p1, id_p2);
        assert!(id_p1 < id_p2, "peer 1 < peer 2 at equal counter");
    }

    #[test]
    fn merge_two_empty_docs() {
        // Merging two empty docs produces an empty doc.
        let mut doc_a = DocState::new(PeerId(700));
        let doc_b = DocState::new(PeerId(701));
        doc_a.merge(&doc_b);
        assert_eq!(doc_a.text(), "");
        assert_eq!(doc_a.op_log().len(), 0);
    }

    #[test]
    fn merge_insert_from_both_peers() {
        // Peer A inserts "A", peer B inserts "B", merged doc has both chars.
        let mut peer_a = DocState::new(PeerId(710));
        peer_a.local_insert(RgaPos::Head, "A");

        let mut peer_b = DocState::new(PeerId(711));
        peer_b.local_insert(RgaPos::Head, "B");

        peer_a.merge(&peer_b);
        assert!(peer_a.text().contains('A'));
        assert!(peer_a.text().contains('B'));
        assert_eq!(peer_a.text().chars().count(), 2);
    }

    #[test]
    fn merge_idempotent_repeated() {
        // merge(merge(a, b), b) == merge(a, b): merging b twice changes nothing.
        let mut peer_a = DocState::new(PeerId(720));
        peer_a.local_insert(RgaPos::Head, "hello");

        let mut peer_b = DocState::new(PeerId(721));
        peer_b.local_insert(RgaPos::Head, "world");

        peer_a.merge(&peer_b);
        let text_once = peer_a.text();
        let log_len_once = peer_a.op_log().len();

        peer_a.merge(&peer_b); // second merge — must be no-op.
        assert_eq!(peer_a.text(), text_once);
        assert_eq!(peer_a.op_log().len(), log_len_once);
    }

    #[test]
    fn awareness_update_cursor_twice_last_wins() {
        // Applying two SetMeta cursor ops for the same peer; both are logged
        // but only the latest value is semantically current.
        let mut doc = DocState::new(PeerId(800));
        let cursor_v1 = Op {
            id: OpId { peer: PeerId(800), counter: 1 },
            kind: OpKind::SetMeta { key: "cursor:800".to_string(), value: "3".to_string() },
        };
        let cursor_v2 = Op {
            id: OpId { peer: PeerId(800), counter: 2 },
            kind: OpKind::SetMeta { key: "cursor:800".to_string(), value: "7".to_string() },
        };
        doc.apply(cursor_v1);
        doc.apply(cursor_v2);
        assert_eq!(doc.op_log().len(), 2);
        // Retrieve the latest value by scanning from the back.
        let latest = doc.op_log().iter().rev()
            .find_map(|op| {
                if let OpKind::SetMeta { key, value } = &op.kind {
                    if key == "cursor:800" { return Some(value.as_str()); }
                }
                None
            });
        assert_eq!(latest, Some("7"), "latest cursor value must be 7");
    }

    #[test]
    fn awareness_cursor_for_unknown_peer_returns_none() {
        // No SetMeta for peer 999 exists; lookup returns None.
        let doc = DocState::new(PeerId(801));
        let cursor = doc.op_log().iter().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind {
                if key == "cursor:999" { return Some(value.as_str()); }
            }
            None
        });
        assert_eq!(cursor, None);
    }

    #[test]
    fn awareness_peer_list_three_peers() {
        // Apply SetMeta ops from 3 distinct peers; count unique cursor peers == 3.
        let mut doc = DocState::new(PeerId(802));
        for (peer_id, cursor_pos) in [(810u64, "1"), (811, "4"), (812, "9")] {
            doc.apply(Op {
                id: OpId { peer: PeerId(peer_id), counter: 1 },
                kind: OpKind::SetMeta {
                    key: format!("cursor:{peer_id}"),
                    value: cursor_pos.to_string(),
                },
            });
        }
        let peer_cursors: std::collections::HashSet<&str> = doc.op_log().iter()
            .filter_map(|op| {
                if let OpKind::SetMeta { key, .. } = &op.kind {
                    if key.starts_with("cursor:") { return Some(key.as_str()); }
                }
                None
            })
            .collect();
        assert_eq!(peer_cursors.len(), 3);
    }

    #[test]
    fn doc_apply_op_increases_log_len() {
        // Applying an insert op increases op_log length by 1.
        let mut doc = DocState::new(PeerId(900));
        assert_eq!(doc.op_log().len(), 0);
        let op = make_insert(900, 1, RgaPos::Head, "hello");
        doc.apply(op);
        assert_eq!(doc.op_log().len(), 1);
        assert_eq!(doc.text(), "hello");
    }

    #[test]
    fn doc_operations_list_returns_all_applied_ops() {
        // op_log() returns all operations in apply order.
        let mut doc = DocState::new(PeerId(901));
        let op1 = doc.local_insert(RgaPos::Head, "first");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "second");
        let ops = doc.op_log();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].id, op1.id);
        assert_eq!(ops[1].id, op2.id);
    }

    #[test]
    fn doc_counter_increases_after_apply() {
        // Each local_insert produces a strictly greater counter — the internal
        // Lamport clock advances monotonically.
        let mut doc = DocState::new(PeerId(902));
        let op1 = doc.local_insert(RgaPos::Head, "a");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "b");
        let op3 = doc.local_insert(RgaPos::After(op2.id), "c");
        // Counter at each step must be strictly greater than the previous.
        assert!(op1.id.counter < op2.id.counter);
        assert!(op2.id.counter < op3.id.counter);
    }
}
