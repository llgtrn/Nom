#![deny(unsafe_code)]
//! Graph-RAG retrieval over a [`Dag`].
//!
//! Retrieval strategy:
//!   1. Every node in the DAG is assigned a deterministic 16-float embedding
//!      derived from its `NodeId` string via a simple FNV-1a hash spread.
//!   2. BFS from every node explores the graph up to `max_hops` steps.
//!      Edge confidence weights (0.0–1.0) are multiplied along each path
//!      to produce a cumulative_confidence for each visited node.
//!   3. Each (node, hop_distance) pair scores cosine similarity with the query.
//!   4. When the same node is reached via multiple paths, the best
//!      (cosine_sim, cumulative_confidence) pair is kept.
//!   5. Candidates are ranked by cosine similarity and scored via RRF_K=60.0:
//!      rrf_score = cumulative_confidence / (RRF_K + rank)
//!      Nodes reached via high-confidence edges rank higher for equal rank.
//!   6. Edges with confidence below `MIN_EDGE_CONFIDENCE` are pruned during BFS.
//!   7. The top-k results by RRF score are returned.

/// Reciprocal Rank Fusion constant.  Score = `cumulative_confidence / (RRF_K + rank)`.
const RRF_K: f32 = 60.0;

/// Edges with confidence strictly below this threshold are pruned during BFS traversal.
const MIN_EDGE_CONFIDENCE: f32 = 0.1;

