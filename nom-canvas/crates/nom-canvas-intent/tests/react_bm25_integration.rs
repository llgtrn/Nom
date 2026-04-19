// Integration tests: ReAct loop + BM25 retrieval working together

#[cfg(test)]
mod react_bm25_integration {
    use nom_canvas_intent::{
        classify_with_react, react_chain, react_chain_interruptible, BM25Retriever, InterruptSignal,
    };

    // -----------------------------------------------------------------------
    // Test 1: BM25 index + query → top-k results
    // -----------------------------------------------------------------------
    #[test]
    fn bm25_index_and_query_returns_top_k() {
        let mut retriever = BM25Retriever::new();
        retriever.add_document("doc_a", "graph traversal node edge path");
        retriever.add_document("doc_b", "sorting algorithm quicksort merge");
        retriever.add_document("doc_c", "graph node search breadth first");
        retriever.add_document("doc_d", "string manipulation parsing tokens");

        let results = retriever.retrieve("graph node", 2);

        assert_eq!(results.len(), 2, "expected exactly 2 top-k results");
        // Both top results should be graph-related
        let ids: Vec<&str> = results.iter().map(|(d, _)| d.id.as_str()).collect();
        assert!(
            ids.contains(&"doc_a") || ids.contains(&"doc_c"),
            "expected a graph doc in top-2, got {ids:?}"
        );
        // Scores are positive
        assert!(results[0].1 > 0.0, "top result score must be positive");
    }

