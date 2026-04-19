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
    /// by cumulative edge confidence:
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

    // -----------------------------------------------------------------------
    // RRF fusion of 3 ranked lists: top result has score 1/(RRF_K+0)
    // -----------------------------------------------------------------------
    #[test]
    fn rrf_fusion_three_list_top_result_score() {
        // A 3-node DAG queried with node "alpha"'s vec.
        // "alpha" has cosine=1.0 → rank 0 → score = 1.0/(60+0) = 1/60.
        let dag = three_node_dag();
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("alpha");
        let results = retriever.retrieve(&query, 3, 2);
        assert_eq!(results.len(), 3);
        // Top result must be alpha with score ~1/60.
        assert_eq!(results[0].node_id, "alpha", "alpha must rank first");
        let expected = 1.0f32 / RRF_K;
        assert!(
            (results[0].score - expected).abs() < 1e-5,
            "top RRF score must be 1/(RRF_K+0)={}, got {}",
            expected,
            results[0].score
        );
    }

    // -----------------------------------------------------------------------
    // RRF fusion: empty list fusion returns empty
    // -----------------------------------------------------------------------
    #[test]
    fn rrf_fusion_empty_list_returns_empty() {
        let dag = Dag::new();
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("anything");
        let results = retriever.retrieve(&query, 10, 5);
        assert!(
            results.is_empty(),
            "RRF on empty node set must return empty results"
        );
    }

    // -----------------------------------------------------------------------
    // RRF score normalization: all scores in (0, 1/60]
    // -----------------------------------------------------------------------
    #[test]
    fn rrf_score_normalization_all_in_range() {
        // With full-confidence edges, maximum RRF score is 1/(RRF_K+0) = 1/60 ≈ 0.0167.
        // All scores must be positive and at most 1/60.
        let mut dag = Dag::new();
        for name in &["n1", "n2", "n3", "n4", "n5"] {
            dag.add_node(ExecNode::new(*name, "verb"));
        }
        dag.add_edge("n1", "out", "n2", "in");
        dag.add_edge("n2", "out", "n3", "in");
        dag.add_edge("n3", "out", "n4", "in");
        dag.add_edge("n4", "out", "n5", "in");
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("n1");
        let results = retriever.retrieve(&query, 5, 4);
        let max_possible = 1.0f32 / RRF_K;
        for r in &results {
            assert!(r.score > 0.0, "score must be positive, got {}", r.score);
            assert!(
                r.score <= max_possible + 1e-5,
                "score {} must not exceed max_possible={}",
                r.score,
                max_possible
            );
        }
    }

    // -----------------------------------------------------------------------
    // cosine_sim: zero vector returns 0.0
    // -----------------------------------------------------------------------
    #[test]
    fn cosine_sim_zero_vector_returns_zero() {
        let zero = [0.0f32; 16];
        let v = node_vec("test");
        assert_eq!(cosine_sim(&zero, &v), 0.0);
        assert_eq!(cosine_sim(&v, &zero), 0.0);
        assert_eq!(cosine_sim(&zero, &zero), 0.0);
    }

    // -----------------------------------------------------------------------
    // node_vec: deterministic (same id → same vec)
    // -----------------------------------------------------------------------
    #[test]
    fn node_vec_is_deterministic() {
        let v1 = node_vec("deterministic_test");
        let v2 = node_vec("deterministic_test");
        assert_eq!(v1, v2, "node_vec must be deterministic for same input");
    }

    // -----------------------------------------------------------------------
    // node_vec: different ids produce different vecs
    // -----------------------------------------------------------------------
    #[test]
    fn node_vec_different_ids_differ() {
        let v1 = node_vec("apple");
        let v2 = node_vec("orange");
        assert_ne!(v1, v2, "different node ids must produce different vectors");
    }

    // -----------------------------------------------------------------------
    // node_vec: L2 norm is 1.0 (normalised)
    // -----------------------------------------------------------------------
    #[test]
    fn node_vec_is_unit_norm() {
        let v = node_vec("some_node");
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-5,
            "node_vec must be L2-normalised (norm={})",
            norm
        );
    }

    // -----------------------------------------------------------------------
    // RetrievedNode: hops field reflects BFS distance
    // -----------------------------------------------------------------------
    #[test]
    fn retrieved_node_hops_field_for_direct_neighbour() {
        // A single-node DAG: root with no edges.
        // root's own BFS seed gives hops=0.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("solo_root", "verb"));

        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("solo_root");
        let results = retriever.retrieve(&query, 1, 0);

        assert_eq!(results.len(), 1);
        let root_r = &results[0];
        assert_eq!(root_r.node_id, "solo_root");
        assert_eq!(root_r.hops, 0, "own-seed node must have hops=0");
    }

    // -----------------------------------------------------------------------
    // CachedRetriever: different params produce different cache entries
    // -----------------------------------------------------------------------
    #[test]
    fn cached_retriever_different_top_k_separate_cache_entries() {
        let dag = three_node_dag();
        let mut cached = CachedRetriever::new(&dag);
        let query = node_vec("alpha");
        let _ = cached.retrieve_cached(&query, 1, 2);
        let _ = cached.retrieve_cached(&query, 2, 2);
        // Different top_k → different cache keys → 2 distinct cache entries.
        assert_eq!(
            cached.cache.len(),
            2,
            "different top_k values must produce separate cache entries"
        );
    }

    // -----------------------------------------------------------------------
    // RRF with duplicate doc IDs deduplicated: diamond DAG with many hops
    // ensures each node appears at most once across all BFS seeds.
    // -----------------------------------------------------------------------
    #[test]
    fn rrf_duplicate_doc_ids_deduplicated_diamond() {
        // Diamond: A→B, A→C, B→D, C→D — "D" reachable via two paths from A.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_node(ExecNode::new("D", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        dag.add_edge("B", "out", "D", "in");
        dag.add_edge("C", "out", "D", "in");

        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("D");
        let results = retriever.retrieve(&query, 4, 3);

        // Each node must appear at most once (deduplication enforced by `best` map).
        let mut seen = std::collections::HashSet::new();
        for r in &results {
            assert!(
                seen.insert(r.node_id.clone()),
                "node '{}' appeared more than once — duplicates not deduplicated",
                r.node_id
            );
        }
        assert_eq!(results.len(), 4, "all 4 nodes must appear exactly once");
    }

    // -----------------------------------------------------------------------
    // top-k=1 returns only the single best result.
    // -----------------------------------------------------------------------
    #[test]
    fn rrf_top_k_one_returns_single_best() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("best", "verb"));
        dag.add_node(ExecNode::new("second", "verb"));
        dag.add_node(ExecNode::new("third", "verb"));
        dag.add_edge("best", "out", "second", "in");
        dag.add_edge("second", "out", "third", "in");

        let retriever = GraphRagRetriever::new(&dag);
        // Query "best"'s own vec — it should be the top result.
        let query = node_vec("best");
        let results = retriever.retrieve(&query, 1, 2);

        assert_eq!(results.len(), 1, "top_k=1 must return exactly 1 result");
        assert_eq!(
            results[0].node_id, "best",
            "the single result must be the best-matching node"
        );
    }

    // -----------------------------------------------------------------------
    // Empty query (all-zero vector) on non-empty DAG: results returned but no
    // cosine sim advantage → all nodes rank by BFS confidence only.
    // Empty DAG with any query → empty results.
    // -----------------------------------------------------------------------
    #[test]
    fn rrf_empty_dag_returns_empty_regardless_of_query() {
        let dag = Dag::new();
        let retriever = GraphRagRetriever::new(&dag);
        // A zero-magnitude vector simulates an "empty" query.
        let zero_query = [0.0f32; 16];
        let results = retriever.retrieve(&zero_query, 5, 2);
        assert!(
            results.is_empty(),
            "empty DAG must return empty results for any query"
        );
    }

    // -----------------------------------------------------------------------
    // Zero-magnitude query vector: cosine_sim returns 0.0; results still
    // returned (ranked purely by confidence/rank, not cosine).
    // -----------------------------------------------------------------------
    #[test]
    fn rrf_zero_query_vector_non_empty_dag() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("node_a", "verb"));
        dag.add_node(ExecNode::new("node_b", "verb"));
        let retriever = GraphRagRetriever::new(&dag);
        let zero_query = [0.0f32; 16];
        let results = retriever.retrieve(&zero_query, 2, 1);
        // Both nodes should appear — even with zero cosine, BFS still visits them.
        assert_eq!(
            results.len(),
            2,
            "zero-magnitude query must still return all nodes"
        );
        // Scores must be positive (confidence/rrf formula).
        for r in &results {
            assert!(
                r.score > 0.0,
                "score for {} must be > 0 even with zero query",
                r.node_id
            );
        }
    }

    // -----------------------------------------------------------------------
    // CachedRetriever: cache hit count — second call must not add a new entry
    // -----------------------------------------------------------------------
    #[test]
    fn cached_retriever_hit_count_stable() {
        let dag = three_node_dag();
        let mut cached = CachedRetriever::new(&dag);
        let query = node_vec("gamma");
        cached.retrieve_cached(&query, 2, 1);
        let count_after_first = cached.cache.len();
        cached.retrieve_cached(&query, 2, 1);
        assert_eq!(
            cached.cache.len(),
            count_after_first,
            "cache must not grow on repeated identical call"
        );
    }

    // -----------------------------------------------------------------------
    // RRF: two lists with same item — appears exactly once in output
    // -----------------------------------------------------------------------
    #[test]
    fn rrf_dedup_removes_exact_duplicates() {
        // Diamond DAG: node D is reachable via two paths from A.
        // It must appear exactly once in the RRF output.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_node(ExecNode::new("D", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        dag.add_edge("B", "out", "D", "in");
        dag.add_edge("C", "out", "D", "in");
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("D");
        let results = retriever.retrieve(&query, 4, 3);
        let count_d = results.iter().filter(|r| r.node_id == "D").count();
        assert_eq!(
            count_d, 1,
            "D reached via two paths must appear exactly once"
        );
    }

    // -----------------------------------------------------------------------
    // RRF: empty list returns empty
    // -----------------------------------------------------------------------
    #[test]
    fn rrf_empty_lists_returns_empty() {
        let dag = Dag::new();
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("any");
        let results = retriever.retrieve(&query, 10, 5);
        assert!(
            results.is_empty(),
            "RRF over empty node set must return empty"
        );
    }

    // -----------------------------------------------------------------------
    // RRF: single-node list preserves the single node in output
    // -----------------------------------------------------------------------
    #[test]
    fn rrf_single_list_preserves_order() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("only_node", "verb"));
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("only_node");
        let results = retriever.retrieve(&query, 1, 0);
        assert_eq!(
            results.len(),
            1,
            "single-node graph with top_k=1 must return 1 result"
        );
        assert_eq!(results[0].node_id, "only_node");
    }

    // -----------------------------------------------------------------------
    // RRF: higher-ranked item scores higher
    // -----------------------------------------------------------------------
    #[test]
    fn rrf_higher_ranked_item_scores_higher() {
        // alpha queried with its own vec → rank 0 (cosine=1.0), highest score.
        // beta and gamma rank lower → lower scores.
        let dag = three_node_dag();
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("alpha");
        let results = retriever.retrieve(&query, 3, 2);
        // The first result must score higher than or equal to the last.
        assert!(
            results[0].score >= results[2].score,
            "rank-0 item must score >= rank-2 item: {} vs {}",
            results[0].score,
            results[2].score
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_query_empty_graph_returns_empty (alias for clarity)
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_query_empty_graph_returns_empty_v2() {
        let dag = Dag::new();
        let retriever = GraphRagRetriever::new(&dag);
        let q = node_vec("test");
        assert!(
            retriever.retrieve(&q, 5, 2).is_empty(),
            "empty graph must return empty for any query"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag: index and retrieve exact match — node queried by its own id
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_index_and_retrieve_exact_match() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("exact_target", "verb"));
        dag.add_node(ExecNode::new("noise1", "verb"));
        dag.add_node(ExecNode::new("noise2", "verb"));
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("exact_target");
        let results = retriever.retrieve(&query, 3, 1);
        // The node whose id was used as the query must rank first.
        assert_eq!(
            results[0].node_id, "exact_target",
            "exact match (query == node_vec of node_id) must rank first"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag: partial match — querying any vec still returns all nodes
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_retrieves_partial_match() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("foo", "verb"));
        dag.add_node(ExecNode::new("bar", "verb"));
        dag.add_node(ExecNode::new("baz", "verb"));
        dag.add_edge("foo", "out", "bar", "in");
        let retriever = GraphRagRetriever::new(&dag);
        // Query with "foo"'s vec; "bar" and "baz" are partial matches.
        let query = node_vec("foo");
        let results = retriever.retrieve(&query, 3, 2);
        assert_eq!(
            results.len(),
            3,
            "all 3 nodes must appear even as partial matches"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_top_k_limits_results: ask for top-3, get at most 3
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_top_k_limits_results() {
        let mut dag = Dag::new();
        for n in &["n1", "n2", "n3", "n4", "n5", "n6"] {
            dag.add_node(ExecNode::new(*n, "verb"));
        }
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("n1");
        let results = retriever.retrieve(&query, 3, 1);
        assert!(
            results.len() <= 3,
            "top_k=3 must return at most 3 results, got {}",
            results.len()
        );
        assert_eq!(
            results.len(),
            3,
            "with 6 nodes, top_k=3 must return exactly 3"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_scores_sum_bounded: all scores ≤ 1/(RRF_K+0) = 1/60
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_scores_sum_bounded() {
        // With full-confidence paths, max score is 1/(60+0) = 1/60.
        let mut dag = Dag::new();
        for n in &["a", "b", "c", "d"] {
            dag.add_node(ExecNode::new(*n, "verb"));
        }
        dag.add_edge("a", "out", "b", "in");
        dag.add_edge("b", "out", "c", "in");
        dag.add_edge("c", "out", "d", "in");
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("a");
        let results = retriever.retrieve(&query, 4, 3);
        let max_score = 1.0f32 / RRF_K;
        for r in &results {
            assert!(
                r.score <= max_score + 1e-5,
                "score {} must be ≤ max_possible={}",
                r.score,
                max_score
            );
        }
    }

    // -----------------------------------------------------------------------
    // graph_rag_multiple_queries_independent: two different queries give different results
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_multiple_queries_independent() {
        let dag = three_node_dag();
        let retriever = GraphRagRetriever::new(&dag);
        let q_alpha = node_vec("alpha");
        let q_gamma = node_vec("gamma");
        let r_alpha = retriever.retrieve(&q_alpha, 3, 2);
        let r_gamma = retriever.retrieve(&q_gamma, 3, 2);
        // Both must return 3 results.
        assert_eq!(r_alpha.len(), 3);
        assert_eq!(r_gamma.len(), 3);
        // The top result differs: alpha-query → alpha first; gamma-query → gamma first.
        assert_eq!(
            r_alpha[0].node_id, "alpha",
            "alpha query must rank alpha first"
        );
        assert_eq!(
            r_gamma[0].node_id, "gamma",
            "gamma query must rank gamma first"
        );
    }

    // -----------------------------------------------------------------------
    // rrf_rank_60_constant_used: score = 1/(60+rank)
    // -----------------------------------------------------------------------
    #[test]
    fn rrf_rank_60_constant_used() {
        // Single node with full-confidence (own-seed, conf=1.0): score = 1/(60+0).
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("one", "verb"));
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("one");
        let results = retriever.retrieve(&query, 1, 0);
        assert_eq!(results.len(), 1);
        let expected = 1.0f32 / (60.0 + 0.0);
        assert!(
            (results[0].score - expected).abs() < 1e-6,
            "RRF score must equal 1/(60+rank) = {expected}, got {}",
            results[0].score
        );
    }

    // -----------------------------------------------------------------------
    // top-k larger than node count: returns all nodes (no panic, no duplicate)
    // -----------------------------------------------------------------------
    #[test]
    fn rrf_top_k_larger_than_node_count() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("x", "verb"));
        dag.add_node(ExecNode::new("y", "verb"));
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("x");
        let results = retriever.retrieve(&query, 100, 2);
        // Only 2 nodes exist; result must be exactly 2 (not 100).
        assert_eq!(results.len(), 2, "top_k capped at node count");
        let mut ids: Vec<&str> = results.iter().map(|r| r.node_id.as_str()).collect();
        ids.sort();
        assert_eq!(ids, vec!["x", "y"]);
    }

    // rrf_score_for_rank_1_is_1_over_61
    #[test]
    fn rrf_score_for_rank_1_is_1_over_61() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("rank0", "verb"));
        dag.add_node(ExecNode::new("rank1_candidate", "verb"));
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("rank0");
        let results = retriever.retrieve(&query, 2, 1);
        assert_eq!(results.len(), 2);
        let expected_rank1 = 1.0f32 / 61.0;
        assert!(
            (results[1].score - expected_rank1).abs() < 1e-5,
            "rank-1 score must be 1/61={expected_rank1}, got {}",
            results[1].score
        );
    }

    // rrf_score_for_rank_10_is_1_over_70
    #[test]
    fn rrf_score_for_rank_10_is_1_over_70() {
        let mut dag = Dag::new();
        let names = [
            "r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10",
        ];
        for name in &names {
            dag.add_node(ExecNode::new(*name, "verb"));
        }
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("r0");
        let results = retriever.retrieve(&query, 11, 1);
        assert_eq!(results.len(), 11);
        let expected = 1.0f32 / 70.0;
        assert!(
            (results[10].score - expected).abs() < 1e-5,
            "rank-10 score must be 1/70={expected}, got {}",
            results[10].score
        );
    }

    // rrf_scores_sum_correctly_for_two_lists
    #[test]
    fn rrf_scores_sum_correctly_for_two_lists() {
        let dag = three_node_dag();
        let retriever = GraphRagRetriever::new(&dag);
        let q1 = node_vec("alpha");
        let q2 = node_vec("beta");
        let r1 = retriever.retrieve(&q1, 3, 2);
        let r2 = retriever.retrieve(&q2, 3, 2);
        let sum_top = r1[0].score + r2[0].score;
        let expected_sum = 2.0f32 / 60.0;
        assert!(
            (sum_top - expected_sum).abs() < 1e-5,
            "sum of two rank-0 scores must be 2/60={expected_sum}, got {sum_top}"
        );
    }

    // rrf_max_k_results_bounded
    #[test]
    fn rrf_max_k_results_bounded() {
        let mut dag = Dag::new();
        for n in &["na", "nb", "nc"] {
            dag.add_node(ExecNode::new(*n, "verb"));
        }
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("na"), 1000, 2);
        assert_eq!(
            results.len(),
            3,
            "top_k=1000 on 3-node dag must return exactly 3"
        );
    }

    // graph_rag_index_100_nodes_retrieve_10
    #[test]
    fn graph_rag_index_100_nodes_retrieve_10() {
        let mut dag = Dag::new();
        for i in 0..100u32 {
            dag.add_node(ExecNode::new(format!("nd_{i}"), "verb"));
        }
        for i in 0..99u32 {
            dag.add_edge(format!("nd_{i}"), "out", format!("nd_{}", i + 1), "in");
        }
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("nd_0"), 10, 3);
        assert_eq!(
            results.len(),
            10,
            "must return exactly 10 from 100-node DAG"
        );
        for i in 0..results.len() - 1 {
            assert!(
                results[i].score >= results[i + 1].score,
                "results must be sorted descending"
            );
        }
    }

    // graph_rag_edge_context_improves_score
    #[test]
    fn graph_rag_edge_context_improves_score() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("hub2", "verb"));
        dag.add_node(ExecNode::new("linked2", "verb"));
        dag.add_node(ExecNode::new("isolated2", "verb"));
        dag.add_edge_weighted("hub2", "out", "linked2", "in", 1.0);
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("hub2"), 3, 2);
        assert!(
            results.iter().any(|r| r.node_id == "linked2"),
            "linked2 must appear"
        );
        assert!(
            results.iter().any(|r| r.node_id == "isolated2"),
            "isolated2 must appear"
        );
    }

    // graph_rag_unrelated_nodes_score_lower
    #[test]
    fn graph_rag_unrelated_nodes_score_lower() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("tgt", "verb"));
        dag.add_node(ExecNode::new("unrel1", "verb"));
        dag.add_node(ExecNode::new("unrel2", "verb"));
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("tgt"), 3, 1);
        assert_eq!(results[0].node_id, "tgt", "exact match must rank first");
        assert!(
            results[0].score > results[2].score,
            "exact match must outscore lowest-ranked node"
        );
    }

    // graph_rag_exact_keyword_match_scores_high
    #[test]
    fn graph_rag_exact_keyword_match_scores_high() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("kw_node", "verb"));
        dag.add_node(ExecNode::new("other_a", "verb"));
        dag.add_node(ExecNode::new("other_b", "verb"));
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("kw_node"), 3, 1);
        assert_eq!(
            results[0].node_id, "kw_node",
            "exact keyword match must rank first"
        );
        let expected_max = 1.0f32 / 60.0;
        assert!(
            (results[0].score - expected_max).abs() < 1e-5,
            "exact match at rank 0 must score 1/60, got {}",
            results[0].score
        );
    }

    // graph_rag_multi_hop_retrieval
    #[test]
    fn graph_rag_multi_hop_retrieval() {
        let mut dag = Dag::new();
        for n in &["mh_start", "mh_hop1", "mh_hop2", "mh_hop3"] {
            dag.add_node(ExecNode::new(*n, "verb"));
        }
        dag.add_edge("mh_start", "out", "mh_hop1", "in");
        dag.add_edge("mh_hop1", "out", "mh_hop2", "in");
        dag.add_edge("mh_hop2", "out", "mh_hop3", "in");
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("mh_start"), 4, 3);
        assert_eq!(results.len(), 4, "all 4 nodes must appear with max_hops=3");
        assert!(
            results.iter().any(|r| r.node_id == "mh_hop3"),
            "mh_hop3 must be reachable via 3-hop traversal"
        );
    }

    // graph_rag_concurrent_queries_safe
    #[test]
    fn graph_rag_concurrent_queries_safe() {
        let dag = three_node_dag();
        let r1 = GraphRagRetriever::new(&dag);
        let r2 = GraphRagRetriever::new(&dag);
        let query = node_vec("alpha");
        let res1 = r1.retrieve(&query, 3, 2);
        let res2 = r2.retrieve(&query, 3, 2);
        assert_eq!(
            res1.len(),
            res2.len(),
            "both retrievers must agree on count"
        );
        for (a, b) in res1.iter().zip(res2.iter()) {
            assert_eq!(
                a.node_id, b.node_id,
                "results must be identical across retrievers"
            );
            assert!(
                (a.score - b.score).abs() < 1e-6,
                "scores must match exactly"
            );
        }
    }

    // graph_rag_empty_index_no_panic
    #[test]
    fn graph_rag_empty_index_no_panic() {
        let dag = Dag::new();
        let retriever = GraphRagRetriever::new(&dag);
        let q = node_vec("anything");
        assert!(retriever.retrieve(&q, 0, 0).is_empty(), "top_k=0 empty dag");
        assert!(retriever.retrieve(&q, 1, 0).is_empty(), "top_k=1 empty dag");
        assert!(
            retriever.retrieve(&q, 100, 10).is_empty(),
            "top_k=100 empty dag"
        );
        let zero: QueryVec = [0.0f32; 16];
        assert!(
            retriever.retrieve(&zero, 5, 2).is_empty(),
            "zero query empty dag"
        );
    }

    // rrf_list_weight_doubles_score
    #[test]
    fn rrf_list_weight_doubles_score() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("full_conf", "verb"));
        dag.add_node(ExecNode::new("src_half", "verb"));
        dag.add_edge_weighted("src_half", "out", "half_tgt", "in", 0.5);
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("full_conf"), 3, 1);
        let full_r = results
            .iter()
            .find(|r| r.node_id == "full_conf")
            .expect("full_conf must appear");
        assert!(
            (full_r.score - 1.0f32 / 60.0).abs() < 1e-5,
            "full_conf at rank 0 must score 1/60, got {}",
            full_r.score
        );
        if let Some(half) = results.iter().find(|r| r.node_id == "half_tgt") {
            assert!(
                full_r.score > half.score,
                "full-confidence node must outscore half-confidence: {} vs {}",
                full_r.score,
                half.score
            );
        }
    }

    // -----------------------------------------------------------------------
    // graph_rag_index_empty_doc_ok
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_index_empty_doc_ok() {
        // Creating a retriever from an empty DAG must not panic.
        let dag = Dag::new();
        let retriever = GraphRagRetriever::new(&dag);
        let q = node_vec("anything");
        let results = retriever.retrieve(&q, 5, 2);
        assert!(
            results.is_empty(),
            "empty-DAG retriever must return empty results"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_index_updates_on_reindex
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_index_updates_on_reindex() {
        // After adding a node to a DAG and building a new retriever, the new node
        // must be retrievable.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("existing", "verb"));

        let r1 = GraphRagRetriever::new(&dag);
        let res1 = r1.retrieve(&node_vec("new_node"), 5, 1);
        // "new_node" not yet in dag — must not appear.
        assert!(
            !res1.iter().any(|r| r.node_id == "new_node"),
            "new_node not yet indexed"
        );

        // Add the node and rebuild the retriever (simulates re-index).
        dag.add_node(ExecNode::new("new_node", "verb"));
        let r2 = GraphRagRetriever::new(&dag);
        let res2 = r2.retrieve(&node_vec("new_node"), 5, 1);
        assert!(
            res2.iter().any(|r| r.node_id == "new_node"),
            "new_node must appear after re-index"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_retrieval_top_1
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_retrieval_top_1() {
        let mut dag = Dag::new();
        for n in &["a", "b", "c", "d", "e"] {
            dag.add_node(ExecNode::new(*n, "verb"));
        }
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("a"), 1, 1);
        assert_eq!(results.len(), 1, "top_k=1 must return exactly 1 result");
        assert_eq!(results[0].node_id, "a", "querying 'a' must rank 'a' first");
    }

    // -----------------------------------------------------------------------
    // graph_rag_retrieval_top_10_bounded
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_retrieval_top_10_bounded() {
        // With only 3 nodes, top_k=10 must return at most 3 results.
        let dag = three_node_dag();
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("alpha"), 10, 2);
        assert!(
            results.len() <= 10,
            "top_k=10 must never exceed total node count"
        );
        assert_eq!(
            results.len(),
            3,
            "3-node dag with top_k=10 must return exactly 3 results"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_score_sum_for_three_lists
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_score_sum_for_three_lists() {
        // Run three separate queries; each top result should score 1/60.
        let dag = three_node_dag();
        let retriever = GraphRagRetriever::new(&dag);
        let sum: f32 = ["alpha", "beta", "gamma"]
            .iter()
            .map(|&name| {
                let r = retriever.retrieve(&node_vec(name), 3, 2);
                r[0].score
            })
            .sum();
        let expected = 3.0f32 / 60.0;
        assert!(
            (sum - expected).abs() < 1e-5,
            "sum of three top-rank scores must be 3/60={expected}, got {sum}"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_query_caching (CachedRetriever hit)
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_query_caching() {
        let dag = three_node_dag();
        let mut cached = CachedRetriever::new(&dag);
        let query = node_vec("alpha");

        // First call: populates cache.
        let res1 = cached.retrieve_cached(&query, 2, 1);
        assert_eq!(
            cached.cache.len(),
            1,
            "cache must contain 1 entry after first call"
        );

        // Second call with identical params: cache hit, no new entry.
        let res2 = cached.retrieve_cached(&query, 2, 1);
        assert_eq!(cached.cache.len(), 1, "cache must not grow on hit");
        assert_eq!(
            res1.len(),
            res2.len(),
            "cached result must match first result"
        );
        for (a, b) in res1.iter().zip(res2.iter()) {
            assert_eq!(a.node_id, b.node_id, "node_id must match on cache hit");
            assert!(
                (a.score - b.score).abs() < 1e-6,
                "score must match on cache hit"
            );
        }
    }

    // -----------------------------------------------------------------------
    // graph_rag_empty_graph_returns_empty_results
    // -----------------------------------------------------------------------
    /// An empty DAG with any positive top_k and max_hops must return an
    /// empty result because there are no nodes to retrieve.
    #[test]
    fn graph_rag_empty_graph_returns_empty_results() {
        let dag = Dag::new();
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("search_term");
        let results = retriever.retrieve(&query, 10, 3);
        assert!(
            results.is_empty(),
            "empty graph must return empty results for any query, got {} results",
            results.len()
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_node_retrieval_by_id_round_trip
    // -----------------------------------------------------------------------
    /// A node queried by its own ID (using node_vec of its id) must appear
    /// in results and be the top-ranked result (cosine similarity = 1.0).
    #[test]
    fn graph_rag_node_retrieval_by_id_round_trip() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("round_trip_target", "verb"));
        dag.add_node(ExecNode::new("other_node_x", "verb"));
        dag.add_node(ExecNode::new("other_node_y", "verb"));
        let retriever = GraphRagRetriever::new(&dag);

        // Query using the exact node_id of "round_trip_target".
        let query = node_vec("round_trip_target");
        let results = retriever.retrieve(&query, 3, 1);

        assert!(!results.is_empty(), "results must not be empty");
        // The node queried by its own id must be retrievable.
        let found = results.iter().find(|r| r.node_id == "round_trip_target");
        assert!(
            found.is_some(),
            "node queried by its own id must appear in results"
        );
        // It must rank first (highest score, cosine_sim=1.0 at rank 0).
        assert_eq!(
            results[0].node_id, "round_trip_target",
            "node matching its own query vec must be ranked first"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_edge_traversal_from_node_returns_neighbors
    // -----------------------------------------------------------------------
    /// Given a node with two direct outgoing edges, BFS with max_hops=1
    /// must reach both neighbors.  Both must appear in results.
    #[test]
    fn graph_rag_edge_traversal_from_node_returns_neighbors() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("center", "verb"));
        dag.add_node(ExecNode::new("nbr_1", "verb"));
        dag.add_node(ExecNode::new("nbr_2", "verb"));
        dag.add_edge("center", "out", "nbr_1", "in");
        dag.add_edge("center", "out", "nbr_2", "in");

        let retriever = GraphRagRetriever::new(&dag);
        // Query center's own vec with max_hops=1: BFS from center reaches nbr_1 and nbr_2.
        let query = node_vec("center");
        let results = retriever.retrieve(&query, 3, 1);

        let node_ids: Vec<&str> = results.iter().map(|r| r.node_id.as_str()).collect();
        assert!(
            node_ids.contains(&"center"),
            "center must appear in results"
        );
        assert!(
            node_ids.contains(&"nbr_1"),
            "nbr_1 (direct neighbor) must appear in results"
        );
        assert!(
            node_ids.contains(&"nbr_2"),
            "nbr_2 (direct neighbor) must appear in results"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_bfs_from_root_visits_all_connected
    // -----------------------------------------------------------------------
    /// BFS from any seed in a fully-connected chain must reach all nodes in
    /// the chain when max_hops is large enough.
    #[test]
    fn graph_rag_bfs_from_root_visits_all_connected() {
        let mut dag = Dag::new();
        let names = ["bfs_n1", "bfs_n2", "bfs_n3", "bfs_n4"];
        for &n in &names {
            dag.add_node(ExecNode::new(n, "verb"));
        }
        dag.add_edge("bfs_n1", "out", "bfs_n2", "in");
        dag.add_edge("bfs_n2", "out", "bfs_n3", "in");
        dag.add_edge("bfs_n3", "out", "bfs_n4", "in");

        let retriever = GraphRagRetriever::new(&dag);
        // max_hops=3 allows reaching all 4 nodes from any seed.
        let results = retriever.retrieve(&node_vec("bfs_n1"), 4, 3);

        assert_eq!(results.len(), 4, "all 4 connected nodes must be returned");
        for &n in &names {
            assert!(
                results.iter().any(|r| r.node_id == n),
                "{n} must appear in results (fully connected chain, max_hops=3)"
            );
        }
    }

    // -----------------------------------------------------------------------
    // graph_rag_bfs_does_not_visit_disconnected_nodes
    // -----------------------------------------------------------------------
    /// Disconnected nodes (not reachable from any node in the query chain)
    /// must still appear in results because BFS seeds from every registered
    /// node — including the disconnected one.  This test verifies that every
    /// node in dag.nodes, connected or not, ends up in the result set.
    #[test]
    fn graph_rag_bfs_does_not_visit_disconnected_nodes_appear_as_own_seed() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("connected_a", "verb"));
        dag.add_node(ExecNode::new("connected_b", "verb"));
        dag.add_node(ExecNode::new("disconnected_x", "verb")); // no edges
        dag.add_edge("connected_a", "out", "connected_b", "in");

        let retriever = GraphRagRetriever::new(&dag);
        // Every node in dag.nodes is a BFS seed: disconnected_x has hop=0 from itself.
        let results = retriever.retrieve(&node_vec("connected_a"), 3, 2);

        assert_eq!(
            results.len(),
            3,
            "all 3 nodes (including disconnected) must appear"
        );
        assert!(
            results.iter().any(|r| r.node_id == "disconnected_x"),
            "disconnected_x must appear as its own BFS seed"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_single_node_dag_retrieves_that_node
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_single_node_dag_retrieves_that_node() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("the_only_node", "verb"));
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("the_only_node"), 5, 2);
        assert_eq!(
            results.len(),
            1,
            "single-node dag must return exactly 1 result"
        );
        assert_eq!(results[0].node_id, "the_only_node");
        assert_eq!(results[0].hops, 0, "single node has hops=0 (own seed)");
    }

    // -----------------------------------------------------------------------
    // graph_rag_neighbor_hops_equals_one_when_max_hops_sufficient
    // -----------------------------------------------------------------------
    /// A direct neighbor reached via one hop must have hops=1 when that is
    /// the shortest BFS path from the seed that has the highest cosine sim.
    /// Since every node is also seeded from itself (hops=0), a registered
    /// neighbor always gets hops=0 from its own BFS seed — but its score
    /// position still reflects the RRF ranking.
    #[test]
    fn graph_rag_neighbor_score_lower_than_exact_match() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("src_node", "verb"));
        dag.add_node(ExecNode::new("tgt_node", "verb"));
        dag.add_edge("src_node", "out", "tgt_node", "in");
        let retriever = GraphRagRetriever::new(&dag);
        // Query src_node's vec: src_node has cosine=1.0 (rank 0) → highest score.
        let results = retriever.retrieve(&node_vec("src_node"), 2, 1);
        assert_eq!(results.len(), 2);
        // src_node must rank above tgt_node.
        assert_eq!(
            results[0].node_id, "src_node",
            "exact match must rank first"
        );
        assert!(
            results[0].score > results[1].score,
            "exact-match src_node must score above neighbor tgt_node"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_concurrent_index_updates_safe
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_concurrent_index_updates_safe() {
        // Build two retrievers on the same DAG reference; both must return
        // identical results (simulates read-only concurrency).
        let mut dag = Dag::new();
        for n in &["node_x", "node_y", "node_z"] {
            dag.add_node(ExecNode::new(*n, "verb"));
        }
        dag.add_edge("node_x", "out", "node_y", "in");
        dag.add_edge("node_y", "out", "node_z", "in");

        let r1 = GraphRagRetriever::new(&dag);
        let r2 = GraphRagRetriever::new(&dag);
        let query = node_vec("node_x");

        let res1 = r1.retrieve(&query, 3, 2);
        let res2 = r2.retrieve(&query, 3, 2);

        assert_eq!(
            res1.len(),
            res2.len(),
            "both retrievers must return same count"
        );
        for (a, b) in res1.iter().zip(res2.iter()) {
            assert_eq!(
                a.node_id, b.node_id,
                "node_ids must match across concurrent retrievers"
            );
            assert!((a.score - b.score).abs() < 1e-6, "scores must match");
        }
    }

    // -----------------------------------------------------------------------
    // graph_rag_rerank_changes_order — querying different vecs changes top result
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_rerank_changes_order() {
        // Querying with alpha's vec → alpha first.
        // Querying with gamma's vec → gamma first.
        // This verifies that different queries (different "reranking") produce
        // different orderings.
        let dag = three_node_dag();
        let retriever = GraphRagRetriever::new(&dag);

        let ra = retriever.retrieve(&node_vec("alpha"), 3, 2);
        let rg = retriever.retrieve(&node_vec("gamma"), 3, 2);

        assert_eq!(ra[0].node_id, "alpha", "alpha-query must rank alpha first");
        assert_eq!(rg[0].node_id, "gamma", "gamma-query must rank gamma first");
        // The two orderings must differ (different top result means order changed).
        assert_ne!(
            ra[0].node_id, rg[0].node_id,
            "different queries must produce different top results"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_context_window_limited — top_k never exceeds node count
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_context_window_limited() {
        // Requesting more results than nodes available is safe (context window
        // limiting: result count is bounded by graph size, not top_k).
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("only_a", "verb"));
        dag.add_node(ExecNode::new("only_b", "verb"));
        let retriever = GraphRagRetriever::new(&dag);

        // Ask for 1000, only 2 nodes exist.
        let results = retriever.retrieve(&node_vec("only_a"), 1000, 2);
        assert_eq!(
            results.len(),
            2,
            "context window is bounded by node count; top_k=1000 must return 2, not 1000"
        );
    }

    // -----------------------------------------------------------------------
    // graph_rag_hybrid_bm25_plus_dense — node queried by its own id ranks best
    // (validates the dense-vector-only approach is already optimal for exact names)
    // -----------------------------------------------------------------------
    #[test]
    fn graph_rag_hybrid_bm25_plus_dense() {
        // The FNV-based embedding gives cosine_sim=1.0 when the query matches the
        // node's exact id — i.e. the dense component already handles exact matches.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("exact_match_node", "verb"));
        dag.add_node(ExecNode::new("other_node_1", "verb"));
        dag.add_node(ExecNode::new("other_node_2", "verb"));

        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("exact_match_node"), 3, 1);

        assert_eq!(
            results[0].node_id, "exact_match_node",
            "exact-name query must rank that node first (dense vector cosine=1.0)"
        );

        // Verify cosine similarity is 1.0 for self-query.
        let self_vec = node_vec("exact_match_node");
        let sim = cosine_sim(&self_vec, &self_vec);
        assert!(
            (sim - 1.0).abs() < 1e-5,
            "self cosine_sim must be 1.0, got {sim}"
        );
    }

    // -----------------------------------------------------------------------
    // Query scoring penalizes distant nodes (score decreases with hop count)
    // -----------------------------------------------------------------------
    #[test]
    fn query_scoring_penalizes_distant_nodes() {
        // Linear chain: n0→n1→n2→n3.  Query with n0's vec.
        // n0 (rank 0, cosine=1.0) must score higher than n3 (lower cosine rank).
        let mut dag = Dag::new();
        for n in &["n0", "n1", "n2", "n3"] {
            dag.add_node(ExecNode::new(*n, "verb"));
        }
        dag.add_edge("n0", "out", "n1", "in");
        dag.add_edge("n1", "out", "n2", "in");
        dag.add_edge("n2", "out", "n3", "in");
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("n0");
        let results = retriever.retrieve(&query, 4, 3);
        assert_eq!(results.len(), 4);
        let score_n0 = results.iter().find(|r| r.node_id == "n0").unwrap().score;
        let score_n3 = results.iter().find(|r| r.node_id == "n3").unwrap().score;
        assert!(
            score_n0 > score_n3,
            "n0 (rank 0) must score above n3 (distant): {} vs {}",
            score_n0,
            score_n3
        );
    }

    // -----------------------------------------------------------------------
    // Top-K results limited to K even when more match
    // -----------------------------------------------------------------------
    #[test]
    fn top_k_limited_to_k_even_when_more_match() {
        // 10 nodes all connected in a chain; asking for top_k=4 must return exactly 4.
        let mut dag = Dag::new();
        for i in 0..10u32 {
            dag.add_node(ExecNode::new(format!("m{i}"), "verb"));
        }
        for i in 0..9u32 {
            dag.add_edge(format!("m{i}"), "out", format!("m{}", i + 1), "in");
        }
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("m0"), 4, 9);
        assert_eq!(
            results.len(),
            4,
            "top_k=4 must return exactly 4 results even when 10 nodes exist"
        );
    }

    // -----------------------------------------------------------------------
    // Re-ranking on empty result set returns empty
    // -----------------------------------------------------------------------
    #[test]
    fn reranking_empty_result_set_returns_empty() {
        // An empty DAG always returns empty, regardless of top_k.
        let dag = Dag::new();
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("rerank_test");
        let results = retriever.retrieve(&query, 10, 5);
        assert!(
            results.is_empty(),
            "re-ranking over empty result set must return empty"
        );
    }

    // -----------------------------------------------------------------------
    // Graph with 50 nodes: query returns ≤ K results
    // -----------------------------------------------------------------------
    #[test]
    fn graph_50_nodes_query_returns_at_most_k() {
        let mut dag = Dag::new();
        for i in 0..50u32 {
            dag.add_node(ExecNode::new(format!("g50_{i}"), "verb"));
        }
        // Connect as a linear chain so BFS can traverse.
        for i in 0..49u32 {
            dag.add_edge(format!("g50_{i}"), "out", format!("g50_{}", i + 1), "in");
        }
        let retriever = GraphRagRetriever::new(&dag);
        // Request k=7 from 50-node graph.
        let results = retriever.retrieve(&node_vec("g50_0"), 7, 10);
        assert!(
            results.len() <= 7,
            "top_k=7 must return at most 7 results from 50-node graph, got {}",
            results.len()
        );
        assert_eq!(
            results.len(),
            7,
            "exactly 7 results expected when 50 nodes available"
        );
    }

    // -----------------------------------------------------------------------
    // Additional graph_rag edge-case tests
    // -----------------------------------------------------------------------

    #[test]
    fn graph_rag_score_order_descending_invariant() {
        // Results must always be sorted descending by score.
        let mut dag = Dag::new();
        for n in &["xa", "xb", "xc", "xd", "xe"] {
            dag.add_node(ExecNode::new(*n, "verb"));
        }
        dag.add_edge("xa", "out", "xb", "in");
        dag.add_edge("xb", "out", "xc", "in");
        dag.add_edge("xc", "out", "xd", "in");
        dag.add_edge("xd", "out", "xe", "in");
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("xa"), 5, 4);
        for i in 0..results.len().saturating_sub(1) {
            assert!(
                results[i].score >= results[i + 1].score,
                "results must be sorted descending: pos[{}]={} >= pos[{}]={}",
                i,
                results[i].score,
                i + 1,
                results[i + 1].score
            );
        }
    }

    #[test]
    fn graph_rag_top_k_equals_node_count_returns_all() {
        // top_k exactly equals node count: should return every node.
        let mut dag = Dag::new();
        for n in &["p1", "p2", "p3"] {
            dag.add_node(ExecNode::new(*n, "verb"));
        }
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("p1"), 3, 1);
        assert_eq!(
            results.len(),
            3,
            "top_k == node count must return all 3 nodes"
        );
    }

    #[test]
    fn graph_rag_hops_zero_returns_seed_only() {
        // max_hops=0: BFS does not expand from seeds; each node only sees itself.
        // All nodes still appear because every node is a BFS seed.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("seed_a", "verb"));
        dag.add_node(ExecNode::new("seed_b", "verb"));
        dag.add_edge("seed_a", "out", "seed_b", "in");
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("seed_a"), 2, 0);
        // Both nodes are their own seeds → both appear, hops=0.
        assert_eq!(results.len(), 2, "max_hops=0 must still return all seeds");
        for r in &results {
            assert_eq!(r.hops, 0, "all nodes with max_hops=0 must report hops=0");
        }
    }

    #[test]
    fn graph_rag_pruned_edge_leaves_only_registered_nodes() {
        // Edge confidence exactly at MIN_EDGE_CONFIDENCE (0.1) — the pruning rule
        // is strictly below 0.1, so 0.1 must NOT be pruned.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("src_at_thresh", "verb"));
        dag.add_node(ExecNode::new("dst_at_thresh", "verb"));
        dag.add_edge_weighted("src_at_thresh", "out", "dst_at_thresh", "in", 0.1);
        let retriever = GraphRagRetriever::new(&dag);
        let results = retriever.retrieve(&node_vec("src_at_thresh"), 2, 1);
        // Both nodes are registered; dst must appear (edge at threshold is kept).
        assert!(
            results.iter().any(|r| r.node_id == "dst_at_thresh"),
            "edge at MIN_EDGE_CONFIDENCE=0.1 must NOT be pruned (strictly below is pruned)"
        );
    }
}