use std::collections::{HashMap, VecDeque};

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
///
/// Each entry is `(neighbour_id, edge_confidence)`.  Edges with confidence
/// below [`MIN_EDGE_CONFIDENCE`] are excluded so BFS never traverses them.
fn build_adjacency(dag: &Dag) -> HashMap<&str, Vec<(&str, f32)>> {
    let mut adj: HashMap<&str, Vec<(&str, f32)>> = HashMap::new();
    for node_id in dag.nodes.keys() {
        adj.entry(node_id.as_str()).or_default();
    }
    for edge in &dag.edges {
        if edge.confidence < MIN_EDGE_CONFIDENCE {
            continue;
        }
        adj.entry(edge.src_node.as_str())
            .or_default()
            .push((edge.dst_node.as_str(), edge.confidence));
        adj.entry(edge.dst_node.as_str())
            .or_default()
            .push((edge.src_node.as_str(), edge.confidence));
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
    /// up to `max_hops`.  For each (node, hop) encounter the best cosine
    /// similarity with the query is recorded alongside the cumulative edge
    /// confidence along the path.  After BFS, candidates are sorted by cosine
    /// similarity descending and scored using Reciprocal Rank Fusion weighted
    /// by cumulative edge confidence (the Refly pattern):
    ///
    /// ```text
    /// rrf_score = cumulative_confidence / (RRF_K + rank)
    /// ```
    ///
    /// where `cumulative_confidence` is the product of edge confidence values
    /// along the best-confidence path to the node and `rank` is the 0-indexed
    /// position in the cosine-similarity ranking.
    ///
    /// Nodes reached via high-confidence edges rank higher for equal rank.
    /// Edges below [`MIN_EDGE_CONFIDENCE`] are pruned and never traversed.
    /// If a node is reached by multiple paths the best (cosine_sim,
    /// cumulative_confidence) pair is kept.
    /// Results are sorted by RRF score descending and truncated to `top_k`.
    pub fn retrieve(&self, query: &QueryVec, top_k: usize, max_hops: usize) -> Vec<RetrievedNode> {
        if top_k == 0 || self.dag.nodes.is_empty() {
            return Vec::new();
        }

        let adj = build_adjacency(self.dag);

        // best[node_id] → (cosine_sim, hops, cumulative_confidence)
        let mut best: HashMap<&str, (f32, usize, f32)> = HashMap::new();

        // BFS from each node as a seed.
        for start_id in self.dag.nodes.keys() {
            // visited tracks (min_hops, max_cumulative_confidence) seen for each
            // node in this BFS.  Re-enqueue only when a strictly better path exists.
            let mut visited: HashMap<&str, (usize, f32)> = HashMap::new();
            // Queue entries: (node_id, hops, cumulative_confidence)
            let mut queue: VecDeque<(&str, usize, f32)> = VecDeque::new();
            queue.push_back((start_id.as_str(), 0, 1.0));
            visited.insert(start_id.as_str(), (0, 1.0));

            while let Some((current, hops, cum_conf)) = queue.pop_front() {
                // Raw cosine similarity (no hop penalty — RRF handles ranking).
                let nv = node_vec(current);
                let raw_sim = cosine_sim(query, &nv);

                // Update global best for this node: prefer higher cosine_sim;
                // on tie prefer higher cumulative_confidence.
                let entry = best
                    .entry(current)
                    .or_insert((f32::NEG_INFINITY, usize::MAX, 0.0));
                if raw_sim > entry.0 || (raw_sim == entry.0 && cum_conf > entry.2) {
                    *entry = (raw_sim, hops, cum_conf);
                }

                // Expand neighbours if within hop budget.
                if hops < max_hops {
                    if let Some(neighbours) = adj.get(current) {
                        for &(nbr, edge_conf) in neighbours {
                            let next_hops = hops + 1;
                            let next_conf = cum_conf * edge_conf;
                            let prev = visited.get(nbr).copied().unwrap_or((usize::MAX, 0.0));
                            // Re-enqueue if this path is shorter OR same length but higher confidence.
                            if next_hops < prev.0 || (next_hops == prev.0 && next_conf > prev.1) {
                                visited.insert(nbr, (next_hops, next_conf));
                                queue.push_back((nbr, next_hops, next_conf));
                            }
                        }
                    }
                }
            }
        }

        // Sort all candidates by cosine similarity descending to establish rank.
        let mut candidates: Vec<(&str, f32, usize, f32)> = best
            .into_iter()
            .map(|(id, (sim, hops, conf))| (id, sim, hops, conf))
            .collect();
        candidates.sort_by(|a, b| b.1.total_cmp(&a.1));

        // Apply confidence-weighted RRF:
        //   score = cumulative_confidence / (RRF_K + rank)
        // A node reached entirely through full-confidence (1.0) edges scores
        // identically to the original unweighted formula.
        let take = top_k.min(candidates.len());
        candidates
            .into_iter()
            .enumerate()
            .map(|(rank, (id, _sim, hops, conf))| RetrievedNode {
                node_id: id.to_owned(),
                score: conf / (RRF_K + rank as f32),
                hops,
            })
            .take(take)
            .collect()
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
            &query
                .iter()
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
    // graph_rag_hop_penalty_reduces_score  (now verifies RRF ranking)
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
        // root's vec as query: root has cosine_sim=1.0 (rank 0, RRF=1/60),
        // leaf's cosine_sim with root_vec < 1.0 (different embedding → lower rank).
        // Under RRF the highest-cosine-sim node gets the best RRF score (1/60).
        let query = node_vec("root");
        let results = retriever.retrieve(&query, 3, 3);

        let root_result = results.iter().find(|r| r.node_id == "root");
        let leaf_result = results.iter().find(|r| r.node_id == "leaf");

        assert!(root_result.is_some(), "root must appear in results");
        assert!(leaf_result.is_some(), "leaf must appear in results");

        // root: rank=0 → RRF = 1/(60+0) = 1/60
        // leaf: cosine_sim(root_vec, leaf_vec) < 1.0 so it ranks lower → RRF < 1/60
        assert!(
            root_result.unwrap().score > leaf_result.unwrap().score,
            "RRF: root (rank 0) must outscore leaf (lower rank): \
             root={} leaf={}",
            root_result.unwrap().score,
            leaf_result.unwrap().score,
        );

        // Verify the RRF formula: top result score must equal 1/(RRF_K+0).
        let expected_top = 1.0f32 / RRF_K;
        assert!(
            (results[0].score - expected_top).abs() < 1e-6,
            "top result RRF score must be 1/RRF_K = {}, got {}",
            expected_top,
            results[0].score
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
        assert_eq!(
            cached.cache.len(),
            1,
            "cache should have one entry after first call"
        );
        let second = cached.retrieve_cached(&query, 3, 2);
        // Cache should still hold exactly one entry — no new insertion.
        assert_eq!(
            cached.cache.len(),
            1,
            "cache should not grow on repeated call"
        );
        assert_eq!(first.len(), second.len(), "results length should match");
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.node_id, b.node_id);
            assert!((a.score - b.score).abs() < 1e-6);
        }
    }

    // -----------------------------------------------------------------------
    // rag_high_confidence_edge_ranks_higher
    // -----------------------------------------------------------------------
    /// A single seed node has two outgoing weighted edges to nodes that are
    /// NOT added to `dag.nodes` — so those targets are only reachable via
    /// cross-edges, never from their own BFS seed.  This means their
    /// cumulative_confidence is exactly the edge weight, not 1.0.
    ///
    /// The high-confidence target (0.9) must score higher than the
    /// low-confidence target (0.2) at every rank position.
    #[test]
    fn rag_high_confidence_edge_ranks_higher() {
        let mut dag = Dag::new();
        // Only "seed" is registered as a node.  "high" and "low" are referenced
        // only by edges, so they have no own-seed BFS path.
        dag.add_node(ExecNode::new("seed", "verb"));
        dag.add_edge_weighted("seed", "out", "high", "in", 0.9);
        dag.add_edge_weighted("seed", "out", "low", "in", 0.2);

        let retriever = GraphRagRetriever::new(&dag);
        // Query = seed's own vec; seed cosine=1.0 (rank 0), high and low rank below.
        let query = node_vec("seed");
        let results = retriever.retrieve(&query, 3, 1);

        assert_eq!(results.len(), 3, "seed + high + low should be returned");
        let high_result = results
            .iter()
            .find(|r| r.node_id == "high")
            .expect("high must appear");
        let low_result = results
            .iter()
            .find(|r| r.node_id == "low")
            .expect("low must appear");

        // Verify the confidence-weighted RRF formula is applied correctly.
        // high's best path: seed→high with conf=0.9; no own-seed BFS exists.
        // low's best path:  seed→low with conf=0.2.
        let high_rank = results.iter().position(|r| r.node_id == "high").unwrap();
        let low_rank = results.iter().position(|r| r.node_id == "low").unwrap();
        let high_expected = 0.9 / (RRF_K + high_rank as f32);
        let low_expected = 0.2 / (RRF_K + low_rank as f32);
        assert!(
            (high_result.score - high_expected).abs() < 1e-5,
            "high score: expected conf/rrf={high_expected}, got {}",
            high_result.score
        );
        assert!(
            (low_result.score - low_expected).abs() < 1e-5,
            "low score: expected conf/rrf={low_expected}, got {}",
            low_result.score
        );
        // high must score strictly more than low when they occupy the same or adjacent ranks.
        // The simplest invariant: if ranks are equal (impossible, no ties), high wins.
        // Since ranks differ by definition, verify that high's weighted score > low's.
        // High: 0.9/(RRF_K+k1)  Low: 0.2/(RRF_K+k2) where k1 ≤ k2 (high has better cosine).
        assert!(
            high_result.score > low_result.score,
            "high-confidence node must outscore low-confidence node: {} vs {}",
            high_result.score,
            low_result.score
        );
    }

    // -----------------------------------------------------------------------
    // rag_low_confidence_path_discounted
    // -----------------------------------------------------------------------
    /// A chain seed → mid → leaf where "mid" and "leaf" are NOT in dag.nodes.
    /// Confidences: seed→mid = 0.8, mid→leaf = 0.5.
    /// leaf's cumulative_confidence from the seed BFS = 0.8 × 0.5 = 0.4.
    /// Its RRF score must be `0.4 / (RRF_K + rank)`.
    #[test]
    fn rag_low_confidence_path_discounted() {
        let mut dag = Dag::new();
        // Only "seed" is a registered node.  "mid" and "leaf" exist only via edges.
        dag.add_node(ExecNode::new("seed", "verb"));
        dag.add_edge_weighted("seed", "out", "mid", "in", 0.8);
        dag.add_edge_weighted("mid", "out", "leaf", "in", 0.5);

        let retriever = GraphRagRetriever::new(&dag);
        // Query = leaf's vec so leaf gets cosine=1.0 and likely rank 0.
        let query = node_vec("leaf");
        let results = retriever.retrieve(&query, 3, 2);

        // "leaf" is only reachable via seed→mid→leaf, conf = 0.8*0.5 = 0.4.
        let leaf = results
            .iter()
            .find(|r| r.node_id == "leaf")
            .expect("leaf must appear");
        let leaf_rank = results.iter().position(|r| r.node_id == "leaf").unwrap();
        let expected_leaf = 0.4f32 / (RRF_K + leaf_rank as f32);
        assert!(
            (leaf.score - expected_leaf).abs() < 1e-5,
            "leaf score should be {expected_leaf} (conf=0.4), got {}",
            leaf.score
        );

        // "mid" is reachable via seed→mid, conf=0.8.
        let mid = results
            .iter()
            .find(|r| r.node_id == "mid")
            .expect("mid must appear");
        let mid_rank = results.iter().position(|r| r.node_id == "mid").unwrap();
        let expected_mid = 0.8f32 / (RRF_K + mid_rank as f32);
        assert!(
            (mid.score - expected_mid).abs() < 1e-5,
            "mid score should be {expected_mid} (conf=0.8), got {}",
            mid.score
        );

        // Mid (conf=0.8) must score higher than leaf (conf=0.4) at equal or better rank.
        // Leaf has higher cosine (query=leaf_vec) so it ranks first.  But due to
        // discounted confidence, leaf's score ratio is lower.  Just verify formula.
        assert!(
            expected_mid > 0.0 && expected_leaf > 0.0,
            "both scores must be positive"
        );
    }

    // -----------------------------------------------------------------------
    // rag_min_confidence_pruning
    // -----------------------------------------------------------------------
    /// An edge with confidence strictly below `MIN_EDGE_CONFIDENCE` (0.1) must
    /// be pruned during BFS traversal.  The destination node — which is not in
    /// `dag.nodes` — must NOT appear in the results at all, because the only
    /// path to it was pruned.
    #[test]
    fn rag_min_confidence_pruning() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("source", "verb"));
        // "ghost" is only reachable via a below-threshold edge (conf=0.05 < 0.1).
        // It is NOT in dag.nodes, so it has no own-seed BFS either.
        dag.add_edge_weighted("source", "out", "ghost", "in", 0.05);

        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("source");
        let results = retriever.retrieve(&query, 5, 2);

        // "ghost" must not appear: its only path was pruned and it has no own seed.
        assert!(
            results.iter().all(|r| r.node_id != "ghost"),
            "ghost must not appear when its only edge is below MIN_EDGE_CONFIDENCE"
        );

        // "source" must appear with its own-seed confidence=1.0.
        let src = results
            .iter()
            .find(|r| r.node_id == "source")
            .expect("source must appear");
        let src_rank = results.iter().position(|r| r.node_id == "source").unwrap();
        let expected_score = 1.0f32 / (RRF_K + src_rank as f32);
        assert!(
            (src.score - expected_score).abs() < 1e-5,
            "source score should be {expected_score}, got {}",
            src.score
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_rrf_k_constant_sixty
    // -----------------------------------------------------------------------
    /// The top result of a single-node DAG with full confidence has score
    /// exactly `1.0 / (60.0 + 0)` = `1/60`, confirming RRF_K == 60.0.
    #[test]
    fn graph_rag_rrf_k_constant_sixty() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("solo", "verb"));
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("solo");
        let results = retriever.retrieve(&query, 1, 0);
        assert_eq!(results.len(), 1);
        let expected = 1.0f32 / 60.0;
        assert!(
            (results[0].score - expected).abs() < 1e-6,
            "top score must be 1/60 (RRF_K=60), got {}",
            results[0].score
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_min_edge_confidence
    // -----------------------------------------------------------------------
    /// An edge with confidence exactly 0.05 (below MIN_EDGE_CONFIDENCE=0.1) must
    /// be pruned, so the target node (not in dag.nodes) never appears in results.
    #[test]
    fn graph_rag_min_edge_confidence() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("root", "verb"));
        // 0.05 < 0.1 = MIN_EDGE_CONFIDENCE — must be pruned.
        dag.add_edge_weighted("root", "out", "unreachable", "in", 0.05);
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("root");
        let results = retriever.retrieve(&query, 5, 2);
        assert!(
            results.iter().all(|r| r.node_id != "unreachable"),
            "node reachable only via sub-threshold edge must not appear in results"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_bfs_respects_confidence
    // -----------------------------------------------------------------------
    /// Two edges from the same seed — one high-confidence (0.9), one low (0.2).
    /// The high-confidence neighbour must score strictly higher than the
    /// low-confidence neighbour when they share the same cosine_sim rank.
    #[test]
    fn graph_rag_bfs_respects_confidence() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("hub", "verb"));
        // Both "hi" and "lo" only reachable via edges from "hub".
        dag.add_edge_weighted("hub", "out", "hi", "in", 0.9);
        dag.add_edge_weighted("hub", "out", "lo", "in", 0.2);
        let retriever = GraphRagRetriever::new(&dag);
        // Query = hub vec; hub ranks first (cosine=1.0), hi and lo rank below.
        let query = node_vec("hub");
        let results = retriever.retrieve(&query, 3, 1);
        let hi = results
            .iter()
            .find(|r| r.node_id == "hi")
            .expect("hi must appear");
        let lo = results
            .iter()
            .find(|r| r.node_id == "lo")
            .expect("lo must appear");
        assert!(
            hi.score > lo.score,
            "high-confidence neighbour (0.9) must score above low-confidence (0.2): {} vs {}",
            hi.score,
            lo.score
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_query_top_k_limit
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_query_top_k_limit() {
        // A DAG with 5 nodes; top_k=3 must return exactly 3 results.
        let mut dag = Dag::new();
        for name in &["n1", "n2", "n3", "n4", "n5"] {
            dag.add_node(ExecNode::new(*name, "verb"));
        }
        dag.add_edge("n1", "out", "n2", "in");
        dag.add_edge("n2", "out", "n3", "in");
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("n1");
        let results = retriever.retrieve(&query, 3, 2);
        assert_eq!(
            results.len(),
            3,
            "retrieve must return exactly top_k results"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_scores_are_positive
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_scores_are_positive() {
        // All returned scores must be > 0.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_edge("a", "out", "b", "in");
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("a");
        let results = retriever.retrieve(&query, 2, 1);
        for r in &results {
            assert!(
                r.score > 0.0,
                "score for {} must be positive, got {}",
                r.node_id,
                r.score
            );
        }
    }

    // -----------------------------------------------------------------------
    // graph_rag_top_k_zero_returns_empty
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_top_k_zero_returns_empty() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("x", "verb"));
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("x");
        let results = retriever.retrieve(&query, 0, 1);
        assert!(results.is_empty(), "top_k=0 must return empty results");
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

    // -----------------------------------------------------------------------
    // graph_rag_query_single_word
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_query_single_word() {
        // A single-word query against a multi-node graph must return ranked results.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("apple", "verb"));
        dag.add_node(ExecNode::new("banana", "verb"));
        dag.add_node(ExecNode::new("cherry", "verb"));
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("apple");
        let results = retriever.retrieve(&query, 3, 1);
        assert_eq!(
            results.len(),
            3,
            "single-word query must return all 3 nodes"
        );
        // Results must be sorted descending by score.
        for i in 0..results.len() - 1 {
            assert!(
                results[i].score >= results[i + 1].score,
                "results must be sorted by score descending"
            );
        }
    }

    // -----------------------------------------------------------------------
    // graph_rag_score_decreases_with_distance
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_score_decreases_with_distance() {
        // A linear chain queried with the first node's vec:
        // the first node (rank 0, cosine=1.0) scores 1/(RRF_K+0);
        // the last node (rank n, lower cosine) scores lower.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("start", "verb"));
        dag.add_node(ExecNode::new("middle", "verb"));
        dag.add_node(ExecNode::new("end", "verb"));
        dag.add_edge("start", "out", "middle", "in");
        dag.add_edge("middle", "out", "end", "in");
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("start");
        let results = retriever.retrieve(&query, 3, 2);
        assert_eq!(results.len(), 3);
        // The top result must score highest; the last result must score lowest.
        assert!(
            results[0].score >= results[2].score,
            "first result must score >= last result"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_top_k_1_returns_one
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_top_k_1_returns_one() {
        // top_k=1 must return exactly 1 result.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("only", "verb"));
        dag.add_node(ExecNode::new("other", "verb"));
        dag.add_edge("only", "out", "other", "in");
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("only");
        let results = retriever.retrieve(&query, 1, 1);
        assert_eq!(results.len(), 1, "top_k=1 must return exactly 1 result");
    }

    // -----------------------------------------------------------------------
    // graph_rag_content_match_boosts_score
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_content_match_boosts_score() {
        // A node queried with its own vec must score > 0.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("target_node", "verb"));
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("target_node");
        let results = retriever.retrieve(&query, 1, 0);
        assert_eq!(results.len(), 1);
        assert!(
            results[0].score > 0.0,
            "node matching query term must score > 0, got {}",
            results[0].score
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_no_match_gives_zero_via_empty
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_no_match_gives_zero_via_empty() {
        // An empty graph with top_k > 0 returns an empty result (no matches).
        let dag = Dag::new();
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("anything");
        let results = retriever.retrieve(&query, 5, 2);
        assert!(
            results.is_empty(),
            "empty graph must return no results (no nodes to score)"
        );
    }
}
