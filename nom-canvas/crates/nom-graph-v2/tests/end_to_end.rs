//! End-to-end DAG execution tests against the public nom-graph-v2 API.

use nom_graph_v2::cache::{CacheBackend, ClassicCache, LruCache, NoCache};
use nom_graph_v2::cancel::InterruptFlag;
use nom_graph_v2::execution::Executor;
use nom_graph_v2::fingerprint::fingerprint_inputs;
use nom_graph_v2::node_schema::NodeSchema;
use nom_graph_v2::progress::NoProgress;
use nom_graph_v2::topology::Topology;
use std::collections::HashMap;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a 5-node diamond DAG:
///   1 → 2, 1 → 3, 2 → 4, 3 → 4, 4 → 5
fn build_diamond_dag() -> (Vec<NodeSchema>, Topology) {
    let nodes = vec![
        NodeSchema::new(1, "source").with_output("bytes"),
        NodeSchema::new(2, "transform_b")
            .with_input("bytes", "bytes")
            .with_output("bytes"),
        NodeSchema::new(3, "transform_d")
            .with_input("bytes", "bytes")
            .with_output("bytes"),
        NodeSchema::new(4, "merge_c")
            .with_input("left", "bytes")
            .with_input("right", "bytes")
            .with_output("bytes"),
        NodeSchema::new(5, "sink_e")
            .with_input("bytes", "bytes")
            .with_output("bytes"),
    ];
    let topo = Topology {
        edges: vec![(1, 2), (1, 3), (2, 4), (3, 4), (4, 5)],
    };
    (nodes, topo)
}

fn seed_inputs() -> HashMap<u64, Vec<(String, Vec<u8>)>> {
    let mut m = HashMap::new();
    m.insert(1u64, vec![("bytes".to_string(), b"seed".to_vec())]);
    m
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn kahn_order_is_consistent() {
    let (_, topo) = build_diamond_dag();
    let order = topo
        .kahn_order(&[1, 2, 3, 4, 5])
        .expect("kahn should succeed on a DAG");

    assert_eq!(order[0], 1, "source node must come first");

    let pos = |id: u64| order.iter().position(|&n| n == id).unwrap();
    assert!(pos(4) > pos(2) && pos(4) > pos(3), "merge must come after both branches");
    assert_eq!(*order.last().unwrap(), 5, "sink must come last");
}

#[test]
fn executor_runs_with_nocache() {
    let (nodes, topo) = build_diamond_dag();
    let order = topo.kahn_order(&[1, 2, 3, 4, 5]).unwrap();
    let mut exec = Executor::new(NoCache, NoProgress);
    for node in nodes {
        exec.register(node);
    }
    let result = exec.execute(&order, seed_inputs());
    assert!(result.is_ok(), "executor should succeed on a well-formed DAG");
    let outputs = result.unwrap();
    assert_eq!(outputs.len(), 5, "all 5 nodes must produce output");
}

#[test]
fn executor_cache_hit_identical_results() {
    let (nodes, topo) = build_diamond_dag();
    let order = topo.kahn_order(&[1, 2, 3, 4, 5]).unwrap();
    let mut exec = Executor::new(ClassicCache::new(), NoProgress);
    for node in nodes {
        exec.register(node);
    }
    let first = exec.execute(&order, seed_inputs()).unwrap();
    let second = exec.execute(&order, seed_inputs()).unwrap();
    assert_eq!(
        first.len(),
        second.len(),
        "both runs should produce the same number of outputs"
    );
    for id in 1u64..=5 {
        assert_eq!(
            first[&id], second[&id],
            "node {id} output must be identical on cache hit"
        );
    }
}

#[test]
fn fingerprint_changes_with_inputs() {
    let a = fingerprint_inputs(
        "source",
        &[("bytes".to_string(), "seed".to_string())],
        &[],
    );
    let b = fingerprint_inputs(
        "source",
        &[("bytes".to_string(), "seed".to_string())],
        &[],
    );
    let c = fingerprint_inputs(
        "source",
        &[("bytes".to_string(), "different".to_string())],
        &[],
    );
    assert_eq!(a, b, "same inputs must produce same fingerprint");
    assert_ne!(a, c, "different inputs must produce different fingerprints");
}

#[test]
fn interrupt_stops_execution() {
    let (nodes, topo) = build_diamond_dag();
    let order = topo.kahn_order(&[1, 2, 3, 4, 5]).unwrap();

    let flag = InterruptFlag::new();
    flag.trigger(); // pre-triggered before execution starts

    let mut exec = Executor::new(NoCache, NoProgress).with_interrupt(flag.0.clone());
    for node in nodes {
        exec.register(node);
    }
    let result = exec.execute(&order, seed_inputs());
    assert!(
        result.is_err(),
        "executor should return an error when interrupt flag is set"
    );
}

#[test]
fn lru_cache_evicts_at_capacity() {
    let mut cache = LruCache::new(2);
    cache.set(1, vec![1]);
    cache.set(2, vec![2]);
    cache.set(3, vec![3]); // should evict entry 1 (oldest/LRU)

    assert!(
        cache.get(1).is_none() || cache.get(2).is_none(),
        "LRU should have evicted at least one entry at capacity 2"
    );
    assert!(
        cache.get(3).is_some(),
        "most recent insert must stay in cache"
    );
}
