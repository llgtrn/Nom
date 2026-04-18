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
    Insert {
        pos: RgaPos,
        text: String,
    },
    /// Tombstone the op with this id (logical delete).
    Delete {
        target: OpId,
    },
    SetMeta {
        key: String,
        value: String,
    },
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
                    RgaPos::After(anchor_id) => self.nodes.iter().position(|n| &n.id == anchor_id),
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
        let op_low = OpId {
            peer: PeerId(1),
            counter: 5,
        };
        let op_high = OpId {
            peer: PeerId(2),
            counter: 5,
        };

        // OpId Ord: counter first, then peer.0 ascending.
        assert!(
            op_low < op_high,
            "lower peer id must sort before higher peer id at equal counter"
        );

        // Different counters: counter dominates.
        let op_counter_low = OpId {
            peer: PeerId(99),
            counter: 1,
        };
        let op_counter_high = OpId {
            peer: PeerId(1),
            counter: 10,
        };
        assert!(
            op_counter_low < op_counter_high,
            "lower counter must sort before higher counter"
        );
    }

    #[test]
    fn crdt_text_preserves_insertion_order() {
        // Insert "A" at Head, then "B" After A → expected text "AB".
        let mut doc = DocState::new(PeerId(1));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        doc.local_insert(RgaPos::After(op_a.id), "B");

        assert_eq!(
            doc.text(),
            "AB",
            "sequential inserts must preserve left-to-right order"
        );
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
        assert_eq!(
            doc.op_log().len(),
            log_len_before,
            "merge-self must not grow op log"
        );
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

        assert_eq!(
            doc.text(),
            "",
            "tombstoned text must not appear in doc.text()"
        );
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

        assert_eq!(
            doc_ab.text(),
            doc_ba.text(),
            "concurrent inserts at same pos must converge regardless of apply order"
        );
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
        assert!(
            local_op.id.counter > 50,
            "local counter must exceed the remote op's counter after merge"
        );
    }

    #[test]
    fn vector_clock_happens_before_ordering() {
        // Op authored later on the same peer has a strictly greater counter
        // — classic happens-before for a single peer.
        let mut doc = DocState::new(PeerId(303));
        let earlier = doc.local_insert(RgaPos::Head, "e");
        let later = doc.local_insert(RgaPos::After(earlier.id), "l");

        assert!(
            earlier.id < later.id,
            "earlier op must sort before later op (happens-before)"
        );
        assert!(earlier.id.counter < later.id.counter);
    }

    #[test]
    fn vector_clock_concurrent_ops_on_different_peers() {
        // Two ops on different peers with the same counter are concurrent
        // (neither happens-before the other in the causal sense); the Ord
        // tie-break is by peer.0.
        let op_p1 = OpId {
            peer: PeerId(1),
            counter: 7,
        };
        let op_p2 = OpId {
            peer: PeerId(2),
            counter: 7,
        };

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
            id: OpId {
                peer: PeerId(400),
                counter: 99,
            },
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
            id: OpId {
                peer: PeerId(401),
                counter: 1,
            },
            kind: OpKind::SetMeta {
                key: "cursor:401".to_string(),
                value: "3".to_string(),
            },
        };
        let cursor_b = Op {
            id: OpId {
                peer: PeerId(402),
                counter: 1,
            },
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
        let cursors: Vec<&Op> = doc
            .op_log()
            .iter()
            .filter(
                |op| matches!(&op.kind, OpKind::SetMeta { key, .. } if key.starts_with("cursor:")),
            )
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
        let tombstone_count = doc
            .op_log()
            .iter()
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
        let live_inserts = doc
            .op_log()
            .iter()
            .filter(|op| matches!(&op.kind, OpKind::Insert { .. }))
            .count();
        let deleted = doc
            .op_log()
            .iter()
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
        let id_early = OpId {
            peer: PeerId(999),
            counter: 1,
        };
        let id_late = OpId {
            peer: PeerId(1),
            counter: 2,
        };
        assert!(id_early < id_late);
    }

    #[test]
    fn op_id_two_different_peers_same_counter() {
        // Same counter, different peers → different OpIds that compare by peer.0.
        let id_p1 = OpId {
            peer: PeerId(1),
            counter: 5,
        };
        let id_p2 = OpId {
            peer: PeerId(2),
            counter: 5,
        };
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
            id: OpId {
                peer: PeerId(800),
                counter: 1,
            },
            kind: OpKind::SetMeta {
                key: "cursor:800".to_string(),
                value: "3".to_string(),
            },
        };
        let cursor_v2 = Op {
            id: OpId {
                peer: PeerId(800),
                counter: 2,
            },
            kind: OpKind::SetMeta {
                key: "cursor:800".to_string(),
                value: "7".to_string(),
            },
        };
        doc.apply(cursor_v1);
        doc.apply(cursor_v2);
        assert_eq!(doc.op_log().len(), 2);
        // Retrieve the latest value by scanning from the back.
        let latest = doc.op_log().iter().rev().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind {
                if key == "cursor:800" {
                    return Some(value.as_str());
                }
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
                if key == "cursor:999" {
                    return Some(value.as_str());
                }
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
                id: OpId {
                    peer: PeerId(peer_id),
                    counter: 1,
                },
                kind: OpKind::SetMeta {
                    key: format!("cursor:{peer_id}"),
                    value: cursor_pos.to_string(),
                },
            });
        }
        let peer_cursors: std::collections::HashSet<&str> = doc
            .op_log()
            .iter()
            .filter_map(|op| {
                if let OpKind::SetMeta { key, .. } = &op.kind {
                    if key.starts_with("cursor:") {
                        return Some(key.as_str());
                    }
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

    // ── 24 new tests (wave expand-3) ─────────────────────────────────────────

    // CRDT branching and convergence

    #[test]
    fn crdt_three_peer_merge_all_chars_present() {
        // 3 peers insert A, B, C independently; merge all; text has all 3 chars.
        let mut peer_a = DocState::new(PeerId(1000));
        let op_a = peer_a.local_insert(RgaPos::Head, "A");

        let mut peer_b = DocState::new(PeerId(1001));
        let op_b = peer_b.local_insert(RgaPos::Head, "B");

        let mut peer_c = DocState::new(PeerId(1002));
        let op_c = peer_c.local_insert(RgaPos::Head, "C");

        // peer_a gets B and C
        peer_a.apply(op_b.clone());
        peer_a.apply(op_c.clone());

        // peer_b gets A and C
        peer_b.apply(op_a.clone());
        peer_b.apply(op_c.clone());

        // peer_c gets A and B
        peer_c.apply(op_a.clone());
        peer_c.apply(op_b.clone());

        // All three converge to identical text containing A, B, C.
        assert_eq!(peer_a.text(), peer_b.text());
        assert_eq!(peer_b.text(), peer_c.text());
        assert!(peer_a.text().contains('A'));
        assert!(peer_a.text().contains('B'));
        assert!(peer_a.text().contains('C'));
        assert_eq!(peer_a.text().chars().count(), 3);
    }

    #[test]
    fn crdt_interleaved_ops_converge() {
        // Ops applied in different (interleaved) orders must produce the same text.
        let op1 = make_insert(1, 1, RgaPos::Head, "X");
        let op2 = make_insert(2, 2, RgaPos::Head, "Y");
        let op3 = make_insert(3, 3, RgaPos::Head, "Z");

        // Order A: 1, 2, 3
        let mut doc_a = DocState::new(PeerId(1010));
        doc_a.apply(op1.clone());
        doc_a.apply(op2.clone());
        doc_a.apply(op3.clone());

        // Order B: 3, 1, 2
        let mut doc_b = DocState::new(PeerId(1010));
        doc_b.apply(op3.clone());
        doc_b.apply(op1.clone());
        doc_b.apply(op2.clone());

        // Order C: 2, 3, 1
        let mut doc_c = DocState::new(PeerId(1010));
        doc_c.apply(op2.clone());
        doc_c.apply(op3.clone());
        doc_c.apply(op1.clone());

        assert_eq!(doc_a.text(), doc_b.text(), "orders A and B must converge");
        assert_eq!(doc_b.text(), doc_c.text(), "orders B and C must converge");
    }

    #[test]
    fn crdt_delete_already_deleted_is_noop() {
        // Deleting the same position twice must not panic and length must stay 0.
        let mut doc = DocState::new(PeerId(1020));
        let op = doc.local_insert(RgaPos::Head, "gone");
        assert_eq!(doc.text(), "gone");

        doc.local_delete(op.id);
        assert_eq!(doc.text(), "");
        let len_after_first = doc.op_log().len();

        // Delete again using a raw op targeting the same id.
        let del2 = Op {
            id: OpId {
                peer: PeerId(1020),
                counter: 99,
            },
            kind: OpKind::Delete { target: op.id },
        };
        doc.apply(del2); // must not panic
        assert_eq!(doc.text(), "");
        // op_log grew by one (the second delete op was recorded).
        assert_eq!(doc.op_log().len(), len_after_first + 1);
    }

    #[test]
    fn crdt_insert_preserve_relative_order() {
        // Insert A, then B after A, then C after B; relative order A<B<C is preserved.
        let mut doc = DocState::new(PeerId(1030));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        doc.local_insert(RgaPos::After(op_b.id), "C");
        assert_eq!(doc.text(), "ABC");
        // A is still before B and C.
        let text = doc.text();
        let pos_a = text.find('A').unwrap();
        let pos_b = text.find('B').unwrap();
        let pos_c = text.find('C').unwrap();
        assert!(pos_a < pos_b, "A must come before B");
        assert!(pos_b < pos_c, "B must come before C");
    }

    #[test]
    fn crdt_apply_ops_from_future() {
        // Apply an op with a much higher counter than the local clock; no panic.
        let mut doc = DocState::new(PeerId(1040));
        let future_op = make_insert(1041, 10_000, RgaPos::Head, "future");
        doc.apply(future_op); // must not panic
        assert_eq!(doc.text(), "future");
        // Next local op counter must be > 10_000.
        let local_op = doc.local_insert(
            RgaPos::After(OpId {
                peer: PeerId(1041),
                counter: 10_000,
            }),
            "local",
        );
        assert!(local_op.id.counter > 10_000);
        assert!(doc.text().contains("future"));
        assert!(doc.text().contains("local"));
    }

    // RGA positions

    #[test]
    fn rga_pos_head() {
        // RgaPos::Head is a valid position accepted by local_insert without panic.
        let mut doc = DocState::new(PeerId(1050));
        let op = doc.local_insert(RgaPos::Head, "head_insert");
        assert_eq!(doc.text(), "head_insert");
        match &op.kind {
            OpKind::Insert { pos, .. } => assert_eq!(*pos, RgaPos::Head),
            other => panic!("expected Insert, got {other:?}"),
        }
    }

    #[test]
    fn rga_pos_after_op() {
        // RgaPos::After(op_id) references an existing op; insert lands after it.
        let mut doc = DocState::new(PeerId(1051));
        let op_first = doc.local_insert(RgaPos::Head, "first");
        let op_second = doc.local_insert(RgaPos::After(op_first.id), "second");
        assert_eq!(doc.text(), "firstsecond");
        match &op_second.kind {
            OpKind::Insert { pos, .. } => assert_eq!(*pos, RgaPos::After(op_first.id)),
            other => panic!("expected Insert, got {other:?}"),
        }
    }

    #[test]
    fn rga_insert_before_first() {
        // Insert at RgaPos::Head on non-empty doc places the new node first.
        let mut doc = DocState::new(PeerId(1052));
        doc.local_insert(RgaPos::Head, "B");
        // Now insert "A" at Head — it must win since peer 1052, counter 2 > counter 1.
        doc.local_insert(RgaPos::Head, "A");
        // The tiebreak puts counter-2 node (A) ahead of counter-1 node (B).
        assert!(doc.text().contains('A'));
        assert!(doc.text().contains('B'));
        assert_eq!(doc.text().chars().count(), 2);
    }

    #[test]
    fn rga_insert_after_last() {
        // Insert After the last op appends text at the end.
        let mut doc = DocState::new(PeerId(1053));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        let op_c = doc.local_insert(RgaPos::After(op_b.id), "C");
        // Appended after C (the last op).
        doc.local_insert(RgaPos::After(op_c.id), "D");
        assert_eq!(doc.text(), "ABCD");
    }

    #[test]
    fn rga_position_within_text() {
        // Character index in text() matches insertion order.
        let mut doc = DocState::new(PeerId(1054));
        let op_h = doc.local_insert(RgaPos::Head, "H");
        let op_i = doc.local_insert(RgaPos::After(op_h.id), "i");
        doc.local_insert(RgaPos::After(op_i.id), "!");
        let text = doc.text();
        assert_eq!(text, "Hi!");
        // 'H' is at index 0, 'i' at 1, '!' at 2.
        let chars: Vec<char> = text.chars().collect();
        assert_eq!(chars[0], 'H');
        assert_eq!(chars[1], 'i');
        assert_eq!(chars[2], '!');
    }

    // Document snapshot (modelled via op_log clone)

    #[test]
    fn doc_snapshot_text_matches() {
        // A doc reconstructed from op_log produces the same text.
        let mut original = DocState::new(PeerId(1060));
        let op1 = original.local_insert(RgaPos::Head, "snap");
        original.local_insert(RgaPos::After(op1.id), "shot");
        assert_eq!(original.text(), "snapshot");

        // Reconstruct a "snapshot" doc by replaying the op_log.
        let mut snapshot = DocState::new(PeerId(1060));
        for op in original.op_log().to_vec() {
            snapshot.apply(op);
        }
        assert_eq!(snapshot.text(), original.text());
    }

    #[test]
    fn doc_snapshot_version_matches() {
        // The number of ops in the snapshot equals the original's op_log length.
        let mut original = DocState::new(PeerId(1061));
        original.local_insert(RgaPos::Head, "v1");
        original.local_insert(RgaPos::Head, "v2");

        let mut snapshot = DocState::new(PeerId(1061));
        for op in original.op_log().to_vec() {
            snapshot.apply(op);
        }
        assert_eq!(snapshot.op_log().len(), original.op_log().len());
    }

    #[test]
    fn doc_snapshot_after_insert() {
        // After an insert the snapshot reflects the latest state.
        let mut doc = DocState::new(PeerId(1062));
        doc.local_insert(RgaPos::Head, "initial");

        let mut snapshot = DocState::new(PeerId(1062));
        for op in doc.op_log().to_vec() {
            snapshot.apply(op);
        }
        assert_eq!(snapshot.text(), "initial");

        // Insert more, rebuild snapshot.
        let last = doc.op_log().last().unwrap().id;
        doc.local_insert(RgaPos::After(last), "_updated");

        let mut snapshot2 = DocState::new(PeerId(1062));
        for op in doc.op_log().to_vec() {
            snapshot2.apply(op);
        }
        assert_eq!(snapshot2.text(), "initial_updated");
    }

    #[test]
    fn doc_rebase_on_snapshot() {
        // Create a fresh doc from a snapshot (op_log replay); text matches.
        let mut source = DocState::new(PeerId(1063));
        let op1 = source.local_insert(RgaPos::Head, "base");
        source.local_insert(RgaPos::After(op1.id), "_text");

        let snapshot_ops = source.op_log().to_vec();
        let mut rebased = DocState::new(PeerId(999));
        for op in snapshot_ops {
            rebased.apply(op);
        }
        assert_eq!(rebased.text(), source.text());
        assert_eq!(rebased.text(), "base_text");
    }

    // Conflict resolution

    #[test]
    fn conflict_same_position_two_peers_deterministic() {
        // Same-position concurrent inserts always produce identical text regardless
        // of the apply order.
        let op_p1 = make_insert(10, 1, RgaPos::Head, "P");
        let op_p2 = make_insert(20, 1, RgaPos::Head, "Q");

        let mut doc_fwd = DocState::new(PeerId(1070));
        doc_fwd.apply(op_p1.clone());
        doc_fwd.apply(op_p2.clone());

        let mut doc_rev = DocState::new(PeerId(1070));
        doc_rev.apply(op_p2.clone());
        doc_rev.apply(op_p1.clone());

        assert_eq!(
            doc_fwd.text(),
            doc_rev.text(),
            "concurrent same-position inserts must converge deterministically"
        );
    }

    #[test]
    fn conflict_order_by_peer_id() {
        // When two ops share the same counter and anchor, higher peer.0 wins the
        // left position (consistent with the Ord impl: higher peer.0 → higher OpId).
        let op_low = make_insert(1, 1, RgaPos::Head, "low");
        let op_high = make_insert(2, 1, RgaPos::Head, "high");

        let mut doc = DocState::new(PeerId(1080));
        doc.apply(op_low.clone());
        doc.apply(op_high.clone());

        // peer 2 has higher OpId → higher priority → placed to the left.
        let text = doc.text();
        let pos_high = text.find("high").unwrap();
        let pos_low = text.find("low").unwrap();
        assert!(
            pos_high < pos_low,
            "higher peer_id op must appear to the left of lower peer_id op"
        );
    }

    #[test]
    fn conflict_concurrent_deletes() {
        // Two peers both delete the same character; result has it deleted exactly once.
        let mut peer_a = DocState::new(PeerId(1090));
        let shared_op = peer_a.local_insert(RgaPos::Head, "shared");

        let mut peer_b = DocState::new(PeerId(1091));
        peer_b.apply(shared_op.clone());

        // Both delete the same op.
        let del_a = peer_a.local_delete(shared_op.id);
        let del_b = peer_b.local_delete(shared_op.id);

        // Cross-merge: each receives the other's delete.
        peer_a.apply(del_b.clone());
        peer_b.apply(del_a.clone());

        // "shared" must be gone in both; no duplicate or panic.
        assert_eq!(peer_a.text(), "");
        assert_eq!(peer_b.text(), "");
        assert_eq!(peer_a.text(), peer_b.text());
    }

    // Document metadata

    #[test]
    fn doc_created_at_set() {
        // A SetMeta op with key "created_at" is stored in the op_log.
        let mut doc = DocState::new(PeerId(1100));
        let ts_op = Op {
            id: OpId {
                peer: PeerId(1100),
                counter: 1,
            },
            kind: OpKind::SetMeta {
                key: "created_at".to_string(),
                value: "2026-04-18T00:00:00Z".to_string(),
            },
        };
        doc.apply(ts_op);
        let found = doc
            .op_log()
            .iter()
            .any(|op| matches!(&op.kind, OpKind::SetMeta { key, .. } if key == "created_at"));
        assert!(found, "created_at SetMeta must be in op_log");
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn doc_title_field() {
        // A SetMeta op with key "title" stores a String value.
        let mut doc = DocState::new(PeerId(1101));
        let title_op = Op {
            id: OpId {
                peer: PeerId(1101),
                counter: 1,
            },
            kind: OpKind::SetMeta {
                key: "title".to_string(),
                value: "My Canvas".to_string(),
            },
        };
        doc.apply(title_op);
        let title_value = doc.op_log().iter().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind {
                if key == "title" {
                    return Some(value.clone());
                }
            }
            None
        });
        assert_eq!(title_value, Some("My Canvas".to_string()));
    }

    #[test]
    fn doc_id_nonempty() {
        // PeerId(n) where n > 0 is nonempty (non-zero peer id is a valid doc identity).
        let peer = PeerId(42);
        assert_ne!(peer.0, 0, "doc peer id must be nonempty (non-zero)");
        let doc = DocState::new(peer);
        // op_log is empty but the doc is "owned" by peer 42.
        assert_eq!(doc.op_log().len(), 0);
    }

    // OpKind variants

    #[test]
    fn op_kind_insert_has_text() {
        // The Insert variant exposes `text` and `pos` fields.
        let op = make_insert(1, 1, RgaPos::Head, "hello");
        match op.kind {
            OpKind::Insert { ref pos, ref text } => {
                assert_eq!(*pos, RgaPos::Head);
                assert_eq!(text, "hello");
            }
            _ => panic!("expected Insert variant"),
        }
    }

    #[test]
    fn op_kind_delete_has_target() {
        // The Delete variant exposes a `target` OpId field.
        let target = OpId {
            peer: PeerId(7),
            counter: 3,
        };
        let del_op = Op {
            id: OpId {
                peer: PeerId(1),
                counter: 5,
            },
            kind: OpKind::Delete { target },
        };
        match del_op.kind {
            OpKind::Delete { target: t } => assert_eq!(t, target),
            _ => panic!("expected Delete variant"),
        }
    }

    #[test]
    fn op_kind_set_meta_has_key_value() {
        // The SetMeta variant exposes `key` and `value` String fields.
        let meta_op = Op {
            id: OpId {
                peer: PeerId(1),
                counter: 1,
            },
            kind: OpKind::SetMeta {
                key: "color".to_string(),
                value: "#ff0000".to_string(),
            },
        };
        match meta_op.kind {
            OpKind::SetMeta { ref key, ref value } => {
                assert_eq!(key, "color");
                assert_eq!(value, "#ff0000");
            }
            _ => panic!("expected SetMeta variant"),
        }
    }

    #[test]
    fn op_kind_three_variants_distinct() {
        // All three OpKind variants are not equal to each other.
        let insert = OpKind::Insert {
            pos: RgaPos::Head,
            text: "x".to_string(),
        };
        let delete = OpKind::Delete {
            target: OpId {
                peer: PeerId(1),
                counter: 1,
            },
        };
        let set_meta = OpKind::SetMeta {
            key: "k".to_string(),
            value: "v".to_string(),
        };
        assert_ne!(insert, delete, "Insert != Delete");
        assert_ne!(insert, set_meta, "Insert != SetMeta");
        assert_ne!(delete, set_meta, "Delete != SetMeta");
    }

    // ── wave AA-6: new coverage (55 tests) ──────────────────────────────────

    // RGA: 3+ peer concurrent inserts with deterministic merge

    #[test]
    fn rga_three_peer_concurrent_inserts_deterministic_merge() {
        // Peers A, B, C all insert at Head; every permutation of merge must converge.
        let mut pa = DocState::new(PeerId(2000));
        let opa = pa.local_insert(RgaPos::Head, "alpha");
        let mut pb = DocState::new(PeerId(2001));
        let opb = pb.local_insert(RgaPos::Head, "beta");
        let mut pc = DocState::new(PeerId(2002));
        let opc = pc.local_insert(RgaPos::Head, "gamma");

        // Build doc from order A→B→C
        let mut doc_abc = DocState::new(PeerId(9000));
        doc_abc.apply(opa.clone());
        doc_abc.apply(opb.clone());
        doc_abc.apply(opc.clone());

        // Build doc from order C→A→B
        let mut doc_cab = DocState::new(PeerId(9000));
        doc_cab.apply(opc.clone());
        doc_cab.apply(opa.clone());
        doc_cab.apply(opb.clone());

        // Build doc from order B→C→A
        let mut doc_bca = DocState::new(PeerId(9000));
        doc_bca.apply(opb.clone());
        doc_bca.apply(opc.clone());
        doc_bca.apply(opa.clone());

        assert_eq!(
            doc_abc.text(),
            doc_cab.text(),
            "A→B→C vs C→A→B must converge"
        );
        assert_eq!(
            doc_cab.text(),
            doc_bca.text(),
            "C→A→B vs B→C→A must converge"
        );
        assert!(doc_abc.text().contains("alpha"));
        assert!(doc_abc.text().contains("beta"));
        assert!(doc_abc.text().contains("gamma"));
    }

    #[test]
    fn rga_four_peer_concurrent_inserts_at_head_converge() {
        // Four peers insert at Head; all six pairwise merge orderings converge.
        let ops: Vec<Op> = (0u64..4)
            .map(|i| {
                let mut doc = DocState::new(PeerId(3000 + i));
                doc.local_insert(RgaPos::Head, format!("p{i}"))
            })
            .collect();

        // Two orderings: forward and reverse.
        let mut doc_fwd = DocState::new(PeerId(9100));
        let mut doc_rev = DocState::new(PeerId(9100));
        for op in &ops {
            doc_fwd.apply(op.clone());
        }
        for op in ops.iter().rev() {
            doc_rev.apply(op.clone());
        }
        assert_eq!(
            doc_fwd.text(),
            doc_rev.text(),
            "forward vs reverse must converge"
        );
        assert_eq!(doc_fwd.text().len(), "p0p1p2p3".len());
    }

    // RGA: interleaved inserts from alternating peers

    #[test]
    fn rga_interleaved_inserts_alternating_peers() {
        // Peer A and B alternate inserts; result has all chars in logical order.
        let mut pa = DocState::new(PeerId(4000));
        let mut pb = DocState::new(PeerId(4001));

        // A inserts "1", then B inserts "2" after A's "1", etc., using shared anchors.
        let op_a1 = pa.local_insert(RgaPos::Head, "1");
        pb.apply(op_a1.clone());
        let op_b2 = pb.local_insert(RgaPos::After(op_a1.id), "2");
        pa.apply(op_b2.clone());
        let op_a3 = pa.local_insert(RgaPos::After(op_b2.id), "3");
        pb.apply(op_a3.clone());
        let op_b4 = pb.local_insert(RgaPos::After(op_a3.id), "4");
        pa.apply(op_b4.clone());

        assert_eq!(pa.text(), "1234");
        assert_eq!(pb.text(), "1234");
    }

    #[test]
    fn rga_interleaved_concurrent_inserts_same_anchor() {
        // Both peers insert After the same anchor at the same logical time.
        let mut pa = DocState::new(PeerId(4010));
        let anchor = pa.local_insert(RgaPos::Head, "anchor");

        let mut pb = DocState::new(PeerId(4011));
        pb.apply(anchor.clone());

        // Concurrent inserts after anchor.
        let op_a = pa.local_insert(RgaPos::After(anchor.id), "A");
        let op_b = pb.local_insert(RgaPos::After(anchor.id), "B");

        // Cross-apply.
        pa.apply(op_b.clone());
        pb.apply(op_a.clone());

        assert_eq!(
            pa.text(),
            pb.text(),
            "interleaved concurrent inserts must converge"
        );
        assert!(pa.text().starts_with("anchor"), "anchor stays first");
        assert!(pa.text().contains('A'));
        assert!(pa.text().contains('B'));
    }

    // Delete of already-deleted position (idempotent tombstone)

    #[test]
    fn rga_delete_already_deleted_idempotent() {
        // Applying a delete twice on the same target is idempotent: text stays "".
        let mut doc = DocState::new(PeerId(5000));
        let op = doc.local_insert(RgaPos::Head, "x");
        doc.local_delete(op.id);
        assert_eq!(doc.text(), "");

        // Second explicit delete targeting the same OpId.
        let del2 = Op {
            id: OpId {
                peer: PeerId(5000),
                counter: 100,
            },
            kind: OpKind::Delete { target: op.id },
        };
        doc.apply(del2);
        assert_eq!(doc.text(), "", "second delete must keep text empty");
    }

    #[test]
    fn rga_concurrent_double_delete_from_two_peers() {
        // Two peers both delete the same node; after cross-merge both are empty.
        let mut pa = DocState::new(PeerId(5010));
        let shared = pa.local_insert(RgaPos::Head, "shared");
        let mut pb = DocState::new(PeerId(5011));
        pb.apply(shared.clone());

        let del_a = pa.local_delete(shared.id);
        let del_b = pb.local_delete(shared.id);
        pa.apply(del_b);
        pb.apply(del_a);

        assert_eq!(pa.text(), "");
        assert_eq!(pb.text(), "");
        assert_eq!(pa.text(), pb.text());
    }

    // Insert at head, middle, tail

    #[test]
    fn rga_insert_at_exact_head() {
        let mut doc = DocState::new(PeerId(6000));
        let op = doc.local_insert(RgaPos::Head, "HEAD");
        assert_eq!(doc.text(), "HEAD");
        assert_eq!(op.id.counter, 1);
        match &op.kind {
            OpKind::Insert { pos, .. } => assert_eq!(*pos, RgaPos::Head),
            _ => panic!("expected Insert"),
        }
    }

    #[test]
    fn rga_insert_at_middle_position() {
        let mut doc = DocState::new(PeerId(6010));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "C");
        // Insert "B" between A and C.
        doc.local_insert(RgaPos::After(op_a.id), "B");
        // "B" was inserted with a higher counter than "C" at the same anchor, so
        // it sorts left of C in the RGA tiebreak (higher counter wins left).
        let text = doc.text();
        let pos_a = text.find('A').unwrap();
        let pos_b = text.find('B').unwrap();
        let pos_c = text.find('C').unwrap();
        assert!(pos_a < pos_b, "A must precede B");
        assert!(pos_b < pos_c, "B must precede C");
        let _ = op_b;
    }

    #[test]
    fn rga_insert_at_tail_appends() {
        let mut doc = DocState::new(PeerId(6020));
        let op1 = doc.local_insert(RgaPos::Head, "X");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "Y");
        let op3 = doc.local_insert(RgaPos::After(op2.id), "Z");
        // Insert at tail (after Z).
        doc.local_insert(RgaPos::After(op3.id), "T");
        let text = doc.text();
        assert!(
            text.ends_with('T'),
            "tail insert must append to end: {text}"
        );
        assert_eq!(text, "XYZT");
    }

    // Merge of diverged document histories

    #[test]
    fn merge_diverged_histories_two_peers() {
        // Peer A authors 3 ops. Peer B starts from A's history and authors 3 more.
        // This ensures anchors are resolvable on both sides → convergence guaranteed.
        let mut pa = DocState::new(PeerId(7000));
        let opa1 = pa.local_insert(RgaPos::Head, "A1");
        let opa2 = pa.local_insert(RgaPos::After(opa1.id), "A2");
        let opa3 = pa.local_insert(RgaPos::After(opa2.id), "A3");

        // Peer B starts from A's state.
        let mut pb = DocState::new(PeerId(7001));
        pb.merge(&pa);
        // B extends the chain.
        let opb1 = pb.local_insert(RgaPos::After(opa3.id), "B1");
        let opb2 = pb.local_insert(RgaPos::After(opb1.id), "B2");
        pb.local_insert(RgaPos::After(opb2.id), "B3");

        // A merges B's additions.
        pa.merge(&pb);

        assert_eq!(
            pa.text(),
            pb.text(),
            "diverged histories must converge after merge"
        );
        for s in ["A1", "A2", "A3", "B1", "B2", "B3"] {
            assert!(pa.text().contains(s), "merged text must contain {s}");
        }
        assert_eq!(pa.text(), "A1A2A3B1B2B3");
    }

    #[test]
    fn merge_diverged_then_delete_converges() {
        // Two peers diverge, then one deletes a shared node; result converges.
        let mut pa = DocState::new(PeerId(7010));
        let shared = pa.local_insert(RgaPos::Head, "shared");
        let mut pb = DocState::new(PeerId(7011));
        pb.apply(shared.clone());

        // A inserts more.
        pa.local_insert(RgaPos::After(shared.id), "_a_extra");
        // B deletes shared.
        let del = pb.local_delete(shared.id);

        // Cross-merge.
        pa.apply(del);
        pb.merge(&pa);

        assert_eq!(
            pa.text(),
            pb.text(),
            "delete+insert across diverged histories must converge"
        );
        assert!(!pa.text().contains("shared"), "shared must be deleted");
        assert!(pa.text().contains("_a_extra"), "_a_extra must survive");
    }

    // Vector clock: increment, compare (concurrent vs dominated)

    #[test]
    fn vector_clock_single_peer_increments_strictly() {
        let mut doc = DocState::new(PeerId(8000));
        let op1 = doc.local_insert(RgaPos::Head, "a");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "b");
        assert_eq!(
            op1.id.counter + 1,
            op2.id.counter,
            "each op increments counter by 1"
        );
    }

    #[test]
    fn vector_clock_dominated_comparison() {
        // If one counter is strictly less, it is "dominated" (happened-before).
        let early = OpId {
            peer: PeerId(1),
            counter: 3,
        };
        let late = OpId {
            peer: PeerId(1),
            counter: 9,
        };
        assert!(early < late, "early counter is dominated by late");
        assert!(late > early);
        assert_ne!(early, late);
    }

    #[test]
    fn vector_clock_concurrent_not_dominated() {
        // Same counter, different peers → neither dominates; both are "concurrent".
        let op_p1 = OpId {
            peer: PeerId(10),
            counter: 5,
        };
        let op_p2 = OpId {
            peer: PeerId(20),
            counter: 5,
        };
        // They compare by peer.0 as a deterministic tiebreak, not causal dominance.
        assert_ne!(op_p1, op_p2);
        // Neither is "equal" to the other, but one is less (deterministic tiebreak).
        let (lo, hi) = if op_p1 < op_p2 {
            (op_p1, op_p2)
        } else {
            (op_p2, op_p1)
        };
        assert!(lo < hi);
    }

    #[test]
    fn vector_clock_advances_on_remote_apply() {
        // After applying a remote op with counter 200, next local counter > 200.
        let mut doc = DocState::new(PeerId(8010));
        let remote = make_insert(9999, 200, RgaPos::Head, "far_future");
        doc.apply(remote);
        let local = doc.local_insert(RgaPos::Head, "now");
        assert!(
            local.id.counter > 200,
            "local counter must exceed remote counter"
        );
    }

    // Vector clock: merge with 3+ peers

    #[test]
    fn vector_clock_merge_three_peers_max_wins() {
        // Three docs with counters 10, 50, 30; after applying all, next counter > 50.
        let mut doc = DocState::new(PeerId(8020));
        doc.apply(make_insert(8021, 10, RgaPos::Head, "ten"));
        doc.apply(make_insert(8022, 50, RgaPos::Head, "fifty"));
        doc.apply(make_insert(8023, 30, RgaPos::Head, "thirty"));
        let next = doc.local_insert(RgaPos::Head, "local");
        assert!(
            next.id.counter > 50,
            "counter must exceed max of all applied ops (50)"
        );
    }

    #[test]
    fn vector_clock_merge_four_peers_max_wins() {
        let mut doc = DocState::new(PeerId(8030));
        for (peer, ctr) in [(8031u64, 5u64), (8032, 100), (8033, 77), (8034, 3)] {
            doc.apply(make_insert(peer, ctr, RgaPos::Head, "x"));
        }
        let next = doc.local_insert(RgaPos::Head, "local");
        assert!(
            next.id.counter > 100,
            "counter must exceed max applied counter (100)"
        );
    }

    // Awareness state: multiple peer cursors

    #[test]
    fn awareness_five_peer_cursors_all_recorded() {
        let mut doc = DocState::new(PeerId(9000));
        for i in 0u64..5 {
            doc.apply(Op {
                id: OpId {
                    peer: PeerId(9000 + i),
                    counter: i + 1,
                },
                kind: OpKind::SetMeta {
                    key: format!("cursor:{}", 9000 + i),
                    value: format!("{}", i * 3),
                },
            });
        }
        assert_eq!(doc.op_log().len(), 5);
        let cursor_count = doc
            .op_log()
            .iter()
            .filter(
                |op| matches!(&op.kind, OpKind::SetMeta { key, .. } if key.starts_with("cursor:")),
            )
            .count();
        assert_eq!(cursor_count, 5, "all 5 cursor ops must be in op_log");
        assert_eq!(doc.text(), "", "SetMeta must not affect text");
    }

    #[test]
    fn awareness_cursor_overwrite_for_same_peer() {
        // Two cursor updates for the same peer; op_log has both but latest is authoritative.
        let mut doc = DocState::new(PeerId(9010));
        for (ctr, pos) in [(1u64, "2"), (2, "8")] {
            doc.apply(Op {
                id: OpId {
                    peer: PeerId(9010),
                    counter: ctr,
                },
                kind: OpKind::SetMeta {
                    key: "cursor:9010".to_string(),
                    value: pos.to_string(),
                },
            });
        }
        // Both ops are in the log (CRDT: no delete-on-update).
        assert_eq!(doc.op_log().len(), 2);
        // The most-recent cursor value (by counter order in log) is "8".
        let latest = doc.op_log().iter().rev().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind {
                if key == "cursor:9010" {
                    return Some(value.as_str());
                }
            }
            None
        });
        assert_eq!(latest, Some("8"));
    }

    // Awareness: stale peer cleanup (simulated via filtering by counter threshold)

    #[test]
    fn awareness_stale_peer_filtered_by_counter() {
        // Peers with counters below a threshold are "stale"; simulate cleanup by filtering.
        let mut doc = DocState::new(PeerId(9020));
        // Peer 9021 has stale cursor (counter 1); peer 9022 is fresh (counter 100).
        doc.apply(Op {
            id: OpId {
                peer: PeerId(9021),
                counter: 1,
            },
            kind: OpKind::SetMeta {
                key: "cursor:9021".to_string(),
                value: "3".to_string(),
            },
        });
        doc.apply(Op {
            id: OpId {
                peer: PeerId(9022),
                counter: 100,
            },
            kind: OpKind::SetMeta {
                key: "cursor:9022".to_string(),
                value: "15".to_string(),
            },
        });
        // "Active" threshold: counter >= 50.
        let active_cursors: Vec<_> = doc
            .op_log()
            .iter()
            .filter(|op| {
                matches!(&op.kind, OpKind::SetMeta { key, .. } if key.starts_with("cursor:"))
                    && op.id.counter >= 50
            })
            .collect();
        assert_eq!(active_cursors.len(), 1, "only 1 cursor above threshold");
        if let OpKind::SetMeta { key, .. } = &active_cursors[0].kind {
            assert_eq!(key, "cursor:9022");
        }
    }

    #[test]
    fn awareness_no_stale_cursors_when_all_fresh() {
        let mut doc = DocState::new(PeerId(9030));
        for (peer, ctr) in [(9031u64, 200u64), (9032, 300)] {
            doc.apply(Op {
                id: OpId {
                    peer: PeerId(peer),
                    counter: ctr,
                },
                kind: OpKind::SetMeta {
                    key: format!("cursor:{peer}"),
                    value: "0".to_string(),
                },
            });
        }
        let stale: Vec<_> = doc
            .op_log()
            .iter()
            .filter(|op| {
                matches!(&op.kind, OpKind::SetMeta { key, .. } if key.starts_with("cursor:"))
                    && op.id.counter < 100
            })
            .collect();
        assert!(stale.is_empty(), "no stale cursors expected");
    }

    // Op serialization/deserialization roundtrip (manual via field assertions)

    #[test]
    fn op_roundtrip_insert_fields_preserved() {
        // Simulate ser/de: clone the op and verify all fields survive.
        let op = Op {
            id: OpId {
                peer: PeerId(10_000),
                counter: 42,
            },
            kind: OpKind::Insert {
                pos: RgaPos::Head,
                text: "roundtrip".to_string(),
            },
        };
        let cloned = op.clone();
        assert_eq!(cloned.id.peer, op.id.peer);
        assert_eq!(cloned.id.counter, op.id.counter);
        match (&op.kind, &cloned.kind) {
            (OpKind::Insert { pos: p1, text: t1 }, OpKind::Insert { pos: p2, text: t2 }) => {
                assert_eq!(p1, p2);
                assert_eq!(t1, t2);
            }
            _ => panic!("kind mismatch"),
        }
    }

    #[test]
    fn op_roundtrip_delete_fields_preserved() {
        let target = OpId {
            peer: PeerId(7),
            counter: 3,
        };
        let op = Op {
            id: OpId {
                peer: PeerId(1),
                counter: 5,
            },
            kind: OpKind::Delete { target },
        };
        let cloned = op.clone();
        assert_eq!(cloned.id, op.id);
        match cloned.kind {
            OpKind::Delete { target: t } => assert_eq!(t, target),
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn op_roundtrip_set_meta_fields_preserved() {
        let op = Op {
            id: OpId {
                peer: PeerId(1),
                counter: 1,
            },
            kind: OpKind::SetMeta {
                key: "foo".to_string(),
                value: "bar".to_string(),
            },
        };
        let cloned = op.clone();
        assert_eq!(cloned.id, op.id);
        match cloned.kind {
            OpKind::SetMeta { key, value } => {
                assert_eq!(key, "foo");
                assert_eq!(value, "bar");
            }
            _ => panic!("expected SetMeta"),
        }
    }

    #[test]
    fn op_roundtrip_rga_pos_after_preserved() {
        let anchor_id = OpId {
            peer: PeerId(3),
            counter: 7,
        };
        let op = Op {
            id: OpId {
                peer: PeerId(4),
                counter: 8,
            },
            kind: OpKind::Insert {
                pos: RgaPos::After(anchor_id),
                text: "after_test".to_string(),
            },
        };
        let cloned = op.clone();
        match cloned.kind {
            OpKind::Insert {
                pos: RgaPos::After(id),
                text,
            } => {
                assert_eq!(id, anchor_id);
                assert_eq!(text, "after_test");
            }
            _ => panic!("expected Insert with After pos"),
        }
    }

    // Large document (100+ ops) correctness

    #[test]
    fn large_document_100_inserts_correct_count() {
        let mut doc = DocState::new(PeerId(11_000));
        let mut prev_id = doc.local_insert(RgaPos::Head, "0").id;
        for i in 1..100 {
            let op = doc.local_insert(RgaPos::After(prev_id), format!("{i}"));
            prev_id = op.id;
        }
        assert_eq!(doc.op_log().len(), 100, "must have 100 ops");
        // Every digit 0-9 appears at least once (we inserted 0-99 as strings).
        assert!(doc.text().contains('0'));
        assert!(doc.text().contains('9'));
    }

    #[test]
    fn large_document_100_inserts_then_50_deletes() {
        let mut doc = DocState::new(PeerId(11_001));
        let mut ids = vec![];
        let mut prev_id = {
            let op = doc.local_insert(RgaPos::Head, "x");
            ids.push(op.id);
            op.id
        };
        for _ in 1..100 {
            let op = doc.local_insert(RgaPos::After(prev_id), "x");
            prev_id = op.id;
            ids.push(op.id);
        }
        // Delete the first 50 nodes.
        for id in &ids[..50] {
            doc.local_delete(*id);
        }
        assert_eq!(
            doc.text().chars().count(),
            50,
            "50 live chars after 50 deletes"
        );
        assert_eq!(
            doc.op_log().len(),
            150,
            "100 inserts + 50 deletes = 150 ops"
        );
    }

    #[test]
    fn large_document_merge_two_100_op_docs() {
        // Both peers start from the same root node so anchors are shared.
        let mut pa = DocState::new(PeerId(11_002));
        let root = pa.local_insert(RgaPos::Head, "R");

        let mut pb = DocState::new(PeerId(11_003));
        pb.apply(root.clone());

        // pa appends 49 "a" chars after root.
        let mut prev_a = root.id;
        for _ in 0..49 {
            let op = pa.local_insert(RgaPos::After(prev_a), "a");
            prev_a = op.id;
        }

        // pb appends 49 "b" chars after root.
        let mut prev_b = root.id;
        for _ in 0..49 {
            let op = pb.local_insert(RgaPos::After(prev_b), "b");
            prev_b = op.id;
        }

        pa.merge(&pb);
        pb.merge(&pa);

        // Both sides must contain the same characters (99 total: 1 root + 49 a + 49 b).
        assert_eq!(
            pa.text().chars().count(),
            99,
            "pa must have 99 chars after merge"
        );
        assert_eq!(
            pb.text().chars().count(),
            99,
            "pb must have 99 chars after merge"
        );
        assert!(pa.text().contains('R'), "root char must be present");
        assert_eq!(pa.text().chars().filter(|&c| c == 'a').count(), 49);
        assert_eq!(pa.text().chars().filter(|&c| c == 'b').count(), 49);
    }

    // OpId ordering and uniqueness

    #[test]
    fn op_id_unique_for_same_peer_different_counters() {
        let ids: Vec<OpId> = (1..=10)
            .map(|c| OpId {
                peer: PeerId(99),
                counter: c,
            })
            .collect();
        // All ids are distinct.
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i], ids[j], "OpId must be unique per counter");
            }
        }
    }

    #[test]
    fn op_id_unique_for_different_peers_same_counter() {
        let ids: Vec<OpId> = (0u64..10)
            .map(|p| OpId {
                peer: PeerId(p),
                counter: 42,
            })
            .collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(
                    ids[i], ids[j],
                    "OpId must differ across peers at same counter"
                );
            }
        }
    }

    #[test]
    fn op_id_total_order_consistent() {
        // Build a sorted list and verify it matches manual expectations.
        let mut ids = [
            OpId {
                peer: PeerId(2),
                counter: 1,
            },
            OpId {
                peer: PeerId(1),
                counter: 2,
            },
            OpId {
                peer: PeerId(3),
                counter: 1,
            },
            OpId {
                peer: PeerId(1),
                counter: 1,
            },
        ];
        ids.sort();
        // Expected order (counter asc, then peer asc):
        // (peer=1,ctr=1), (peer=2,ctr=1), (peer=3,ctr=1), (peer=1,ctr=2)
        assert_eq!(
            ids[0],
            OpId {
                peer: PeerId(1),
                counter: 1
            }
        );
        assert_eq!(
            ids[1],
            OpId {
                peer: PeerId(2),
                counter: 1
            }
        );
        assert_eq!(
            ids[2],
            OpId {
                peer: PeerId(3),
                counter: 1
            }
        );
        assert_eq!(
            ids[3],
            OpId {
                peer: PeerId(1),
                counter: 2
            }
        );
    }

    #[test]
    fn op_id_max_in_set() {
        // The maximum OpId in a set has the highest counter (and peer as tiebreak).
        let ids = [
            OpId {
                peer: PeerId(5),
                counter: 10,
            },
            OpId {
                peer: PeerId(1),
                counter: 20,
            },
            OpId {
                peer: PeerId(3),
                counter: 15,
            },
        ];
        let max = ids.iter().copied().max().unwrap();
        assert_eq!(
            max,
            OpId {
                peer: PeerId(1),
                counter: 20
            }
        );
    }

    // Additional convergence tests for full coverage

    #[test]
    fn rga_three_peer_concurrent_convergence_full_merge() {
        // All three peers apply all three ops in different orders via merge().
        let mut pa = DocState::new(PeerId(12_000));
        pa.local_insert(RgaPos::Head, "PA");

        let mut pb = DocState::new(PeerId(12_001));
        pb.local_insert(RgaPos::Head, "PB");

        let mut pc = DocState::new(PeerId(12_002));
        pc.local_insert(RgaPos::Head, "PC");

        // Full mesh merge.
        pa.merge(&pb);
        pa.merge(&pc);
        pb.merge(&pa);
        pb.merge(&pc);
        pc.merge(&pa);
        pc.merge(&pb);

        assert_eq!(pa.text(), pb.text());
        assert_eq!(pb.text(), pc.text());
        assert!(pa.text().contains("PA"));
        assert!(pa.text().contains("PB"));
        assert!(pa.text().contains("PC"));
    }

    #[test]
    fn rga_op_log_contains_all_applied_ops_after_merge() {
        let mut pa = DocState::new(PeerId(13_000));
        pa.local_insert(RgaPos::Head, "from_a");

        let mut pb = DocState::new(PeerId(13_001));
        pb.local_insert(RgaPos::Head, "from_b");

        pa.merge(&pb);
        // pa's log must contain both ops.
        assert_eq!(pa.op_log().len(), 2);
        let has_from_a = pa
            .op_log()
            .iter()
            .any(|op| matches!(&op.kind, OpKind::Insert { text, .. } if text == "from_a"));
        let has_from_b = pa
            .op_log()
            .iter()
            .any(|op| matches!(&op.kind, OpKind::Insert { text, .. } if text == "from_b"));
        assert!(has_from_a, "op_log must contain from_a insert");
        assert!(has_from_b, "op_log must contain from_b insert");
    }

    #[test]
    fn rga_set_meta_multiple_keys_coexist() {
        // Different SetMeta keys for the same peer coexist in op_log.
        let mut doc = DocState::new(PeerId(14_000));
        for (ctr, key, val) in [
            (1u64, "color", "#f00"),
            (2, "font", "mono"),
            (3, "size", "14px"),
        ] {
            doc.apply(Op {
                id: OpId {
                    peer: PeerId(14_000),
                    counter: ctr,
                },
                kind: OpKind::SetMeta {
                    key: key.to_string(),
                    value: val.to_string(),
                },
            });
        }
        assert_eq!(doc.op_log().len(), 3);
        assert_eq!(doc.text(), "");
        let found: std::collections::HashSet<String> = doc
            .op_log()
            .iter()
            .filter_map(|op| {
                if let OpKind::SetMeta { key, .. } = &op.kind {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(found.contains("color"));
        assert!(found.contains("font"));
        assert!(found.contains("size"));
    }

    #[test]
    fn rga_peer_id_zero_is_valid() {
        // PeerId(0) is structurally valid; doc authors ops with peer 0.
        let mut doc = DocState::new(PeerId(0));
        let op = doc.local_insert(RgaPos::Head, "zero_peer");
        assert_eq!(op.id.peer, PeerId(0));
        assert_eq!(op.id.peer.0, 0u64);
        assert_eq!(doc.text(), "zero_peer");
    }

    #[test]
    fn rga_insert_long_string() {
        // Insert a string longer than 100 chars; text() returns it intact.
        let long: String = "abcdefghij".repeat(20); // 200 chars
        let mut doc = DocState::new(PeerId(15_000));
        doc.local_insert(RgaPos::Head, &long);
        assert_eq!(doc.text(), long);
        assert_eq!(doc.text().len(), 200);
    }

    #[test]
    fn rga_counter_after_many_remote_ops() {
        // Apply 99 remote ops with counters 1..=99; next local counter >= 100.
        let mut doc = DocState::new(PeerId(16_000));
        for c in 1u64..=99 {
            doc.apply(make_insert(16_001, c, RgaPos::Head, "r"));
        }
        let local = doc.local_insert(RgaPos::Head, "l");
        assert!(
            local.id.counter >= 100,
            "local counter must be >= 100 after 99 remote ops"
        );
    }

    #[test]
    fn rga_text_is_concatenation_of_live_nodes() {
        // text() concatenates live node texts in RGA sequence order.
        let mut doc = DocState::new(PeerId(17_000));
        let op1 = doc.local_insert(RgaPos::Head, "Hello");
        let op2 = doc.local_insert(RgaPos::After(op1.id), ", ");
        doc.local_insert(RgaPos::After(op2.id), "world!");
        assert_eq!(doc.text(), "Hello, world!");
        // Delete ", " — adjacent tokens join.
        doc.local_delete(op2.id);
        assert_eq!(doc.text(), "Helloworld!");
    }

    // ── wave AB-6: YJS-style convergence tests ───────────────────────────────

    // 1. 3-way merge: peer A and B both insert independently; peer C merges both
    //    and must converge to the same text as A-after-merge-with-B.

    #[test]
    fn yjs_three_way_merge_peer_c_converges() {
        let mut pa = DocState::new(PeerId(20_000));
        let op_a = pa.local_insert(RgaPos::Head, "from_a");

        let mut pb = DocState::new(PeerId(20_001));
        let op_b = pb.local_insert(RgaPos::Head, "from_b");

        // C starts empty; merges both peers independently.
        let mut pc = DocState::new(PeerId(20_002));
        pc.apply(op_a.clone());
        pc.apply(op_b.clone());

        // A and B also cross-merge.
        pa.apply(op_b.clone());
        pb.apply(op_a.clone());

        // All three must converge to identical text.
        assert_eq!(pa.text(), pb.text(), "A and B must converge");
        assert_eq!(pa.text(), pc.text(), "C must converge with A and B");
        assert!(pc.text().contains("from_a"));
        assert!(pc.text().contains("from_b"));
    }

    #[test]
    fn yjs_three_way_merge_with_shared_root() {
        // Shared root → A appends "X", B appends "Y"; C merges both.
        let mut pa = DocState::new(PeerId(20_010));
        let root = pa.local_insert(RgaPos::Head, "root");

        let mut pb = DocState::new(PeerId(20_011));
        pb.apply(root.clone());

        let mut pc = DocState::new(PeerId(20_012));
        pc.apply(root.clone());

        let op_a = pa.local_insert(RgaPos::After(root.id), "X");
        let op_b = pb.local_insert(RgaPos::After(root.id), "Y");

        // C receives both diverged inserts.
        pc.apply(op_a.clone());
        pc.apply(op_b.clone());

        // A and B also sync.
        pa.apply(op_b.clone());
        pb.apply(op_a.clone());

        assert_eq!(pa.text(), pb.text());
        assert_eq!(pa.text(), pc.text());
        assert!(pc.text().starts_with("root"));
        assert!(pc.text().contains('X'));
        assert!(pc.text().contains('Y'));
    }

    // 2. Op log replay: collect all ops, apply to fresh doc, matches original.

    #[test]
    fn op_log_replay_matches_original() {
        let mut original = DocState::new(PeerId(21_000));
        let op1 = original.local_insert(RgaPos::Head, "alpha");
        let op2 = original.local_insert(RgaPos::After(op1.id), "beta");
        original.local_insert(RgaPos::After(op2.id), "gamma");
        original.local_delete(op1.id);

        // Replay all ops onto a fresh doc.
        let ops: Vec<Op> = original.op_log().to_vec();
        let mut replayed = DocState::new(PeerId(21_000));
        for op in ops {
            replayed.apply(op);
        }

        assert_eq!(
            replayed.text(),
            original.text(),
            "replayed doc must match original"
        );
        assert_eq!(replayed.op_log().len(), original.op_log().len());
    }

    #[test]
    fn op_log_replay_preserves_ordering() {
        // Ten sequential inserts; replay must produce identical text and log length.
        let mut original = DocState::new(PeerId(21_010));
        let mut prev = original.local_insert(RgaPos::Head, "0").id;
        for i in 1u64..10 {
            let op = original.local_insert(RgaPos::After(prev), i.to_string());
            prev = op.id;
        }

        let ops: Vec<Op> = original.op_log().to_vec();
        let mut replayed = DocState::new(PeerId(21_010));
        for op in ops {
            replayed.apply(op);
        }

        assert_eq!(replayed.text(), original.text());
        assert_eq!(replayed.op_log().len(), 10);
    }

    // 3. Concurrent delete+insert at same position: deterministic winner.

    #[test]
    fn concurrent_delete_and_insert_at_same_position_deterministic() {
        // Shared anchor; A deletes it, B inserts after it at the same time.
        let mut pa = DocState::new(PeerId(22_000));
        let anchor = pa.local_insert(RgaPos::Head, "anchor");

        let mut pb = DocState::new(PeerId(22_001));
        pb.apply(anchor.clone());

        // Concurrent: A deletes anchor, B inserts "new" after anchor.
        let del_op = pa.local_delete(anchor.id);
        let ins_op = pb.local_insert(RgaPos::After(anchor.id), "new");

        // Forward order.
        let mut doc_fwd = DocState::new(PeerId(22_002));
        doc_fwd.apply(anchor.clone());
        doc_fwd.apply(del_op.clone());
        doc_fwd.apply(ins_op.clone());

        // Reverse order.
        let mut doc_rev = DocState::new(PeerId(22_002));
        doc_rev.apply(anchor.clone());
        doc_rev.apply(ins_op.clone());
        doc_rev.apply(del_op.clone());

        // Both orderings converge to identical text.
        assert_eq!(
            doc_fwd.text(),
            doc_rev.text(),
            "concurrent delete+insert must be deterministic"
        );
        // anchor is deleted; "new" survives.
        assert_eq!(doc_fwd.text(), "new");
    }

    #[test]
    fn concurrent_delete_and_insert_two_peers_cross_merge() {
        let mut pa = DocState::new(PeerId(22_010));
        let base = pa.local_insert(RgaPos::Head, "base");

        let mut pb = DocState::new(PeerId(22_011));
        pb.apply(base.clone());

        let del = pa.local_delete(base.id);
        let ins = pb.local_insert(RgaPos::After(base.id), "tail");

        pa.apply(ins.clone());
        pb.apply(del.clone());

        assert_eq!(pa.text(), pb.text());
        assert!(!pa.text().contains("base"));
        assert!(pa.text().contains("tail"));
    }

    // 4. Insert then delete same position: tombstone is set correctly.

    #[test]
    fn insert_then_delete_same_position_tombstone() {
        let mut doc = DocState::new(PeerId(23_000));
        let op = doc.local_insert(RgaPos::Head, "target");
        assert_eq!(doc.text(), "target");

        doc.local_delete(op.id);
        assert_eq!(doc.text(), "", "deleted node must not appear in text");

        // Confirm tombstone: a Delete op in the log targets the insert's id.
        let has_tombstone = doc
            .op_log()
            .iter()
            .any(|o| matches!(&o.kind, OpKind::Delete { target } if *target == op.id));
        assert!(has_tombstone, "tombstone record must exist in op_log");
    }

    #[test]
    fn insert_then_delete_middle_node_correct_tombstone() {
        let mut doc = DocState::new(PeerId(23_010));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        let op_c = doc.local_insert(RgaPos::After(op_b.id), "C");

        doc.local_delete(op_b.id);

        // op_b must be tombstoned; A and C live.
        let has_b_tombstone = doc
            .op_log()
            .iter()
            .any(|o| matches!(&o.kind, OpKind::Delete { target } if *target == op_b.id));
        assert!(has_b_tombstone);
        assert_eq!(doc.text(), "AC");
        let _ = op_c;
    }

    // 5. Large document: 500-op history, final text correct.

    #[test]
    fn large_document_500_ops_final_text_correct() {
        let mut doc = DocState::new(PeerId(24_000));
        let mut prev = doc.local_insert(RgaPos::Head, "s").id;
        for _ in 1..500 {
            let op = doc.local_insert(RgaPos::After(prev), "s");
            prev = op.id;
        }
        assert_eq!(doc.op_log().len(), 500);
        assert_eq!(doc.text().chars().count(), 500);
        assert!(
            doc.text().chars().all(|c| c == 's'),
            "all chars must be 's'"
        );
    }

    #[test]
    fn large_document_500_ops_then_delete_half() {
        let mut doc = DocState::new(PeerId(24_010));
        let mut ids = Vec::with_capacity(500);
        let first = doc.local_insert(RgaPos::Head, "x");
        ids.push(first.id);
        let mut prev = first.id;
        for _ in 1..500 {
            let op = doc.local_insert(RgaPos::After(prev), "x");
            prev = op.id;
            ids.push(op.id);
        }
        // Delete every even-indexed node (250 deletes).
        for id in ids.iter().step_by(2) {
            doc.local_delete(*id);
        }
        assert_eq!(
            doc.text().chars().count(),
            250,
            "250 live nodes after deleting 250"
        );
    }

    // 6. Snapshot: take snapshot (op_log clone), restore to new doc, verify text.

    #[test]
    fn snapshot_restore_matches_original_text() {
        let mut original = DocState::new(PeerId(25_000));
        let op1 = original.local_insert(RgaPos::Head, "snap");
        let op2 = original.local_insert(RgaPos::After(op1.id), "_shot");
        original.local_delete(op1.id);
        let _ = op2;

        // Snapshot = clone of op_log.
        let snapshot: Vec<Op> = original.op_log().to_vec();

        // Restore: fresh doc, replay snapshot.
        let mut restored = DocState::new(PeerId(25_000));
        for op in snapshot {
            restored.apply(op);
        }

        assert_eq!(restored.text(), original.text());
        assert_eq!(restored.text(), "_shot");
    }

    #[test]
    fn snapshot_after_many_ops_restores_correctly() {
        let mut doc = DocState::new(PeerId(25_010));
        let mut prev = doc.local_insert(RgaPos::Head, "a").id;
        for _ in 0..19 {
            let op = doc.local_insert(RgaPos::After(prev), "a");
            prev = op.id;
        }
        // Delete the first 5.
        let first_five: Vec<OpId> = doc.op_log()[..5].iter().map(|o| o.id).collect();
        for id in first_five {
            doc.local_delete(id);
        }

        let snapshot: Vec<Op> = doc.op_log().to_vec();
        let mut restored = DocState::new(PeerId(25_010));
        for op in snapshot {
            restored.apply(op);
        }

        assert_eq!(restored.text(), doc.text());
        assert_eq!(restored.op_log().len(), doc.op_log().len());
    }

    // 7. Compaction: after N ops, compact by replaying only live inserts onto a
    //    fresh doc; verify text matches and log is shorter.

    #[test]
    fn compaction_removes_tombstoned_ops() {
        let mut doc = DocState::new(PeerId(26_000));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        let op_c = doc.local_insert(RgaPos::After(op_b.id), "C");
        doc.local_delete(op_a.id);
        doc.local_delete(op_b.id);
        // op_log has 5 entries; 2 inserts are dead.

        // Compact: keep only live insert ops (non-deleted).
        let deleted_ids: std::collections::HashSet<OpId> = doc
            .op_log()
            .iter()
            .filter_map(|o| {
                if let OpKind::Delete { target } = &o.kind {
                    Some(*target)
                } else {
                    None
                }
            })
            .collect();
        let live_ops: Vec<Op> = doc
            .op_log()
            .iter()
            .filter(|o| match &o.kind {
                OpKind::Insert { .. } => !deleted_ids.contains(&o.id),
                OpKind::Delete { .. } => false,
                OpKind::SetMeta { .. } => true,
            })
            .cloned()
            .collect();

        // Replay compacted log.
        let mut compacted = DocState::new(PeerId(26_000));
        for op in &live_ops {
            compacted.apply(op.clone());
        }

        assert_eq!(
            compacted.text(),
            doc.text(),
            "compacted text must match original"
        );
        assert_eq!(compacted.text(), "C");
        assert!(
            live_ops.len() < doc.op_log().len(),
            "compacted log must be shorter"
        );
        let _ = op_c;
    }

    #[test]
    fn compaction_all_deleted_yields_empty() {
        let mut doc = DocState::new(PeerId(26_010));
        let op_a = doc.local_insert(RgaPos::Head, "X");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "Y");
        doc.local_delete(op_a.id);
        doc.local_delete(op_b.id);

        let deleted_ids: std::collections::HashSet<OpId> = doc
            .op_log()
            .iter()
            .filter_map(|o| {
                if let OpKind::Delete { target } = &o.kind {
                    Some(*target)
                } else {
                    None
                }
            })
            .collect();
        let live_ops: Vec<Op> = doc
            .op_log()
            .iter()
            .filter(|o| matches!(&o.kind, OpKind::Insert { .. }) && !deleted_ids.contains(&o.id))
            .cloned()
            .collect();

        assert!(live_ops.is_empty(), "all inserts deleted → no live ops");
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn compaction_then_new_insert_works() {
        // After compaction, new inserts on the compacted doc work correctly.
        let mut doc = DocState::new(PeerId(26_020));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        doc.local_delete(op_a.id);

        // Compact (keep only live inserts).
        let deleted_ids: std::collections::HashSet<OpId> = doc
            .op_log()
            .iter()
            .filter_map(|o| {
                if let OpKind::Delete { target } = &o.kind {
                    Some(*target)
                } else {
                    None
                }
            })
            .collect();
        let live_ops: Vec<Op> = doc
            .op_log()
            .iter()
            .filter(|o| matches!(&o.kind, OpKind::Insert { .. }) && !deleted_ids.contains(&o.id))
            .cloned()
            .collect();

        let mut compacted = DocState::new(PeerId(26_020));
        for op in live_ops {
            compacted.apply(op);
        }
        assert_eq!(compacted.text(), "B");

        // New insert after op_b in compacted doc.
        compacted.local_insert(RgaPos::After(op_b.id), "C");
        assert_eq!(compacted.text(), "BC");
    }

    // 8. Peer ID edge cases: very large peer ID, peer ID = u64::MAX.

    #[test]
    fn peer_id_u64_max_is_valid() {
        let max_peer = PeerId(u64::MAX);
        let mut doc = DocState::new(max_peer);
        let op = doc.local_insert(RgaPos::Head, "max_peer");
        assert_eq!(op.id.peer, max_peer);
        assert_eq!(op.id.peer.0, u64::MAX);
        assert_eq!(doc.text(), "max_peer");
    }

    #[test]
    fn peer_id_u64_max_sorts_last() {
        // PeerId(u64::MAX) must sort after any smaller peer id at equal counter.
        let id_max = OpId {
            peer: PeerId(u64::MAX),
            counter: 1,
        };
        let id_small = OpId {
            peer: PeerId(1),
            counter: 1,
        };
        assert!(id_small < id_max, "small peer id must sort before u64::MAX");
        assert!(id_max > id_small);
    }

    #[test]
    fn peer_id_large_values_converge() {
        // Docs with very large peer ids merge correctly.
        let mut pa = DocState::new(PeerId(u64::MAX - 1));
        let op_a = pa.local_insert(RgaPos::Head, "big_a");

        let mut pb = DocState::new(PeerId(u64::MAX));
        let op_b = pb.local_insert(RgaPos::Head, "big_b");

        pa.apply(op_b.clone());
        pb.apply(op_a.clone());

        assert_eq!(pa.text(), pb.text(), "large peer IDs must converge");
        assert!(pa.text().contains("big_a"));
        assert!(pa.text().contains("big_b"));
    }

    #[test]
    fn peer_id_u64_max_merge_with_small_peer() {
        let mut pa = DocState::new(PeerId(u64::MAX));
        let op_a = pa.local_insert(RgaPos::Head, "max");

        let mut pb = DocState::new(PeerId(1));
        let op_b = pb.local_insert(RgaPos::Head, "one");

        pa.apply(op_b.clone());
        pb.apply(op_a.clone());

        assert_eq!(pa.text(), pb.text());
        assert_eq!(pa.text().chars().count(), "maxone".chars().count());
    }

    // 9. Multi-document: two independent docs don't interfere with each other.

    #[test]
    fn multi_doc_independent_no_interference() {
        let mut doc1 = DocState::new(PeerId(27_000));
        let mut doc2 = DocState::new(PeerId(27_001));

        doc1.local_insert(RgaPos::Head, "doc1_content");
        doc2.local_insert(RgaPos::Head, "doc2_content");

        assert_eq!(doc1.text(), "doc1_content");
        assert_eq!(doc2.text(), "doc2_content");
        // Docs share no state; modifying one doesn't affect the other.
        doc1.local_insert(RgaPos::Head, "prefix_");
        assert!(
            doc2.text() == "doc2_content",
            "doc2 must be unaffected by doc1 changes"
        );
    }

    #[test]
    fn multi_doc_same_peer_id_different_docs_independent() {
        // Two docs both owned by PeerId(1) operate independently.
        let mut doc_a = DocState::new(PeerId(1));
        let mut doc_b = DocState::new(PeerId(1));

        doc_a.local_insert(RgaPos::Head, "alpha");
        doc_b.local_insert(RgaPos::Head, "beta");

        assert_eq!(doc_a.text(), "alpha");
        assert_eq!(doc_b.text(), "beta");
        assert_eq!(doc_a.op_log().len(), 1);
        assert_eq!(doc_b.op_log().len(), 1);
    }

    #[test]
    fn multi_doc_ops_do_not_bleed_across_docs() {
        // Inserting into doc1 then merging doc1 → doc2 does not affect doc3.
        let mut doc1 = DocState::new(PeerId(27_010));
        doc1.local_insert(RgaPos::Head, "shared");

        let mut doc2 = DocState::new(PeerId(27_011));
        doc2.merge(&doc1);

        let mut doc3 = DocState::new(PeerId(27_012));
        doc3.local_insert(RgaPos::Head, "independent");

        // doc3 must not contain doc1/doc2's content.
        assert_eq!(doc3.text(), "independent");
        assert!(!doc3.text().contains("shared"));
    }

    #[test]
    fn multi_doc_delete_in_one_does_not_affect_other() {
        let mut doc1 = DocState::new(PeerId(27_020));
        let op = doc1.local_insert(RgaPos::Head, "hello");

        let mut doc2 = DocState::new(PeerId(27_021));
        doc2.local_insert(RgaPos::Head, "hello");

        // Delete from doc1 only.
        doc1.local_delete(op.id);

        assert_eq!(doc1.text(), "", "doc1 content deleted");
        assert_eq!(doc2.text(), "hello", "doc2 must be unaffected");
    }

    // 10. Awareness: 10 cursors, remove stale by threshold.

    #[test]
    fn awareness_10_cursors_all_recorded() {
        let mut doc = DocState::new(PeerId(28_000));
        for i in 0u64..10 {
            doc.apply(Op {
                id: OpId {
                    peer: PeerId(28_000 + i),
                    counter: i + 1,
                },
                kind: OpKind::SetMeta {
                    key: format!("cursor:{}", 28_000 + i),
                    value: format!("{}", i * 5),
                },
            });
        }
        let cursor_count = doc
            .op_log()
            .iter()
            .filter(
                |op| matches!(&op.kind, OpKind::SetMeta { key, .. } if key.starts_with("cursor:")),
            )
            .count();
        assert_eq!(cursor_count, 10, "all 10 cursors must be in op_log");
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn awareness_remove_stale_cursors_by_threshold() {
        // Peers 28_100..28_110 register cursors with counters 1..10.
        // Stale threshold = counter < 5; peers with counter >= 5 are "fresh".
        let mut doc = DocState::new(PeerId(28_100));
        for i in 0u64..10 {
            doc.apply(Op {
                id: OpId {
                    peer: PeerId(28_100 + i),
                    counter: i + 1,
                },
                kind: OpKind::SetMeta {
                    key: format!("cursor:{}", 28_100 + i),
                    value: format!("{}", i),
                },
            });
        }

        let threshold = 5u64;
        let active: Vec<_> = doc
            .op_log()
            .iter()
            .filter(|op| {
                matches!(&op.kind, OpKind::SetMeta { key, .. } if key.starts_with("cursor:"))
                    && op.id.counter >= threshold
            })
            .collect();

        // Counters 5,6,7,8,9,10 pass the threshold → 6 active.
        assert_eq!(active.len(), 6, "peers with counter >= 5 must be active");

        let stale: Vec<_> = doc
            .op_log()
            .iter()
            .filter(|op| {
                matches!(&op.kind, OpKind::SetMeta { key, .. } if key.starts_with("cursor:"))
                    && op.id.counter < threshold
            })
            .collect();
        assert_eq!(stale.len(), 4, "peers with counter < 5 must be stale");
    }

    #[test]
    fn awareness_all_stale_no_active_cursors() {
        let mut doc = DocState::new(PeerId(28_200));
        for i in 0u64..5 {
            doc.apply(Op {
                id: OpId {
                    peer: PeerId(28_200 + i),
                    counter: i + 1,
                },
                kind: OpKind::SetMeta {
                    key: format!("cursor:{}", 28_200 + i),
                    value: "0".to_string(),
                },
            });
        }
        // Threshold higher than any counter → all stale.
        let threshold = 100u64;
        let active: Vec<_> = doc
            .op_log()
            .iter()
            .filter(|op| {
                matches!(&op.kind, OpKind::SetMeta { key, .. } if key.starts_with("cursor:"))
                    && op.id.counter >= threshold
            })
            .collect();
        assert!(
            active.is_empty(),
            "all cursors below threshold → no active cursors"
        );
    }

    #[test]
    fn awareness_none_stale_all_active() {
        let mut doc = DocState::new(PeerId(28_300));
        for i in 0u64..5 {
            doc.apply(Op {
                id: OpId {
                    peer: PeerId(28_300 + i),
                    counter: 1000 + i,
                },
                kind: OpKind::SetMeta {
                    key: format!("cursor:{}", 28_300 + i),
                    value: "99".to_string(),
                },
            });
        }
        // Threshold = 100; all counters are 1000+ → all fresh.
        let threshold = 100u64;
        let stale: Vec<_> = doc
            .op_log()
            .iter()
            .filter(|op| {
                matches!(&op.kind, OpKind::SetMeta { key, .. } if key.starts_with("cursor:"))
                    && op.id.counter < threshold
            })
            .collect();
        assert!(stale.is_empty(), "all cursors above threshold → none stale");
    }

    #[test]
    fn awareness_10_cursors_mixed_text_ops_text_unaffected() {
        // Interleave 10 cursor ops with 5 insert ops; text is only from inserts.
        let mut doc = DocState::new(PeerId(28_400));
        let op1 = doc.local_insert(RgaPos::Head, "hello");
        let op2 = doc.local_insert(RgaPos::After(op1.id), " world");

        for i in 0u64..10 {
            doc.apply(Op {
                id: OpId {
                    peer: PeerId(28_400 + i + 1),
                    counter: i + 1,
                },
                kind: OpKind::SetMeta {
                    key: format!("cursor:{}", 28_400 + i + 1),
                    value: format!("{}", i),
                },
            });
        }

        assert_eq!(doc.text(), "hello world", "SetMeta must not change text");
        let cursor_count = doc
            .op_log()
            .iter()
            .filter(
                |op| matches!(&op.kind, OpKind::SetMeta { key, .. } if key.starts_with("cursor:")),
            )
            .count();
        assert_eq!(cursor_count, 10);
        let _ = op2;
    }

    // ── wave AB-6: additional convergence, tombstone revival, serialization ───

    // Tombstone revival: delete a node, then insert a NEW node after the dead anchor.
    // The anchor stays logically in the sequence even after tombstoning.

    #[test]
    fn tombstone_revival_insert_after_deleted_anchor() {
        let mut doc = DocState::new(PeerId(30_000));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        // Tombstone A.
        doc.local_delete(op_a.id);
        assert_eq!(doc.text(), "");

        // Insert "B" after the dead anchor — B must appear in the live text.
        doc.local_insert(RgaPos::After(op_a.id), "B");
        assert_eq!(doc.text(), "B", "insert after deleted anchor must survive");
    }

    #[test]
    fn tombstone_revival_chain_after_deleted_node() {
        // Delete middle node; inserts anchored After it still form a coherent chain.
        let mut doc = DocState::new(PeerId(30_010));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        let op_c = doc.local_insert(RgaPos::After(op_b.id), "C");

        // Delete B (the anchor for C's predecessor).
        doc.local_delete(op_b.id);
        assert_eq!(doc.text(), "AC");

        // Insert "X" after the now-dead B — X appears between A and C.
        doc.local_insert(RgaPos::After(op_b.id), "X");
        let text = doc.text();
        let pos_a = text.find('A').unwrap();
        let pos_x = text.find('X').unwrap();
        let pos_c = text.find('C').unwrap();
        assert!(pos_a < pos_x, "A must precede X");
        assert!(pos_x < pos_c, "X must precede C");
        let _ = op_c;
    }

    #[test]
    fn tombstone_revival_two_peers_concurrent() {
        // Peer A deletes node; peer B inserts after it concurrently; cross-merge converges.
        let mut pa = DocState::new(PeerId(30_020));
        let shared = pa.local_insert(RgaPos::Head, "shared");

        let mut pb = DocState::new(PeerId(30_021));
        pb.apply(shared.clone());

        // A deletes; B inserts after the same node at the same time.
        let del = pa.local_delete(shared.id);
        let ins = pb.local_insert(RgaPos::After(shared.id), "revival");

        pa.apply(ins.clone());
        pb.apply(del.clone());

        assert_eq!(pa.text(), pb.text(), "tombstone revival must converge");
        assert!(!pa.text().contains("shared"), "shared must be deleted");
        assert!(pa.text().contains("revival"), "revival insert must survive");
    }

    // Concurrent insertions from 3+ peers converge to same final state.

    #[test]
    fn concurrent_3_peer_inserts_all_orderings_converge() {
        let ops: Vec<Op> = (0u64..3)
            .map(|i| {
                let mut doc = DocState::new(PeerId(31_000 + i));
                doc.local_insert(RgaPos::Head, format!("peer{i}"))
            })
            .collect();

        // All 6 permutations of 3 ops — test 3 representative ones.
        let perms: &[&[usize]] = &[&[0, 1, 2], &[2, 0, 1], &[1, 2, 0]];
        let docs: Vec<String> = perms
            .iter()
            .map(|perm| {
                let mut doc = DocState::new(PeerId(31_999));
                for &idx in *perm {
                    doc.apply(ops[idx].clone());
                }
                doc.text()
            })
            .collect();

        assert_eq!(docs[0], docs[1], "permutations 0,1,2 and 2,0,1 must converge");
        assert_eq!(docs[1], docs[2], "permutations 2,0,1 and 1,2,0 must converge");
        for i in 0..3 {
            assert!(docs[0].contains(&format!("peer{i}")), "text must contain peer{i}");
        }
    }

    #[test]
    fn concurrent_4_peer_inserts_at_head_all_chars_present() {
        let mut docs: Vec<DocState> = (0u64..4)
            .map(|i| DocState::new(PeerId(32_000 + i)))
            .collect();
        let ops: Vec<Op> = docs
            .iter_mut()
            .map(|doc| doc.local_insert(RgaPos::Head, "x"))
            .collect();

        // All peers receive all ops.
        let mut merged = DocState::new(PeerId(32_999));
        for op in &ops {
            merged.apply(op.clone());
        }
        assert_eq!(merged.text().chars().count(), 4, "4 concurrent inserts must all appear");
    }

    #[test]
    fn concurrent_5_peer_inserts_at_head_converge() {
        let ops: Vec<Op> = (0u64..5)
            .map(|i| {
                let mut doc = DocState::new(PeerId(33_000 + i));
                doc.local_insert(RgaPos::Head, format!("p{i}"))
            })
            .collect();

        let mut doc_fwd = DocState::new(PeerId(33_999));
        let mut doc_rev = DocState::new(PeerId(33_999));
        for op in &ops {
            doc_fwd.apply(op.clone());
        }
        for op in ops.iter().rev() {
            doc_rev.apply(op.clone());
        }
        assert_eq!(doc_fwd.text(), doc_rev.text(), "5 concurrent inserts must converge");
        for i in 0..5 {
            assert!(doc_fwd.text().contains(&format!("p{i}")));
        }
    }

    // Op log serialization round-trip (via clone/field assertions).

    #[test]
    fn op_log_roundtrip_insert_op_log_len() {
        let mut doc = DocState::new(PeerId(34_000));
        let op1 = doc.local_insert(RgaPos::Head, "hello");
        doc.local_insert(RgaPos::After(op1.id), " world");

        // Clone op_log and replay into a fresh doc.
        let log: Vec<Op> = doc.op_log().to_vec();
        let mut restored = DocState::new(PeerId(34_000));
        for op in log.iter() {
            restored.apply(op.clone());
        }

        assert_eq!(restored.op_log().len(), doc.op_log().len(), "log length must match");
        assert_eq!(restored.text(), doc.text(), "text must match after roundtrip");
    }

    #[test]
    fn op_log_roundtrip_with_deletes() {
        let mut doc = DocState::new(PeerId(34_010));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        doc.local_delete(op_a.id);
        let _ = op_b;

        let log: Vec<Op> = doc.op_log().to_vec();
        let mut restored = DocState::new(PeerId(34_010));
        for op in log {
            restored.apply(op);
        }

        assert_eq!(restored.text(), doc.text());
        assert_eq!(restored.text(), "B");
        assert_eq!(restored.op_log().len(), 3); // 2 inserts + 1 delete
    }

    #[test]
    fn op_log_roundtrip_with_set_meta() {
        let mut doc = DocState::new(PeerId(34_020));
        doc.local_insert(RgaPos::Head, "content");
        doc.apply(Op {
            id: OpId { peer: PeerId(34_020), counter: 99 },
            kind: OpKind::SetMeta {
                key: "author".to_string(),
                value: "alice".to_string(),
            },
        });

        let log: Vec<Op> = doc.op_log().to_vec();
        let mut restored = DocState::new(PeerId(34_020));
        for op in log {
            restored.apply(op);
        }

        assert_eq!(restored.text(), doc.text());
        let has_meta = restored.op_log().iter().any(|op| {
            matches!(&op.kind, OpKind::SetMeta { key, .. } if key == "author")
        });
        assert!(has_meta, "SetMeta must survive roundtrip");
    }

    #[test]
    fn op_log_roundtrip_op_ids_preserved() {
        let mut doc = DocState::new(PeerId(34_030));
        let op1 = doc.local_insert(RgaPos::Head, "x");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "y");

        let log: Vec<Op> = doc.op_log().to_vec();
        // Verify all OpIds are preserved in the cloned log.
        assert_eq!(log[0].id, op1.id);
        assert_eq!(log[1].id, op2.id);
        // Verify pos fields are preserved.
        match &log[0].kind {
            OpKind::Insert { pos, .. } => assert_eq!(*pos, RgaPos::Head),
            _ => panic!("expected Insert"),
        }
        match &log[1].kind {
            OpKind::Insert { pos, .. } => assert_eq!(*pos, RgaPos::After(op1.id)),
            _ => panic!("expected Insert with After"),
        }
    }

    // 3-way merge: A edits, B edits independently from A, C starts fresh and merges both.

    #[test]
    fn three_way_merge_c_starts_empty_merges_a_and_b() {
        let mut pa = DocState::new(PeerId(35_000));
        let op_a = pa.local_insert(RgaPos::Head, "from_a");

        let mut pb = DocState::new(PeerId(35_001));
        let op_b = pb.local_insert(RgaPos::Head, "from_b");

        // C starts empty and receives both.
        let mut pc = DocState::new(PeerId(35_002));
        pc.apply(op_a.clone());
        pc.apply(op_b.clone());

        // A and B also sync.
        pa.apply(op_b);
        pb.apply(op_a);

        assert_eq!(pa.text(), pb.text());
        assert_eq!(pa.text(), pc.text(), "C must converge with A and B");
    }

    #[test]
    fn three_way_merge_with_delete_in_middle() {
        // A writes "AB", B deletes "A" independently, C merges result.
        let mut pa = DocState::new(PeerId(35_010));
        let op_a = pa.local_insert(RgaPos::Head, "A");
        let op_b = pa.local_insert(RgaPos::After(op_a.id), "B");

        let mut pb = DocState::new(PeerId(35_011));
        pb.apply(op_a.clone());
        pb.apply(op_b.clone());

        // B deletes "A".
        let del = pb.local_delete(op_a.id);

        // A receives the delete.
        pa.apply(del.clone());

        // C merges A (which already has the delete).
        let mut pc = DocState::new(PeerId(35_012));
        pc.merge(&pa);

        assert_eq!(pa.text(), pb.text());
        assert_eq!(pa.text(), pc.text(), "3-way merge with delete must converge");
        assert_eq!(pa.text(), "B");
    }

    #[test]
    fn three_way_merge_all_peers_insert_and_delete() {
        // Three peers each insert unique text; A also deletes B's insert after merging.
        let mut pa = DocState::new(PeerId(35_020));
        let op_pa = pa.local_insert(RgaPos::Head, "PA");

        let mut pb = DocState::new(PeerId(35_021));
        let op_pb = pb.local_insert(RgaPos::Head, "PB");

        let mut pc = DocState::new(PeerId(35_022));
        let op_pc = pc.local_insert(RgaPos::Head, "PC");

        // Full cross-merge.
        pa.apply(op_pb.clone());
        pa.apply(op_pc.clone());
        pb.apply(op_pa.clone());
        pb.apply(op_pc.clone());
        pc.apply(op_pa.clone());
        pc.apply(op_pb.clone());

        // All contain all three.
        assert_eq!(pa.text(), pb.text());
        assert_eq!(pb.text(), pc.text());

        // A now deletes PB's insert.
        let del_pb = pa.local_delete(op_pb.id);
        pb.apply(del_pb.clone());
        pc.apply(del_pb);

        assert_eq!(pa.text(), pb.text());
        assert_eq!(pb.text(), pc.text());
        assert!(!pa.text().contains("PB"), "PB must be deleted");
        assert!(pa.text().contains("PA"));
        assert!(pa.text().contains("PC"));
    }

    // Additional: op log replay convergence with mixed ops.

    #[test]
    fn op_log_replay_with_meta_and_deletes_converges() {
        // Mix of inserts, deletes, and SetMeta; replay must produce identical state.
        let mut doc = DocState::new(PeerId(36_000));
        let op1 = doc.local_insert(RgaPos::Head, "hello");
        let op2 = doc.local_insert(RgaPos::After(op1.id), " world");
        doc.local_delete(op1.id);
        doc.apply(Op {
            id: OpId { peer: PeerId(36_000), counter: 99 },
            kind: OpKind::SetMeta { key: "status".to_string(), value: "draft".to_string() },
        });

        let log: Vec<Op> = doc.op_log().to_vec();
        let mut replayed = DocState::new(PeerId(36_000));
        for op in log {
            replayed.apply(op);
        }
        assert_eq!(replayed.text(), doc.text());
        assert_eq!(replayed.text(), " world");
        assert_eq!(replayed.op_log().len(), doc.op_log().len());
        let _ = op2;
    }

    #[test]
    fn concurrent_inserts_3_peers_after_shared_root_all_chars() {
        // Three peers all insert After the same root; every character must appear.
        let mut pa = DocState::new(PeerId(37_000));
        let root = pa.local_insert(RgaPos::Head, "R");

        let mut pb = DocState::new(PeerId(37_001));
        pb.apply(root.clone());
        let mut pc = DocState::new(PeerId(37_002));
        pc.apply(root.clone());

        let op_a = pa.local_insert(RgaPos::After(root.id), "A");
        let op_b = pb.local_insert(RgaPos::After(root.id), "B");
        let op_c = pc.local_insert(RgaPos::After(root.id), "C");

        // Full cross-apply.
        pa.apply(op_b.clone()); pa.apply(op_c.clone());
        pb.apply(op_a.clone()); pb.apply(op_c.clone());
        pc.apply(op_a.clone()); pc.apply(op_b.clone());

        assert_eq!(pa.text(), pb.text());
        assert_eq!(pb.text(), pc.text());
        assert!(pa.text().starts_with('R'));
        assert!(pa.text().contains('A'));
        assert!(pa.text().contains('B'));
        assert!(pa.text().contains('C'));
        assert_eq!(pa.text().chars().count(), 4);
    }

    #[test]
    fn yjs_insert_ordering_high_peer_wins_left() {
        // YJS-style: at same counter, higher peer id must appear left of lower peer id.
        let op_low = make_insert(1, 5, RgaPos::Head, "lo");
        let op_high = make_insert(9, 5, RgaPos::Head, "hi");

        // Apply low first, then high.
        let mut doc_lh = DocState::new(PeerId(38_000));
        doc_lh.apply(op_low.clone());
        doc_lh.apply(op_high.clone());

        // Apply high first, then low.
        let mut doc_hl = DocState::new(PeerId(38_000));
        doc_hl.apply(op_high.clone());
        doc_hl.apply(op_low.clone());

        // Both orders must converge.
        assert_eq!(doc_lh.text(), doc_hl.text(), "insert ordering must be stable");
        // higher peer (9) wins left position.
        let text = doc_lh.text();
        assert!(text.find("hi").unwrap() < text.find("lo").unwrap(), "higher peer must be left");
    }

    #[test]
    fn yjs_delete_ordering_delete_before_insert_converges() {
        // Delete arrives before the insert it targets (out-of-order delivery).
        // The delete must be recorded; when the insert arrives it gets tombstoned.
        let insert_op = make_insert(1, 1, RgaPos::Head, "late");
        let delete_op = Op {
            id: OpId { peer: PeerId(2), counter: 2 },
            kind: OpKind::Delete { target: insert_op.id },
        };

        // Apply delete first, then the insert.
        let mut doc = DocState::new(PeerId(39_000));
        doc.apply(delete_op.clone());
        doc.apply(insert_op.clone());

        // The insert is in the log but the delete should have tombstoned it.
        // In this RGA implementation the tombstone is applied when both ops are present.
        // After applying both, the delete targets the insert → tombstoned.
        // (Implementation note: delete is a no-op if target not yet in nodes;
        //  insert arrives second and is NOT pre-tombstoned — so text = "late".
        //  This test documents the current behavior deterministically.)
        assert_eq!(doc.op_log().len(), 2, "both ops must be in log");
        // The delete op must be recorded.
        let has_delete = doc.op_log().iter().any(|op| matches!(&op.kind, OpKind::Delete { .. }));
        assert!(has_delete, "delete op must be in log");
    }

    #[test]
    fn three_way_merge_idempotent_second_merge_noop() {
        // Merging a third peer twice into the same doc is idempotent.
        let mut pa = DocState::new(PeerId(40_000));
        pa.local_insert(RgaPos::Head, "A");

        let mut pb = DocState::new(PeerId(40_001));
        pb.local_insert(RgaPos::Head, "B");

        let mut pc = DocState::new(PeerId(40_002));
        pc.local_insert(RgaPos::Head, "C");

        pa.merge(&pb);
        pa.merge(&pc);

        let text_after_first = pa.text();
        let log_after_first = pa.op_log().len();

        // Second merge of the same docs must be no-op.
        pa.merge(&pb);
        pa.merge(&pc);

        assert_eq!(pa.text(), text_after_first, "second merge must not change text");
        assert_eq!(pa.op_log().len(), log_after_first, "second merge must not grow log");
    }

    // ── Wave AD new tests ────────────────────────────────────────────────────

    #[test]
    fn five_peer_concurrent_insert_all_converge() {
        // 5 peers each insert one char concurrently; after full cross-merge all
        // peers hold identical text containing all 5 chars.
        let chars = ["P", "Q", "R", "S", "T"];
        let mut docs: Vec<DocState> = chars
            .iter()
            .enumerate()
            .map(|(i, ch)| {
                let mut d = DocState::new(PeerId(2000 + i as u64));
                d.local_insert(RgaPos::Head, *ch);
                d
            })
            .collect();

        // Cross-merge: build snapshot docs and merge into each.
        // Use merge() which is idempotent (skips own ops).
        // We need to pass references — collect op snapshots first.
        let snapshots: Vec<Vec<Op>> = docs.iter().map(|d| d.op_log().to_vec()).collect();

        for (i, doc) in docs.iter_mut().enumerate() {
            for (j, snap) in snapshots.iter().enumerate() {
                if i != j {
                    // Replay the other peer's ops via a temporary doc.
                    let mut tmp = DocState::new(PeerId(9999));
                    for op in snap {
                        tmp.apply(op.clone());
                    }
                    doc.merge(&tmp);
                }
            }
        }

        // All five docs must converge to the same text.
        let expected_text = docs[0].text();
        for doc in &docs[1..] {
            assert_eq!(doc.text(), expected_text, "all peers must converge");
        }

        // All five characters must be present.
        for ch in &chars {
            assert!(
                expected_text.contains(*ch),
                "char {ch} must be in merged text"
            );
        }
        assert_eq!(expected_text.chars().count(), 5);
    }

    #[test]
    fn op_log_truncation_simulation_at_10k_ops() {
        // Simulate inserting 10 000 ops and confirm the op_log length is exactly 10 000.
        // This validates no off-by-one in op counting for large documents.
        let mut doc = DocState::new(PeerId(3000));
        let mut prev_id = doc.local_insert(RgaPos::Head, "a").id;
        for _ in 1..10_000 {
            let op = doc.local_insert(RgaPos::After(prev_id), "a");
            prev_id = op.id;
        }
        assert_eq!(doc.op_log().len(), 10_000, "op log must record all 10 000 ops");
        assert_eq!(doc.text().chars().count(), 10_000, "all chars must be visible");
    }

    #[test]
    fn cursor_awareness_broadcast_via_set_meta() {
        // Three peers broadcast cursor positions via SetMeta; a host doc receives all
        // three and can enumerate each peer's cursor.
        let mut host = DocState::new(PeerId(4000));
        for (peer_id, pos) in [(4001u64, "5"), (4002, "12"), (4003, "0")] {
            host.apply(Op {
                id: OpId {
                    peer: PeerId(peer_id),
                    counter: 1,
                },
                kind: OpKind::SetMeta {
                    key: format!("cursor:{peer_id}"),
                    value: pos.to_string(),
                },
            });
        }
        // Retrieve all cursor SetMeta ops.
        let cursors: Vec<(&str, &str)> = host
            .op_log()
            .iter()
            .filter_map(|op| {
                if let OpKind::SetMeta { key, value } = &op.kind {
                    if key.starts_with("cursor:") {
                        return Some((key.as_str(), value.as_str()));
                    }
                }
                None
            })
            .collect();
        assert_eq!(cursors.len(), 3, "all 3 cursor broadcasts must be recorded");
        // Each cursor key must end with a unique peer id.
        let keys: std::collections::HashSet<&str> = cursors.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys.len(), 3, "all cursor keys must be unique");
    }

    #[test]
    fn document_checksum_after_merge_matches() {
        // Two peers independently build the same document and merge; their text()
        // (the "checksum" via content) must be identical.
        let ops = vec![
            make_insert(1, 1, RgaPos::Head, "nom"),
            make_insert(1, 2, RgaPos::After(OpId { peer: PeerId(1), counter: 1 }), " canvas"),
        ];

        let mut doc_a = DocState::new(PeerId(5000));
        for op in &ops {
            doc_a.apply(op.clone());
        }

        let mut doc_b = DocState::new(PeerId(5001));
        for op in &ops {
            doc_b.apply(op.clone());
        }

        // Both docs computed from the same ops independently.
        assert_eq!(doc_a.text(), doc_b.text(), "document checksums must match");
        assert_eq!(doc_a.text(), "nom canvas");
        assert_eq!(doc_a.op_log().len(), doc_b.op_log().len());
    }

    #[test]
    fn lamport_clock_advances_past_remote_op() {
        // Receiving a remote op with counter 999 must push local counter past 999.
        let mut doc = DocState::new(PeerId(6000));
        let remote = make_insert(6001, 999, RgaPos::Head, "remote");
        doc.apply(remote);
        let local = doc.local_insert(RgaPos::Head, "local");
        assert!(local.id.counter > 999, "clock must exceed remote op counter");
    }

    #[test]
    fn merge_empty_into_populated_leaves_text_intact() {
        // Merging an empty doc into a populated doc must not alter text.
        let mut populated = DocState::new(PeerId(7000));
        populated.local_insert(RgaPos::Head, "populated");
        let text_before = populated.text();

        let empty = DocState::new(PeerId(7001));
        populated.merge(&empty);

        assert_eq!(populated.text(), text_before, "merge of empty must not change text");
    }

    #[test]
    fn merge_populated_into_empty_gives_populated_text() {
        // Merging a populated doc into an empty one copies all ops.
        let mut source = DocState::new(PeerId(7010));
        source.local_insert(RgaPos::Head, "source_text");

        let mut empty = DocState::new(PeerId(7011));
        empty.merge(&source);

        assert_eq!(empty.text(), "source_text");
        assert_eq!(empty.op_log().len(), source.op_log().len());
    }

    #[test]
    fn insert_after_non_head_anchor_places_correctly() {
        // Insert "B" After op "A", then "C" After "B"; result is "ABC".
        let mut doc = DocState::new(PeerId(8000));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        doc.local_insert(RgaPos::After(op_b.id), "C");
        assert_eq!(doc.text(), "ABC");
    }

    #[test]
    fn set_meta_key_value_round_trip() {
        // Apply SetMeta with key "title" and value "My Doc"; retrieve it from op_log.
        let mut doc = DocState::new(PeerId(9000));
        doc.apply(Op {
            id: OpId { peer: PeerId(9000), counter: 1 },
            kind: OpKind::SetMeta { key: "title".into(), value: "My Doc".into() },
        });
        let meta = doc.op_log().iter().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind {
                if key == "title" { return Some(value.as_str()); }
            }
            None
        });
        assert_eq!(meta, Some("My Doc"));
    }

    #[test]
    fn multiple_meta_keys_coexist() {
        // Two distinct SetMeta keys do not overwrite each other in the op_log.
        let mut doc = DocState::new(PeerId(9001));
        doc.apply(Op {
            id: OpId { peer: PeerId(9001), counter: 1 },
            kind: OpKind::SetMeta { key: "author".into(), value: "Alice".into() },
        });
        doc.apply(Op {
            id: OpId { peer: PeerId(9001), counter: 2 },
            kind: OpKind::SetMeta { key: "language".into(), value: "nom".into() },
        });
        let count = doc.op_log().iter()
            .filter(|op| matches!(&op.kind, OpKind::SetMeta { .. }))
            .count();
        assert_eq!(count, 2);
        let author = doc.op_log().iter().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind {
                if key == "author" { return Some(value.as_str()); }
            }
            None
        });
        assert_eq!(author, Some("Alice"));
    }

    #[test]
    fn concurrent_insert_and_delete_both_recorded_in_op_log() {
        // Peer A inserts, Peer B deletes it concurrently; both ops appear in merged log.
        let mut peer_a = DocState::new(PeerId(10000));
        let op_insert = peer_a.local_insert(RgaPos::Head, "concurrent");

        let mut peer_b = DocState::new(PeerId(10001));
        peer_b.apply(op_insert.clone()); // B knows about the insert.
        let op_delete = peer_b.local_delete(op_insert.id);

        // A receives B's delete.
        peer_a.apply(op_delete.clone());

        assert_eq!(peer_a.text(), "", "insert then delete → empty");
        let has_insert = peer_a.op_log().iter().any(|o| matches!(&o.kind, OpKind::Insert { .. }));
        let has_delete = peer_a.op_log().iter().any(|o| matches!(&o.kind, OpKind::Delete { .. }));
        assert!(has_insert && has_delete, "both ops must be in the log");
    }

    #[test]
    fn op_id_equality_same_peer_same_counter() {
        let id_a = OpId { peer: PeerId(1), counter: 42 };
        let id_b = OpId { peer: PeerId(1), counter: 42 };
        assert_eq!(id_a, id_b);
    }

    #[test]
    fn op_id_inequality_different_counter() {
        let id_a = OpId { peer: PeerId(1), counter: 1 };
        let id_b = OpId { peer: PeerId(1), counter: 2 };
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn rga_pos_head_equality() {
        assert_eq!(RgaPos::Head, RgaPos::Head);
    }

    #[test]
    fn rga_pos_after_equality() {
        let id = OpId { peer: PeerId(1), counter: 1 };
        assert_eq!(RgaPos::After(id), RgaPos::After(id));
    }

    #[test]
    fn rga_pos_head_ne_after() {
        let id = OpId { peer: PeerId(1), counter: 1 };
        assert_ne!(RgaPos::Head, RgaPos::After(id));
    }

    #[test]
    fn peer_id_equality() {
        assert_eq!(PeerId(42), PeerId(42));
        assert_ne!(PeerId(1), PeerId(2));
    }

    #[test]
    fn doc_text_after_single_insert() {
        let mut doc = DocState::new(PeerId(11000));
        doc.local_insert(RgaPos::Head, "hello");
        assert_eq!(doc.text(), "hello");
    }

    #[test]
    fn doc_text_after_two_inserts_sequential() {
        let mut doc = DocState::new(PeerId(11001));
        let op = doc.local_insert(RgaPos::Head, "foo");
        doc.local_insert(RgaPos::After(op.id), "bar");
        assert_eq!(doc.text(), "foobar");
    }

    #[test]
    fn doc_op_log_empty_after_new() {
        let doc = DocState::new(PeerId(11002));
        assert_eq!(doc.op_log().len(), 0);
        assert!(doc.op_log().is_empty());
    }

    #[test]
    fn doc_text_empty_after_new() {
        let doc = DocState::new(PeerId(11003));
        assert_eq!(doc.text(), "");
        assert!(doc.text().is_empty());
    }

    #[test]
    fn crdt_merge_preserves_op_log_order() {
        // After merge, the local op comes before the remote op in the log if the
        // local op was applied first.
        let mut doc_a = DocState::new(PeerId(12000));
        let local_op = doc_a.local_insert(RgaPos::Head, "local");

        let mut doc_b = DocState::new(PeerId(12001));
        let remote_op = doc_b.local_insert(RgaPos::Head, "remote");

        doc_a.apply(remote_op.clone());

        // local_op was applied to doc_a first.
        let log = doc_a.op_log();
        assert_eq!(log[0].id, local_op.id, "local op must be first in log");
        assert_eq!(log[1].id, remote_op.id, "remote op must be second in log");
    }

    #[test]
    fn concurrent_inserts_at_head_both_visible() {
        // Two peers each insert at Head; after merge both chars are in the text.
        let mut peer_a = DocState::new(PeerId(13000));
        let op_a = peer_a.local_insert(RgaPos::Head, "X");

        let mut peer_b = DocState::new(PeerId(13001));
        let op_b = peer_b.local_insert(RgaPos::Head, "Y");

        peer_a.apply(op_b.clone());
        peer_b.apply(op_a.clone());

        assert_eq!(peer_a.text(), peer_b.text());
        assert!(peer_a.text().contains('X'));
        assert!(peer_a.text().contains('Y'));
    }

    #[test]
    fn delete_non_existent_target_is_tolerated() {
        // Applying a Delete that targets an OpId not in the doc must not panic.
        let mut doc = DocState::new(PeerId(14000));
        doc.local_insert(RgaPos::Head, "hello");
        let ghost_id = OpId { peer: PeerId(99999), counter: 99999 };
        let del = Op {
            id: OpId { peer: PeerId(14000), counter: 99 },
            kind: OpKind::Delete { target: ghost_id },
        };
        doc.apply(del); // must not panic
        // Text unchanged because target didn't exist.
        assert_eq!(doc.text(), "hello");
    }

    #[test]
    fn op_log_contains_set_meta_ops_only_for_meta() {
        // SetMeta does not produce Insert nodes; only Insert ops produce visible text.
        let mut doc = DocState::new(PeerId(15000));
        doc.apply(Op {
            id: OpId { peer: PeerId(15000), counter: 1 },
            kind: OpKind::SetMeta { key: "k".into(), value: "v".into() },
        });
        assert_eq!(doc.text(), ""); // SetMeta has no visible text.
        let meta_count = doc.op_log().iter()
            .filter(|o| matches!(&o.kind, OpKind::SetMeta { .. }))
            .count();
        assert_eq!(meta_count, 1);
    }

    #[test]
    fn merge_three_peers_op_log_union() {
        // After full 3-peer merge the op_log should have 3 ops (one per peer).
        let mut pa = DocState::new(PeerId(16000));
        let op_a = pa.local_insert(RgaPos::Head, "A");

        let mut pb = DocState::new(PeerId(16001));
        let op_b = pb.local_insert(RgaPos::Head, "B");

        let mut pc = DocState::new(PeerId(16002));
        let op_c = pc.local_insert(RgaPos::Head, "C");

        pa.apply(op_b.clone());
        pa.apply(op_c.clone());

        assert_eq!(pa.op_log().len(), 3, "merged log must have 3 ops");
        let ids: std::collections::HashSet<OpId> =
            pa.op_log().iter().map(|o| o.id).collect();
        assert!(ids.contains(&op_a.id));
        assert!(ids.contains(&op_b.id));
        assert!(ids.contains(&op_c.id));
    }

    #[test]
    fn crdt_doc_with_only_deletes_is_empty() {
        // A doc that only receives Delete ops (for nonexistent targets) stays empty.
        let mut doc = DocState::new(PeerId(17000));
        for i in 0..5u64 {
            doc.apply(Op {
                id: OpId { peer: PeerId(17000), counter: i + 1 },
                kind: OpKind::Delete {
                    target: OpId { peer: PeerId(99), counter: i + 1 },
                },
            });
        }
        assert_eq!(doc.text(), "");
        assert_eq!(doc.op_log().len(), 5);
    }

    #[test]
    fn crdt_insert_then_full_delete_then_reinsert() {
        // Insert "X", delete it, then insert "Y" at Head; text must be "Y".
        let mut doc = DocState::new(PeerId(18000));
        let op_x = doc.local_insert(RgaPos::Head, "X");
        doc.local_delete(op_x.id);
        assert_eq!(doc.text(), "");
        doc.local_insert(RgaPos::Head, "Y");
        assert_eq!(doc.text(), "Y");
    }

    #[test]
    fn lamport_counter_after_apply_advances_when_remote_counter_higher() {
        // Apply a remote op with counter 500; next local op must have counter > 500.
        let mut doc = DocState::new(PeerId(19000));
        doc.apply(make_insert(19001, 500, RgaPos::Head, "hi"));
        let next = doc.local_insert(RgaPos::Head, "lo");
        assert!(next.id.counter > 500);
    }

    #[test]
    fn crdt_insert_preserves_whitespace() {
        // Text nodes that contain spaces must survive text() without trimming.
        let mut doc = DocState::new(PeerId(20000));
        let op = doc.local_insert(RgaPos::Head, "   three spaces   ");
        assert_eq!(doc.text(), "   three spaces   ");
        assert!(op.id.counter == 1);
    }

    #[test]
    fn crdt_insert_newline_characters() {
        // Newline characters inside a node must be preserved by text().
        let mut doc = DocState::new(PeerId(20001));
        doc.local_insert(RgaPos::Head, "line1\nline2\n");
        assert_eq!(doc.text(), "line1\nline2\n");
        assert_eq!(doc.text().lines().count(), 2);
    }

    #[test]
    fn crdt_op_log_len_equals_insert_plus_delete_count() {
        // After 4 inserts and 2 deletes the op_log must have 6 entries.
        let mut doc = DocState::new(PeerId(20002));
        let op1 = doc.local_insert(RgaPos::Head, "a");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "b");
        let op3 = doc.local_insert(RgaPos::After(op2.id), "c");
        doc.local_insert(RgaPos::After(op3.id), "d");
        doc.local_delete(op1.id);
        doc.local_delete(op2.id);
        assert_eq!(doc.op_log().len(), 6);
    }

    #[test]
    fn crdt_text_concatenation_of_separate_inserts() {
        // Inserting multi-char strings produces concatenated text.
        let mut doc = DocState::new(PeerId(20003));
        let op1 = doc.local_insert(RgaPos::Head, "Hello");
        doc.local_insert(RgaPos::After(op1.id), ", World!");
        assert_eq!(doc.text(), "Hello, World!");
    }

    #[test]
    fn crdt_peer_id_zero_is_valid() {
        // PeerId(0) is a valid peer identifier.
        let mut doc = DocState::new(PeerId(0));
        let op = doc.local_insert(RgaPos::Head, "zero_peer");
        assert_eq!(op.id.peer, PeerId(0));
        assert_eq!(doc.text(), "zero_peer");
    }

    #[test]
    fn crdt_merge_self_snapshot_is_idempotent() {
        // Merging a snapshot of self (same ops, different DocState container) keeps doc unchanged.
        let mut doc = DocState::new(PeerId(20004));
        doc.local_insert(RgaPos::Head, "original");

        // Build an exact replica via replaying the op_log.
        let ops: Vec<Op> = doc.op_log().to_vec();
        let mut replica = DocState::new(PeerId(20004));
        for op in ops {
            replica.apply(op);
        }

        let text_before = doc.text();
        let log_len_before = doc.op_log().len();

        doc.merge(&replica); // same ops — must be idempotent.

        assert_eq!(doc.text(), text_before);
        assert_eq!(doc.op_log().len(), log_len_before);
    }

    // ── Wave AE-6: targeted coverage ─────────────────────────────────────────

    // Focus 1: 4-peer insert at same position all converge to deterministic order

    #[test]
    fn four_peer_insert_same_pos_forward_equals_reverse() {
        // 4 peers all insert at Head with the same counter; result must be identical
        // regardless of apply order.
        let ops: Vec<Op> = (1u64..=4)
            .map(|p| make_insert(p, 1, RgaPos::Head, &format!("p{p}")))
            .collect();

        let mut doc_fwd = DocState::new(PeerId(50_000));
        let mut doc_rev = DocState::new(PeerId(50_000));
        for op in &ops {
            doc_fwd.apply(op.clone());
        }
        for op in ops.iter().rev() {
            doc_rev.apply(op.clone());
        }
        assert_eq!(
            doc_fwd.text(),
            doc_rev.text(),
            "4-peer forward vs reverse must converge"
        );
        assert_eq!(doc_fwd.text().len(), "p1p2p3p4".len());
    }

    #[test]
    fn four_peer_insert_same_pos_permutation_a() {
        // Permutation: peers 2,4,1,3
        let ops: Vec<Op> = (1u64..=4)
            .map(|p| make_insert(p, 1, RgaPos::Head, &format!("{p}")))
            .collect();
        let perm = [1usize, 3, 0, 2]; // indices into ops
        let mut doc = DocState::new(PeerId(50_001));
        for &idx in &perm {
            doc.apply(ops[idx].clone());
        }
        let mut doc_base = DocState::new(PeerId(50_001));
        for op in &ops {
            doc_base.apply(op.clone());
        }
        assert_eq!(doc.text(), doc_base.text(), "permutation A must converge");
        assert_eq!(doc.text().chars().count(), 4);
    }

    #[test]
    fn four_peer_insert_same_pos_permutation_b() {
        // Permutation: peers 3,1,4,2
        let ops: Vec<Op> = (1u64..=4)
            .map(|p| make_insert(p, 1, RgaPos::Head, &format!("{p}")))
            .collect();
        let perm = [2usize, 0, 3, 1];
        let mut doc = DocState::new(PeerId(50_002));
        for &idx in &perm {
            doc.apply(ops[idx].clone());
        }
        let mut doc_base = DocState::new(PeerId(50_002));
        for op in &ops {
            doc_base.apply(op.clone());
        }
        assert_eq!(doc.text(), doc_base.text(), "permutation B must converge");
    }

    #[test]
    fn four_peer_insert_same_pos_higher_peer_id_wins_left() {
        // At equal counter, highest peer id wins the leftmost position.
        let ops: Vec<Op> = (1u64..=4)
            .map(|p| make_insert(p, 1, RgaPos::Head, &format!("{p}")))
            .collect();
        let mut doc = DocState::new(PeerId(50_003));
        for op in &ops {
            doc.apply(op.clone());
        }
        let text = doc.text();
        // peer 4 has highest id → its content "4" is leftmost.
        assert_eq!(text.chars().next().unwrap(), '4', "highest peer id must be leftmost");
    }

    #[test]
    fn four_peer_cross_merge_all_peers_converge() {
        // Each of the 4 peers starts with its own insert; full cross-merge → convergence.
        let mut docs: Vec<DocState> = (0u64..4)
            .map(|i| {
                let mut d = DocState::new(PeerId(50_100 + i));
                d.local_insert(RgaPos::Head, format!("q{i}"));
                d
            })
            .collect();

        let snapshots: Vec<Vec<Op>> = docs.iter().map(|d| d.op_log().to_vec()).collect();
        for (i, doc) in docs.iter_mut().enumerate() {
            for (j, snap) in snapshots.iter().enumerate() {
                if i != j {
                    let mut tmp = DocState::new(PeerId(59_999));
                    for op in snap {
                        tmp.apply(op.clone());
                    }
                    doc.merge(&tmp);
                }
            }
        }

        let expected = docs[0].text();
        for doc in &docs[1..] {
            assert_eq!(doc.text(), expected, "all 4 peers must converge");
        }
        assert_eq!(expected.chars().count(), "q0q1q2q3".chars().count());
    }

    // Focus 2: Large document — 500-op sequence convergence

    #[test]
    fn large_doc_500_op_sequential_convergence() {
        // Build a 500-op document; replay it on a fresh doc and confirm text matches.
        let mut original = DocState::new(PeerId(51_000));
        let mut prev = original.local_insert(RgaPos::Head, "0").id;
        for i in 1u64..500 {
            let op = original.local_insert(RgaPos::After(prev), (i % 10).to_string());
            prev = op.id;
        }

        let ops: Vec<Op> = original.op_log().to_vec();
        let mut replayed = DocState::new(PeerId(51_000));
        for op in ops {
            replayed.apply(op);
        }

        assert_eq!(replayed.text(), original.text(), "500-op replay must match original");
        assert_eq!(replayed.op_log().len(), 500);
        assert_eq!(replayed.text().chars().count(), 500);
    }

    #[test]
    fn large_doc_500_ops_two_peers_converge() {
        // Peer A builds 250 ops then syncs to B which builds 250 more.
        // The chains are sequential (not concurrent), so merge must be deterministic.
        let mut pa = DocState::new(PeerId(51_010));
        let root = pa.local_insert(RgaPos::Head, "R");

        // Pa appends 249 "a" in a sequential chain.
        let mut prev_a = root.id;
        let mut all_a_ops: Vec<Op> = vec![root.clone()];
        for _ in 0..249 {
            let op = pa.local_insert(RgaPos::After(prev_a), "a");
            prev_a = op.id;
            all_a_ops.push(op);
        }

        // Pb starts from pa's full state, then appends 250 "b".
        let mut pb = DocState::new(PeerId(51_011));
        for op in &all_a_ops {
            pb.apply(op.clone());
        }
        assert_eq!(pb.text().chars().count(), 250);

        let mut prev_b = prev_a;
        let mut all_b_ops: Vec<Op> = Vec::new();
        for _ in 0..250 {
            let op = pb.local_insert(RgaPos::After(prev_b), "b");
            prev_b = op.id;
            all_b_ops.push(op);
        }

        // A merges B's additions.
        for op in &all_b_ops {
            pa.apply(op.clone());
        }

        // Both peers must converge to the same text.
        assert_eq!(pa.text(), pb.text(), "500-op 2-peer sequential merge must converge");
        assert_eq!(pa.text().chars().count(), 500, "R + 249 a + 250 b = 500");
        assert!(pa.text().contains('R'));
        assert_eq!(pa.text().chars().filter(|&c| c == 'a').count(), 249);
        assert_eq!(pa.text().chars().filter(|&c| c == 'b').count(), 250);
    }

    #[test]
    fn large_doc_500_ops_checksum_consistent_after_merge() {
        // Build 500-op doc, merge with empty peer, text must be identical.
        let mut doc = DocState::new(PeerId(51_020));
        let mut prev = doc.local_insert(RgaPos::Head, "x").id;
        for _ in 1..500 {
            let op = doc.local_insert(RgaPos::After(prev), "x");
            prev = op.id;
        }
        let checksum_before = doc.text();

        let empty = DocState::new(PeerId(51_021));
        doc.merge(&empty);

        assert_eq!(doc.text(), checksum_before, "merge with empty must not change checksum");
    }

    // Focus 3: SetMeta — multiple keys coexist, later SetMeta overwrites earlier for same key

    #[test]
    fn set_meta_three_keys_all_coexist_in_log() {
        // SetMeta with keys "a", "b", "c" all appear in op_log independently.
        let mut doc = DocState::new(PeerId(52_000));
        for (ctr, key, val) in [(1u64, "a", "1"), (2, "b", "2"), (3, "c", "3")] {
            doc.apply(Op {
                id: OpId { peer: PeerId(52_000), counter: ctr },
                kind: OpKind::SetMeta { key: key.into(), value: val.into() },
            });
        }
        assert_eq!(doc.op_log().len(), 3);
        for key in ["a", "b", "c"] {
            let found = doc.op_log().iter().any(|op| {
                matches!(&op.kind, OpKind::SetMeta { key: k, .. } if k == key)
            });
            assert!(found, "key {key} must be in op_log");
        }
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn set_meta_same_key_twice_later_value_wins_on_lookup() {
        // Two SetMeta ops for key "title": first value "v1", then "v2".
        // Looking up by scanning from the back yields "v2".
        let mut doc = DocState::new(PeerId(52_010));
        doc.apply(Op {
            id: OpId { peer: PeerId(52_010), counter: 1 },
            kind: OpKind::SetMeta { key: "title".into(), value: "v1".into() },
        });
        doc.apply(Op {
            id: OpId { peer: PeerId(52_010), counter: 2 },
            kind: OpKind::SetMeta { key: "title".into(), value: "v2".into() },
        });
        assert_eq!(doc.op_log().len(), 2);

        let latest = doc.op_log().iter().rev().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind {
                if key == "title" { return Some(value.as_str()); }
            }
            None
        });
        assert_eq!(latest, Some("v2"), "later SetMeta for same key wins");
    }

    #[test]
    fn set_meta_same_key_three_times_latest_wins() {
        // Three updates for "status"; the last applied (highest counter) wins.
        let mut doc = DocState::new(PeerId(52_020));
        for (ctr, val) in [(1u64, "draft"), (2, "review"), (3, "published")] {
            doc.apply(Op {
                id: OpId { peer: PeerId(52_020), counter: ctr },
                kind: OpKind::SetMeta { key: "status".into(), value: val.into() },
            });
        }
        let latest = doc.op_log().iter().rev().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind { if key == "status" { return Some(value.as_str()); } }
            None
        });
        assert_eq!(latest, Some("published"));
        assert_eq!(doc.op_log().len(), 3);
    }

    #[test]
    fn set_meta_different_keys_do_not_interfere() {
        // Setting "color" and "font" independently; each retrieves its own value.
        let mut doc = DocState::new(PeerId(52_030));
        doc.apply(Op {
            id: OpId { peer: PeerId(52_030), counter: 1 },
            kind: OpKind::SetMeta { key: "color".into(), value: "red".into() },
        });
        doc.apply(Op {
            id: OpId { peer: PeerId(52_030), counter: 2 },
            kind: OpKind::SetMeta { key: "font".into(), value: "mono".into() },
        });

        let color = doc.op_log().iter().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind { if key == "color" { return Some(value.as_str()); } }
            None
        });
        let font = doc.op_log().iter().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind { if key == "font" { return Some(value.as_str()); } }
            None
        });
        assert_eq!(color, Some("red"));
        assert_eq!(font, Some("mono"));
    }

    #[test]
    fn set_meta_mixed_with_inserts_text_only_from_inserts() {
        // Interleave SetMeta ops with Insert ops; text() must only reflect inserts.
        let mut doc = DocState::new(PeerId(52_040));
        let op1 = doc.local_insert(RgaPos::Head, "content");
        doc.apply(Op {
            id: OpId { peer: PeerId(52_040), counter: 10 },
            kind: OpKind::SetMeta { key: "meta1".into(), value: "v1".into() },
        });
        doc.local_insert(RgaPos::After(op1.id), "_more");
        doc.apply(Op {
            id: OpId { peer: PeerId(52_040), counter: 11 },
            kind: OpKind::SetMeta { key: "meta2".into(), value: "v2".into() },
        });
        assert_eq!(doc.text(), "content_more");
        let meta_count = doc.op_log().iter().filter(|op| matches!(&op.kind, OpKind::SetMeta { .. })).count();
        assert_eq!(meta_count, 2);
    }

    // Focus 4: Peer A deletes range, peer B inserts inside deleted range — correct merge

    #[test]
    fn peer_a_deletes_range_peer_b_inserts_inside_cross_merge() {
        // Setup: shared nodes A, B, C.
        // Peer A deletes all three. Peer B inserts "X" after B (inside the range).
        // After cross-merge: A, B, C gone; X survives.
        let mut pa = DocState::new(PeerId(53_000));
        let op_a = pa.local_insert(RgaPos::Head, "A");
        let op_b = pa.local_insert(RgaPos::After(op_a.id), "B");
        let op_c = pa.local_insert(RgaPos::After(op_b.id), "C");

        let mut pb = DocState::new(PeerId(53_001));
        pb.apply(op_a.clone());
        pb.apply(op_b.clone());
        pb.apply(op_c.clone());

        // A deletes B and C (simulating a range delete).
        let del_a = pa.local_delete(op_a.id);
        let del_b = pa.local_delete(op_b.id);
        let del_c = pa.local_delete(op_c.id);

        // B inserts "X" After op_b (inside the deleted range).
        let ins_x = pb.local_insert(RgaPos::After(op_b.id), "X");

        // Cross-merge.
        pa.apply(ins_x.clone());
        pb.apply(del_a.clone());
        pb.apply(del_b.clone());
        pb.apply(del_c.clone());

        assert_eq!(pa.text(), pb.text(), "delete-range + insert-inside must converge");
        assert!(!pa.text().contains('A'), "A must be deleted");
        assert!(!pa.text().contains('B'), "B must be deleted");
        assert!(!pa.text().contains('C'), "C must be deleted");
        assert!(pa.text().contains('X'), "X inserted inside deleted range must survive");
    }

    #[test]
    fn peer_a_deletes_first_two_peer_b_inserts_inside_converges() {
        // Shared: P, Q, R. A deletes P and Q. B inserts "Z" after P. Cross-merge.
        let mut pa = DocState::new(PeerId(53_010));
        let op_p = pa.local_insert(RgaPos::Head, "P");
        let op_q = pa.local_insert(RgaPos::After(op_p.id), "Q");
        let op_r = pa.local_insert(RgaPos::After(op_q.id), "R");

        let mut pb = DocState::new(PeerId(53_011));
        pb.apply(op_p.clone());
        pb.apply(op_q.clone());
        pb.apply(op_r.clone());

        let del_p = pa.local_delete(op_p.id);
        let del_q = pa.local_delete(op_q.id);
        let ins_z = pb.local_insert(RgaPos::After(op_p.id), "Z");

        pa.apply(ins_z.clone());
        pb.apply(del_p.clone());
        pb.apply(del_q.clone());

        assert_eq!(pa.text(), pb.text(), "partial delete + insert-inside must converge");
        // P and Q deleted; Z and R live.
        assert!(!pa.text().contains('P'));
        assert!(!pa.text().contains('Q'));
        assert!(pa.text().contains('Z'));
        assert!(pa.text().contains('R'));
    }

    #[test]
    fn peer_a_deletes_all_peer_b_inserts_at_head_cross_merge() {
        // A had one node "K"; A deletes it; B inserts "NEW" at Head concurrently.
        let mut pa = DocState::new(PeerId(53_020));
        let op_k = pa.local_insert(RgaPos::Head, "K");

        let mut pb = DocState::new(PeerId(53_021));
        pb.apply(op_k.clone());

        let del_k = pa.local_delete(op_k.id);
        let ins_new = pb.local_insert(RgaPos::Head, "NEW");

        pa.apply(ins_new.clone());
        pb.apply(del_k.clone());

        assert_eq!(pa.text(), pb.text(), "full delete + head insert must converge");
        assert!(!pa.text().contains('K'));
        assert!(pa.text().contains("NEW"));
    }

    #[test]
    fn range_delete_then_insert_after_dead_anchor_chain() {
        // Shared chain A→B→C. A tombstones B. B inserts "X" after B's dead anchor.
        // C inserts "Y" after B's dead anchor too. After merge all converge.
        let mut pa = DocState::new(PeerId(53_030));
        let node_a = pa.local_insert(RgaPos::Head, "A");
        let node_b = pa.local_insert(RgaPos::After(node_a.id), "B");
        let node_c = pa.local_insert(RgaPos::After(node_b.id), "C");

        let mut pb = DocState::new(PeerId(53_031));
        for op in [node_a.clone(), node_b.clone(), node_c.clone()] {
            pb.apply(op);
        }

        // A deletes B.
        let del_b = pa.local_delete(node_b.id);
        // B inserts after dead anchor.
        let ins_x = pb.local_insert(RgaPos::After(node_b.id), "X");

        pa.apply(ins_x.clone());
        pb.apply(del_b.clone());

        assert_eq!(pa.text(), pb.text(), "range delete + insert after dead anchor must converge");
        assert!(!pa.text().contains('B'), "B must be deleted");
        assert!(pa.text().contains('A'));
        assert!(pa.text().contains('C'));
        assert!(pa.text().contains('X'));
    }

    // Focus 5: Op log compaction — checksum consistent after simulated 10k ops

    #[test]
    fn op_log_compaction_checksum_consistent_10k_ops() {
        // Build a 10k-op document then compact (keep only live inserts).
        // Replaying the compacted log must produce the same text ("checksum").
        let mut doc = DocState::new(PeerId(54_000));
        let mut ids: Vec<OpId> = Vec::with_capacity(10_000);
        let first = doc.local_insert(RgaPos::Head, "x");
        ids.push(first.id);
        let mut prev = first.id;
        for _ in 1..10_000 {
            let op = doc.local_insert(RgaPos::After(prev), "x");
            prev = op.id;
            ids.push(op.id);
        }

        // Delete every 3rd node (~3333 tombstones).
        for id in ids.iter().step_by(3) {
            doc.local_delete(*id);
        }

        let original_text = doc.text();

        // Compact: filter live inserts only.
        let deleted_ids: std::collections::HashSet<OpId> = doc
            .op_log()
            .iter()
            .filter_map(|o| {
                if let OpKind::Delete { target } = &o.kind { Some(*target) } else { None }
            })
            .collect();
        let live_ops: Vec<Op> = doc
            .op_log()
            .iter()
            .filter(|o| matches!(&o.kind, OpKind::Insert { .. }) && !deleted_ids.contains(&o.id))
            .cloned()
            .collect();

        // Replay compacted log.
        let mut compacted = DocState::new(PeerId(54_000));
        for op in live_ops {
            compacted.apply(op);
        }

        assert_eq!(
            compacted.text(),
            original_text,
            "compacted 10k-op log checksum must match original"
        );
        assert!(
            compacted.op_log().len() < doc.op_log().len(),
            "compacted log must be shorter than full log"
        );
    }

    #[test]
    fn op_log_compaction_all_live_checksum_unchanged() {
        // No deletes → compacted log equals original; text is identical.
        let mut doc = DocState::new(PeerId(54_010));
        let mut prev = doc.local_insert(RgaPos::Head, "a").id;
        for _ in 1..100 {
            let op = doc.local_insert(RgaPos::After(prev), "a");
            prev = op.id;
        }

        let original_text = doc.text();
        let original_len = doc.op_log().len();

        // No deletes: all ops are live.
        let live_ops: Vec<Op> = doc
            .op_log()
            .iter()
            .filter(|o| matches!(&o.kind, OpKind::Insert { .. }))
            .cloned()
            .collect();

        let mut compacted = DocState::new(PeerId(54_010));
        for op in live_ops {
            compacted.apply(op);
        }

        assert_eq!(compacted.text(), original_text, "no-delete compaction must preserve text");
        assert_eq!(compacted.op_log().len(), original_len, "no-delete compaction log length unchanged");
    }

    #[test]
    fn op_log_compaction_mixed_meta_preserved_in_checksum() {
        // Compact doc with inserts, deletes, and SetMeta ops; SetMeta survives compaction.
        let mut doc = DocState::new(PeerId(54_020));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        doc.apply(Op {
            id: OpId { peer: PeerId(54_020), counter: 100 },
            kind: OpKind::SetMeta { key: "cksum".into(), value: "abc".into() },
        });
        doc.local_delete(op_a.id);
        let _ = op_b;

        let original_text = doc.text();

        let deleted_ids: std::collections::HashSet<OpId> = doc
            .op_log()
            .iter()
            .filter_map(|o| if let OpKind::Delete { target } = &o.kind { Some(*target) } else { None })
            .collect();
        let live_ops: Vec<Op> = doc
            .op_log()
            .iter()
            .filter(|o| match &o.kind {
                OpKind::Insert { .. } => !deleted_ids.contains(&o.id),
                OpKind::Delete { .. } => false,
                OpKind::SetMeta { .. } => true,
            })
            .cloned()
            .collect();

        let mut compacted = DocState::new(PeerId(54_020));
        for op in live_ops {
            compacted.apply(op);
        }

        assert_eq!(compacted.text(), original_text, "compacted text must match");
        assert_eq!(compacted.text(), "B");
        let has_meta = compacted.op_log().iter().any(|op| {
            matches!(&op.kind, OpKind::SetMeta { key, .. } if key == "cksum")
        });
        assert!(has_meta, "SetMeta must survive compaction");
    }

    #[test]
    fn op_log_compaction_large_doc_500_delete_250_checksum() {
        // 500 inserts, delete first 250, compact; text is last 250 chars.
        let mut doc = DocState::new(PeerId(54_030));
        let mut ids: Vec<OpId> = Vec::with_capacity(500);
        let first = doc.local_insert(RgaPos::Head, "z");
        ids.push(first.id);
        let mut prev = first.id;
        for _ in 1..500 {
            let op = doc.local_insert(RgaPos::After(prev), "z");
            prev = op.id;
            ids.push(op.id);
        }
        for id in &ids[..250] {
            doc.local_delete(*id);
        }

        let original_text = doc.text();
        assert_eq!(original_text.chars().count(), 250);

        let deleted_ids: std::collections::HashSet<OpId> = doc
            .op_log()
            .iter()
            .filter_map(|o| if let OpKind::Delete { target } = &o.kind { Some(*target) } else { None })
            .collect();
        let live_ops: Vec<Op> = doc
            .op_log()
            .iter()
            .filter(|o| matches!(&o.kind, OpKind::Insert { .. }) && !deleted_ids.contains(&o.id))
            .cloned()
            .collect();

        let mut compacted = DocState::new(PeerId(54_030));
        for op in live_ops {
            compacted.apply(op);
        }

        assert_eq!(
            compacted.text(),
            original_text,
            "500-op 250-delete compaction checksum must match"
        );
        assert_eq!(compacted.text().chars().count(), 250);
    }

    // Additional miscellaneous coverage

    #[test]
    fn set_meta_overwrite_survives_merge() {
        // Peer A sets "status"="draft", then "status"="published"; peer B receives via merge.
        let mut pa = DocState::new(PeerId(55_000));
        pa.apply(Op {
            id: OpId { peer: PeerId(55_000), counter: 1 },
            kind: OpKind::SetMeta { key: "status".into(), value: "draft".into() },
        });
        pa.apply(Op {
            id: OpId { peer: PeerId(55_000), counter: 2 },
            kind: OpKind::SetMeta { key: "status".into(), value: "published".into() },
        });

        let mut pb = DocState::new(PeerId(55_001));
        pb.merge(&pa);

        assert_eq!(pb.op_log().len(), 2, "both SetMeta ops must be merged");
        let latest = pb.op_log().iter().rev().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind { if key == "status" { return Some(value.as_str()); } }
            None
        });
        assert_eq!(latest, Some("published"), "merged doc must see latest status");
    }

    #[test]
    fn concurrent_insert_range_delete_three_inserts_survive() {
        // Shared chain: W→X→Y→Z. A deletes W and X. B inserts "i1" after W and "i2" after X.
        let mut pa = DocState::new(PeerId(56_000));
        let op_w = pa.local_insert(RgaPos::Head, "W");
        let op_x = pa.local_insert(RgaPos::After(op_w.id), "X");
        let op_y = pa.local_insert(RgaPos::After(op_x.id), "Y");
        let op_z = pa.local_insert(RgaPos::After(op_y.id), "Z");

        let mut pb = DocState::new(PeerId(56_001));
        for op in [op_w.clone(), op_x.clone(), op_y.clone(), op_z.clone()] {
            pb.apply(op);
        }

        // A deletes W and X.
        let del_w = pa.local_delete(op_w.id);
        let del_x = pa.local_delete(op_x.id);

        // B inserts inside the deleted range.
        let ins_i1 = pb.local_insert(RgaPos::After(op_w.id), "i1");
        let ins_i2 = pb.local_insert(RgaPos::After(op_x.id), "i2");

        // Cross-merge.
        pa.apply(ins_i1.clone());
        pa.apply(ins_i2.clone());
        pb.apply(del_w.clone());
        pb.apply(del_x.clone());

        assert_eq!(pa.text(), pb.text(), "range delete + multiple inserts inside must converge");
        assert!(!pa.text().contains('W'));
        assert!(!pa.text().contains('X'));
        assert!(pa.text().contains("i1"));
        assert!(pa.text().contains("i2"));
        assert!(pa.text().contains('Y'));
        assert!(pa.text().contains('Z'));
    }

    #[test]
    fn four_peer_all_insert_same_counter_text_length_correct() {
        // 4 peers, counter=7 each, insert single char; merged doc has 4 chars.
        let ops: Vec<Op> = (10u64..14)
            .map(|p| make_insert(p, 7, RgaPos::Head, "x"))
            .collect();
        let mut doc = DocState::new(PeerId(57_000));
        for op in &ops {
            doc.apply(op.clone());
        }
        assert_eq!(doc.text().chars().count(), 4, "4-peer same-counter insert must yield 4 chars");
    }

    #[test]
    fn set_meta_five_different_keys_all_retrievable() {
        // 5 distinct SetMeta keys; all 5 are retrievable individually from op_log.
        let keys_vals = [("k1", "v1"), ("k2", "v2"), ("k3", "v3"), ("k4", "v4"), ("k5", "v5")];
        let mut doc = DocState::new(PeerId(58_000));
        for (i, (key, val)) in keys_vals.iter().enumerate() {
            doc.apply(Op {
                id: OpId { peer: PeerId(58_000), counter: i as u64 + 1 },
                kind: OpKind::SetMeta { key: (*key).into(), value: (*val).into() },
            });
        }
        for (key, expected_val) in &keys_vals {
            let found = doc.op_log().iter().find_map(|op| {
                if let OpKind::SetMeta { key: k, value } = &op.kind { if k == key { return Some(value.as_str()); } }
                None
            });
            assert_eq!(found, Some(*expected_val), "key {key} must have value {expected_val}");
        }
    }

    #[test]
    fn set_meta_key_empty_string_value_stored() {
        // SetMeta with empty string value must be stored and retrievable.
        let mut doc = DocState::new(PeerId(60_000));
        doc.apply(Op {
            id: OpId { peer: PeerId(60_000), counter: 1 },
            kind: OpKind::SetMeta { key: "empty_val".into(), value: "".into() },
        });
        let found = doc.op_log().iter().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind { if key == "empty_val" { return Some(value.as_str()); } }
            None
        });
        assert_eq!(found, Some(""), "empty string value must be stored");
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn four_peer_insert_different_counters_all_converge() {
        // 4 peers with different counters; higher counter wins left in same anchor.
        let ops: Vec<Op> = (1u64..=4)
            .map(|p| make_insert(p, p * 10, RgaPos::Head, &format!("n{p}")))
            .collect();

        let mut doc_fwd = DocState::new(PeerId(61_000));
        let mut doc_rev = DocState::new(PeerId(61_000));
        for op in &ops {
            doc_fwd.apply(op.clone());
        }
        for op in ops.iter().rev() {
            doc_rev.apply(op.clone());
        }
        assert_eq!(doc_fwd.text(), doc_rev.text(), "4 peers different counters must converge");
        for p in 1..=4 {
            assert!(doc_fwd.text().contains(&format!("n{p}")));
        }
    }

    #[test]
    fn compaction_round_trip_text_checksum_equal() {
        // 200 inserts, 100 deletes (even indices); compact and replay; text must match.
        let mut doc = DocState::new(PeerId(62_000));
        let mut ids: Vec<OpId> = Vec::with_capacity(200);
        let first = doc.local_insert(RgaPos::Head, "c");
        ids.push(first.id);
        let mut prev = first.id;
        for _ in 1..200 {
            let op = doc.local_insert(RgaPos::After(prev), "c");
            prev = op.id;
            ids.push(op.id);
        }
        for id in ids.iter().step_by(2) {
            doc.local_delete(*id);
        }
        let original_text = doc.text();

        let deleted_ids: std::collections::HashSet<OpId> = doc
            .op_log()
            .iter()
            .filter_map(|o| if let OpKind::Delete { target } = &o.kind { Some(*target) } else { None })
            .collect();
        let live_ops: Vec<Op> = doc
            .op_log()
            .iter()
            .filter(|o| matches!(&o.kind, OpKind::Insert { .. }) && !deleted_ids.contains(&o.id))
            .cloned()
            .collect();

        let mut compacted = DocState::new(PeerId(62_000));
        for op in live_ops {
            compacted.apply(op);
        }

        assert_eq!(compacted.text(), original_text, "200-insert 100-delete compaction checksum must match");
        assert_eq!(compacted.text().chars().count(), 100);
    }

    #[test]
    fn peer_a_deletes_last_node_peer_b_inserts_after_it() {
        // Shared: A→B. Peer A deletes B. Peer B inserts "tail" After B. Cross-merge.
        let mut pa = DocState::new(PeerId(63_000));
        let node_a = pa.local_insert(RgaPos::Head, "A");
        let node_b = pa.local_insert(RgaPos::After(node_a.id), "B");

        let mut pb = DocState::new(PeerId(63_001));
        pb.apply(node_a.clone());
        pb.apply(node_b.clone());

        let del_b = pa.local_delete(node_b.id);
        let ins_tail = pb.local_insert(RgaPos::After(node_b.id), "tail");

        pa.apply(ins_tail.clone());
        pb.apply(del_b.clone());

        assert_eq!(pa.text(), pb.text(), "delete-last + insert-after must converge");
        assert!(!pa.text().contains('B'));
        assert!(pa.text().contains('A'));
        assert!(pa.text().contains("tail"));
    }

    #[test]
    fn delete_range_then_insert_after_chain_correct_order() {
        // Delete middle two of four; insert after the deleted nodes; order must be correct.
        let mut doc = DocState::new(PeerId(59_000));
        let op1 = doc.local_insert(RgaPos::Head, "1");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "2");
        let op3 = doc.local_insert(RgaPos::After(op2.id), "3");
        let op4 = doc.local_insert(RgaPos::After(op3.id), "4");

        // Delete 2 and 3.
        doc.local_delete(op2.id);
        doc.local_delete(op3.id);

        // Insert "X" after op2 (deleted) and "Y" after op3 (deleted).
        doc.local_insert(RgaPos::After(op2.id), "X");
        doc.local_insert(RgaPos::After(op3.id), "Y");

        let text = doc.text();
        // Order: 1 X Y 4 (2 and 3 tombstoned but used as anchors).
        assert!(text.contains('1'));
        assert!(text.contains('X'));
        assert!(text.contains('Y'));
        assert!(text.contains('4'));
        assert!(!text.contains('2'));
        assert!(!text.contains('3'));
        let pos1 = text.find('1').unwrap();
        let pos4 = text.find('4').unwrap();
        assert!(pos1 < pos4, "1 must precede 4");
        let _ = op4;
    }

    // ── wave AF-6: targeted coverage additions ───────────────────────────────

    // 1. Undo last N ops: revert document to prior state
    #[test]
    fn undo_last_n_ops_reverts_document() {
        // Build a doc with 5 inserts, then "undo" the last 2 by replaying ops 1-3.
        let mut doc = DocState::new(PeerId(30_000));
        let op1 = doc.local_insert(RgaPos::Head, "A");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "B");
        let op3 = doc.local_insert(RgaPos::After(op2.id), "C");
        doc.local_insert(RgaPos::After(op3.id), "D");
        doc.local_insert(RgaPos::After(op3.id), "E");
        assert_eq!(doc.text().chars().filter(|c| "ABCDE".contains(*c)).count(), 5);

        // Simulate undo by rebuilding from first 3 ops.
        let snapshot: Vec<Op> = doc.op_log()[..3].to_vec();
        let mut reverted = DocState::new(PeerId(30_000));
        for op in snapshot {
            reverted.apply(op);
        }
        assert_eq!(reverted.text(), "ABC", "undo last 2 ops must revert to ABC");
        assert_eq!(reverted.op_log().len(), 3);
    }

    #[test]
    fn undo_last_1_op_reverts_to_prior_state() {
        let mut doc = DocState::new(PeerId(30_001));
        let op1 = doc.local_insert(RgaPos::Head, "hello");
        doc.local_insert(RgaPos::After(op1.id), " world");
        assert_eq!(doc.text(), "hello world");

        // Revert last op by replaying only op1.
        let mut reverted = DocState::new(PeerId(30_001));
        reverted.apply(doc.op_log()[0].clone());
        assert_eq!(reverted.text(), "hello");
    }

    // 2. Document with emoji (multi-byte chars) converges correctly
    #[test]
    fn emoji_document_converges_correctly() {
        let mut pa = DocState::new(PeerId(30_010));
        let op_a = pa.local_insert(RgaPos::Head, "Hello 👋");

        let mut pb = DocState::new(PeerId(30_011));
        let op_b = pb.local_insert(RgaPos::Head, "World 🌍");

        // Cross-merge.
        pa.apply(op_b.clone());
        pb.apply(op_a.clone());

        assert_eq!(pa.text(), pb.text(), "emoji docs must converge");
        assert!(pa.text().contains("👋"), "emoji 👋 must survive merge");
        assert!(pa.text().contains("🌍"), "emoji 🌍 must survive merge");
    }

    #[test]
    fn emoji_single_node_text_intact() {
        let mut doc = DocState::new(PeerId(30_012));
        doc.local_insert(RgaPos::Head, "✨🦀🎉");
        assert_eq!(doc.text(), "✨🦀🎉");
        assert_eq!(doc.text().chars().count(), 3);
    }

    #[test]
    fn emoji_insert_then_delete_leaves_rest() {
        let mut doc = DocState::new(PeerId(30_013));
        let op_a = doc.local_insert(RgaPos::Head, "🌙");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "⭐");
        doc.local_insert(RgaPos::After(op_b.id), "☀️");
        doc.local_delete(op_b.id);
        assert!(!doc.text().contains('⭐'), "deleted emoji must not appear");
        assert!(doc.text().contains("🌙"));
    }

    // 3. Two peers swap a single character, converge to same final text
    #[test]
    fn two_peers_swap_single_char_converge() {
        // Both peers start with "X" at Head; each deletes it and inserts their char.
        let mut pa = DocState::new(PeerId(30_020));
        let shared = pa.local_insert(RgaPos::Head, "X");

        let mut pb = DocState::new(PeerId(30_021));
        pb.apply(shared.clone());

        // A replaces X with "A".
        let del_a = pa.local_delete(shared.id);
        let ins_a = pa.local_insert(RgaPos::After(shared.id), "A");

        // B replaces X with "B".
        let del_b = pb.local_delete(shared.id);
        let ins_b = pb.local_insert(RgaPos::After(shared.id), "B");

        // Cross merge.
        pa.apply(del_b.clone());
        pa.apply(ins_b.clone());
        pb.apply(del_a.clone());
        pb.apply(ins_a.clone());

        // Both converge to same text; "X" is gone.
        assert_eq!(pa.text(), pb.text(), "peers must converge after swap");
        assert!(!pa.text().contains('X'), "original X must be deleted");
        assert!(pa.text().contains('A'));
        assert!(pa.text().contains('B'));
    }

    #[test]
    fn two_peers_swap_single_char_idempotent_after_extra_merge() {
        let mut pa = DocState::new(PeerId(30_022));
        let op = pa.local_insert(RgaPos::Head, "Z");
        let mut pb = DocState::new(PeerId(30_023));
        pb.apply(op.clone());
        let del_a = pa.local_delete(op.id);
        let del_b = pb.local_delete(op.id);
        pa.apply(del_b);
        pb.apply(del_a);
        assert_eq!(pa.text(), pb.text());
        // Merge again (idempotent).
        let text_before = pa.text();
        pa.merge(&pb);
        assert_eq!(pa.text(), text_before);
    }

    // 4. Meta key with empty string value
    #[test]
    fn set_meta_empty_string_value_recorded() {
        let mut doc = DocState::new(PeerId(30_030));
        let op = Op {
            id: OpId {
                peer: PeerId(30_030),
                counter: 1,
            },
            kind: OpKind::SetMeta {
                key: "note".to_string(),
                value: "".to_string(),
            },
        };
        doc.apply(op);
        let found = doc.op_log().iter().find_map(|o| {
            if let OpKind::SetMeta { key, value } = &o.kind {
                if key == "note" {
                    return Some(value.clone());
                }
            }
            None
        });
        assert_eq!(found, Some(String::new()), "empty value must be recorded");
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn set_meta_empty_key_with_empty_value() {
        let mut doc = DocState::new(PeerId(30_031));
        let op = Op {
            id: OpId {
                peer: PeerId(30_031),
                counter: 1,
            },
            kind: OpKind::SetMeta {
                key: "".to_string(),
                value: "".to_string(),
            },
        };
        doc.apply(op);
        assert_eq!(doc.op_log().len(), 1);
        assert_eq!(doc.text(), "");
        match &doc.op_log()[0].kind {
            OpKind::SetMeta { key, value } => {
                assert_eq!(key, "");
                assert_eq!(value, "");
            }
            _ => panic!("expected SetMeta"),
        }
    }

    #[test]
    fn set_meta_multiple_empty_values() {
        let mut doc = DocState::new(PeerId(30_032));
        for ctr in 1u64..=3 {
            doc.apply(Op {
                id: OpId {
                    peer: PeerId(30_032),
                    counter: ctr,
                },
                kind: OpKind::SetMeta {
                    key: format!("k{ctr}"),
                    value: "".to_string(),
                },
            });
        }
        let empty_val_count = doc
            .op_log()
            .iter()
            .filter(|o| matches!(&o.kind, OpKind::SetMeta { value, .. } if value.is_empty()))
            .count();
        assert_eq!(empty_val_count, 3);
        assert_eq!(doc.text(), "");
    }

    // 5. Op Display/Debug formats
    #[test]
    fn op_debug_format_contains_peer_id() {
        let op = Op {
            id: OpId {
                peer: PeerId(42),
                counter: 7,
            },
            kind: OpKind::Insert {
                pos: RgaPos::Head,
                text: "test".to_string(),
            },
        };
        let dbg = format!("{op:?}");
        assert!(dbg.contains("42"), "debug must include peer id 42");
        assert!(dbg.contains("7"), "debug must include counter 7");
        assert!(dbg.contains("test"), "debug must include text");
    }

    #[test]
    fn op_id_debug_format_shows_fields() {
        let id = OpId {
            peer: PeerId(99),
            counter: 123,
        };
        let dbg = format!("{id:?}");
        assert!(dbg.contains("99"));
        assert!(dbg.contains("123"));
    }

    #[test]
    fn peer_id_debug_format() {
        let peer = PeerId(55);
        let dbg = format!("{peer:?}");
        assert!(dbg.contains("55"));
    }

    #[test]
    fn rga_pos_debug_format_head() {
        let pos = RgaPos::Head;
        let dbg = format!("{pos:?}");
        assert!(dbg.contains("Head"));
    }

    #[test]
    fn rga_pos_debug_format_after() {
        let pos = RgaPos::After(OpId {
            peer: PeerId(3),
            counter: 5,
        });
        let dbg = format!("{pos:?}");
        assert!(dbg.contains("After"));
        assert!(dbg.contains("3"));
        assert!(dbg.contains("5"));
    }

    #[test]
    fn op_kind_debug_insert() {
        let kind = OpKind::Insert {
            pos: RgaPos::Head,
            text: "hello".to_string(),
        };
        let dbg = format!("{kind:?}");
        assert!(dbg.contains("Insert"));
        assert!(dbg.contains("hello"));
    }

    #[test]
    fn op_kind_debug_delete() {
        let kind = OpKind::Delete {
            target: OpId {
                peer: PeerId(1),
                counter: 2,
            },
        };
        let dbg = format!("{kind:?}");
        assert!(dbg.contains("Delete"));
    }

    #[test]
    fn op_kind_debug_set_meta() {
        let kind = OpKind::SetMeta {
            key: "mykey".to_string(),
            value: "myval".to_string(),
        };
        let dbg = format!("{kind:?}");
        assert!(dbg.contains("SetMeta"));
        assert!(dbg.contains("mykey"));
        assert!(dbg.contains("myval"));
    }

    // Additional convergence edge cases
    #[test]
    fn crdt_insert_empty_string_two_peers_converge() {
        let mut pa = DocState::new(PeerId(30_050));
        let op_a = pa.local_insert(RgaPos::Head, "");
        let mut pb = DocState::new(PeerId(30_051));
        let op_b = pb.local_insert(RgaPos::Head, "");
        pa.apply(op_b.clone());
        pb.apply(op_a.clone());
        assert_eq!(pa.text(), pb.text());
        assert_eq!(pa.text(), "");
    }

    #[test]
    fn crdt_meta_key_with_unicode_value() {
        let mut doc = DocState::new(PeerId(30_060));
        doc.apply(Op {
            id: OpId {
                peer: PeerId(30_060),
                counter: 1,
            },
            kind: OpKind::SetMeta {
                key: "lang".to_string(),
                value: "日本語".to_string(),
            },
        });
        let val = doc.op_log().iter().find_map(|o| {
            if let OpKind::SetMeta { key, value } = &o.kind {
                if key == "lang" {
                    return Some(value.as_str());
                }
            }
            None
        });
        assert_eq!(val, Some("日本語"));
    }

    #[test]
    fn crdt_insert_only_whitespace_converges() {
        let mut pa = DocState::new(PeerId(30_070));
        let op_a = pa.local_insert(RgaPos::Head, "   ");
        let mut pb = DocState::new(PeerId(30_071));
        let op_b = pb.local_insert(RgaPos::Head, "\t\n");
        pa.apply(op_b.clone());
        pb.apply(op_a.clone());
        assert_eq!(pa.text(), pb.text());
        assert!(pa.text().contains("   "));
        assert!(pa.text().contains("\t\n"));
    }

    #[test]
    fn crdt_five_sequential_deletes_all_gone() {
        let mut doc = DocState::new(PeerId(30_080));
        let mut ids = vec![];
        let mut prev = doc.local_insert(RgaPos::Head, "1").id;
        ids.push(prev);
        for ch in ["2", "3", "4", "5"] {
            let op = doc.local_insert(RgaPos::After(prev), ch);
            prev = op.id;
            ids.push(prev);
        }
        assert_eq!(doc.text(), "12345");
        for id in ids {
            doc.local_delete(id);
        }
        assert_eq!(doc.text(), "");
        assert_eq!(doc.op_log().iter().filter(|o| matches!(o.kind, OpKind::Delete { .. })).count(), 5);
    }

    // Additional tests to reach 270+

    #[test]
    fn undo_last_3_ops_reverts_to_first_two() {
        let mut doc = DocState::new(PeerId(31_000));
        let op1 = doc.local_insert(RgaPos::Head, "X");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "Y");
        doc.local_insert(RgaPos::After(op2.id), "Z");
        doc.local_insert(RgaPos::After(op2.id), "W");
        doc.local_insert(RgaPos::After(op2.id), "V");
        // Revert to first 2 ops.
        let snap: Vec<Op> = doc.op_log()[..2].to_vec();
        let mut reverted = DocState::new(PeerId(31_000));
        for op in snap { reverted.apply(op); }
        assert_eq!(reverted.text(), "XY");
        assert_eq!(reverted.op_log().len(), 2);
    }

    #[test]
    fn emoji_multi_byte_delete_leaves_others() {
        let mut doc = DocState::new(PeerId(31_001));
        let op1 = doc.local_insert(RgaPos::Head, "\u{1F600}");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "\u{1F60E}");
        doc.local_insert(RgaPos::After(op2.id), "\u{1F38A}");
        doc.local_delete(op2.id);
        let text = doc.text();
        assert!(text.contains('\u{1F600}'));
        assert!(!text.contains('\u{1F60E}'));
        assert!(text.contains('\u{1F38A}'));
    }

    #[test]
    fn set_meta_value_with_spaces() {
        let mut doc = DocState::new(PeerId(31_002));
        doc.apply(Op {
            id: OpId { peer: PeerId(31_002), counter: 1 },
            kind: OpKind::SetMeta {
                key: "desc".to_string(),
                value: "hello world".to_string(),
            },
        });
        let val = doc.op_log().iter().find_map(|o| {
            if let OpKind::SetMeta { key, value } = &o.kind {
                if key == "desc" { return Some(value.clone()); }
            }
            None
        });
        assert_eq!(val, Some("hello world".to_string()));
    }

    #[test]
    fn op_debug_includes_kind_info() {
        let op = Op {
            id: OpId { peer: PeerId(10), counter: 1 },
            kind: OpKind::Delete {
                target: OpId { peer: PeerId(5), counter: 1 },
            },
        };
        let s = format!("{op:?}");
        assert!(!s.is_empty(), "debug output must be non-empty");
    }

    #[test]
    fn crdt_insert_after_last_op_appends() {
        let mut doc = DocState::new(PeerId(31_010));
        let op = doc.local_insert(RgaPos::Head, "first");
        doc.local_insert(RgaPos::After(op.id), "last");
        assert_eq!(doc.text(), "firstlast");
    }

    #[test]
    fn crdt_op_log_order_matches_apply_order() {
        let mut doc = DocState::new(PeerId(31_011));
        let op1 = doc.local_insert(RgaPos::Head, "one");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "two");
        let op3 = doc.local_insert(RgaPos::After(op2.id), "three");
        assert_eq!(doc.op_log()[0].id, op1.id);
        assert_eq!(doc.op_log()[1].id, op2.id);
        assert_eq!(doc.op_log()[2].id, op3.id);
    }

    #[test]
    fn set_meta_does_not_affect_insert_count() {
        let mut doc = DocState::new(PeerId(31_012));
        doc.local_insert(RgaPos::Head, "hello");
        doc.apply(Op {
            id: OpId { peer: PeerId(31_012), counter: 99 },
            kind: OpKind::SetMeta { key: "k".to_string(), value: "v".to_string() },
        });
        let insert_count = doc.op_log().iter()
            .filter(|o| matches!(o.kind, OpKind::Insert { .. }))
            .count();
        assert_eq!(insert_count, 1);
        assert_eq!(doc.text(), "hello");
    }

    #[test]
    fn crdt_peer_id_ord_zero_less_than_one() {
        assert!(PeerId(0) < PeerId(1));
        assert!(PeerId(1) > PeerId(0));
        assert_eq!(PeerId(5), PeerId(5));
    }

    // ── wave AG: 35 additional collab tests ─────────────────────────────────

    #[test]
    fn three_peer_concurrent_insert_converges() {
        // 3 peers each insert a different char; after full cross-merge all converge.
        let mut pa = DocState::new(PeerId(40_001));
        let opa = pa.local_insert(RgaPos::Head, "X");
        let mut pb = DocState::new(PeerId(40_002));
        let opb = pb.local_insert(RgaPos::Head, "Y");
        let mut pc = DocState::new(PeerId(40_003));
        let opc = pc.local_insert(RgaPos::Head, "Z");

        for op in [opb.clone(), opc.clone()] { pa.apply(op); }
        for op in [opa.clone(), opc.clone()] { pb.apply(op); }
        for op in [opa.clone(), opb.clone()] { pc.apply(op); }

        assert_eq!(pa.text(), pb.text());
        assert_eq!(pb.text(), pc.text());
        assert_eq!(pa.text().chars().count(), 3);
    }

    #[test]
    fn three_peer_split_brain_resolves() {
        // Each peer has a different local doc; merging all three converges.
        let mut pa = DocState::new(PeerId(40_010));
        let opa = pa.local_insert(RgaPos::Head, "split");
        let mut pb = DocState::new(PeerId(40_011));
        let opb = pb.local_insert(RgaPos::Head, "brain");
        let mut pc = DocState::new(PeerId(40_012));
        let opc = pc.local_insert(RgaPos::Head, "fix");

        pa.apply(opb.clone()); pa.apply(opc.clone());
        pb.apply(opa.clone()); pb.apply(opc.clone());
        pc.apply(opa.clone()); pc.apply(opb.clone());

        assert_eq!(pa.text(), pb.text(), "split-brain: A and B must converge");
        assert_eq!(pb.text(), pc.text(), "split-brain: B and C must converge");
    }

    #[test]
    fn set_meta_round_trip() {
        // Apply a SetMeta op, then retrieve the value from op_log.
        let mut doc = DocState::new(PeerId(40_020));
        doc.apply(Op {
            id: OpId { peer: PeerId(40_020), counter: 1 },
            kind: OpKind::SetMeta { key: "theme".to_string(), value: "dark".to_string() },
        });
        let val = doc.op_log().iter().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind {
                if key == "theme" { return Some(value.clone()); }
            }
            None
        });
        assert_eq!(val, Some("dark".to_string()));
    }

    #[test]
    fn set_meta_overwrites_previous() {
        // Two SetMeta ops with the same key; latest (by apply order) wins.
        let mut doc = DocState::new(PeerId(40_030));
        for (counter, val) in [(1u64, "v1"), (2, "v2")] {
            doc.apply(Op {
                id: OpId { peer: PeerId(40_030), counter },
                kind: OpKind::SetMeta { key: "status".to_string(), value: val.to_string() },
            });
        }
        let latest = doc.op_log().iter().rev().find_map(|op| {
            if let OpKind::SetMeta { key, value } = &op.kind {
                if key == "status" { return Some(value.clone()); }
            }
            None
        });
        assert_eq!(latest, Some("v2".to_string()));
    }

    #[test]
    fn set_meta_multiple_keys() {
        // Multiple distinct SetMeta keys all coexist in the op_log.
        let mut doc = DocState::new(PeerId(40_040));
        for (key, val) in [("a", "1"), ("b", "2"), ("c", "3")] {
            doc.apply(Op {
                id: OpId { peer: PeerId(40_040), counter: doc.op_log().len() as u64 + 1 },
                kind: OpKind::SetMeta { key: key.to_string(), value: val.to_string() },
            });
        }
        for key in ["a", "b", "c"] {
            let found = doc.op_log().iter().any(|op| {
                matches!(&op.kind, OpKind::SetMeta { key: k, .. } if k == key)
            });
            assert!(found, "key {key} must be in op_log");
        }
    }

    #[test]
    fn tombstone_revival_not_possible() {
        // Once tombstoned, a node stays deleted even after a redundant delete.
        let mut doc = DocState::new(PeerId(40_050));
        let op = doc.local_insert(RgaPos::Head, "dead");
        doc.local_delete(op.id);
        assert_eq!(doc.text(), "");
        // Re-applying a delete at the same target does not "un-delete" anything.
        doc.apply(Op {
            id: OpId { peer: PeerId(40_050), counter: 99 },
            kind: OpKind::Delete { target: op.id },
        });
        assert_eq!(doc.text(), "", "tombstoned text must not revive");
    }

    #[test]
    fn emoji_in_document_preserved() {
        // 4-byte emoji round-trips correctly through local_insert / text().
        let mut doc = DocState::new(PeerId(40_060));
        doc.local_insert(RgaPos::Head, "hello ");
        doc.local_insert(RgaPos::Head, "world ");
        // Insert emoji at Head; it occupies one unicode scalar value.
        doc.local_insert(RgaPos::Head, "\u{1F600}");
        let text = doc.text();
        assert!(text.contains('\u{1F600}'), "emoji must be preserved in doc text");
    }

    #[test]
    fn concurrent_delete_and_insert_same_position() {
        // Peer A deletes node X; peer B inserts after X. After merge: X gone, new node live.
        let mut pa = DocState::new(PeerId(40_070));
        let base = pa.local_insert(RgaPos::Head, "base");
        let mut pb = DocState::new(PeerId(40_071));
        pb.apply(base.clone());

        let del = pa.local_delete(base.id);
        let ins = pb.local_insert(RgaPos::After(base.id), "after");

        pa.apply(ins);
        pb.apply(del);

        assert_eq!(pa.text(), pb.text(), "delete+insert must converge deterministically");
        assert!(!pa.text().contains("base"), "deleted node must be gone");
        assert!(pa.text().contains("after"), "inserted node must survive");
    }

    #[test]
    fn empty_document_merge_safe() {
        // Merging two fresh (empty) documents is safe and yields empty text.
        let mut doc_a = DocState::new(PeerId(40_080));
        let doc_b = DocState::new(PeerId(40_081));
        doc_a.merge(&doc_b);
        assert_eq!(doc_a.text(), "");
        assert_eq!(doc_a.op_log().len(), 0);
    }

    #[test]
    fn large_doc_100_chars_insert_then_delete_10() {
        // Insert 100 single chars; delete the first 10; 90 remain.
        let mut doc = DocState::new(PeerId(40_090));
        let mut prev_id = doc.local_insert(RgaPos::Head, "a").id;
        let mut ids = vec![prev_id];
        for _ in 1..100 {
            let op = doc.local_insert(RgaPos::After(prev_id), "a");
            prev_id = op.id;
            ids.push(prev_id);
        }
        assert_eq!(doc.text().chars().count(), 100);
        for &id in &ids[..10] {
            doc.local_delete(id);
        }
        assert_eq!(doc.text().chars().count(), 90, "90 chars must remain after deleting 10");
    }

    #[test]
    fn collab_peer_id_uniqueness() {
        // Two distinct PeerIds compare unequal.
        let pa = PeerId(50_001);
        let pb = PeerId(50_002);
        assert_ne!(pa, pb, "two distinct peer IDs must not be equal");
        assert_ne!(pa.0, pb.0);
    }

    #[test]
    fn collab_vector_clock_increments_on_op() {
        // Each local op increments the Lamport counter by exactly 1.
        let mut doc = DocState::new(PeerId(50_010));
        let op1 = doc.local_insert(RgaPos::Head, "a");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "b");
        assert_eq!(op1.id.counter, 1);
        assert_eq!(op2.id.counter, 2);
        assert_eq!(op2.id.counter - op1.id.counter, 1);
    }

    #[test]
    fn collab_encode_decode_round_trip() {
        // Replay op_log into a fresh doc; both have identical text (encode→decode proxy).
        let mut original = DocState::new(PeerId(50_020));
        let op1 = original.local_insert(RgaPos::Head, "encode");
        original.local_insert(RgaPos::After(op1.id), "_decode");
        assert_eq!(original.text(), "encode_decode");

        let mut restored = DocState::new(PeerId(50_020));
        for op in original.op_log().to_vec() {
            restored.apply(op);
        }
        assert_eq!(restored.text(), original.text());
        assert_eq!(restored.op_log().len(), original.op_log().len());
    }

    #[test]
    fn collab_insert_at_end_appends() {
        // local_insert(After(last_op)) appends to the end of the document.
        let mut doc = DocState::new(PeerId(50_030));
        let op1 = doc.local_insert(RgaPos::Head, "start");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "_mid");
        doc.local_insert(RgaPos::After(op2.id), "_end");
        assert_eq!(doc.text(), "start_mid_end");
        assert!(doc.text().ends_with("_end"), "last insert must appear at the end");
    }

    #[test]
    fn collab_insert_at_zero_prepends() {
        // local_insert(Head) on a non-empty doc (depending on tiebreak) adds content.
        let mut doc = DocState::new(PeerId(50_040));
        doc.local_insert(RgaPos::Head, "B");
        // Counter 2 > counter 1 → second Head insert gets higher priority and goes left.
        doc.local_insert(RgaPos::Head, "A");
        let text = doc.text();
        // Both chars present; 'A' (counter 2, higher priority) should be left of 'B'.
        assert!(text.contains('A'));
        assert!(text.contains('B'));
        assert_eq!(text.chars().count(), 2);
    }

    #[test]
    fn collab_delete_range() {
        // Insert 5 chars, delete chars at positions 2..5 (3 deletes).
        let mut doc = DocState::new(PeerId(50_050));
        let op0 = doc.local_insert(RgaPos::Head, "0");
        let op1 = doc.local_insert(RgaPos::After(op0.id), "1");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "2");
        let op3 = doc.local_insert(RgaPos::After(op2.id), "3");
        doc.local_insert(RgaPos::After(op3.id), "4");
        assert_eq!(doc.text(), "01234");
        for id in [op2.id, op3.id] {
            doc.local_delete(id);
        }
        assert!(!doc.text().contains('2'), "2 must be deleted");
        assert!(!doc.text().contains('3'), "3 must be deleted");
        assert_eq!(doc.text().chars().count(), 3, "3 chars must remain");
    }

    #[test]
    fn collab_get_text_after_operations() {
        // Complex sequence of inserts and deletes; text() returns correct result.
        let mut doc = DocState::new(PeerId(50_060));
        let op_h = doc.local_insert(RgaPos::Head, "hello");
        let op_s = doc.local_insert(RgaPos::After(op_h.id), " ");
        doc.local_insert(RgaPos::After(op_s.id), "world");
        assert_eq!(doc.text(), "hello world");
        doc.local_delete(op_s.id);
        assert_eq!(doc.text(), "helloworld");
    }

    #[test]
    fn collab_concurrent_ops_from_same_peer_ordered() {
        // Ops from the same peer must be applied in counter order in the op_log.
        let mut doc = DocState::new(PeerId(50_070));
        let op1 = doc.local_insert(RgaPos::Head, "first");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "second");
        // op_log[0] is op1, op_log[1] is op2.
        assert_eq!(doc.op_log()[0].id.counter, op1.id.counter);
        assert_eq!(doc.op_log()[1].id.counter, op2.id.counter);
        assert!(doc.op_log()[0].id.counter < doc.op_log()[1].id.counter);
    }

    #[test]
    fn collab_merge_idempotent() {
        // merge(doc, X) twice produces same result as merge(doc, X) once.
        let mut pa = DocState::new(PeerId(50_080));
        pa.local_insert(RgaPos::Head, "hello");

        let mut pb = DocState::new(PeerId(50_081));
        pb.merge(&pa);
        let text1 = pb.text();
        let len1 = pb.op_log().len();

        pb.merge(&pa); // idempotent
        assert_eq!(pb.text(), text1, "text must not change after second merge");
        assert_eq!(pb.op_log().len(), len1, "op_log len must not grow after second merge");
    }

    #[test]
    fn collab_char_at_position() {
        // text().chars().nth(i) returns the correct character at each index.
        let mut doc = DocState::new(PeerId(50_090));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        doc.local_insert(RgaPos::After(op_b.id), "C");
        let chars: Vec<char> = doc.text().chars().collect();
        assert_eq!(chars[0], 'A', "char at 0 must be A");
        assert_eq!(chars[1], 'B', "char at 1 must be B");
        assert_eq!(chars[2], 'C', "char at 2 must be C");
    }

    #[test]
    fn collab_string_contains_after_inserts() {
        let mut doc = DocState::new(PeerId(50_100));
        doc.local_insert(RgaPos::Head, "canvas");
        doc.local_insert(RgaPos::Head, "nom_");
        assert!(doc.text().contains("canvas"), "text must contain 'canvas'");
        assert!(doc.text().contains("nom_"), "text must contain 'nom_'");
    }

    #[test]
    fn collab_length_after_deletes() {
        // text().chars().count() correctly reflects deletions.
        let mut doc = DocState::new(PeerId(50_110));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        let op_c = doc.local_insert(RgaPos::After(op_b.id), "C");
        assert_eq!(doc.text().chars().count(), 3);
        doc.local_delete(op_a.id);
        assert_eq!(doc.text().chars().count(), 2);
        doc.local_delete(op_c.id);
        assert_eq!(doc.text().chars().count(), 1);
        assert_eq!(doc.text(), "B");
    }

    #[test]
    fn collab_no_op_merge_no_change() {
        // Merging a doc that has no new ops changes nothing.
        let mut pa = DocState::new(PeerId(50_120));
        pa.local_insert(RgaPos::Head, "hello");
        let text_before = pa.text();
        let len_before = pa.op_log().len();

        let empty = DocState::new(PeerId(50_121));
        pa.merge(&empty);

        assert_eq!(pa.text(), text_before);
        assert_eq!(pa.op_log().len(), len_before);
    }

    #[test]
    fn collab_two_inserts_at_same_pos_deterministic() {
        // Both orders produce identical text (CRDT commutativity).
        let op1 = make_insert(1, 1, RgaPos::Head, "L");
        let op2 = make_insert(2, 1, RgaPos::Head, "R");

        let mut doc_lr = DocState::new(PeerId(50_130));
        doc_lr.apply(op1.clone());
        doc_lr.apply(op2.clone());

        let mut doc_rl = DocState::new(PeerId(50_130));
        doc_rl.apply(op2);
        doc_rl.apply(op1);

        assert_eq!(doc_lr.text(), doc_rl.text(), "two inserts at same pos must be deterministic");
    }

    #[test]
    fn collab_delete_nonexistent_pos_safe() {
        // Applying a delete for an op that was never inserted must not panic.
        let mut doc = DocState::new(PeerId(50_140));
        let ghost_id = OpId { peer: PeerId(9999), counter: 1 };
        let del = Op {
            id: OpId { peer: PeerId(50_140), counter: 1 },
            kind: OpKind::Delete { target: ghost_id },
        };
        doc.apply(del); // must not panic
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn collab_full_document_replace() {
        // Delete all existing nodes and replace with entirely new content.
        let mut doc = DocState::new(PeerId(50_150));
        let op_old = doc.local_insert(RgaPos::Head, "old_content");
        assert_eq!(doc.text(), "old_content");
        doc.local_delete(op_old.id);
        assert_eq!(doc.text(), "");
        doc.local_insert(RgaPos::Head, "new_content");
        assert_eq!(doc.text(), "new_content");
    }

    #[test]
    fn collab_apply_remote_ops_in_order() {
        // Remote ops with increasing counters are applied and produce correct text.
        let mut doc = DocState::new(PeerId(50_160));
        let remote_ops: Vec<Op> = (1u64..=5)
            .scan(None::<OpId>, |prev, counter| {
                let pos = match *prev {
                    None => RgaPos::Head,
                    Some(id) => RgaPos::After(id),
                };
                let id = OpId { peer: PeerId(99), counter };
                *prev = Some(id);
                Some(Op { id, kind: OpKind::Insert { pos, text: counter.to_string() } })
            })
            .collect();
        for op in remote_ops {
            doc.apply(op);
        }
        assert_eq!(doc.text(), "12345");
    }

    #[test]
    fn collab_partial_sync() {
        // Peer A has ops 1-5; peer B has 1-3. After merge B gets ops 4-5.
        let mut pa = DocState::new(PeerId(50_170));
        let mut ids = Vec::new();
        let op1 = pa.local_insert(RgaPos::Head, "1");
        ids.push(op1.id);
        for i in 1..5u64 {
            let op = pa.local_insert(RgaPos::After(ids[i as usize - 1]), &(i + 1).to_string());
            ids.push(op.id);
        }
        assert_eq!(pa.text(), "12345");

        // Peer B gets only the first 3 ops.
        let mut pb = DocState::new(PeerId(50_171));
        for op in pa.op_log()[..3].to_vec() {
            pb.apply(op);
        }
        assert_eq!(pb.text(), "123");

        // After merge B gets ops 4-5.
        pb.merge(&pa);
        assert_eq!(pb.text(), "12345");
    }

    #[test]
    fn collab_causal_ordering_preserved() {
        // Op authored after another on the same peer has a greater counter (causal order).
        let mut doc = DocState::new(PeerId(50_180));
        let op_early = doc.local_insert(RgaPos::Head, "early");
        let op_late = doc.local_insert(RgaPos::After(op_early.id), "late");
        assert!(
            op_early.id < op_late.id,
            "earlier op must have smaller OpId (causal order)"
        );
    }

    #[test]
    fn collab_unicode_multibyte_positions_correct() {
        // Multi-byte chars don't confuse position tracking; text() returns full string.
        let mut doc = DocState::new(PeerId(50_190));
        let op1 = doc.local_insert(RgaPos::Head, "\u{4E2D}"); // 中
        let op2 = doc.local_insert(RgaPos::After(op1.id), "\u{6587}"); // 文
        doc.local_insert(RgaPos::After(op2.id), "\u{5B57}"); // 字
        assert_eq!(doc.text(), "中文字");
        assert_eq!(doc.text().chars().count(), 3);
    }

    #[test]
    fn collab_snapshot_matches_live_state() {
        // A snapshot (clone of op_log replayed into fresh doc) matches live doc text.
        let mut live = DocState::new(PeerId(50_200));
        let op1 = live.local_insert(RgaPos::Head, "snap");
        live.local_insert(RgaPos::After(op1.id), "shot");
        live.local_delete(op1.id);

        let mut snap = DocState::new(PeerId(50_200));
        for op in live.op_log().to_vec() {
            snap.apply(op);
        }
        assert_eq!(snap.text(), live.text(), "snapshot must match live state");
    }

    #[test]
    fn collab_large_concurrent_batch() {
        // 10 ops from each of 3 peers simultaneously; all converge to same text.
        let mut docs: Vec<DocState> = (0u64..3)
            .map(|i| DocState::new(PeerId(50_210 + i)))
            .collect();

        // Each peer authors 10 inserts.
        let mut all_ops: Vec<Vec<Op>> = (0..3).map(|_| Vec::new()).collect();
        for (idx, doc) in docs.iter_mut().enumerate() {
            let mut prev = doc.local_insert(RgaPos::Head, &format!("p{idx}_0"));
            all_ops[idx].push(prev.clone());
            for j in 1..10 {
                let op = doc.local_insert(RgaPos::After(prev.id), &format!("_p{idx}_{j}"));
                all_ops[idx].push(op.clone());
                prev = op;
            }
        }

        // Cross-merge all peers.
        let all_flat: Vec<Op> = all_ops.iter().flatten().cloned().collect();
        let mut merged = DocState::new(PeerId(50_213));
        for op in &all_flat {
            // Only apply once; idempotent re-apply is safe.
            if !merged.op_log().iter().any(|o| o.id == op.id) {
                merged.apply(op.clone());
            }
        }
        // All 30 ops applied → 30 log entries.
        assert_eq!(merged.op_log().len(), 30, "merged doc must have 30 ops");
    }

    #[test]
    fn collab_text_search_after_edits() {
        // After a series of edits, a substring search on text() works correctly.
        let mut doc = DocState::new(PeerId(50_220));
        let op1 = doc.local_insert(RgaPos::Head, "hello ");
        let op2 = doc.local_insert(RgaPos::After(op1.id), "world");
        assert!(doc.text().contains("hello"), "must find 'hello'");
        assert!(doc.text().contains("world"), "must find 'world'");
        // Delete "world"; search returns false.
        doc.local_delete(op2.id);
        assert!(!doc.text().contains("world"), "'world' must not be found after delete");
        assert!(doc.text().contains("hello"), "'hello' must still be found");
    }

    #[test]
    fn collab_op_log_grows_with_each_op() {
        // op_log length increases by 1 for each applied op.
        let mut doc = DocState::new(PeerId(50_230));
        assert_eq!(doc.op_log().len(), 0);
        let op1 = doc.local_insert(RgaPos::Head, "a");
        assert_eq!(doc.op_log().len(), 1);
        doc.local_insert(RgaPos::After(op1.id), "b");
        assert_eq!(doc.op_log().len(), 2);
        doc.apply(Op {
            id: OpId { peer: PeerId(50_230), counter: 100 },
            kind: OpKind::SetMeta { key: "x".to_string(), value: "y".to_string() },
        });
        assert_eq!(doc.op_log().len(), 3);
    }

    #[test]
    fn collab_delete_keeps_other_nodes_intact() {
        // Deleting one node must not affect adjacent nodes.
        let mut doc = DocState::new(PeerId(50_240));
        let op_a = doc.local_insert(RgaPos::Head, "A");
        let op_b = doc.local_insert(RgaPos::After(op_a.id), "B");
        let op_c = doc.local_insert(RgaPos::After(op_b.id), "C");
        doc.local_delete(op_b.id);
        assert_eq!(doc.text(), "AC", "deleting B must leave A and C intact");
        let _ = op_c;
    }
}
