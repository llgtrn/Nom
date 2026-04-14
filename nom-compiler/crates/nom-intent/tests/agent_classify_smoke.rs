//! T2.2 first slice — agent_classify_smoke
//!
//! End-to-end "AI invokes compiler" loop: prose → ReAct driver →
//! `DictTools::query` → `DictTools::render` → `Answer(Symbol)`. Proves
//! the render path is wired through the full classify_with_react driver
//! (not just the standalone `tools.render(uid, target)` unit tests in
//! `dict_tools::tests`).
//!
//! Per the approved plan T2.2: "agent_classify_smoke test extended to
//! assert a successful end-to-end render of a single nomtu against a
//! target." The assertion shape:
//!
//!   1. classify_with_react drives the loop with a stub LLM that issues
//!      Query, then Render, then Answer.
//!   2. The Render observation must be `Observation::Rendered { … }` —
//!      not `Observation::Error`.
//!   3. The render-plan hash must be a 64-char hex string (deterministic
//!      SHA-256 of the closure walk).
//!   4. The transcript ends with `Answer(Symbol("add"))`, proving the
//!      driver propagates the LLM's terminal step through.

use nom_dict::{EntityRow, NomDict};
use nom_intent::dict_tools::DictTools;
use nom_intent::react::{
    AgentAction, Observation, ReActBudget, ReActLlmFn, ReActStep, classify_with_react,
};
use nom_intent::{IntentError, NomIntent};

const HASH_ADD: &str = "a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0";

fn seed_add(d: &NomDict) {
    d.upsert_entity(&EntityRow {
        hash: HASH_ADD.into(),
        word: "add".into(),
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
}

/// Stub LLM that emits a fixed Thought→Action(Query)→[obs]→
/// Action(Render)→[obs]→Answer sequence regardless of the prose. The
/// driver pairs each Thought with the next emitted Action, then auto-
/// dispatches the Action through `DictTools` and appends the
/// Observation. After three iterations the LLM emits Answer.
fn scripted_llm() -> ReActLlmFn {
    let counter = std::cell::Cell::new(0usize);
    Box::new(move |_prose, _transcript| -> Result<ReActStep, IntentError> {
        let n = counter.get();
        counter.set(n + 1);
        Ok(match n {
            0 => ReActStep::Thought("look up add-like functions".into()),
            1 => ReActStep::Action(AgentAction::Query {
                subject: HASH_ADD.into(),
                kind: None,
                depth: 0,
            }),
            2 => ReActStep::Thought("render the resolved closure".into()),
            3 => ReActStep::Action(AgentAction::Render {
                uid: HASH_ADD.into(),
                target: "llvm-native".into(),
            }),
            _ => ReActStep::Answer(NomIntent::Symbol("add".into())),
        })
    })
}

#[test]
fn classify_with_react_drives_dict_tools_render_to_completion() {
    let d = NomDict::open_in_memory().unwrap();
    seed_add(&d);
    let tools = DictTools::new(&d);
    let llm = scripted_llm();
    let budget = ReActBudget {
        max_iterations: 4,
        ..Default::default()
    };

    let transcript = classify_with_react("add two numbers", &budget, &llm, &tools).unwrap();

    // Transcript shape after two Thought+Action+Observation pairs + Answer.
    // 0: Thought, 1: Action(Query), 2: Observation(Candidates),
    // 3: Thought, 4: Action(Render), 5: Observation(Rendered), 6: Answer
    assert_eq!(transcript.len(), 7, "transcript shape mismatch: {transcript:#?}");

    // Render observation must be Rendered { target, bytes_hash } with a
    // valid SHA-256 hex hash. Rejects the error path explicitly.
    match &transcript[5] {
        ReActStep::Observation(Observation::Rendered { target, bytes_hash }) => {
            assert_eq!(target, "llvm-native");
            assert_eq!(bytes_hash.len(), 64, "render plan hash must be SHA-256 hex");
            assert!(bytes_hash.chars().all(|c| c.is_ascii_hexdigit()));
        }
        other => panic!("expected Rendered observation at step 5, got {other:?}"),
    }

    // Driver propagated the terminal Answer.
    match &transcript[6] {
        ReActStep::Answer(NomIntent::Symbol(s)) => assert_eq!(s, "add"),
        other => panic!("expected Answer(Symbol(\"add\")) at step 6, got {other:?}"),
    }
}

/// A render of a uid that isn't in the dict must surface as
/// `Observation::Error`, NOT as a panic or as a falsified Rendered. The
/// ReAct driver does NOT rewrite Errors as Rejects (an unknown uid is a
/// tool-level signal, not a bounded NomIntent failure) — the LLM is
/// expected to handle the Error in its next Thought.
#[test]
fn classify_with_react_render_of_unknown_uid_surfaces_error_observation() {
    let d = NomDict::open_in_memory().unwrap();
    // No seeding — the dict is empty, so any render must fail at lookup.
    let tools = DictTools::new(&d);
    let counter = std::cell::Cell::new(0usize);
    let llm: ReActLlmFn = Box::new(move |_prose, _t| {
        let n = counter.get();
        counter.set(n + 1);
        Ok(match n {
            0 => ReActStep::Thought("try to render a missing uid".into()),
            1 => ReActStep::Action(AgentAction::Render {
                uid: HASH_ADD.into(),
                target: "llvm-native".into(),
            }),
            _ => ReActStep::Answer(NomIntent::Symbol("add".into())),
        })
    });
    let budget = ReActBudget::default();
    let transcript = classify_with_react("missing", &budget, &llm, &tools).unwrap();

    // Step 2 must be the Error observation.
    match &transcript[2] {
        ReActStep::Observation(Observation::Error(msg)) => {
            assert!(msg.contains("not found in dict"), "wrong error: {msg}");
        }
        other => panic!("expected Observation::Error at step 2, got {other:?}"),
    }
}
