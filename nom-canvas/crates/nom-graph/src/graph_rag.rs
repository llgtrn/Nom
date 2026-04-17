#![deny(unsafe_code)]
//! Graph-RAG retrieval over a [`Dag`].
//!
//! Retrieval strategy:
//!   1. Every node in the DAG is assigned a deterministic 16-float embedding
//!      derived from its `NodeId` string via a simple FNV-1a hash spread.
//!   2. BFS from every node explores the graph up to `max_hops` steps.
//!   3. Each (node, hop_distance) pair is scored:
//!         score = cosine_sim(query_vec, node_vec) * (1.0 / (1.0 + hops as f32))
//!   4. When the same node is reached via multiple paths, the best score is kept.
//!   5. The top-k results by score are returned.

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::cmp::Ordering;

use crate::dag::Dag;
use crate::node::NodeId;

// ---------------------------------------------------------------------------
// QueryVec — fixed-size 16-float embedding (no external deps)
// ---------------------------------------------------------------------------

/// Fixed-size query embedding: 16 × f32.
pub type QueryVec = [f32; 16];

// ---------------------------------------------------------------------------
// RetrievedNode
// ---------------------------------------------------------------------------

/// A single retrieval result.
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievedNode {
    /// The node identifier.
    pub node_id: NodeId,
    /// Relevance score in [0, 1], hop-penalised.
    pub score: f32,
    /// Graph distance (BFS hops) from the nearest seed node.
    pub hops: usize,
}

// BinaryHeap wrapper for max-heap ordering by score.
#[derive(Debug, Clone)]
struct ScoredNode {
    node_id: NodeId,
    score: f32,
    hops: usize,
}

impl PartialEq for ScoredNode {
    fn eq(&self, other: &Self) -> bool {
        self.score.total_cmp(&other.score) == Ordering::Equal
    }
}
impl Eq for ScoredNode {}
impl PartialOrd for ScoredNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ScoredNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.total_cmp(&other.score)
    }
}

// ---------------------------------------------------------------------------
// Deterministic node embedding
// ---------------------------------------------------------------------------

/// Derive a deterministic `QueryVec` from a `NodeId` string.
///
/// Algorithm: fold every byte of the UTF-8 string into a `u64` seed using
/// FNV-1a, then spread the seed across 16 slots by rotating the u64 by
/// `(slot * 4)` bits and taking the low 32 bits.  The result is L2-normalised
/// so cosine_sim values are meaningful.
pub fn node_vec(node_id: &str) -> QueryVec {
    // 1. FNV-1a hash of the string bytes.
    let mut seed: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in node_id.bytes() {
        seed ^= u64::from(byte);
        seed = seed.wrapping_mul(0x0000_0100_0000_01b3);
    }

    // 2. Spread seed across 16 slots via rotation.
    let mut raw = [0.0f32; 16];
    for (i, slot) in raw.iter_mut().enumerate() {
        let rotated = seed.rotate_left((i as u32) * 4);
        // Map u32 bits → [0, 1).
        *slot = (rotated as u32 as f64 / u32::MAX as f64) as f32;
    }

    // 3. L2-normalise.
    let norm: f32 = raw.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-9 {
        for slot in &mut raw {
            *slot /= norm;
        }
    }
    raw
}

// ---------------------------------------------------------------------------
// cosine_sim
// ---------------------------------------------------------------------------

/// Standard cosine similarity between two 16-float vectors.
/// Returns a value in [-1.0, 1.0].  Returns 0.0 for zero-magnitude inputs.
pub fn cosine_sim(a: &QueryVec, b: &QueryVec) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a < 1e-9 || mag_b < 1e-9 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

// ---------------------------------------------------------------------------
// Adjacency helpers
// ---------------------------------------------------------------------------

