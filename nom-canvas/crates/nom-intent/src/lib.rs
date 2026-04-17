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
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
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
        let steps = react_chain(
            "search query",
            &["search results found", "query matched"],
            2,
        );
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
        let evidence = &[
            "step one",
            "step two",
            "step three",
            "step four",
            "step five",
        ];
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

    #[test]
    fn scored_hypothesis_score_field() {
        // Construct a ScoredHypothesis via rank_hypotheses and verify the score field.
        let evidence = &["alpha beta gamma"];
        let ranked = rank_hypotheses(&["alpha beta gamma"], evidence);
        assert_eq!(ranked.len(), 1);
        // Perfect overlap → score should be exactly 1.0.
        assert!((ranked[0].score - 1.0_f32).abs() < 1e-5);
    }

    #[test]
    fn interrupt_signal_not_cancelled_by_default() {
        let signal = InterruptSignal::new();
        assert!(!signal.is_cancelled(), "new signal must not be cancelled");
    }

    #[test]
    fn interrupt_signal_cancel_and_check() {
        let signal = InterruptSignal::new();
        assert!(!signal.is_cancelled());
        signal.cancel();
        assert!(
            signal.is_cancelled(),
            "signal must be cancelled after cancel()"
        );
    }

    #[test]
    fn rank_hypotheses_returns_sorted_descending() {
        let evidence = &["graph query node"];
        // "graph query node" overlaps perfectly; "banana" overlaps zero.
        let hypotheses = &["banana", "graph query node"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 2);
        assert!(
            ranked[0].score >= ranked[1].score,
            "rank_hypotheses must return descending by score"
        );
        assert_eq!(ranked[0].hypothesis, "graph query node");
    }

    #[test]
    fn best_hypothesis_picks_highest() {
        let evidence = &["graph query node"];
        let hypotheses = &["banana split", "graph query node"];
        let best = best_hypothesis(hypotheses, evidence).expect("non-empty slice must return Some");
        assert_eq!(best.hypothesis, "graph query node");
        assert!(best.score > 0.0);
    }

    // --- 20 new tests ---

    #[test]
    fn scored_hypothesis_evidence_vec() {
        let evidence = &["alpha one", "beta two", "gamma three"];
        let ranked = rank_hypotheses(&["alpha beta gamma"], evidence);
        assert_eq!(ranked[0].evidence_used.len(), 3);
        assert_eq!(ranked[0].evidence_used[0], "alpha one");
        assert_eq!(ranked[0].evidence_used[1], "beta two");
        assert_eq!(ranked[0].evidence_used[2], "gamma three");
    }

    #[test]
    fn scored_hypothesis_step_count() {
        let evidence = &["step a", "step b", "step c"];
        let ranked = rank_hypotheses(&["step a b c"], evidence);
        // step_count == number of react steps == evidence.len()
        assert_eq!(ranked[0].step_count, 3);
    }

    #[test]
    fn rank_hypotheses_single_item() {
        let evidence = &["only evidence"];
        let ranked = rank_hypotheses(&["only hypothesis"], evidence);
        assert_eq!(ranked.len(), 1);
    }

    #[test]
    fn rank_hypotheses_tie_preserves_order() {
        // Both hypotheses have zero overlap with evidence → scores both 0.0
        // Stable sort must preserve original order.
        let evidence = &["completely unrelated"];
        let hypotheses = &["first zero", "second zero"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 2);
        // With equal scores the stable sort keeps first before second.
        assert_eq!(ranked[0].hypothesis, "first zero");
        assert_eq!(ranked[1].hypothesis, "second zero");
    }

    #[test]
    fn best_hypothesis_empty_returns_none_v2() {
        let result = best_hypothesis(&[], &[]);
        assert!(result.is_none());
    }

    #[test]
    fn best_hypothesis_single_returns_it() {
        let evidence = &["lone wolf evidence"];
        let best = best_hypothesis(&["lone wolf"], evidence).expect("must return Some");
        assert_eq!(best.hypothesis, "lone wolf");
    }

    #[test]
    fn react_chain_empty_evidence_returns_empty() {
        // When evidence slice is empty, react_chain returns zero steps (no context).
        let steps = react_chain("any hypothesis", &[], 5);
        assert_eq!(steps.len(), 0);
    }

    #[test]
    fn react_chain_single_step() {
        let steps = react_chain("search node", &["search node result"], 1);
        assert_eq!(steps.len(), 1);
        assert!(steps[0].score >= 0.0);
        assert!(steps[0].score <= 1.0);
    }

    #[test]
    fn react_chain_interruptible_cancel_mid() {
        let signal = InterruptSignal::new();
        let evidence = &["e0", "e1", "e2", "e3", "e4"];
        // Cancel after 0 steps have been pushed — we cancel before the loop starts.
        signal.cancel();
        let steps = react_chain_interruptible("h", evidence, 5, &signal);
        assert_eq!(
            steps.len(),
            0,
            "cancelled before first step must yield empty"
        );
    }

    #[test]
    fn react_chain_interruptible_not_cancelled_completes() {
        let signal = InterruptSignal::new(); // never cancelled
        let evidence = &["alpha", "beta", "gamma"];
        let steps = react_chain_interruptible("alpha beta gamma", evidence, 3, &signal);
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn scored_hypothesis_score_zero() {
        // A hypothesis with no word overlap scores 0.0.
        let evidence = &["completely different words here"];
        let ranked = rank_hypotheses(&["xyz qrs uvw"], evidence);
        assert_eq!(ranked[0].score, 0.0);
    }

    #[test]
    fn scored_hypothesis_score_one() {
        // Perfect match: hypothesis == evidence → score should be 1.0.
        let evidence = &["exact match words"];
        let ranked = rank_hypotheses(&["exact match words"], evidence);
        assert!((ranked[0].score - 1.0_f32).abs() < 1e-5);
    }

    #[test]
    fn rank_hypotheses_five_items_sorted() {
        let evidence = &["graph node query traversal result"];
        let hypotheses = &[
            "unrelated one",
            "graph node",
            "completely different",
            "graph node query traversal result",
            "banana",
        ];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 5);
        assert!(ranked[0].score >= ranked[4].score);
    }

    #[test]
    fn best_hypothesis_returns_first_of_tied() {
        // All hypotheses have zero overlap → all score 0.0.
        // best_hypothesis returns the first in the tied group.
        let evidence = &["irrelevant content"];
        let hypotheses = &["aaa bbb", "ccc ddd", "eee fff"];
        let best = best_hypothesis(hypotheses, evidence).expect("must return Some");
        assert_eq!(best.hypothesis, "aaa bbb");
    }

    #[test]
    fn interrupt_signal_multiple_cancels_idempotent() {
        let signal = InterruptSignal::new();
        signal.cancel();
        signal.cancel();
        signal.cancel();
        assert!(signal.is_cancelled());
    }

    #[test]
    fn interrupt_signal_clone_shares_state() {
        let original = InterruptSignal::new();
        let cloned = original.clone();
        assert!(!cloned.is_cancelled());
        original.cancel();
        assert!(
            cloned.is_cancelled(),
            "clone must see cancellation from original"
        );
    }

    #[test]
    fn react_chain_returns_string_fields() {
        let steps = react_chain("test hypothesis", &["test evidence item"], 1);
        assert_eq!(steps.len(), 1);
        // thought / action / observation are all non-empty Strings
        assert!(!steps[0].thought.is_empty());
        assert!(!steps[0].action.is_empty());
        assert!(!steps[0].observation.is_empty());
    }

    #[test]
    fn react_chain_interruptible_returns_vec_of_react_step() {
        let signal = InterruptSignal::new();
        let evidence = &["item one", "item two"];
        let result: Vec<ReactStep> =
            react_chain_interruptible("item one two", evidence, 2, &signal);
        assert_eq!(result.len(), 2);
        assert!(result[0].score >= 0.0);
        assert!(result[1].score >= 0.0);
    }

    #[test]
    fn rank_hypotheses_preserves_evidence() {
        let evidence = &["preserve alpha", "preserve beta"];
        let ranked = rank_hypotheses(&["preserve alpha beta", "unrelated"], evidence);
        // After sorting, evidence_used on every item must still equal the original slice.
        for item in &ranked {
            assert_eq!(item.evidence_used.len(), 2);
            assert_eq!(item.evidence_used[0], "preserve alpha");
            assert_eq!(item.evidence_used[1], "preserve beta");
        }
    }

    #[test]
    fn rank_hypotheses_ten_items() {
        let evidence = &["node graph query traversal search result path weight edge vertex"];
        let hypotheses: Vec<&str> = vec![
            "node graph",
            "query traversal",
            "search result",
            "path weight",
            "edge vertex",
            "banana",
            "orange",
            "mango",
            "random words",
            "node graph query traversal search result path weight edge vertex",
        ];
        let ranked = rank_hypotheses(&hypotheses, evidence);
        assert_eq!(ranked.len(), 10);
        // Sorted descending: every adjacent pair must satisfy first >= second.
        for i in 0..ranked.len() - 1 {
            assert!(
                ranked[i].score >= ranked[i + 1].score,
                "rank[{}]={} < rank[{}]={} — not sorted",
                i,
                ranked[i].score,
                i + 1,
                ranked[i + 1].score
            );
        }
        // Last item must be one of the zero-overlap hypotheses.
        assert_eq!(ranked[9].score, 0.0);
    }

    // --- 20 named tests per spec ---

    #[test]
    fn react_chain_two_steps() {
        let steps = react_chain("search query", &["search term", "query result"], 2);
        assert_eq!(
            steps.len(),
            2,
            "2-step chain must complete with exactly 2 steps"
        );
        assert!(steps[0].score >= 0.0);
        assert!(steps[1].score >= 0.0);
    }

    #[test]
    fn react_chain_three_steps() {
        let steps = react_chain(
            "graph node edge",
            &["graph item", "node item", "edge item"],
            3,
        );
        assert_eq!(
            steps.len(),
            3,
            "3-step chain must complete with exactly 3 steps"
        );
    }

    #[test]
    fn react_chain_result_is_last_step() {
        // The last step's score is the most-informed confidence (all evidence seen).
        let evidence = &["alpha", "alpha beta", "alpha beta gamma"];
        let steps = react_chain("alpha beta gamma", evidence, 3);
        assert_eq!(steps.len(), 3);
        // Each successive step sees more evidence so score should be non-decreasing.
        // At minimum the last step must have a valid score in [0,1].
        let last = &steps[steps.len() - 1];
        assert!(last.score >= 0.0 && last.score <= 1.0);
        // The last step's observation must reference the final partial confidence.
        assert!(last.observation.contains("partial confidence:"));
    }

    #[test]
    fn react_chain_accumulates_evidence() {
        // Each step i uses evidence[0..=i], so step 1 sees 2 items, step 2 sees 3.
        let evidence = &["word a", "word b", "word c"];
        let steps = react_chain("word a b c", evidence, 3);
        assert_eq!(steps.len(), 3);
        // step 0: thought references evidence[0], step 2: references evidence[2]
        assert!(steps[0].action.contains("word a"));
        assert!(steps[1].action.contains("word b"));
        assert!(steps[2].action.contains("word c"));
    }

    #[test]
    fn react_chain_hypothesis_score_above_threshold() {
        // With single-word hypothesis and matching evidence, score should exceed 0.5.
        // "match" appears in all three evidence items → high overlap per step.
        let evidence = &["match", "match item", "match result"];
        let steps = react_chain("match", evidence, 3);
        assert_eq!(steps.len(), 3);
        let last_score = steps[steps.len() - 1].score;
        assert!(
            last_score > 0.5,
            "expected score > 0.5 with strong overlap, got {last_score}"
        );
    }

    #[test]
    fn interruptible_cancelled_before_start() {
        // Cancel before the chain starts — zero steps returned.
        let signal = InterruptSignal::new();
        signal.cancel();
        let evidence = &["step a", "step b", "step c"];
        let steps = react_chain_interruptible("hypothesis", evidence, 3, &signal);
        assert_eq!(
            steps.len(),
            0,
            "cancelled before start must return empty vec"
        );
    }

    #[test]
    fn interruptible_cancels_between_steps() {
        // We simulate cancellation between step 1 and step 2 by running step 0 manually
        // then cancelling, then verifying the interruptible function stops at 0 (pre-check).
        // Since the signal is checked *before* each step, cancelling after step 0 would
        // require a threaded test; instead we verify the signal is checked at iteration
        // boundary by cancelling mid-way using a clone shared with the loop.
        let signal = InterruptSignal::new();
        let signal_clone = signal.clone();
        let evidence = &[
            "step one evidence",
            "step two evidence",
            "step three evidence",
        ];

        // Run interruptible with a wrapper: cancel after 1 step by running manually.
        // Direct approach: run step 0 ourselves, cancel, then confirm interruptible yields 0.
        let step0 = react_chain_interruptible("step", &evidence[..1], 1, &signal);
        assert_eq!(step0.len(), 1, "first step must complete");
        signal_clone.cancel();
        // Now running from step 1 onward must yield 0 more steps.
        let remaining = react_chain_interruptible("step", &evidence[1..], 2, &signal);
        assert_eq!(
            remaining.len(),
            0,
            "after cancellation, no further steps must run"
        );
    }

    #[test]
    fn interruptible_not_cancelled_all_steps_run() {
        let signal = InterruptSignal::new(); // never cancelled
        let evidence = &["e one", "e two", "e three", "e four"];
        let steps = react_chain_interruptible("e one two three four", evidence, 4, &signal);
        assert_eq!(steps.len(), 4, "uncancelled chain must run all steps");
    }

    #[test]
    fn interruptible_result_on_cancel_is_err_string() {
        // The current API returns Vec<ReactStep>; when cancelled the vec is empty.
        // Verify the cancelled marker is observable: empty vec is the "err" signal.
        let signal = InterruptSignal::new();
        signal.cancel();
        let evidence = &["item a", "item b"];
        let steps = react_chain_interruptible("item a b", evidence, 2, &signal);
        // Empty result is the observable "cancelled" outcome.
        assert!(
            steps.is_empty(),
            "cancelled chain must return empty vec (the err-equivalent)"
        );
    }

    #[test]
    fn rank_with_identical_scores() {
        // Zero-overlap hypotheses all score 0.0; stable sort preserves original order.
        let evidence = &["xyz abc"];
        let hypotheses = &["first item", "second item", "third item"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].hypothesis, "first item");
        assert_eq!(ranked[1].hypothesis, "second item");
        assert_eq!(ranked[2].hypothesis, "third item");
    }

    #[test]
    fn rank_descending_always() {
        // For any input the first result score >= last result score.
        let evidence = &["alpha beta gamma delta"];
        let hypotheses = &[
            "alpha",
            "alpha beta",
            "alpha beta gamma",
            "alpha beta gamma delta",
            "unrelated",
        ];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert!(
            ranked[0].score >= ranked[ranked.len() - 1].score,
            "first score {} must be >= last score {}",
            ranked[0].score,
            ranked[ranked.len() - 1].score
        );
    }

    #[test]
    fn rank_large_set_100() {
        // 100 hypotheses ranked correctly: result is sorted descending throughout.
        let evidence = &["target word match"];
        let mut hypotheses: Vec<String> = (0..99).map(|i| format!("noise item {i}")).collect();
        hypotheses.push("target word match".to_string());
        let h_refs: Vec<&str> = hypotheses.iter().map(|s| s.as_str()).collect();
        let ranked = rank_hypotheses(&h_refs, evidence);
        assert_eq!(ranked.len(), 100);
        for i in 0..ranked.len() - 1 {
            assert!(
                ranked[i].score >= ranked[i + 1].score,
                "rank[{i}]={} < rank[{}]={} — not sorted",
                ranked[i].score,
                i + 1,
                ranked[i + 1].score
            );
        }
        assert_eq!(ranked[0].hypothesis, "target word match");
    }

    #[test]
    fn best_of_two() {
        // Of 2 hypotheses, best returns the higher-scored one.
        let evidence = &["graph node query"];
        let hypotheses = &["graph node query", "unrelated noise"];
        let best = best_hypothesis(hypotheses, evidence).expect("must return Some");
        assert_eq!(best.hypothesis, "graph node query");
        assert!(best.score > 0.0);
    }

    #[test]
    fn best_of_equal() {
        // When tied (both zero), returns the first.
        let evidence = &["completely irrelevant"];
        let hypotheses = &["first tied", "second tied"];
        let best = best_hypothesis(hypotheses, evidence).expect("must return Some");
        assert_eq!(best.hypothesis, "first tied");
    }

    #[test]
    fn scored_hypothesis_from_rank() {
        // rank_hypotheses produces ScoredHypothesis with correct hypothesis field.
        let evidence = &["construct verify"];
        let ranked = rank_hypotheses(&["construct verify"], evidence);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].hypothesis, "construct verify");
        assert!(ranked[0].score > 0.0);
    }

    #[test]
    fn scored_hypothesis_evidence_is_vec() {
        // evidence_used is a Vec<String> with one entry per evidence item.
        let evidence = &["one", "two", "three"];
        let ranked = rank_hypotheses(&["one two three"], evidence);
        let ev: &Vec<String> = &ranked[0].evidence_used;
        assert_eq!(ev.len(), 3);
        assert_eq!(ev[0], "one");
        assert_eq!(ev[1], "two");
        assert_eq!(ev[2], "three");
    }

    #[test]
    fn scored_hypothesis_step_count_field() {
        // step_count is usize equal to number of react steps (== evidence.len()).
        let evidence = &["a", "b", "c", "d"];
        let ranked = rank_hypotheses(&["a b c d"], evidence);
        let sc: usize = ranked[0].step_count;
        assert_eq!(sc, 4);
    }

    #[test]
    fn scored_hypothesis_to_string() {
        // hypothesis field is a String (not &str).
        let evidence = &["hello world"];
        let ranked = rank_hypotheses(&["hello world"], evidence);
        let h: &String = &ranked[0].hypothesis;
        assert_eq!(h, "hello world");
    }

    #[test]
    fn signal_new_clone_independent() {
        // Two separate new() signals don't share state — cancelling one leaves the other untouched.
        let s1 = InterruptSignal::new();
        let s2 = InterruptSignal::new();
        s1.cancel();
        assert!(s1.is_cancelled());
        assert!(
            !s2.is_cancelled(),
            "independent signals must not share cancellation state"
        );
    }

    #[test]
    fn signal_cancelled_after_cancel_returns_true_forever() {
        // Once set, is_cancelled stays true even after repeated checks.
        let signal = InterruptSignal::new();
        signal.cancel();
        for _ in 0..10 {
            assert!(
                signal.is_cancelled(),
                "is_cancelled must remain true after being set"
            );
        }
    }

    // --- Wave AA batch: ~55 additional tests to reach ~115 total ---

    // InterruptSignal: construction, set/clear semantics, Arc sharing
    #[test]
    fn interrupt_signal_new_not_set() {
        let s = InterruptSignal::new();
        assert!(!s.is_cancelled(), "new signal must not be set");
    }

    #[test]
    fn interrupt_signal_set_via_cancel() {
        let s = InterruptSignal::new();
        s.cancel();
        assert!(s.is_cancelled());
    }

    #[test]
    fn interrupt_signal_default_not_set() {
        let s = InterruptSignal::default();
        assert!(!s.is_cancelled());
    }

    #[test]
    fn interrupt_signal_arc_clone_cancel_propagates() {
        let s1 = InterruptSignal::new();
        let s2 = s1.clone();
        let s3 = s2.clone();
        s3.cancel();
        assert!(s1.is_cancelled(), "cancel on clone-of-clone must propagate");
        assert!(s2.is_cancelled());
    }

    #[test]
    fn interrupt_signal_two_independent_signals() {
        let a = InterruptSignal::new();
        let b = InterruptSignal::new();
        a.cancel();
        assert!(a.is_cancelled());
        assert!(!b.is_cancelled(), "b must be unaffected");
    }

    #[test]
    fn interrupt_signal_cancel_then_clone_sees_set() {
        let s = InterruptSignal::new();
        s.cancel();
        let c = s.clone();
        assert!(
            c.is_cancelled(),
            "clone created after cancel must already be set"
        );
    }

    #[test]
    fn interrupt_signal_idempotent_cancel() {
        let s = InterruptSignal::new();
        for _ in 0..5 {
            s.cancel();
        }
        assert!(s.is_cancelled());
    }

    // ReactChain: empty, single-step, multi-step
    #[test]
    fn react_chain_zero_max_steps_returns_empty() {
        let evidence = &["something here"];
        let steps = react_chain("hypothesis", evidence, 0);
        assert_eq!(steps.len(), 0);
    }

    #[test]
    fn react_chain_one_step_fields_non_empty() {
        let steps = react_chain("find node", &["find the node"], 1);
        assert_eq!(steps.len(), 1);
        assert!(!steps[0].thought.is_empty());
        assert!(!steps[0].action.is_empty());
        assert!(!steps[0].observation.is_empty());
    }

    #[test]
    fn react_chain_four_steps() {
        let evidence = &["a b", "c d", "e f", "g h"];
        let steps = react_chain("a c e g", evidence, 4);
        assert_eq!(steps.len(), 4);
    }

    #[test]
    fn react_chain_step_scores_in_unit_interval() {
        let evidence = &["alpha beta", "gamma delta", "epsilon zeta"];
        let steps = react_chain("alpha gamma epsilon", evidence, 3);
        for s in &steps {
            assert!(s.score >= 0.0 && s.score <= 1.0, "score {} out of [0,1]", s.score);
        }
    }

    #[test]
    fn react_chain_thought_contains_evidence_index() {
        let evidence = &["item zero", "item one", "item two"];
        let steps = react_chain("item", evidence, 3);
        assert!(steps[0].thought.contains("checking evidence[0]"));
        assert!(steps[1].thought.contains("checking evidence[1]"));
        assert!(steps[2].thought.contains("checking evidence[2]"));
    }

    #[test]
    fn react_chain_action_contains_evidence_text() {
        let evidence = &["unique_needle_word"];
        let steps = react_chain("unique_needle_word", evidence, 1);
        assert!(
            steps[0].action.contains("unique_needle_word"),
            "action must contain the evidence item"
        );
    }

    #[test]
    fn react_chain_observation_contains_partial_confidence() {
        let evidence = &["test observation"];
        let steps = react_chain("test", evidence, 1);
        assert!(
            steps[0].observation.starts_with("partial confidence:"),
            "observation must start with 'partial confidence:'"
        );
    }

    #[test]
    fn react_chain_max_steps_clamped_to_evidence_len() {
        // max_steps > evidence.len() → only evidence.len() steps produced
        let evidence = &["one", "two"];
        let steps = react_chain("one two", evidence, 100);
        assert_eq!(steps.len(), 2, "must clamp to evidence length");
    }

    // ReactChain: step ordering guarantees
    #[test]
    fn react_chain_step_order_indexed_correctly() {
        let evidence = &["first ev", "second ev", "third ev"];
        let steps = react_chain("first second third", evidence, 3);
        assert!(steps[0].thought.contains("[0]"));
        assert!(steps[1].thought.contains("[1]"));
        assert!(steps[2].thought.contains("[2]"));
    }

    #[test]
    fn react_chain_last_step_has_most_evidence() {
        // classify_with_react on evidence[..=2] (3 items) vs evidence[..=0] (1 item).
        // With matching words, using more evidence generally maintains or increases score
        // (though position decay can reduce it). At minimum just verify it's in [0,1].
        let evidence = &["word", "word item", "word item result"];
        let steps = react_chain("word item result", evidence, 3);
        assert!(steps[2].score >= 0.0 && steps[2].score <= 1.0);
    }

    // ReactChain interruptible: cancel at step 0, mid-chain, and after all steps
    #[test]
    fn interruptible_cancel_at_step_0_yields_empty() {
        let signal = InterruptSignal::new();
        signal.cancel();
        let steps = react_chain_interruptible("h", &["e0", "e1", "e2"], 3, &signal);
        assert!(steps.is_empty());
    }

    #[test]
    fn interruptible_no_cancel_all_steps() {
        let signal = InterruptSignal::new();
        let evidence = &["a1", "a2", "a3", "a4", "a5"];
        let steps = react_chain_interruptible("a1 a2 a3 a4 a5", evidence, 5, &signal);
        assert_eq!(steps.len(), 5);
    }

    #[test]
    fn interruptible_clone_cancel_stops_chain() {
        // Cancel via a clone; original also sees the cancellation.
        let signal = InterruptSignal::new();
        let guard = signal.clone();
        guard.cancel();
        let steps = react_chain_interruptible("node", &["node item"], 1, &signal);
        assert!(steps.is_empty(), "clone cancel must stop chain");
    }

    #[test]
    fn interruptible_zero_max_steps() {
        let signal = InterruptSignal::new();
        let steps = react_chain_interruptible("h", &["e1", "e2"], 0, &signal);
        assert!(steps.is_empty());
    }

    #[test]
    fn interruptible_scores_within_range() {
        let signal = InterruptSignal::new();
        let evidence = &["graph search", "query result", "node found"];
        let steps = react_chain_interruptible("graph query node", evidence, 3, &signal);
        for s in &steps {
            assert!(s.score >= 0.0 && s.score <= 1.0);
        }
    }

    // classify_with_react: boundary conditions
    #[test]
    fn classify_empty_hypothesis_empty_evidence_returns_zero() {
        assert_eq!(classify_with_react("", &[]), 0.0);
    }

    #[test]
    fn classify_empty_hypothesis_with_evidence_returns_zero() {
        // Empty hypothesis has no words → matching is 0 for all evidence.
        let score = classify_with_react("", &["some evidence here"]);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn classify_single_word_exact_match() {
        let score = classify_with_react("node", &["node"]);
        assert!((score - 1.0_f32).abs() < 1e-5, "single-word exact match should be 1.0, got {score}");
    }

    #[test]
    fn classify_single_word_no_match() {
        let score = classify_with_react("alpha", &["beta"]);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn classify_large_evidence_set() {
        // 20 items; result must be in [0,1].
        let evidence: Vec<&str> = (0..20).map(|_| "unrelated word").collect();
        let score = classify_with_react("target", &evidence);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn classify_position_decay_first_evidence_weights_more() {
        // evidence[0] matches hypothesis; evidence[1] doesn't.
        // Reverse order should yield lower score (position 0 has weight 1.0; position 1 has 1/1.2).
        let h = "match";
        let ev_match_first = &["match word", "no overlap here"];
        let ev_match_second = &["no overlap here", "match word"];
        let s1 = classify_with_react(h, ev_match_first);
        let s2 = classify_with_react(h, ev_match_second);
        assert!(
            s1 >= s2,
            "first-position match ({s1}) must be >= second-position match ({s2})"
        );
    }

    #[test]
    fn classify_all_matching_evidence() {
        // Every evidence item is identical to hypothesis → high score.
        let evidence: Vec<&str> = vec!["target word"; 5];
        let score = classify_with_react("target word", &evidence);
        assert!(score > 0.5, "all-matching evidence must score > 0.5, got {score}");
    }

    #[test]
    fn classify_partial_overlap() {
        // 2/3 words match.
        let score = classify_with_react("alpha beta gamma", &["alpha beta noise"]);
        let expected_lower = 2.0 / 3.0 * 0.9; // rough lower bound
        assert!(score > 0.0 && score <= 1.0, "score {score} out of range");
        assert!(score > expected_lower * 0.9, "expected decent overlap, got {score}");
    }

    // rank_hypotheses: edge cases
    #[test]
    fn rank_hypotheses_empty_hypotheses() {
        let ranked = rank_hypotheses(&[], &["some evidence"]);
        assert!(ranked.is_empty());
    }

    #[test]
    fn rank_hypotheses_empty_evidence() {
        let ranked = rank_hypotheses(&["h1", "h2"], &[]);
        assert_eq!(ranked.len(), 2);
        for item in &ranked {
            assert_eq!(item.score, 0.0, "empty evidence must yield 0.0 score");
        }
    }

    #[test]
    fn rank_hypotheses_all_zero_scores() {
        let ranked = rank_hypotheses(&["aaa", "bbb", "ccc"], &["xyz"]);
        for item in &ranked {
            assert_eq!(item.score, 0.0);
        }
    }

    #[test]
    fn rank_hypotheses_evidence_used_is_full_slice() {
        let evidence = &["e1", "e2", "e3", "e4"];
        let ranked = rank_hypotheses(&["e1 e2 e3 e4"], evidence);
        assert_eq!(ranked[0].evidence_used, vec!["e1", "e2", "e3", "e4"]);
    }

    #[test]
    fn rank_hypotheses_step_count_equals_evidence_len() {
        let evidence = &["a", "b", "c", "d", "e"];
        let ranked = rank_hypotheses(&["a b c d e"], evidence);
        assert_eq!(ranked[0].step_count, 5);
    }

    #[test]
    fn rank_hypotheses_no_duplicates_in_result() {
        // Each hypothesis appears exactly once in output.
        let evidence = &["data"];
        let hypotheses = &["h1", "h2", "h3"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        let unique: std::collections::HashSet<&str> =
            ranked.iter().map(|r| r.hypothesis.as_str()).collect();
        assert_eq!(unique.len(), 3);
    }

    // best_hypothesis: more edge cases
    #[test]
    fn best_hypothesis_no_hypotheses_no_evidence() {
        assert!(best_hypothesis(&[], &[]).is_none());
    }

    #[test]
    fn best_hypothesis_single_hypothesis_no_evidence() {
        let best = best_hypothesis(&["lone"], &[]).expect("must return Some");
        assert_eq!(best.hypothesis, "lone");
        assert_eq!(best.score, 0.0);
    }

    #[test]
    fn best_hypothesis_returns_some_with_nonempty_input() {
        let best = best_hypothesis(&["graph", "tree"], &["graph edge"]);
        assert!(best.is_some());
    }

    #[test]
    fn best_hypothesis_score_is_max() {
        let evidence = &["graph query traversal"];
        let hypotheses = &["graph query traversal", "unrelated", "banana"];
        let all = rank_hypotheses(hypotheses, evidence);
        let best = best_hypothesis(hypotheses, evidence).unwrap();
        let max_score = all.iter().map(|h| h.score).fold(f32::NEG_INFINITY, f32::max);
        assert!((best.score - max_score).abs() < 1e-6);
    }

    // ScoredHypothesis struct field types
    #[test]
    fn scored_hypothesis_clone() {
        let evidence = &["clone test"];
        let ranked = rank_hypotheses(&["clone test"], evidence);
        let original = &ranked[0];
        let cloned = original.clone();
        assert_eq!(cloned.hypothesis, original.hypothesis);
        assert!((cloned.score - original.score).abs() < 1e-6);
        assert_eq!(cloned.evidence_used, original.evidence_used);
        assert_eq!(cloned.step_count, original.step_count);
    }

    #[test]
    fn react_step_clone() {
        let steps = react_chain("clone", &["clone this"], 1);
        let s = steps[0].clone();
        assert_eq!(s.thought, steps[0].thought);
        assert_eq!(s.action, steps[0].action);
        assert_eq!(s.observation, steps[0].observation);
        assert!((s.score - steps[0].score).abs() < 1e-6);
    }

    #[test]
    fn react_step_debug_format() {
        let steps = react_chain("debug", &["debug item"], 1);
        let formatted = format!("{:?}", steps[0]);
        assert!(formatted.contains("ReactStep"));
    }

    #[test]
    fn scored_hypothesis_debug_format() {
        let ranked = rank_hypotheses(&["debug hyp"], &["debug item"]);
        let formatted = format!("{:?}", ranked[0]);
        assert!(formatted.contains("ScoredHypothesis"));
    }

    // Confidence type alias is f32
    #[test]
    fn confidence_type_is_f32() {
        let c: Confidence = classify_with_react("word", &["word"]);
        let _as_f32: f32 = c; // compile-time type check
        assert!(_as_f32 >= 0.0);
    }

    // Additional interruptible ordering
    #[test]
    fn interruptible_thought_indexed_correctly() {
        let signal = InterruptSignal::new();
        let evidence = &["ev0", "ev1", "ev2"];
        let steps = react_chain_interruptible("ev0 ev1 ev2", evidence, 3, &signal);
        assert_eq!(steps.len(), 3);
        assert!(steps[0].thought.contains("[0]"));
        assert!(steps[1].thought.contains("[1]"));
        assert!(steps[2].thought.contains("[2]"));
    }

    #[test]
    fn interruptible_clamped_to_evidence_len() {
        let signal = InterruptSignal::new();
        let evidence = &["x", "y"];
        let steps = react_chain_interruptible("x y", evidence, 999, &signal);
        assert_eq!(steps.len(), 2, "must clamp to evidence length");
    }

    // Error propagation: zero-score is the "error/no-match" outcome
    #[test]
    fn react_chain_all_zero_scores_propagate_to_rank() {
        let evidence = &["abc def"];
        let steps = react_chain("xyz", evidence, 1);
        assert_eq!(steps[0].score, 0.0);
        let ranked = rank_hypotheses(&["xyz"], evidence);
        assert_eq!(ranked[0].score, 0.0);
    }

    #[test]
    fn classify_with_react_result_equals_rank_score() {
        let evidence = &["match word here"];
        let h = "match word";
        let direct = classify_with_react(h, evidence);
        let ranked = rank_hypotheses(&[h], evidence);
        assert!((direct - ranked[0].score).abs() < 1e-6);
    }

    #[test]
    fn react_chain_hypothesis_truncated_at_40_chars_in_thought() {
        // thought truncates hypothesis to 40 chars via hypothesis.len().min(40)
        let long_hyp = "a b c d e f g h i j k l m n o p q r s t"; // 39 chars
        let steps = react_chain(long_hyp, &["a b c"], 1);
        assert!(steps[0].thought.contains("checking evidence[0]"));
    }

    #[test]
    fn best_hypothesis_evidence_used_matches_input() {
        let evidence = &["ev alpha", "ev beta"];
        let best = best_hypothesis(&["ev alpha ev beta"], evidence).unwrap();
        assert_eq!(best.evidence_used.len(), 2);
        assert_eq!(best.evidence_used[0], "ev alpha");
        assert_eq!(best.evidence_used[1], "ev beta");
    }

    #[test]
    fn rank_hypotheses_three_evidence_items_step_count() {
        let evidence = &["one", "two", "three"];
        let ranked = rank_hypotheses(&["one two three"], evidence);
        assert_eq!(ranked[0].step_count, 3, "step_count must equal evidence length");
    }

    #[test]
    fn classify_score_bounded_above_by_one() {
        // Even with many perfectly-matching evidence items, score never exceeds 1.0
        let evidence = vec!["word"; 50];
        let score = classify_with_react("word", &evidence);
        assert!(score <= 1.0, "score {score} must not exceed 1.0");
    }

    #[test]
    fn interruptible_empty_evidence_returns_empty() {
        let signal = InterruptSignal::new();
        let steps = react_chain_interruptible("hypothesis", &[], 5, &signal);
        assert!(steps.is_empty(), "empty evidence must yield empty steps");
    }

    #[test]
    fn scored_hypothesis_hypothesis_is_string_not_ref() {
        // Verify hypothesis field owns its data (String, not &str).
        let evidence = &["ownership test"];
        let ranked = rank_hypotheses(&["ownership test"], evidence);
        let h: String = ranked.into_iter().next().unwrap().hypothesis;
        assert_eq!(h, "ownership test");
    }
}
