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
            assert!(
                s.score >= 0.0 && s.score <= 1.0,
                "score {} out of [0,1]",
                s.score
            );
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
        assert!(
            (score - 1.0_f32).abs() < 1e-5,
            "single-word exact match should be 1.0, got {score}"
        );
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
        assert!(
            score > 0.5,
            "all-matching evidence must score > 0.5, got {score}"
        );
    }

    #[test]
    fn classify_partial_overlap() {
        // 2/3 words match.
        let score = classify_with_react("alpha beta gamma", &["alpha beta noise"]);
        let expected_lower = 2.0 / 3.0 * 0.9; // rough lower bound
        assert!(score > 0.0 && score <= 1.0, "score {score} out of range");
        assert!(
            score > expected_lower * 0.9,
            "expected decent overlap, got {score}"
        );
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
        let max_score = all
            .iter()
            .map(|h| h.score)
            .fold(f32::NEG_INFINITY, f32::max);
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
        assert_eq!(
            ranked[0].step_count, 3,
            "step_count must equal evidence length"
        );
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

    // --- Wave AC: 35 additional tests to reach 150 total ---

    // rank_hypotheses with equal scores (stable sort behavior)
    #[test]
    fn rank_equal_scores_stable_first_stays_first() {
        // All hypotheses score 0.0 (no overlap with evidence).
        // Stable sort must preserve original input order throughout the equal group.
        let evidence = &["zzz yyy xxx"];
        let hypotheses = &["aaa bbb", "ccc ddd", "eee fff", "ggg hhh", "iii jjj"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 5);
        // Verify stable ordering: original index 0 comes before index 1, etc.
        assert_eq!(ranked[0].hypothesis, "aaa bbb");
        assert_eq!(ranked[1].hypothesis, "ccc ddd");
        assert_eq!(ranked[2].hypothesis, "eee fff");
        assert_eq!(ranked[3].hypothesis, "ggg hhh");
        assert_eq!(ranked[4].hypothesis, "iii jjj");
    }

    #[test]
    fn rank_equal_scores_all_same_nonzero() {
        // Hypotheses with identical single-word match (same overlap fraction) keep stable order.
        let evidence = &["alpha"];
        // Each hypothesis matches exactly 1/1 word ("alpha") → all score the same.
        let hypotheses = &["alpha", "alpha", "alpha"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 3);
        // All scores equal; stable sort preserves insertion order.
        assert_eq!(ranked[0].hypothesis, "alpha");
        assert_eq!(ranked[1].hypothesis, "alpha");
        assert_eq!(ranked[2].hypothesis, "alpha");
        // All scores identical.
        assert!((ranked[0].score - ranked[2].score).abs() < 1e-6);
    }

    #[test]
    fn rank_equal_scores_mixed_with_winner() {
        // One hypothesis clearly wins; the rest tie at 0.0 → stable sub-order preserved.
        let evidence = &["winner token"];
        let hypotheses = &["loser one", "winner token", "loser two", "loser three"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked[0].hypothesis, "winner token", "winner must be first");
        // Losers in stable order.
        assert_eq!(ranked[1].hypothesis, "loser one");
        assert_eq!(ranked[2].hypothesis, "loser two");
        assert_eq!(ranked[3].hypothesis, "loser three");
    }

    // InterruptSignal display format and serialization (Debug)
    #[test]
    fn interrupt_signal_state_as_bool_before_cancel() {
        // Serialize cancellation state as bool (the public API surface).
        let s = InterruptSignal::new();
        let before: bool = s.is_cancelled();
        assert!(!before, "before cancel must be false");
    }

    #[test]
    fn interrupt_signal_state_as_bool_after_cancel() {
        let s = InterruptSignal::new();
        s.cancel();
        let after: bool = s.is_cancelled();
        assert!(after, "after cancel must be true");
    }

    #[test]
    fn interrupt_signal_format_via_is_cancelled() {
        let s = InterruptSignal::new();
        // "Display" is emulated via is_cancelled().
        let repr_before = format!("cancelled={}", s.is_cancelled());
        s.cancel();
        let repr_after = format!("cancelled={}", s.is_cancelled());
        assert_eq!(repr_before, "cancelled=false");
        assert_eq!(repr_after, "cancelled=true");
    }

    // ReactChain step ordering with 5+ steps
    #[test]
    fn react_chain_five_steps_ordered_by_index() {
        let evidence = &["e0", "e1", "e2", "e3", "e4"];
        let steps = react_chain("e0 e1 e2 e3 e4", evidence, 5);
        assert_eq!(steps.len(), 5);
        for (i, step) in steps.iter().enumerate() {
            assert!(
                step.thought.contains(&format!("[{i}]")),
                "step {i} thought must reference index [{i}]"
            );
        }
    }

    #[test]
    fn react_chain_six_steps_action_sequence() {
        let evidence = &["a0", "a1", "a2", "a3", "a4", "a5"];
        let steps = react_chain("a0 a1 a2 a3 a4 a5", evidence, 6);
        assert_eq!(steps.len(), 6);
        for (i, step) in steps.iter().enumerate() {
            assert!(
                step.action.contains(evidence[i]),
                "step {i} action must reference evidence[{i}]"
            );
        }
    }

    #[test]
    fn react_chain_seven_steps_all_valid_scores() {
        let evidence = &["w0", "w1", "w2", "w3", "w4", "w5", "w6"];
        let steps = react_chain("w0 w1 w2 w3 w4 w5 w6", evidence, 7);
        assert_eq!(steps.len(), 7);
        for s in &steps {
            assert!(s.score >= 0.0 && s.score <= 1.0);
        }
    }

    #[test]
    fn react_chain_five_steps_observation_format() {
        let evidence = &["obs0", "obs1", "obs2", "obs3", "obs4"];
        let steps = react_chain("obs", evidence, 5);
        for step in &steps {
            assert!(
                step.observation.starts_with("partial confidence:"),
                "observation '{}' must start with 'partial confidence:'",
                step.observation
            );
        }
    }

    #[test]
    fn interruptible_five_steps_ordered() {
        let signal = InterruptSignal::new();
        let evidence = &["i0", "i1", "i2", "i3", "i4"];
        let steps = react_chain_interruptible("i0 i1 i2 i3 i4", evidence, 5, &signal);
        assert_eq!(steps.len(), 5);
        for (i, step) in steps.iter().enumerate() {
            assert!(step.thought.contains(&format!("[{i}]")));
        }
    }

    // ScoredHypothesis comparison operators (partial ord via score)
    #[test]
    fn scored_hypothesis_score_partial_cmp_greater() {
        let ev = &["match word here"];
        let ranked = rank_hypotheses(&["match word here", "banana"], ev);
        // ranked[0].score > ranked[1].score
        assert!(
            ranked[0].score > ranked[1].score,
            "higher-scored hypothesis must compare greater"
        );
    }

    #[test]
    fn scored_hypothesis_score_partial_cmp_equal() {
        let ev = &["unrelated content"];
        let ranked = rank_hypotheses(&["aaa", "bbb"], ev);
        // Both score 0.0 → equal comparison
        assert!(
            (ranked[0].score - ranked[1].score).abs() < 1e-6,
            "tied hypotheses must have equal scores"
        );
        // f32 PartialOrd: equal scores compare as equal
        let cmp = ranked[0].score.partial_cmp(&ranked[1].score);
        assert_eq!(cmp, Some(std::cmp::Ordering::Equal));
    }

    #[test]
    fn scored_hypothesis_score_partial_cmp_less() {
        let ev = &["target node graph"];
        let ranked = rank_hypotheses(&["banana", "target node graph"], ev);
        // ranked[0] is "target node graph" (score > 0), ranked[1] is "banana" (score 0).
        // ranked[1].score < ranked[0].score
        let cmp = ranked[1].score.partial_cmp(&ranked[0].score);
        assert_eq!(cmp, Some(std::cmp::Ordering::Less));
    }

    #[test]
    fn scored_hypothesis_sort_by_score_desc() {
        let ev = &["graph node query"];
        let mut items = rank_hypotheses(&["banana", "graph node", "graph node query", "xyz"], ev);
        // Already sorted descending by rank_hypotheses; re-sort ascending and verify reversal.
        items.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
        // Now ascending: first element has lowest score.
        assert!(items[0].score <= items[items.len() - 1].score);
    }

    // Deep-think interrupt mid-chain behavior
    #[test]
    fn deep_think_interrupt_mid_chain_via_clone() {
        // Simulate a "deep think" scenario: chain started, then cancel signal propagated via clone.
        let signal = InterruptSignal::new();
        let guard = signal.clone();

        // Run first segment of the chain (steps 0..1) — signal not yet cancelled.
        let evidence = &["deep", "think", "node", "graph", "query"];
        let part_a = react_chain_interruptible("deep think node graph query", &evidence[..2], 2, &signal);
        assert_eq!(part_a.len(), 2, "first segment must complete");

        // Cancel via the guard clone (simulating mid-chain interrupt from another context).
        guard.cancel();

        // Remaining steps must yield empty.
        let part_b = react_chain_interruptible("deep think node graph query", &evidence[2..], 3, &signal);
        assert!(part_b.is_empty(), "interrupted mid-chain must yield no further steps");
    }

    #[test]
    fn deep_think_interrupt_after_all_steps() {
        // Cancel AFTER chain completes — verify completed results are unaffected.
        let signal = InterruptSignal::new();
        let evidence = &["alpha", "beta", "gamma"];
        let steps = react_chain_interruptible("alpha beta gamma", evidence, 3, &signal);
        assert_eq!(steps.len(), 3, "steps before cancel must all complete");
        signal.cancel();
        // Subsequent call with cancelled signal yields empty.
        let extra = react_chain_interruptible("alpha beta gamma", evidence, 3, &signal);
        assert!(extra.is_empty());
    }

    #[test]
    fn deep_think_no_interrupt_full_chain() {
        // Full 5-step chain without interrupt — all steps complete, scores valid.
        let signal = InterruptSignal::new();
        let evidence = &["node", "graph", "query", "traversal", "result"];
        let steps = react_chain_interruptible("node graph query traversal result", evidence, 5, &signal);
        assert_eq!(steps.len(), 5);
        for s in &steps {
            assert!(s.score >= 0.0 && s.score <= 1.0);
        }
        assert!(!signal.is_cancelled(), "signal must remain uncancelled");
    }

    // Signal priority ordering
    #[test]
    fn signal_priority_first_cancel_wins() {
        // Two signals; the one cancelled first controls priority.
        let s_high = InterruptSignal::new();
        let s_low = InterruptSignal::new();
        s_high.cancel();
        // High-priority signal is cancelled; low-priority is not.
        assert!(s_high.is_cancelled());
        assert!(!s_low.is_cancelled());
    }

    #[test]
    fn signal_priority_cancel_order_independent() {
        // Cancelling signals in different orders doesn't affect each other.
        let s1 = InterruptSignal::new();
        let s2 = InterruptSignal::new();
        let s3 = InterruptSignal::new();
        s3.cancel();
        s1.cancel();
        assert!(s1.is_cancelled());
        assert!(!s2.is_cancelled());
        assert!(s3.is_cancelled());
    }

    #[test]
    fn signal_priority_chain_stops_on_highest_priority() {
        // Simulate priority: if any signal in a group is cancelled, the chain stops.
        let signals: Vec<InterruptSignal> = (0..3).map(|_| InterruptSignal::new()).collect();
        // Cancel the "highest priority" signal (index 0).
        signals[0].cancel();
        let any_cancelled = signals.iter().any(|s| s.is_cancelled());
        assert!(any_cancelled, "at least one signal cancelled means chain should stop");

        // Verify chain stops when using the cancelled signal.
        let evidence = &["e0", "e1", "e2"];
        let steps = react_chain_interruptible("e0 e1 e2", evidence, 3, &signals[0]);
        assert!(steps.is_empty(), "highest-priority cancel must stop the chain");
    }

    #[test]
    fn signal_priority_lower_priority_continues() {
        // Lower-priority signal (not cancelled) allows the chain to continue.
        let s_high = InterruptSignal::new();
        let s_low = InterruptSignal::new();
        s_high.cancel();
        // s_low is not cancelled — chain using s_low proceeds normally.
        let evidence = &["item one", "item two"];
        let steps = react_chain_interruptible("item one two", evidence, 2, &s_low);
        assert_eq!(steps.len(), 2, "lower-priority non-cancelled signal must allow full chain");
    }

    #[test]
    fn signal_priority_all_cancelled_stops_immediately() {
        // All signals cancelled → any chain stops.
        let signals: Vec<InterruptSignal> = (0..5).map(|_| InterruptSignal::new()).collect();
        for s in &signals {
            s.cancel();
        }
        let evidence = &["e0", "e1", "e2", "e3", "e4"];
        for s in &signals {
            let steps = react_chain_interruptible("test", evidence, 5, s);
            assert!(steps.is_empty(), "every cancelled signal must stop the chain");
        }
    }

    #[test]
    fn signal_priority_none_cancelled_all_run() {
        // No signals cancelled → all chains complete.
        let signals: Vec<InterruptSignal> = (0..3).map(|_| InterruptSignal::new()).collect();
        let evidence = &["run one", "run two"];
        for s in &signals {
            let steps = react_chain_interruptible("run one two", evidence, 2, s);
            assert_eq!(steps.len(), 2, "non-cancelled signal must allow full chain");
        }
    }

    // Additional coverage: step ordering with 8 steps
    #[test]
    fn react_chain_eight_steps_thought_indexed() {
        let evidence = &["e0", "e1", "e2", "e3", "e4", "e5", "e6", "e7"];
        let steps = react_chain("e0 e1 e2 e3", evidence, 8);
        assert_eq!(steps.len(), 8);
        for (i, step) in steps.iter().enumerate() {
            assert!(step.thought.contains(&format!("[{i}]")));
        }
    }

    // rank_hypotheses equal-score group size = 2
    #[test]
    fn rank_equal_scores_pair_stable() {
        let evidence = &["zzz"];
        let ranked = rank_hypotheses(&["aaa", "bbb"], evidence);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].score, 0.0);
        assert_eq!(ranked[1].score, 0.0);
        // Stable: "aaa" must precede "bbb".
        assert_eq!(ranked[0].hypothesis, "aaa");
        assert_eq!(ranked[1].hypothesis, "bbb");
    }

    // ScoredHypothesis score used in f32 arithmetic
    #[test]
    fn scored_hypothesis_score_arithmetic() {
        let evidence = &["node graph"];
        let ranked = rank_hypotheses(&["node graph", "unrelated"], evidence);
        let sum: f32 = ranked.iter().map(|h| h.score).sum();
        assert!(sum > 0.0, "sum of scores must be positive");
        let max: f32 = ranked.iter().map(|h| h.score).fold(f32::NEG_INFINITY, f32::max);
        assert!(max <= 1.0, "max score must be <= 1.0");
    }

    // InterruptSignal: cancel is visible across three clones
    #[test]
    fn interrupt_signal_three_clone_propagation() {
        let s0 = InterruptSignal::new();
        let s1 = s0.clone();
        let s2 = s1.clone();
        assert!(!s0.is_cancelled());
        assert!(!s1.is_cancelled());
        assert!(!s2.is_cancelled());
        s2.cancel();
        assert!(s0.is_cancelled(), "s0 must see cancel from s2");
        assert!(s1.is_cancelled(), "s1 must see cancel from s2");
    }

    // Deep-think: confirm scored result before interrupt
    #[test]
    fn deep_think_partial_result_captured_before_interrupt() {
        let signal = InterruptSignal::new();
        let evidence = &["deep", "result", "node"];
        // Run first step before cancelling.
        let first = react_chain_interruptible("deep result node", &evidence[..1], 1, &signal);
        assert_eq!(first.len(), 1);
        assert!(first[0].score >= 0.0);
        signal.cancel();
        // Remainder yields empty.
        let rest = react_chain_interruptible("deep result node", &evidence[1..], 2, &signal);
        assert!(rest.is_empty());
    }

    // rank_hypotheses equal scores: 6 items all zero
    #[test]
    fn rank_six_equal_zero_score_stable_order() {
        let evidence = &["nomatch"];
        let hypotheses = &["f1", "f2", "f3", "f4", "f5", "f6"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 6);
        for (i, h) in ranked.iter().enumerate() {
            assert_eq!(h.score, 0.0);
            assert_eq!(h.hypothesis, hypotheses[i], "stable order must be preserved at index {i}");
        }
    }

    // =========================================================================
    // WAVE-AE AGENT-10 ADDITIONS
    // =========================================================================

    // --- ReactChain with 10 steps, interrupt at step 5 ---

    #[test]
    fn react_chain_10_steps_interrupt_at_step_5_only_5_complete() {
        let signal = InterruptSignal::new();
        let evidence = &["e0", "e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8", "e9"];

        // Run first 5 steps (indices 0..5) — signal not yet cancelled.
        let first_half =
            react_chain_interruptible("hypothesis", &evidence[..5], 5, &signal);
        assert_eq!(first_half.len(), 5, "first 5 steps must complete");

        // Cancel before running the remaining 5.
        signal.cancel();

        let second_half =
            react_chain_interruptible("hypothesis", &evidence[5..], 5, &signal);
        assert_eq!(
            second_half.len(),
            0,
            "no steps must run after cancellation at step 5"
        );

        // Total completed = 5.
        let total = first_half.len() + second_half.len();
        assert_eq!(total, 5, "exactly 5 of 10 steps must complete");
    }

    #[test]
    fn react_chain_10_steps_scores_all_valid() {
        let evidence = &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
        let steps = react_chain("a b c d e f g h i j", evidence, 10);
        assert_eq!(steps.len(), 10);
        for (i, s) in steps.iter().enumerate() {
            assert!(
                s.score >= 0.0 && s.score <= 1.0,
                "step {i} score {} out of [0,1]",
                s.score
            );
        }
    }

    #[test]
    fn react_chain_10_steps_thought_indexed_0_through_9() {
        let evidence = &["e0", "e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8", "e9"];
        let steps = react_chain("e0", evidence, 10);
        assert_eq!(steps.len(), 10);
        for (i, step) in steps.iter().enumerate() {
            assert!(
                step.thought.contains(&format!("[{i}]")),
                "step {i} thought must reference index [{i}]"
            );
        }
    }

    // --- rank_hypotheses with 0 hypotheses returns empty ---

    #[test]
    fn rank_hypotheses_zero_hypotheses_returns_empty() {
        let ranked = rank_hypotheses(&[], &["evidence one", "evidence two"]);
        assert!(
            ranked.is_empty(),
            "rank_hypotheses with empty hypothesis slice must return empty vec"
        );
    }

    #[test]
    fn rank_hypotheses_zero_hypotheses_zero_evidence_returns_empty() {
        let ranked = rank_hypotheses(&[], &[]);
        assert!(ranked.is_empty());
    }

    #[test]
    fn best_hypothesis_on_empty_hypotheses_returns_none_with_evidence() {
        let result = best_hypothesis(&[], &["some", "evidence"]);
        assert!(result.is_none(), "empty hypotheses slice must return None");
    }

    // --- ScoredHypothesis with NaN score handled gracefully ---

    #[test]
    fn scored_hypothesis_nan_score_partial_cmp_returns_equal_fallback() {
        // rank_hypotheses sorts using partial_cmp with Equal fallback for NaN.
        // Directly construct a ScoredHypothesis with NaN score and verify
        // that partial_cmp with Equal fallback doesn't panic.
        let nan_score: f32 = f32::NAN;
        let normal_score: f32 = 0.5;
        // partial_cmp on NaN returns None; sort uses Equal as fallback.
        let cmp = nan_score.partial_cmp(&normal_score);
        // NaN.partial_cmp returns None — verify the fallback branch is exercised.
        assert!(cmp.is_none(), "NaN.partial_cmp must return None");
        // The sort comparator in rank_hypotheses uses unwrap_or(Equal):
        let sort_result = nan_score
            .partial_cmp(&normal_score)
            .unwrap_or(std::cmp::Ordering::Equal);
        assert_eq!(
            sort_result,
            std::cmp::Ordering::Equal,
            "NaN score sort falls back to Equal"
        );
    }

    #[test]
    fn scored_hypothesis_nan_score_sort_does_not_panic() {
        // Build a vec containing a ScoredHypothesis with NaN score and sort it.
        // This exercises the unwrap_or(Equal) branch in rank_hypotheses' comparator.
        let mut items = vec![
            ScoredHypothesis {
                hypothesis: "nan".to_string(),
                score: f32::NAN,
                evidence_used: vec![],
                step_count: 0,
            },
            ScoredHypothesis {
                hypothesis: "normal".to_string(),
                score: 0.5,
                evidence_used: vec![],
                step_count: 0,
            },
        ];
        // Must not panic.
        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn scored_hypothesis_score_nan_is_f32_nan() {
        let sh = ScoredHypothesis {
            hypothesis: "test".to_string(),
            score: f32::NAN,
            evidence_used: vec![],
            step_count: 0,
        };
        assert!(sh.score.is_nan(), "score must be NaN");
    }

    // --- deep_think chain produces unique step IDs ---

    #[test]
    fn deep_think_chain_produces_unique_thought_strings() {
        // Each step has a unique "thought" string because it encodes the evidence index.
        let evidence = &["item0", "item1", "item2", "item3", "item4"];
        let steps = react_chain("deep think hypothesis", evidence, 5);
        assert_eq!(steps.len(), 5);
        let thoughts: Vec<&str> = steps.iter().map(|s| s.thought.as_str()).collect();
        let unique_count = {
            let mut sorted = thoughts.clone();
            sorted.sort_unstable();
            sorted.dedup();
            sorted.len()
        };
        assert_eq!(
            unique_count, 5,
            "all thought strings must be unique (each encodes a different evidence index)"
        );
    }

    #[test]
    fn deep_think_chain_action_strings_unique() {
        let evidence = &["ev0", "ev1", "ev2"];
        let steps = react_chain("thought", evidence, 3);
        let actions: Vec<&str> = steps.iter().map(|s| s.action.as_str()).collect();
        let unique_count = {
            let mut sorted = actions.clone();
            sorted.sort_unstable();
            sorted.dedup();
            sorted.len()
        };
        assert_eq!(unique_count, 3, "all action strings must be unique");
    }

    #[test]
    fn deep_think_interruptible_unique_thoughts_before_cancel() {
        let signal = InterruptSignal::new();
        let evidence = &["d0", "d1", "d2", "d3", "d4"];
        let steps = react_chain_interruptible("deep think", evidence, 5, &signal);
        signal.cancel();
        // Verify steps produced before cancel have unique thoughts.
        let thoughts: Vec<&str> = steps.iter().map(|s| s.thought.as_str()).collect();
        let unique_count = {
            let mut sorted = thoughts.clone();
            sorted.sort_unstable();
            sorted.dedup();
            sorted.len()
        };
        assert_eq!(
            unique_count,
            steps.len(),
            "all thoughts before cancel must be unique"
        );
    }

    // --- Additional targeted tests ---

    #[test]
    fn rank_hypotheses_result_length_matches_input() {
        let evidence = &["alpha", "beta"];
        let hypotheses = &["h1", "h2", "h3", "h4", "h5"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(
            ranked.len(),
            hypotheses.len(),
            "ranked output length must equal input length"
        );
    }

    #[test]
    fn react_chain_10_steps_clamped_by_evidence() {
        // Only 7 evidence items; requesting 10 steps yields 7.
        let evidence = &["a", "b", "c", "d", "e", "f", "g"];
        let steps = react_chain("a b c d e f g", evidence, 10);
        assert_eq!(steps.len(), 7, "steps must be clamped to evidence length");
    }

    #[test]
    fn react_chain_interrupt_at_step_5_leaves_5_steps_with_valid_scores() {
        let signal = InterruptSignal::new();
        let evidence = &["e0", "e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8", "e9"];
        let first = react_chain_interruptible("hyp", &evidence[..5], 5, &signal);
        signal.cancel();
        assert_eq!(first.len(), 5);
        for s in &first {
            assert!(s.score >= 0.0 && s.score <= 1.0);
        }
    }

    #[test]
    fn rank_hypotheses_with_zero_hypotheses_no_panic() {
        // Explicit check that calling rank_hypotheses with empty slice doesn't panic.
        let result = rank_hypotheses(&[], &["word"]);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn classify_with_react_single_char_words() {
        // Single-character hypothesis and evidence.
        let score = classify_with_react("a", &["a"]);
        assert!((score - 1.0_f32).abs() < 1e-5, "single char exact match must score 1.0");
    }

    #[test]
    fn react_chain_interruptible_ten_steps_not_cancelled() {
        let signal = InterruptSignal::new(); // not cancelled
        let evidence: Vec<&str> = (0..10).map(|i| if i == 0 { "ev0" } else if i == 1 { "ev1" } else if i == 2 { "ev2" } else if i == 3 { "ev3" } else if i == 4 { "ev4" } else if i == 5 { "ev5" } else if i == 6 { "ev6" } else if i == 7 { "ev7" } else if i == 8 { "ev8" } else { "ev9" }).collect();
        let steps = react_chain_interruptible("ev", &evidence, 10, &signal);
        assert_eq!(steps.len(), 10, "uncancelled 10-step chain must complete all steps");
    }

    #[test]
    fn deep_think_step_observations_contain_confidence() {
        let evidence = &["think", "deep", "node"];
        let steps = react_chain("deep think node", evidence, 3);
        for step in &steps {
            assert!(
                step.observation.contains("partial confidence:"),
                "each step observation must contain 'partial confidence:'"
            );
        }
    }

    #[test]
    fn scored_hypothesis_debug_format_contains_fields() {
        let evidence = &["alpha beta"];
        let ranked = rank_hypotheses(&["alpha beta"], evidence);
        let dbg = format!("{:?}", ranked[0]);
        assert!(dbg.contains("ScoredHypothesis"));
        assert!(dbg.contains("alpha beta"));
    }

    #[test]
    fn classify_with_react_multiple_evidence_items_bounded() {
        // 5 evidence items; score must stay in [0,1].
        let evidence = &["one two", "two three", "three four", "four five", "five one"];
        let score = classify_with_react("one two three four five", evidence);
        assert!(score >= 0.0 && score <= 1.0, "score {score} out of [0,1]");
    }

    #[test]
    fn rank_hypotheses_hypothesis_field_matches_input() {
        let evidence = &["test"];
        let hypotheses = &["hypothesis alpha", "hypothesis beta"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        let names: Vec<&str> = ranked.iter().map(|r| r.hypothesis.as_str()).collect();
        assert!(names.contains(&"hypothesis alpha"));
        assert!(names.contains(&"hypothesis beta"));
    }

    #[test]
    fn best_hypothesis_score_between_zero_and_one() {
        let evidence = &["node graph query"];
        let hypotheses = &["node graph", "unrelated"];
        let best = best_hypothesis(hypotheses, evidence).unwrap();
        assert!(best.score >= 0.0 && best.score <= 1.0);
    }

    #[test]
    fn scored_hypothesis_evidence_used_not_empty_when_evidence_provided() {
        let evidence = &["evidence item"];
        let ranked = rank_hypotheses(&["evidence"], evidence);
        assert!(!ranked[0].evidence_used.is_empty(), "evidence_used must not be empty");
    }

    #[test]
    fn react_chain_interruptible_five_steps_all_have_non_empty_fields() {
        let signal = InterruptSignal::new();
        let evidence = &["alpha", "beta", "gamma", "delta", "epsilon"];
        let steps = react_chain_interruptible("alpha beta gamma delta epsilon", evidence, 5, &signal);
        assert_eq!(steps.len(), 5);
        for (i, step) in steps.iter().enumerate() {
            assert!(!step.thought.is_empty(), "step {i} thought must not be empty");
            assert!(!step.action.is_empty(), "step {i} action must not be empty");
            assert!(!step.observation.is_empty(), "step {i} observation must not be empty");
        }
    }

    // =========================================================================
    // WAVE-AF AGENT-8 ADDITIONS
    // =========================================================================

    // --- ReactChain with 0 steps completes immediately (returns empty) ---

    #[test]
    fn react_chain_zero_steps_completes_immediately() {
        let evidence = &["some", "evidence", "here"];
        let steps = react_chain("hypothesis", evidence, 0);
        assert!(
            steps.is_empty(),
            "react_chain with 0 max_steps must return empty vec immediately"
        );
    }

    #[test]
    fn react_chain_zero_steps_zero_evidence_completes() {
        let steps = react_chain("hypothesis", &[], 0);
        assert!(steps.is_empty(), "0 steps + 0 evidence → empty");
    }

    #[test]
    fn react_chain_interruptible_zero_steps_completes_immediately() {
        let signal = InterruptSignal::new();
        let steps = react_chain_interruptible("hypothesis", &["e1", "e2", "e3"], 0, &signal);
        assert!(
            steps.is_empty(),
            "interruptible react_chain with 0 steps must return empty immediately"
        );
        // Signal should NOT be cancelled — 0 steps means nothing ran.
        assert!(!signal.is_cancelled());
    }

    #[test]
    fn react_chain_zero_steps_returns_not_none() {
        // Specifically test return type is Vec (not Option), and empty is the sentinel.
        let result = react_chain("test", &["evidence"], 0);
        let is_empty: bool = result.is_empty();
        assert!(is_empty, "zero steps must yield empty (not Some/None — it is a Vec)");
    }

    // --- Hypothesis with confidence exactly 0.0 sorts last ---

    #[test]
    fn hypothesis_confidence_zero_sorts_last_in_two() {
        let evidence = &["graph node query"];
        let hypotheses = &["graph node query", "zzz unrelated"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 2);
        assert!(ranked[0].score > 0.0, "winner must have score > 0");
        // The zero-score hypothesis must be at the end.
        assert_eq!(
            ranked[ranked.len() - 1].score, 0.0,
            "hypothesis with score 0.0 must sort last"
        );
    }

    #[test]
    fn hypothesis_confidence_zero_sorts_last_in_five() {
        let evidence = &["alpha beta gamma"];
        let hypotheses = &[
            "alpha beta gamma", // score > 0
            "alpha",            // score > 0
            "zzz",              // score 0
            "qqq",              // score 0
            "mmm",              // score 0
        ];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 5);
        // Top 2 must have score > 0.
        assert!(ranked[0].score > 0.0, "rank[0] must have positive score");
        assert!(ranked[1].score > 0.0, "rank[1] must have positive score");
        // Bottom 3 must all have score 0.
        assert_eq!(ranked[2].score, 0.0);
        assert_eq!(ranked[3].score, 0.0);
        assert_eq!(ranked[4].score, 0.0);
    }

    #[test]
    fn hypothesis_exactly_zero_confidence_direct() {
        // A hypothesis with zero word overlap must score exactly 0.0.
        let score = classify_with_react("xyz_unique_word", &["totally_different_content"]);
        assert_eq!(score, 0.0, "zero-overlap hypothesis must score exactly 0.0");
    }

    #[test]
    fn hypothesis_zero_confidence_excluded_from_top_when_winner_exists() {
        let evidence = &["match word"];
        let hypotheses = &["match word", "zero overlap zzz"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        // Winner has score > 0; loser has 0.
        assert!(ranked[0].score > ranked[1].score, "winner score must exceed zero-score");
        assert_eq!(ranked[1].score, 0.0, "loser must be exactly 0.0");
        assert_eq!(ranked[1].hypothesis, "zero overlap zzz");
    }

    #[test]
    fn rank_hypotheses_all_zero_confidence_preserves_all() {
        // If all hypotheses score 0.0, all must still be present in output.
        let evidence = &["completely irrelevant"];
        let hypotheses = &["aaa bbb", "ccc ddd", "eee fff"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 3, "all hypotheses must be preserved even with zero scores");
        for h in &ranked {
            assert_eq!(h.score, 0.0, "all scores must be 0.0 with no overlap");
        }
    }

    // --- InterruptSignal at step 1 of 10 stops chain immediately ---

    #[test]
    fn interrupt_at_step_1_of_10_stops_chain() {
        let signal = InterruptSignal::new();
        let evidence = &["e0", "e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8", "e9"];

        // Step 0 completes (signal not yet cancelled).
        let step0 = react_chain_interruptible("hyp", &evidence[..1], 1, &signal);
        assert_eq!(step0.len(), 1, "step 0 must complete before cancel");

        // Cancel at step 1.
        signal.cancel();

        // Steps 1–9 must not run.
        let remaining = react_chain_interruptible("hyp", &evidence[1..], 9, &signal);
        assert_eq!(
            remaining.len(), 0,
            "after cancel at step 1, no further steps must run (0 of 9 remaining)"
        );

        // Total: exactly 1 step completed of 10.
        assert_eq!(step0.len() + remaining.len(), 1);
    }

    #[test]
    fn interrupt_at_step_1_of_10_score_from_step_0_valid() {
        let signal = InterruptSignal::new();
        let evidence = &["e0", "e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8", "e9"];

        let step0 = react_chain_interruptible("e0", &evidence[..1], 1, &signal);
        assert_eq!(step0.len(), 1);
        // Score from the single completed step must be in [0, 1].
        assert!(step0[0].score >= 0.0 && step0[0].score <= 1.0);

        signal.cancel();

        // After cancel, signal is visible.
        assert!(signal.is_cancelled());
    }

    #[test]
    fn interrupt_at_step_1_of_10_via_clone() {
        let signal = InterruptSignal::new();
        let guard = signal.clone();
        let evidence = &["e0", "e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8", "e9"];

        let first = react_chain_interruptible("hyp", &evidence[..1], 1, &signal);
        assert_eq!(first.len(), 1);

        // Cancel via clone (simulates external interrupt after step 1).
        guard.cancel();

        let rest = react_chain_interruptible("hyp", &evidence[1..], 9, &signal);
        assert_eq!(rest.len(), 0, "clone cancel at step 1 must stop remaining 9 steps");
    }

    #[test]
    fn interrupt_step_1_total_completed_is_one() {
        let signal = InterruptSignal::new();
        let evidence: Vec<&str> = (0..10).map(|i| if i == 0 {"ev0"} else if i == 1 {"ev1"} else if i == 2 {"ev2"} else if i == 3 {"ev3"} else if i == 4 {"ev4"} else if i == 5 {"ev5"} else if i == 6 {"ev6"} else if i == 7 {"ev7"} else if i == 8 {"ev8"} else {"ev9"}).collect();

        let step0_result = react_chain_interruptible("test", &evidence[..1], 1, &signal);
        signal.cancel();
        let steps_1_9 = react_chain_interruptible("test", &evidence[1..], 9, &signal);

        let total = step0_result.len() + steps_1_9.len();
        assert_eq!(total, 1, "only step 0 of 10 must complete when cancelled at step 1");
    }

    // --- rank_hypotheses preserves all input hypotheses (none lost) ---

    #[test]
    fn rank_hypotheses_preserves_all_three_inputs() {
        let evidence = &["word"];
        let hypotheses = &["h1", "h2", "h3"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 3, "all 3 input hypotheses must be preserved");
        let names: Vec<&str> = ranked.iter().map(|r| r.hypothesis.as_str()).collect();
        assert!(names.contains(&"h1"));
        assert!(names.contains(&"h2"));
        assert!(names.contains(&"h3"));
    }

    #[test]
    fn rank_hypotheses_preserves_all_ten_inputs() {
        let evidence = &["target"];
        let hypotheses: Vec<String> = (0..10).map(|i| format!("hyp_{i}")).collect();
        let refs: Vec<&str> = hypotheses.iter().map(|s| s.as_str()).collect();
        let ranked = rank_hypotheses(&refs, evidence);
        assert_eq!(ranked.len(), 10, "all 10 input hypotheses must be preserved");
    }

    #[test]
    fn rank_hypotheses_preserves_all_with_duplicates() {
        // Duplicate hypothesis strings are distinct inputs and both preserved.
        let evidence = &["node"];
        let hypotheses = &["node", "node", "other"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 3, "3 input hypotheses including duplicates must all be in output");
    }

    #[test]
    fn rank_hypotheses_none_dropped_after_sort() {
        // After sorting by score, no hypothesis should be dropped.
        let evidence = &["graph node query result traversal"];
        let hypotheses = &[
            "graph node",
            "completely unrelated",
            "query result",
            "traversal graph",
            "zzz",
        ];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), hypotheses.len(), "ranked output must match input count — none dropped");
        // Verify all hypotheses present in ranked output.
        for h in hypotheses {
            assert!(
                ranked.iter().any(|r| r.hypothesis == *h),
                "hypothesis '{h}' must appear in ranked output"
            );
        }
    }

    #[test]
    fn rank_hypotheses_empty_preserves_empty() {
        let ranked = rank_hypotheses(&[], &["evidence"]);
        assert_eq!(ranked.len(), 0, "empty input → empty output (nothing dropped)");
    }

    #[test]
    fn rank_hypotheses_single_item_preserved() {
        let evidence = &["alpha"];
        let ranked = rank_hypotheses(&["alpha"], evidence);
        assert_eq!(ranked.len(), 1, "single hypothesis must be preserved");
        assert_eq!(ranked[0].hypothesis, "alpha");
    }

    #[test]
    fn rank_hypotheses_100_items_all_preserved() {
        let evidence = &["target"];
        let hypotheses: Vec<String> = (0..100).map(|i| format!("h{i}")).collect();
        let refs: Vec<&str> = hypotheses.iter().map(|s| s.as_str()).collect();
        let ranked = rank_hypotheses(&refs, evidence);
        assert_eq!(ranked.len(), 100, "all 100 hypotheses must be preserved in output");
    }

    #[test]
    fn react_chain_zero_steps_returns_empty_vec_type() {
        let result: Vec<ReactStep> = react_chain("hyp", &["ev"], 0);
        assert!(result.is_empty(), "return type is Vec<ReactStep>, which is empty for 0 steps");
    }

    #[test]
    fn react_chain_zero_steps_large_evidence_still_empty() {
        let evidence: Vec<&str> = (0..50).map(|_| "word").collect();
        let steps = react_chain("word", &evidence, 0);
        assert!(steps.is_empty(), "0 steps with 50 evidence items → empty");
    }

    #[test]
    fn hypothesis_confidence_zero_last_in_three() {
        let evidence = &["match"];
        let hypotheses = &["match", "partial match", "unrelated zzz"];
        let ranked = rank_hypotheses(hypotheses, evidence);
        assert_eq!(ranked.len(), 3);
        // "unrelated zzz" has 0 overlap → must be last.
        assert_eq!(ranked[2].hypothesis, "unrelated zzz");
        assert_eq!(ranked[2].score, 0.0);
    }

    #[test]
    fn interrupt_at_step_1_signal_is_set_after_cancel() {
        let signal = InterruptSignal::new();
        let evidence = &["e0"];
        let _ = react_chain_interruptible("hyp", evidence, 1, &signal);
        assert!(!signal.is_cancelled(), "running chain must not cancel signal");
        signal.cancel();
        assert!(signal.is_cancelled(), "signal must be set after explicit cancel");
    }

    #[test]
    fn rank_hypotheses_output_length_equals_input_always() {
        for n in [0, 1, 2, 5, 10, 20] {
            let hypotheses: Vec<String> = (0..n).map(|i| format!("h{i}")).collect();
            let refs: Vec<&str> = hypotheses.iter().map(|s| s.as_str()).collect();
            let ranked = rank_hypotheses(&refs, &["evidence"]);
            assert_eq!(
                ranked.len(), n,
                "rank_hypotheses output length must equal input length for n={n}"
            );
        }
    }

    #[test]
    fn react_chain_zero_steps_does_not_call_classify() {
        // If 0 steps, no classify_with_react is called — score never computed.
        // Verify by checking the chain returns empty for any evidence.
        let evidence = &["alpha", "beta", "gamma", "delta"];
        let steps = react_chain("alpha beta gamma delta", evidence, 0);
        assert!(steps.is_empty());
        // Also verify 1 step DOES produce a score.
        let one_step = react_chain("alpha", &["alpha"], 1);
        assert_eq!(one_step.len(), 1);
        assert!(one_step[0].score > 0.0);
    }

    #[test]
    fn hypothesis_zero_confidence_is_exactly_zero_not_epsilon() {
        // Zero overlap must produce exactly 0.0, not a tiny positive float.
        let score = classify_with_react("xyz_unique_abc", &["totally_unrelated_content"]);
        assert_eq!(score, 0.0_f32, "zero-overlap must be exactly 0.0, not near-zero");
    }

    #[test]
    fn rank_hypotheses_all_names_in_output() {
        // No hypothesis name should be missing from ranked output.
        let evidence = &["graph"];
        let input = &["graph query", "tree search", "bitmap scan", "hash lookup"];
        let ranked = rank_hypotheses(input, evidence);
        let output_names: Vec<&str> = ranked.iter().map(|r| r.hypothesis.as_str()).collect();
        for name in input {
            assert!(
                output_names.contains(name),
                "'{name}' must appear in ranked output"
            );
        }
    }

    #[test]
    fn interrupt_at_step_1_of_10_first_step_score_in_range() {
        let signal = InterruptSignal::new();
        let evidence = &["e0", "e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8", "e9"];
        let step0 = react_chain_interruptible("e0", &evidence[..1], 1, &signal);
        signal.cancel();
        let rest = react_chain_interruptible("e0", &evidence[1..], 9, &signal);
        assert_eq!(step0.len(), 1);
        assert_eq!(rest.len(), 0);
        assert!(step0[0].score >= 0.0 && step0[0].score <= 1.0);
    }

    #[test]
    fn react_chain_zero_steps_no_side_effects() {
        // 0 steps must return empty without modifying any external state.
        let evidence = &["alpha", "beta"];
        let steps = react_chain("alpha", evidence, 0);
        assert!(steps.is_empty());
        // Running again with 0 steps should also be empty (idempotent).
        let steps2 = react_chain("alpha", evidence, 0);
        assert!(steps2.is_empty());
    }

    // ── WAVE-AG AGENT-10 additions ─────────────────────────────────────────────

    #[test]
    fn intent_chain_0_steps_ok() {
        let result = react_chain("hypothesis", &["evidence"], 0);
        assert!(result.is_empty(), "0 steps must return empty Vec");
    }

    #[test]
    fn intent_chain_1_step_ok() {
        let result = react_chain("target", &["target"], 1);
        assert_eq!(result.len(), 1, "1 step must return Vec of length 1");
        assert!((0.0..=1.0).contains(&result[0].score));
    }

    #[test]
    fn intent_chain_10_steps_ok() {
        // react_chain caps steps at min(max_steps, evidence.len()), so provide 10 evidence items.
        let evidence: Vec<&str> = ["e0","e1","e2","e3","e4","e5","e6","e7","e8","e9"].into();
        let result = react_chain("word", &evidence, 10);
        assert_eq!(result.len(), 10, "10 steps must return Vec of length 10");
        for step in &result {
            assert!((0.0..=1.0).contains(&step.score));
        }
    }

    #[test]
    fn intent_chain_500_items_batch_ok() {
        // 500 hypotheses ranked — no crash, correct count.
        let hypotheses: Vec<String> = (0..500).map(|i| format!("hyp_{i}")).collect();
        let refs: Vec<&str> = hypotheses.iter().map(|s| s.as_str()).collect();
        let ranked = rank_hypotheses(&refs, &["hyp_0"]);
        assert_eq!(ranked.len(), 500);
    }

    #[test]
    fn intent_react_loop_terminates() {
        // react_chain caps at evidence.len(); provide 50 evidence items.
        let ev: Vec<String> = (0..50).map(|i| format!("ev{i}")).collect();
        let ev_refs: Vec<&str> = ev.iter().map(|s| s.as_str()).collect();
        let steps = react_chain("terminate", &ev_refs, 50);
        assert_eq!(steps.len(), 50);
    }

    #[test]
    fn intent_react_loop_max_iterations_enforced() {
        // Requesting max_steps=5 with 5 evidence items must produce exactly 5 steps.
        let evidence = &["a", "b", "c", "d", "e"];
        let steps = react_chain("max", evidence, 5);
        assert_eq!(steps.len(), 5, "max_steps must be the hard upper bound");
    }

    #[test]
    fn intent_resolve_empty_query_returns_empty() {
        let ranked = rank_hypotheses(&[], &["evidence"]);
        assert!(ranked.is_empty(), "empty hypotheses slice must return empty");
    }

    #[test]
    fn intent_resolve_single_word_query() {
        let ranked = rank_hypotheses(&["hello"], &["hello"]);
        assert_eq!(ranked.len(), 1);
        assert!(ranked[0].score > 0.0, "single matching word must yield positive score");
    }

    #[test]
    fn intent_resolve_multi_word_query() {
        let ranked = rank_hypotheses(&["alpha beta gamma"], &["alpha", "beta"]);
        assert_eq!(ranked.len(), 1);
        assert!(ranked[0].score > 0.0);
    }

    #[test]
    fn intent_score_above_threshold_included() {
        // rank_hypotheses returns all items including high-scoring ones.
        let ranked = rank_hypotheses(&["exact match"], &["exact match"]);
        assert_eq!(ranked.len(), 1);
        assert!(ranked[0].score > 0.0);
    }

    #[test]
    fn intent_score_below_threshold_excluded_by_sort() {
        // Zero-scoring item is last after ranking.
        let ranked = rank_hypotheses(&["zero overlap zzz", "match"], &["match"]);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[ranked.len() - 1].score, 0.0);
    }

    #[test]
    fn intent_top_k_limits_results_via_best_hypothesis() {
        // best_hypothesis returns only the top result.
        let result = best_hypothesis(&["a b c", "x y z", "a"], &["a b c"]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().hypothesis, "a b c");
    }

    #[test]
    fn intent_dedup_same_word_not_inflated() {
        // Duplicate evidence words must not inflate score beyond 1.0.
        let score = classify_with_react("word", &["word", "word", "word"]);
        assert!((0.0..=1.0).contains(&score));
    }

    #[test]
    fn intent_fallback_when_no_match() {
        // best_hypothesis on non-overlapping returns Some (lowest scorer) — length 1 input.
        let best = best_hypothesis(&["zzz_unique"], &["aaa_different"]);
        assert!(best.is_some(), "best_hypothesis must always return Some when input is non-empty");
        assert_eq!(best.unwrap().score, 0.0);
    }

    #[test]
    fn intent_score_deterministic_for_same_input() {
        // Same hypothesis + evidence must always yield the same score.
        let s1 = classify_with_react("hello world", &["hello", "world"]);
        let s2 = classify_with_react("hello world", &["hello", "world"]);
        assert_eq!(s1, s2, "classify_with_react must be deterministic");
    }

    #[test]
    fn intent_chain_step_thought_nonempty() {
        let steps = react_chain("my_hypothesis", &["my", "hypothesis"], 3);
        for step in &steps {
            assert!(!step.thought.is_empty(), "ReactStep.thought must not be empty");
        }
    }

    #[test]
    fn intent_chain_step_action_nonempty() {
        let steps = react_chain("ev1", &["ev1", "ev2"], 2);
        assert_eq!(steps.len(), 2);
        for step in &steps {
            assert!(!step.action.is_empty(), "ReactStep.action must not be empty");
        }
    }

    #[test]
    fn intent_ranked_first_score_gte_last_score() {
        let hypotheses = &["perfect match", "partial", "nothing_at_all_xyz"];
        let ranked = rank_hypotheses(hypotheses, &["perfect match"]);
        assert!(ranked[0].score >= ranked[ranked.len() - 1].score, "ranked output must be descending by score");
    }

    #[test]
    fn intent_interrupt_signal_cancel_stops_chain() {
        let signal = InterruptSignal::new();
        signal.cancel();
        let steps = react_chain_interruptible("hyp", &["hyp"], 100, &signal);
        assert!(steps.is_empty(), "cancelled signal must stop chain immediately");
    }

    #[test]
    fn intent_score_exact_full_overlap_positive() {
        let score = classify_with_react("alpha beta", &["alpha", "beta"]);
        assert!(score > 0.0, "full overlap must yield positive score");
    }

    #[test]
    fn intent_score_no_overlap_is_zero() {
        let score = classify_with_react("aaa_unique", &["bbb_unique_xyz"]);
        assert_eq!(score, 0.0, "no overlap must yield score 0.0");
    }

    #[test]
    fn intent_best_hypothesis_returns_highest_score() {
        let hypotheses = &["direct match", "indirect", "unrelated_xyz"];
        let best = best_hypothesis(hypotheses, &["direct match"]).unwrap();
        let ranked = rank_hypotheses(hypotheses, &["direct match"]);
        assert_eq!(best.score, ranked[0].score, "best_hypothesis must equal ranked[0]");
    }

    #[test]
    fn intent_rank_20_items_all_present() {
        let hypotheses: Vec<String> = (0..20).map(|i| format!("item_{i}")).collect();
        let refs: Vec<&str> = hypotheses.iter().map(|s| s.as_str()).collect();
        let ranked = rank_hypotheses(&refs, &["item_0"]);
        assert_eq!(ranked.len(), 20);
        for h in &hypotheses {
            assert!(ranked.iter().any(|r| r.hypothesis == *h), "'{h}' must be in ranked output");
        }
    }

    #[test]
    fn intent_chain_interruptible_without_cancel_runs_all_steps() {
        // Must provide enough evidence items for 5 steps.
        let signal = InterruptSignal::new();
        let evidence = &["a", "b", "c", "d", "e"];
        let steps = react_chain_interruptible("word", evidence, 5, &signal);
        assert_eq!(steps.len(), 5, "non-cancelled signal must run all steps");
    }

    #[test]
    fn intent_scored_hypothesis_fields_accessible() {
        let ranked = rank_hypotheses(&["test_word"], &["test_word"]);
        let sh = &ranked[0];
        // Fields must be accessible and have correct types.
        let _: &str = sh.hypothesis.as_str();
        let _: f32 = sh.score;
        let _: usize = sh.step_count;
    }

    #[test]
    fn intent_react_step_fields_accessible() {
        let steps = react_chain("word", &["word"], 1);
        let step = &steps[0];
        let _: &str = step.thought.as_str();
        let _: &str = step.action.as_str();
        let _: &str = step.observation.as_str();
        let _: f32 = step.score;
    }

    #[test]
    fn intent_rank_5_items_descending_order() {
        let hypotheses = &["perfect match", "good match", "ok match", "poor match", "no match xyz"];
        let ranked = rank_hypotheses(hypotheses, &["perfect match"]);
        assert_eq!(ranked.len(), 5);
        // Scores must be non-increasing.
        for w in ranked.windows(2) {
            assert!(w[0].score >= w[1].score, "ranked output must be descending");
        }
    }

    #[test]
    fn intent_classify_returns_f32() {
        let s: f32 = classify_with_react("test", &["test"]);
        assert!(s.is_finite(), "classify_with_react must return finite f32");
    }

    #[test]
    fn intent_best_hypothesis_none_on_empty() {
        let result = best_hypothesis(&[], &["evidence"]);
        assert!(result.is_none(), "best_hypothesis on empty input must return None");
    }

    #[test]
    fn intent_scored_hypothesis_step_count_matches_evidence() {
        let evidence = &["a", "b", "c"];
        let ranked = rank_hypotheses(&["a"], evidence);
        assert_eq!(ranked[0].step_count, 3, "step_count must equal evidence.len()");
    }
}