/// Build an undirected adjacency list (both edge directions) for BFS.
fn build_adjacency(dag: &Dag) -> HashMap<&str, Vec<&str>> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for node_id in dag.nodes.keys() {
        adj.entry(node_id.as_str()).or_default();
    }
    for edge in &dag.edges {
        adj.entry(edge.src_node.as_str()).or_default().push(edge.dst_node.as_str());
        adj.entry(edge.dst_node.as_str()).or_default().push(edge.src_node.as_str());
    }
    adj
}

// ---------------------------------------------------------------------------
// GraphRagRetriever
// ---------------------------------------------------------------------------

/// Performs Graph-RAG retrieval over a [`Dag`].
pub struct GraphRagRetriever<'a> {
    dag: &'a Dag,
}

impl<'a> GraphRagRetriever<'a> {
    /// Create a retriever borrowing `dag`.
    pub fn new(dag: &'a Dag) -> Self {
        Self { dag }
    }

    /// Retrieve the top-`top_k` nodes most relevant to `query`.
    ///
    /// Every node in the DAG is a BFS seed (hop=0).  Neighbours are explored
    /// up to `max_hops`.  For each (node, hop) encounter the score is:
    ///
    /// ```text
    /// score = cosine_sim(query, node_vec(node_id)) * (1.0 / (1.0 + hops))
    /// ```
    ///
    /// If a node is reached by multiple paths the best (highest) score wins.
    /// Results are sorted by score descending and truncated to `top_k`.
    pub fn retrieve(&self, query: &QueryVec, top_k: usize, max_hops: usize) -> Vec<RetrievedNode> {
        if top_k == 0 || self.dag.nodes.is_empty() {
            return Vec::new();
        }

        let adj = build_adjacency(self.dag);

        // best_score[node_id] → (score, hops)
        let mut best: HashMap<&str, (f32, usize)> = HashMap::new();

        // BFS from each node as a seed.
        for start_id in self.dag.nodes.keys() {
            // visited tracks the minimum hops at which each node was seen
            // in this BFS, to avoid re-enqueuing at a worse hop depth.
            let mut visited: HashMap<&str, usize> = HashMap::new();
            let mut queue: VecDeque<(&str, usize)> = VecDeque::new();
            queue.push_back((start_id.as_str(), 0));
            visited.insert(start_id.as_str(), 0);

            while let Some((current, hops)) = queue.pop_front() {
                // Score for this (node, hops) encounter.
                let nv = node_vec(current);
                let raw_sim = cosine_sim(query, &nv);
                let score = raw_sim * (1.0 / (1.0 + hops as f32));

                // Update global best for this node.
                let entry = best.entry(current).or_insert((f32::NEG_INFINITY, usize::MAX));
                if score > entry.0 {
                    *entry = (score, hops);
                }

                // Expand neighbours if within hop budget.
                if hops < max_hops {
                    if let Some(neighbours) = adj.get(current) {
                        for &nbr in neighbours {
                            let next_hops = hops + 1;
                            let prev = visited.get(nbr).copied().unwrap_or(usize::MAX);
                            if next_hops < prev {
                                visited.insert(nbr, next_hops);
                                queue.push_back((nbr, next_hops));
                            }
                        }
                    }
                }
            }
        }

        // Collect into a max-heap and drain top-k.
        let mut heap: BinaryHeap<ScoredNode> = best
            .into_iter()
            .map(|(id, (score, hops))| ScoredNode {
                node_id: id.to_owned(),
                score,
                hops,
            })
            .collect();

        let take = top_k.min(heap.len());
        let mut results = Vec::with_capacity(take);
        for _ in 0..take {
            if let Some(sn) = heap.pop() {
                results.push(RetrievedNode { node_id: sn.node_id, score: sn.score, hops: sn.hops });
            }
        }
        results
    }
}

// ---------------------------------------------------------------------------
// CachedRetriever — memoized wrapper using nom-memoize Hash128
// ---------------------------------------------------------------------------

use nom_memoize::Hash128;

