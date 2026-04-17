use std::collections::{HashMap, VecDeque};
use crate::node_schema::NodeId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TopologyError {
    #[error("cycle detected among nodes: {participants:?}")]
    Cycle { participants: Vec<NodeId> },
}

pub struct Topology {
    pub edges: Vec<(NodeId, NodeId)>,
}

impl Topology {
    pub fn new() -> Self {
        Self { edges: Vec::new() }
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId) {
        self.edges.push((from, to));
    }

    /// Kahn's algorithm: returns nodes in topological order.
    /// `nodes` is the full set of node ids to consider.
    pub fn kahn_order(&self, nodes: &[NodeId]) -> Result<Vec<NodeId>, TopologyError> {
        let mut block_count: HashMap<NodeId, usize> = HashMap::new();
        let mut successors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        for &n in nodes {
            block_count.entry(n).or_insert(0);
            successors.entry(n).or_insert_with(Vec::new);
        }

        for &(from, to) in &self.edges {
            // Only consider edges where both endpoints are in nodes
            if block_count.contains_key(&from) && block_count.contains_key(&to) {
                *block_count.entry(to).or_insert(0) += 1;
                successors.entry(from).or_insert_with(Vec::new).push(to);
            }
        }

        let mut queue: VecDeque<NodeId> = block_count
            .iter()
            .filter(|(_, &cnt)| cnt == 0)
            .map(|(&id, _)| id)
            .collect();
        // Sort for determinism
        let mut sorted: Vec<NodeId> = queue.iter().copied().collect();
        sorted.sort_unstable();
        queue = sorted.into();

        let mut order = Vec::with_capacity(nodes.len());

        while let Some(n) = queue.pop_front() {
            order.push(n);
            if let Some(succs) = successors.get(&n) {
                let mut ready = Vec::new();
                for &s in succs {
                    let cnt = block_count.entry(s).or_insert(0);
                    *cnt -= 1;
                    if *cnt == 0 {
                        ready.push(s);
                    }
                }
                ready.sort_unstable();
                for r in ready {
                    queue.push_back(r);
                }
            }
        }

        if order.len() != nodes.len() {
            let participants: Vec<NodeId> = nodes
                .iter()
                .filter(|&&n| !order.contains(&n))
                .copied()
                .collect();
            return Err(TopologyError::Cycle { participants });
        }

        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_chain() {
        let mut t = Topology::new();
        t.add_edge(1, 2);
        t.add_edge(2, 3);
        let order = t.kahn_order(&[1, 2, 3]).unwrap();
        assert_eq!(order, vec![1, 2, 3]);
    }

    #[test]
    fn diamond() {
        // 1 -> 2, 1 -> 3, 2 -> 4, 3 -> 4
        let mut t = Topology::new();
        t.add_edge(1, 2);
        t.add_edge(1, 3);
        t.add_edge(2, 4);
        t.add_edge(3, 4);
        let order = t.kahn_order(&[1, 2, 3, 4]).unwrap();
        assert_eq!(order[0], 1);
        assert_eq!(*order.last().unwrap(), 4);
        assert!(order.contains(&2) && order.contains(&3));
    }

    #[test]
    fn fan_out() {
        let mut t = Topology::new();
        t.add_edge(1, 2);
        t.add_edge(1, 3);
        t.add_edge(1, 4);
        let order = t.kahn_order(&[1, 2, 3, 4]).unwrap();
        assert_eq!(order[0], 1);
        assert_eq!(order.len(), 4);
    }

    #[test]
    fn fan_in() {
        let mut t = Topology::new();
        t.add_edge(1, 4);
        t.add_edge(2, 4);
        t.add_edge(3, 4);
        let order = t.kahn_order(&[1, 2, 3, 4]).unwrap();
        assert_eq!(*order.last().unwrap(), 4);
    }

    #[test]
    fn cycle_detected() {
        let mut t = Topology::new();
        t.add_edge(1, 2);
        t.add_edge(2, 3);
        t.add_edge(3, 1);
        let result = t.kahn_order(&[1, 2, 3]);
        assert!(matches!(result, Err(TopologyError::Cycle { .. })));
    }

    #[test]
    fn disconnected_subgraphs() {
        let mut t = Topology::new();
        t.add_edge(1, 2);
        t.add_edge(3, 4);
        let order = t.kahn_order(&[1, 2, 3, 4]).unwrap();
        assert_eq!(order.len(), 4);
        // 1 before 2, 3 before 4
        let pos: HashMap<_, _> = order.iter().enumerate().map(|(i, &n)| (n, i)).collect();
        assert!(pos[&1] < pos[&2]);
        assert!(pos[&3] < pos[&4]);
    }
}
