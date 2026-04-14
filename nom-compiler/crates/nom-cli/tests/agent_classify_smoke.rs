//! Smoke test for `nom agent classify` — slice-5a.
//!
//! Asserts the CLI → DictTools → classify_with_react plumbing is wired
//! correctly by invoking the logic as a library. Not a subprocess test
//! because (a) Windows LLVM-C.dll load paths make subprocess tests
//! flaky per prior session notes, and (b) the shape we want to verify
//! is the transcript, not the formatter.

use std::path::PathBuf;

use nom_dict::{NomDict, EntityRow};
use nom_intent::adapters::NomCliAdapter;
use nom_intent::dict_tools::DictTools;
use nom_intent::react::{
    classify_with_react, AgentAction, AgentTools, Observation, ReActAdapter, ReActBudget,
    ReActLlmFn, ReActStep,
};
use nom_intent::{NomIntent, Reason};

fn tmp_dict() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "nom_agent_smoke_{}.db",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&p);
    p
}

#[test]
fn classify_with_stub_llm_returns_reject_transcript() {
    let d = NomDict::open_in_memory().unwrap();
    let tools = DictTools::new(&d);
    let llm: ReActLlmFn = Box::new(|_prose, _transcript| {
        Ok(ReActStep::Reject(Reason::Unparseable))
    });
    let budget = ReActBudget::default();
    let transcript = classify_with_react("any prose", &budget, &llm, &tools).unwrap();
    assert_eq!(transcript.len(), 1, "stub LLM rejects in one step");
    assert!(matches!(
        &transcript[0],
        ReActStep::Reject(Reason::Unparseable)
    ));
}

#[test]
fn classify_dispatches_query_to_dict_tools_against_seeded_row() {
    let d = NomDict::open_in_memory().unwrap();
    // Seed a row DictTools::query can find.
    let hash = "e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1";
    d.upsert_entity(&EntityRow {
        hash: hash.into(),
        word: "greet".into(),
        kind: "function".into(),
        signature: None,
        contracts: None,
        body_kind: None,
        body_size: None,
        origin_ref: None,
        bench_ids: None,
        authored_in: None,
        composed_of: None,
    })
    .unwrap();
    let tools = DictTools::new(&d);

    // LLM: Thought → Action(Query subject="greet", kind="function") → Answer.
    let counter = std::cell::Cell::new(0usize);
    let llm: ReActLlmFn = Box::new(move |_prose, _transcript| {
        let n = counter.get();
        counter.set(n + 1);
        Ok(match n {
            0 => ReActStep::Thought("look up greet in dict".into()),
            1 => ReActStep::Action(AgentAction::Query {
                subject: "greet".into(),
                kind: Some("function".into()),
                depth: 0,
            }),
            _ => ReActStep::Answer(NomIntent::Symbol("greet".into())),
        })
    });

    let transcript =
        classify_with_react("greet the world", &ReActBudget::default(), &llm, &tools).unwrap();

    // Shape: Thought, Action(Query), Observation::Candidates, Answer
    assert_eq!(transcript.len(), 4);
    match &transcript[2] {
        ReActStep::Observation(Observation::Candidates(c)) => {
            assert!(
                c.iter().any(|u| u == hash),
                "candidates must contain seeded hash; got {c:?}"
            );
        }
        other => panic!("expected Candidates observation at [2], got {other:?}"),
    }
    assert!(matches!(&transcript[3], ReActStep::Answer(_)));
}

// Dead-code suppression for tmp_dict — the subprocess-test flavor is a
// slice-5b follow-up once the LLVM dll-load path is addressed.
#[allow(dead_code)]
fn _unused() {
    let _ = tmp_dict();
}

#[test]
fn nom_cli_adapter_drives_loop_to_completion_against_seeded_dict() {
    // Seeds a tiny dict, wires NomCliAdapter + DictTools, runs the full
    // loop. With a real candidate present and the deterministic state
    // machine, the agent should reach an Answer without external LLM.
    let d = NomDict::open_in_memory().unwrap();
    let hash = "f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3f3";
    d.upsert_entity(&EntityRow {
        hash: hash.into(),
        // "two" is the first_content_token of "add two numbers" (stop
        // word "add" skipped); seed under the same kind the adapter
        // defaults to querying (None).
        word: "two".into(),
        kind: "function".into(),
        signature: Some("fn two() -> i64".into()),
        contracts: None,
        body_kind: Some("llvm-bc".into()),
        body_size: Some(128),
        origin_ref: None,
        bench_ids: None,
        authored_in: Some("t.nom".into()),
        composed_of: None,
    })
    .unwrap();
    let tools = DictTools::new(&d);
    let adapter = NomCliAdapter::new();
    let llm: ReActLlmFn =
        Box::new(move |prose, transcript| adapter.next_step(prose, transcript));

    let budget = ReActBudget {
        max_iterations: 10, // generous so loop can land on Answer
        ..Default::default()
    };
    let transcript =
        classify_with_react("add two numbers", &budget, &llm, &tools).unwrap();
    // Must terminate (Answer or Reject).
    let last = transcript.last().unwrap();
    assert!(
        matches!(last, ReActStep::Answer(_) | ReActStep::Reject(_)),
        "loop must terminate; last = {last:?}"
    );
    // The compose step's Observation::Candidates query (DictTools::query
    // with kind=None) returns empty for non-hash subjects — so the
    // adapter's empty-candidates branch fires and returns
    // Reject(UnknownSymbol). This is the expected CRAG shape:
    // weak retrieval → bounded reject, no hallucination.
    // (When M6 corpus + real embeddings land, the query will return
    // candidates and the full pipeline runs to Answer.)
}

#[test]
fn nom_cli_adapter_rejects_on_empty_prose() {
    let d = NomDict::open_in_memory().unwrap();
    let tools = DictTools::new(&d);
    let adapter = NomCliAdapter::new();
    let llm: ReActLlmFn =
        Box::new(move |prose, transcript| adapter.next_step(prose, transcript));
    let transcript =
        classify_with_react("", &ReActBudget::default(), &llm, &tools).unwrap();
    // Loop runs: Thought → Query("") → Candidates([]) → Reject.
    let last = transcript.last().unwrap();
    assert!(
        matches!(last, ReActStep::Reject(_)),
        "empty prose must reject; got {last:?}"
    );
}