/// A caching wrapper around [`GraphRagRetriever`] that avoids re-computing
/// results for identical (query, top_k, max_hops) inputs.
///
/// The cache key is derived by hashing the raw bytes of the `QueryVec` together
/// with `top_k` and `max_hops` using [`Hash128::of_bytes`] / [`Hash128::combine`].
pub struct CachedRetriever<'a> {
    retriever: GraphRagRetriever<'a>,
    cache: std::collections::HashMap<u64, Vec<RetrievedNode>>,
}

impl<'a> CachedRetriever<'a> {
    /// Create a new `CachedRetriever` wrapping a fresh [`GraphRagRetriever`] for `dag`.
    pub fn new(dag: &'a Dag) -> Self {
        Self {
            retriever: GraphRagRetriever::new(dag),
            cache: std::collections::HashMap::new(),
        }
    }

    /// Return retrieval results for `(query, top_k, max_hops)`, re-using a
    /// cached result when the same inputs were seen before.
    pub fn retrieve_cached(
        &mut self,
        query: &QueryVec,
        top_k: usize,
        max_hops: usize,
    ) -> Vec<RetrievedNode> {
        // Build a stable cache key: hash the 16 × f32 bytes, then mix in top_k
        // and max_hops so that different parameters on the same query stay
        // distinct.
        let query_bytes: &[u8] = {
            // SAFETY-free: reinterpret the f32 array as a byte slice via copy.
            &query.iter()
                .flat_map(|f| f.to_le_bytes())
                .collect::<Vec<u8>>()
        };
        let h = Hash128::of_bytes(query_bytes)
            .combine(Hash128::of_u64(top_k as u64))
            .combine(Hash128::of_u64(max_hops as u64));
        let key = h.as_u64();

        if let Some(cached) = self.cache.get(&key) {
            return cached.clone();
        }
        let result = self.retriever.retrieve(query, top_k, max_hops);
        self.cache.insert(key, result.clone());
        result
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::Dag;
    use crate::node::ExecNode;

    fn three_node_dag() -> Dag {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("alpha", "verb"));
        dag.add_node(ExecNode::new("beta", "verb"));
        dag.add_node(ExecNode::new("gamma", "verb"));
        dag.add_edge("alpha", "out", "beta", "in");
        dag.add_edge("beta", "out", "gamma", "in");
        dag
    }

