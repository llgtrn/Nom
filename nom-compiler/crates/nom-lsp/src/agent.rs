//! Slice-6a: "why-this-Nom?" agentic drill-through as pure functions.
//!
//! Drives the full agentic-RAG loop on a prose input and formats the
//! transcript as LSP-friendly markdown. No LSP request-handler wiring
//! yet — slice-6b hooks this into `textDocument/codeAction` and a
//! custom `nom/whyThisNom` command.
//!
//! Pipeline:
//!
//! 1. Open `NomDict::open_in_place(dict_path)`
//! 2. Wrap `DictTools::new(&dict)` in `InstrumentedTools` (log every call)
//! 3. Run `classify_with_react(prose, budget, nom_cli_adapter, tools)`
//! 4. Format transcript + instrumentation log as markdown
//!
//! The result is a `lsp_types::MarkupContent` ready to be returned as
//! a Hover response or as a CodeAction command result. Editors display
//! it directly; no further post-processing needed.
//!
//! Determinism: `NomCliAdapter` + `DictTools` are both deterministic
//! over dict state, so the markdown is byte-identical for the same
//! `(prose, dict)` pair. Glass-box report invariant preserved.

use std::path::Path;

use lsp_types::{MarkupContent, MarkupKind};
use nom_intent::adapters::NomCliAdapter;
use nom_intent::dict_tools::DictTools;
use nom_intent::instrumented::{InstrumentedTools, LogEntry};
use nom_intent::react::{
    classify_with_react, ReActAdapter, ReActBudget, ReActLlmFn, ReActStep,
};

/// Errors returned by `render_agent_transcript`. Structured so callers
/// (LSP handlers, `nom lsp why` CLI in slice-6c) can match variants
/// rather than parse strings.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("cannot open dict at {path}: {message}")]
    OpenDict { path: String, message: String },
    #[error("agent loop error: {0}")]
    Loop(nom_intent::IntentError),
}

/// Drive the agentic ReAct loop against `dict_path` with `prose`,
/// return markdown-formatted transcript.
///
/// `budget` caps iterations; pass `ReActBudget::default()` unless the
/// caller needs a specific limit (editors typically want ≤ 3 to stay
/// responsive).
pub fn render_agent_transcript(
    prose: &str,
    dict_path: &Path,
    budget: &ReActBudget,
) -> Result<MarkupContent, AgentError> {
    let dict =
        nom_dict::NomDict::open_in_place(dict_path).map_err(|e| AgentError::OpenDict {
            path: dict_path.display().to_string(),
            message: e.to_string(),
        })?;
    let dict_tools = DictTools::new(&dict);
    let instrumented = InstrumentedTools::new(&dict_tools);
    let adapter = NomCliAdapter::new();
    let llm: ReActLlmFn = Box::new(move |p, t| adapter.next_step(p, t));
    let transcript = classify_with_react(prose, budget, &llm, &instrumented)
        .map_err(AgentError::Loop)?;
    let log = instrumented.entries();
    Ok(MarkupContent {
        kind: MarkupKind::Markdown,
        value: format_markdown(prose, &transcript, &log),
    })
}

/// Format the transcript + tool log as markdown. Layout:
///
/// ```markdown
/// # Why this Nom?
///
/// Prose: `<prose>`
///
/// ## ReAct transcript (N steps)
/// 1. **Thought** — …
/// 2. **Action** — Query { ... }
/// 3. **Observation** — Candidates(...)
/// 4. **Answer** — Symbol("add")
///
/// ## Tool log (M calls)
/// - `query(subject="two", kind=None, depth=0)` → Candidates([]) · 12μs
/// - …
/// ```
///
/// Separated out so slice-6b's request handler + slice-6c's CLI can
/// share the formatter. `pub(crate)` not `pub` because future slices
/// may iterate the markdown shape without breaking external callers.
pub(crate) fn format_markdown(
    prose: &str,
    transcript: &[ReActStep],
    log: &[LogEntry],
) -> String {
    let mut md = String::new();
    md.push_str("# Why this Nom?\n\n");
    md.push_str(&format!("Prose: `{}`\n\n", prose.replace('`', "\\`")));
    md.push_str(&format!(
        "## ReAct transcript ({} step{})\n",
        transcript.len(),
        if transcript.len() == 1 { "" } else { "s" }
    ));
    for (i, step) in transcript.iter().enumerate() {
        md.push_str(&format!("{}. {}\n", i + 1, format_step(step)));
    }
    md.push_str(&format!(
        "\n## Tool log ({} call{})\n",
        log.len(),
        if log.len() == 1 { "" } else { "s" }
    ));
    for entry in log {
        md.push_str(&format!(
            "- `{}` → `{}` · {}μs\n",
            format_call(&entry.call),
            format_observation_brief(&entry.observation),
            entry.duration_us,
        ));
    }
    md
}

fn format_step(step: &ReActStep) -> String {
    match step {
        ReActStep::Thought(t) => format!("**Thought** — {}", shorten(t, 120)),
        ReActStep::Action(a) => format!("**Action** — `{}`", format_action(a)),
        ReActStep::Observation(o) => format!(
            "**Observation** — `{}`",
            format_observation_brief(o)
        ),
        ReActStep::Answer(intent) => {
            format!("**Answer** — `{}`", format_intent(intent))
        }
        ReActStep::Reject(reason) => format!("**Reject** — `{reason:?}`"),
    }
}