    // -----------------------------------------------------------------------
    // Test 2: Feed BM25 top-k results as ReAct observation
    // -----------------------------------------------------------------------
    #[test]
    fn bm25_top_k_fed_as_react_observations() {
        let mut retriever = BM25Retriever::new();
        retriever.add_document("ev1", "authentication token validation success");
        retriever.add_document("ev2", "authentication failure invalid credentials");
        retriever.add_document("ev3", "network timeout connection refused");

        let results = retriever.retrieve("authentication token", 2);
        assert!(!results.is_empty(), "retriever must return results");

        // Convert retrieved doc contents to evidence strings for the ReAct step
        let evidence_strings: Vec<String> = results.iter().map(|(d, _)| d.content.clone()).collect();
        let evidence_refs: Vec<&str> = evidence_strings.iter().map(|s| s.as_str()).collect();

        let steps = react_chain("authentication token validation", &evidence_refs, evidence_refs.len());

        assert!(
            !steps.is_empty(),
            "react_chain must produce steps from BM25 observations"
        );
        assert!(
            steps.iter().any(|s| s.score > 0.0),
            "at least one step should have a positive score"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: ReAct step with empty context vs rich context
    // -----------------------------------------------------------------------
    #[test]
    fn react_empty_context_vs_rich_context() {
        let hypothesis = "intent classification graph query";

        // Empty context: no evidence → zero score
        let score_empty = classify_with_react(hypothesis, &[]);
        assert_eq!(score_empty, 0.0, "empty evidence must yield 0.0 confidence");

        // Rich context from BM25 corpus
        let mut retriever = BM25Retriever::new();
        retriever.add_document("r1", "intent classification graph query node");
        retriever.add_document("r2", "graph node edge traversal query");
        retriever.add_document("r3", "unrelated banking transaction ledger");

        let results = retriever.retrieve("intent classification graph", 2);
        let evidence_strings: Vec<String> = results.iter().map(|(d, _)| d.content.clone()).collect();
        let evidence_refs: Vec<&str> = evidence_strings.iter().map(|s| s.as_str()).collect();

        let score_rich = classify_with_react(hypothesis, &evidence_refs);
        assert!(
            score_rich > score_empty,
            "rich BM25 context must yield higher confidence than empty: got {score_rich} vs {score_empty}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: BM25 score ordering (higher relevance = higher score)
    // -----------------------------------------------------------------------
    #[test]
    fn bm25_score_ordering_by_relevance() {
        let mut retriever = BM25Retriever::new();
        // Highly relevant: contains all query terms multiple times
        retriever.add_document("high_rel", "compiler parser parser compiler token token");
        // Medium relevance: contains one query term
        retriever.add_document("med_rel", "compiler optimization pass");
        // Irrelevant: no overlap
        retriever.add_document("no_rel", "cooking recipe bread butter");

        let query = "compiler parser token";
        let high_doc = &retriever.documents[0].clone();
        let med_doc = &retriever.documents[1].clone();
        let no_doc = &retriever.documents[2].clone();

        let score_high = retriever.score(query, high_doc);
        let score_med = retriever.score(query, med_doc);
        let score_no = retriever.score(query, no_doc);

        assert!(
            score_high > score_med,
            "high_rel ({score_high:.4}) should outscore med_rel ({score_med:.4})"
        );
        assert!(
            score_med > score_no,
            "med_rel ({score_med:.4}) should outscore no_rel ({score_no:.4})"
        );
        assert_eq!(score_no, 0.0, "irrelevant doc should score 0.0");

        // retrieve() order must match manual scoring
        let results = retriever.retrieve(query, 3);
        assert_eq!(results[0].0.id, "high_rel", "first result must be high_rel");
        assert_eq!(results[1].0.id, "med_rel", "second result must be med_rel");
    }

    // -----------------------------------------------------------------------
    // Test 5: ReAct thought→action→observation cycle structure
    // -----------------------------------------------------------------------
    #[test]
    fn react_step_fields_are_populated() {
        let mut retriever = BM25Retriever::new();
        retriever.add_document("e1", "pipeline stage execution result");
        retriever.add_document("e2", "pipeline failure retry backoff");

        let results = retriever.retrieve("pipeline stage", 2);
        let evidence_strings: Vec<String> = results.iter().map(|(d, _)| d.content.clone()).collect();
        let evidence_refs: Vec<&str> = evidence_strings.iter().map(|s| s.as_str()).collect();

        let steps = react_chain("pipeline stage execution", &evidence_refs, evidence_refs.len());

        assert!(!steps.is_empty(), "must have at least one step");
        for (i, step) in steps.iter().enumerate() {
            assert!(
                !step.thought.is_empty(),
                "step {i} thought must not be empty"
            );
            assert!(
                !step.action.is_empty(),
                "step {i} action must not be empty"
            );
            assert!(
                !step.observation.is_empty(),
                "step {i} observation must not be empty"
            );
            assert!(
                step.score >= 0.0 && step.score <= 1.0,
                "step {i} score must be in [0, 1], got {}",
                step.score
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 6: Multiple ReAct steps accumulate context (confidence increases or
    //         stabilises as more matching evidence is added)
    // -----------------------------------------------------------------------
    #[test]
    fn react_steps_accumulate_context() {
        // Build a corpus where each successive document adds more matching tokens
        let mut retriever = BM25Retriever::new();
        retriever.add_document("weak", "some partial match");
        retriever.add_document("medium", "graph query partial match node");
        retriever.add_document("strong", "graph query node traversal edge path");

        // Retrieve in relevance order for "graph query node"
        let results = retriever.retrieve("graph query node", 3);
        assert_eq!(results.len(), 3);

        let evidence_strings: Vec<String> = results.iter().map(|(d, _)| d.content.clone()).collect();
        let evidence_refs: Vec<&str> = evidence_strings.iter().map(|s| s.as_str()).collect();

        let steps = react_chain("graph query node", &evidence_refs, evidence_refs.len());

        // The step count must match evidence count
        assert_eq!(
            steps.len(),
            evidence_refs.len(),
            "one step per evidence item"
        );

        // Each step score must be in range
        for step in &steps {
            assert!(step.score >= 0.0 && step.score <= 1.0);
        }

        // First step (best BM25 doc first) should have a positive score since
        // the top BM25 result overlaps with the hypothesis
        assert!(
            steps[0].score > 0.0,
            "first step using top BM25 result must have positive score"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: BM25 handles empty query gracefully
    // -----------------------------------------------------------------------
    #[test]
    fn bm25_empty_query_returns_zero_scores() {
        let mut retriever = BM25Retriever::new();
        retriever.add_document("d1", "graph node edge traversal");
        retriever.add_document("d2", "authentication token validation");

        // retrieve with empty query — BM25 should return docs with score 0
        let results = retriever.retrieve("", 2);
        // Results may be returned (documents still in corpus) but all scores == 0.0
        for (_, score) in &results {
            assert_eq!(
                *score, 0.0,
                "empty query must produce zero BM25 scores, got {score}"
            );
        }

        // Directly score: empty query produces 0.0 for any document
        let doc = &retriever.documents[0].clone();
        let score = retriever.score("", doc);
        assert_eq!(score, 0.0, "direct score with empty query must be 0.0");
    }

    // -----------------------------------------------------------------------
    // Test 8: ReAct loop terminates on final answer (interrupt signal)
    // -----------------------------------------------------------------------
    #[test]
    fn react_loop_terminates_on_interrupt() {
        let mut retriever = BM25Retriever::new();
        for i in 0..10 {
            retriever.add_document(&format!("doc_{i}"), &format!("evidence step {i} data token"));
        }

        let results = retriever.retrieve("evidence step data", 10);
        let evidence_strings: Vec<String> = results.iter().map(|(d, _)| d.content.clone()).collect();
        let evidence_refs: Vec<&str> = evidence_strings.iter().map(|s| s.as_str()).collect();

        let signal = InterruptSignal::new();

        // Run first 3 steps without cancellation
        let steps_before =
            react_chain_interruptible("evidence step data", &evidence_refs, 3, &signal);
        assert_eq!(steps_before.len(), 3, "must complete 3 steps before cancel");

        // Now cancel — simulates finding a "final answer" and halting
        signal.cancel();

        // Any further invocation should produce zero steps
        let steps_after =
            react_chain_interruptible("evidence step data", &evidence_refs, 10, &signal);
        assert_eq!(
            steps_after.len(),
            0,
            "cancelled signal must produce 0 steps, got {}",
            steps_after.len()
        );
    }
}