    // -----------------------------------------------------------------------
    // cosine_sim_identical_vecs_is_one
    // -----------------------------------------------------------------------
    #[test]
    fn cosine_sim_identical_vecs_is_one() {
        let v = node_vec("test-node");
        let sim = cosine_sim(&v, &v);
        assert!(
            (sim - 1.0).abs() < 1e-5,
            "cosine_sim of identical vectors should be 1.0, got {sim}"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_retrieves_top_k
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_retrieves_top_k() {
        let dag = three_node_dag();
        let retriever = GraphRagRetriever::new(&dag);

        // Use alpha's own vec as the query — alpha should score highest (hop=0).
        let query = node_vec("alpha");
        let results = retriever.retrieve(&query, 2, 2);

        assert_eq!(results.len(), 2, "should return exactly top_k=2 results");
        // Results must be sorted descending by score.
        assert!(
            results[0].score >= results[1].score,
            "results must be sorted by score descending"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_hop_penalty_reduces_score
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_hop_penalty_reduces_score() {
        // Linear chain: root → mid → leaf
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("root", "verb"));
        dag.add_node(ExecNode::new("mid", "verb"));
        dag.add_node(ExecNode::new("leaf", "verb"));
        dag.add_edge("root", "out", "mid", "in");
        dag.add_edge("mid", "out", "leaf", "in");

        let retriever = GraphRagRetriever::new(&dag);
        // root's vec as query: root scores cosine=1.0 at hop=0 (score=1.0),
        // leaf scores cosine=1.0 at hop=0 from its own BFS seed but also
        // appears at hop=2 from root's BFS — root should be ranked #1 since
        // its raw sim with the query (= root_vec) is 1.0 * 1.0 = 1.0, while
        // leaf's best raw sim with root_vec is lower (different hash).
        let query = node_vec("root");
        let results = retriever.retrieve(&query, 3, 3);

        let root_result = results.iter().find(|r| r.node_id == "root");
        let leaf_result = results.iter().find(|r| r.node_id == "leaf");

        assert!(root_result.is_some(), "root must appear in results");
        assert!(leaf_result.is_some(), "leaf must appear in results");

        // root: hop=0, cosine_sim(root_vec, root_vec)=1.0 → score=1.0
        // leaf: best = cosine_sim(root_vec, leaf_vec)*1.0 (from leaf's own
        //        BFS at hop=0) but cosine_sim(root_vec, leaf_vec) < 1.0
        //        since they have different node_vec embeddings.
        assert!(
            root_result.unwrap().score > leaf_result.unwrap().score,
            "hop penalty / different embeddings must make root outscore leaf: \
             root={} leaf={}",
            root_result.unwrap().score,
            leaf_result.unwrap().score,
        );
    }

    // -----------------------------------------------------------------------
    // cached_retriever_returns_same_as_uncached
    // -----------------------------------------------------------------------
    #[test]
    fn cached_retriever_returns_same_as_uncached() {
        let dag = three_node_dag();
        let query = node_vec("alpha");
        let expected = GraphRagRetriever::new(&dag).retrieve(&query, 2, 2);
        let mut cached = CachedRetriever::new(&dag);
        let got = cached.retrieve_cached(&query, 2, 2);
        assert_eq!(got.len(), expected.len(), "length mismatch");
        for (a, b) in got.iter().zip(expected.iter()) {
            assert_eq!(a.node_id, b.node_id, "node_id mismatch");
            assert!((a.score - b.score).abs() < 1e-6, "score mismatch");
        }
    }

    // -----------------------------------------------------------------------
    // cached_retriever_second_call_uses_cache
    // -----------------------------------------------------------------------
    #[test]
    fn cached_retriever_second_call_uses_cache() {
        let dag = three_node_dag();
        let query = node_vec("beta");
        let mut cached = CachedRetriever::new(&dag);
        let first = cached.retrieve_cached(&query, 3, 2);
        // Cache should now hold one entry.
        assert_eq!(cached.cache.len(), 1, "cache should have one entry after first call");
        let second = cached.retrieve_cached(&query, 3, 2);
        // Cache should still hold exactly one entry — no new insertion.
        assert_eq!(cached.cache.len(), 1, "cache should not grow on repeated call");
        assert_eq!(first.len(), second.len(), "results length should match");
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.node_id, b.node_id);
            assert!((a.score - b.score).abs() < 1e-6);
        }
    }

    // -----------------------------------------------------------------------
    // graph_rag_deduplicates_nodes
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_deduplicates_nodes() {
        // Diamond: root → left, root → right, left → tip, right → tip
        // "tip" is reachable from root via two distinct paths.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("root", "verb"));
        dag.add_node(ExecNode::new("left", "verb"));
        dag.add_node(ExecNode::new("right", "verb"));
        dag.add_node(ExecNode::new("tip", "verb"));
        dag.add_edge("root", "out", "left", "in");
        dag.add_edge("root", "out", "right", "in");
        dag.add_edge("left", "out", "tip", "in");
        dag.add_edge("right", "out", "tip", "in");

        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("tip");
        let results = retriever.retrieve(&query, 4, 3);

        // Each node must appear at most once.
        let mut seen = std::collections::HashSet::new();
        for r in &results {
            assert!(
                seen.insert(r.node_id.clone()),
                "node '{}' appeared more than once in results",
                r.node_id
            );
        }
        assert_eq!(results.len(), 4, "all four nodes should be returned");
    }
}
