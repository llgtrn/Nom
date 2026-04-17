#![deny(unsafe_code)]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Confidence score from 0.0 to 1.0.
pub type Confidence = f32;

/// A single ReAct step: observe evidence, reason about hypothesis.
#[derive(Debug, Clone)]
pub struct ReactStep {
    pub thought: String,
    pub action: String,
    pub observation: String,
    pub score: Confidence,
}

/// A hypothesis with its computed score and supporting evidence.
#[derive(Debug, Clone)]
pub struct ScoredHypothesis {
    pub hypothesis: String,
    pub score: f32,
    pub evidence_used: Vec<String>,
    pub step_count: usize,
}

/// A cancellation signal for interruptible ReAct chains.
#[derive(Clone)]
pub struct InterruptSignal {
    cancelled: Arc<AtomicBool>,
}

impl InterruptSignal {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for InterruptSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a scored ReAct loop for a given hypothesis and evidence slice.
/// Returns a confidence score in [0.0, 1.0] computed from evidence matching.
pub fn classify_with_react(hypothesis: &str, evidence: &[&str]) -> Confidence {
    if evidence.is_empty() {
        return 0.0;
    }

    let h_words: Vec<&str> = hypothesis.split_whitespace().collect();

    let total_score: f32 = evidence
        .iter()
        .enumerate()
        .map(|(i, ev)| {
            let ev_words: Vec<&str> = ev.split_whitespace().collect();
            let matching = h_words.iter().filter(|w| ev_words.contains(w)).count();
            let base = if h_words.is_empty() {
                0.0
            } else {
                matching as f32 / h_words.len() as f32
            };
            // Decay by position (earlier evidence counts more)
            let decay = 1.0 / (1.0 + i as f32 * 0.2);
            base * decay
        })
        .sum();

    (total_score / evidence.len() as f32).clamp(0.0, 1.0)
}

/// Run a full ReAct chain, returning each step.
pub fn react_chain(hypothesis: &str, evidence: &[&str], max_steps: usize) -> Vec<ReactStep> {
    let steps = max_steps.min(evidence.len());
    (0..steps)
        .map(|i| {
            let obs_evidence = &evidence[..=i];
            let score = classify_with_react(hypothesis, obs_evidence);
            ReactStep {
                thought: format!(
                    "checking evidence[{}] for hypothesis: {}",
                    i,
                    &hypothesis[..hypothesis.len().min(40)]
                ),
                action: format!("evaluate: {}", evidence[i]),
                observation: format!("partial confidence: {:.3}", score),
                score,
            }
        })
        .collect()
}

/// Score each hypothesis against the given evidence, returning results sorted
/// by score descending.
pub fn rank_hypotheses(hypotheses: &[&str], evidence: &[&str]) -> Vec<ScoredHypothesis> {
    let mut scored: Vec<ScoredHypothesis> = hypotheses
        .iter()
        .map(|h| {
            let score = classify_with_react(h, evidence);
            let steps = react_chain(h, evidence, evidence.len());
            ScoredHypothesis {
                hypothesis: h.to_string(),
                score,
                evidence_used: evidence.iter().map(|e| e.to_string()).collect(),
                step_count: steps.len(),
            }
        })
        .collect();
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

/// Return the highest-scored hypothesis, or None if the slice is empty.
pub fn best_hypothesis(hypotheses: &[&str], evidence: &[&str]) -> Option<ScoredHypothesis> {
    rank_hypotheses(hypotheses, evidence).into_iter().next()
}

/// Same as `react_chain` but checks the interrupt signal before each step.
/// Stops early if `signal.is_cancelled()` returns true.
pub fn react_chain_interruptible(
    hypothesis: &str,
    evidence: &[&str],
    max_steps: usize,
    signal: &InterruptSignal,
) -> Vec<ReactStep> {
    let steps = max_steps.min(evidence.len());
    let mut result = Vec::with_capacity(steps);
    for i in 0..steps {
        if signal.is_cancelled() {
            break;
        }
        let obs_evidence = &evidence[..=i];
        let score = classify_with_react(hypothesis, obs_evidence);
        result.push(ReactStep {
            thought: format!(
                "checking evidence[{}] for hypothesis: {}",
                i,
                &hypothesis[..hypothesis.len().min(40)]
            ),
            action: format!("evaluate: {}", evidence[i]),
            observation: format!("partial confidence: {:.3}", score),
            score,
        });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_empty_evidence_returns_zero() {
        assert_eq!(classify_with_react("anything", &[]), 0.0);
    }

    #[test]
    fn classify_matching_evidence_returns_positive() {
        let score = classify_with_react("graph node query", &["graph traversal query result"]);
        assert!(score > 0.0, "expected positive score, got {score}");
    }

    #[test]
    fn classify_no_overlap_returns_zero() {
        let score = classify_with_react("completely different", &["unrelated content here"]);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn react_chain_produces_steps() {
        let steps = react_chain("search query", &["search results found", "query matched"], 2);
        assert_eq!(steps.len(), 2);
        assert!(steps[0].score >= 0.0);
    }

    #[test]
    fn classify_clamps_to_one() {
        let score = classify_with_react("word", &["word word word", "word more word"]);
        assert!(score <= 1.0);
    }

    #[test]
    fn rank_hypotheses_orders_by_score() {
        let evidence = &["graph query node traversal"];
        let hypotheses = &["graph node query", "unrelated banana"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 2);
        assert!(ranked[0].score >= ranked[1].score);
        assert_eq!(ranked[0].hypothesis, "graph node query");
    }

    #[test]
    fn best_hypothesis_returns_highest() {
        let evidence = &["graph query node traversal"];
        let hypotheses = &["graph node query", "completely unrelated"];
        let best = best_hypothesis(hypotheses, evidence).expect("should have a result");
        assert_eq!(best.hypothesis, "graph node query");
        assert!(best.score > 0.0);
    }

    #[test]
    fn interrupt_signal_cancels_chain() {
        let signal = InterruptSignal::new();
        signal.cancel();
        let evidence = &["graph query", "node traversal", "result found"];
        let steps = react_chain_interruptible("graph node", evidence, 3, &signal);
        assert_eq!(steps.len(), 0, "cancelled before first step");
    }

    #[test]
    fn react_chain_interruptible_stops_at_max() {
        let signal = InterruptSignal::new(); // not cancelled
        let evidence = &["graph query", "node traversal", "result found"];
        let steps = react_chain_interruptible("graph node", evidence, 2, &signal);
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn react_chain_max_steps_respected() {
        let evidence = &["step one", "step two", "step three", "step four", "step five"];
        let steps = react_chain("hypothesis", evidence, 3);
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn ranked_hypotheses_scores_sum_to_positive() {
        let evidence = &["graph query node traversal result"];
        let hypotheses = &["graph node query", "traversal result"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        let total: f32 = ranked.iter().map(|h| h.score).sum();
        assert!(total > 0.0, "expected positive total score, got {total}");
    }

    #[test]
    fn interrupt_signal_default_is_not_cancelled() {
        let signal = InterruptSignal::default();
        assert!(!signal.is_cancelled());
    }

    #[test]
    fn scored_hypothesis_evidence_used_populated() {
        let evidence = &["alpha beta", "gamma delta"];
        let ranked = rank_hypotheses(&["alpha gamma"], evidence);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].evidence_used.len(), 2);
        assert_eq!(ranked[0].evidence_used[0], "alpha beta");
        assert_eq!(ranked[0].evidence_used[1], "gamma delta");
    }

    #[test]
    fn best_hypothesis_empty_returns_none() {
        let result = best_hypothesis(&[], &["some evidence"]);
        assert!(result.is_none());
    }

    #[test]
    fn scored_hypothesis_fields() {
        let evidence = &["graph query result"];
        let scored = rank_hypotheses(&["graph query"], evidence);
        assert_eq!(scored.len(), 1);
        assert_eq!(scored[0].hypothesis, "graph query");
        assert_eq!(scored[0].evidence_used.len(), 1);
        assert_eq!(scored[0].step_count, 1);
        assert!(scored[0].score > 0.0);
    }
}