fn format_action(action: &nom_intent::react::AgentAction) -> String {
    use nom_intent::react::AgentAction as A;
    match action {
        A::Query { subject, kind, depth } => format!(
            "query(subject={subject:?}, kind={:?}, depth={depth})",
            kind
        ),
        A::Compose { prose, context } => format!(
            "compose(prose={:?}, context=[{} uids])",
            shorten(prose, 40),
            context.len()
        ),
        A::Verify { target } => format!("verify({target:?})"),
        A::Render { uid, target } => format!("render({uid:?}, target={target:?})"),
        A::Explain { uid, depth } => format!("explain({uid:?}, depth={depth})"),
    }
}

fn format_observation_brief(obs: &nom_intent::react::Observation) -> String {
    use nom_intent::react::Observation as O;
    match obs {
        O::Candidates(c) => format!("Candidates[{}]", c.len()),
        O::Proposal(i) => format!("Proposal({})", format_intent(i)),
        O::Verdict { passed, failures, warnings } => format!(
            "Verdict{{passed={passed}, failures={}, warnings={}}}",
            failures.len(),
            warnings.len()
        ),
        O::Rendered { target, bytes_hash } => format!(
            "Rendered{{target={target:?}, bytes_hash={}…}}",
            &bytes_hash[..12.min(bytes_hash.len())]
        ),
        O::Explanation { summary } => {
            format!("Explanation({})", shorten(summary, 60))
        }
        O::Error(e) => format!("Error({})", shorten(e, 60)),
    }
}

fn format_intent(intent: &nom_intent::NomIntent) -> String {
    use nom_intent::NomIntent as I;
    match intent {
        I::Kind(s) => format!("Kind({s:?})"),
        I::Symbol(s) => format!("Symbol({s:?})"),
        I::Flow(s) => format!("Flow({s:?})"),
        I::Reject(r) => format!("Reject({r:?})"),
    }
}

fn format_call(call: &nom_intent::instrumented::CallKind) -> String {
    use nom_intent::instrumented::CallKind as C;
    match call {
        C::Query { subject, kind, depth } => format!(
            "query({subject:?}, kind={:?}, depth={depth})",
            kind
        ),
        C::Compose { prose, context } => format!(
            "compose({:?}, [{} uids])",
            shorten(prose, 40),
            context.len()
        ),
        C::Verify { target } => format!("verify({target:?})"),
        C::Render { uid, target } => format!("render({uid:?}, {target:?})"),
        C::Explain { uid, depth } => format!("explain({uid:?}, {depth})"),
    }
}

fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_intent::react::{AgentAction, Observation};
    use nom_intent::{instrumented::CallKind, NomIntent, Reason};

    #[test]
    fn format_markdown_has_why_this_nom_header() {
        let md = format_markdown("hello", &[], &[]);
        assert!(md.starts_with("# Why this Nom?"));
        assert!(md.contains("Prose: `hello`"));
    }

    #[test]
    fn format_markdown_includes_transcript_and_tool_log_counts() {
        let transcript = vec![
            ReActStep::Thought("look up".into()),
            ReActStep::Answer(NomIntent::Symbol("add".into())),
        ];
        let log = vec![LogEntry {
            call: CallKind::Query {
                subject: "two".into(),
                kind: None,
                depth: 0,
            },
            observation: Observation::Candidates(vec![]),
            duration_us: 12,
        }];
        let md = format_markdown("add two numbers", &transcript, &log);
        assert!(md.contains("ReAct transcript (2 steps)"));
        assert!(md.contains("Tool log (1 call)"));
        assert!(md.contains("**Thought**"));
        assert!(md.contains("**Answer**"));
        assert!(md.contains("Symbol(\"add\")"));
        assert!(md.contains("· 12μs"));
    }

    #[test]
    fn format_markdown_empty_transcript_still_renders() {
        let md = format_markdown("", &[], &[]);
        assert!(md.contains("0 steps"));
        assert!(md.contains("0 calls"));
    }

    #[test]
    fn format_step_covers_all_variants() {
        use nom_intent::react::AgentAction as A;
        let steps = vec![
            ReActStep::Thought("t".into()),
            ReActStep::Action(A::Query {
                subject: "x".into(),
                kind: Some("function".into()),
                depth: 1,
            }),
            ReActStep::Observation(Observation::Candidates(vec![])),
            ReActStep::Answer(NomIntent::Kind("app".into())),
            ReActStep::Reject(Reason::Unparseable),
        ];
        for s in &steps {
            let out = format_step(s);
            assert!(!out.is_empty());
            assert!(
                out.contains("Thought")
                    || out.contains("Action")
                    || out.contains("Observation")
                    || out.contains("Answer")
                    || out.contains("Reject"),
                "step {s:?} produced unexpected markdown: {out}"
            );
        }
    }

    #[test]
    fn shorten_leaves_short_strings_unchanged() {
        assert_eq!(shorten("abc", 10), "abc");
        let long = "a".repeat(100);
        let out = shorten(&long, 10);
        assert!(out.ends_with('…'));
        assert!(out.chars().count() <= 11);
    }

    #[test]
    fn backticks_in_prose_are_escaped() {
        let md = format_markdown("has a `backtick`", &[], &[]);
        // Original unescaped backticks would break the `Prose: <x>` code-span.
        assert!(md.contains("\\`backtick\\`"));
    }
}
