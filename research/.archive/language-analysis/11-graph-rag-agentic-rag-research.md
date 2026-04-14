# 11 — Graph RAG + Agentic RAG + ReAct research synthesis

**Date:** 2026-04-14
**Purpose:** Ground the Graph-RAG/Agentic-RAG/ReAct mission ([memory](../../.. too big to link; see MEMORY.md)) in 2026-era best practices before the brainstorm + design cycle. Three WebSearch pulls covering (1) Microsoft GraphRAG family, (2) Agentic RAG (CRAG, Self-RAG), (3) ReAct + Reflexion + LATS. Synthesized for applicability to Nom's 30-crate workspace.

## 1. GraphRAG family (Microsoft lineage + derivatives)

### Core mechanism
- **Leiden algorithm** for hierarchical community detection. Groups entities into nested communities of "more-connected-internally than externally" nodes.
- **Build time**: extract entities from text → construct graph → detect Leiden communities → summarize each community with an LLM → store summaries.
- **Query time**:
  - **Global Search**: ranks community summaries against the query, synthesizes across top matches. For "themes across the whole corpus" questions.
  - **Local Search**: starts from named entities, expands to graph neighbors. For "what about X and its neighborhood" questions.
  - **DRIFT Search**: combines global + local. Query needing both breadth and depth.

### 2026 optimizations
- **Dynamic community selection**: at query time, skip summarizing deep branches the query is unlikely to hit. Cheaper LLM for relevance gating.
- **LazyGraphRAG (June 2025)**: reduced indexing cost to **0.1%** of original by deferring summarization to query time. Index = just the graph + community assignments; summaries materialized on demand.
- **Cost calibration**: full GraphRAG indexing is $20-500 per typical corpus vs $2-5 for vector RAG. LazyGraphRAG closes this gap.

### What Nom already has
- `nom-graph::NomtuGraph::detect_communities` ships a label-propagation clustering (weaker than Leiden but same shape).
- `nom-concept::ConceptGraph::resolve_closure` is structurally Local Search.
- `nom-cli::cmd_build_report` is the closest analog to a "community summary" but per-entry, not per-community.

### Applicable to Nom
- **Global Search → `nom rag global`** (new CLI): query against community summaries of the dict. Needs a `community_summaries` side-table.
- **Local Search → existing `resolve_closure`**: already implemented; just needs an agent-facing wrapper.
- **LazyGraphRAG discipline**: don't pre-summarize communities at `nom store sync` time. Summarize at query time and cache. Massively cheaper; aligns with doc 04 §10.3.1 "no LLM in build path" since summarization happens outside the build loop.

## 2. Agentic RAG (CRAG, Self-RAG)

### Core mechanism
Flow: **retrieve → evaluate → decide (answer OR re-retrieve)**. The agent owns the loop; the retriever is one tool among several.

### CRAG (Corrective RAG)
Before generating, the system checks whether the retrieved set is "good enough":
- If good → generate.
- If weak → trigger another retrieval pass (maybe with different filters / broader scope / alternative index).
- Reduces hallucinations by keeping answers tied to stronger evidence.

### Self-RAG
Model critiques + refines its own output via reflection. Key insight: the critique step is a separate LLM call with a narrow "evaluator" prompt, not the same prompt that generated.

### Production pain points (2026 field reports)
- **Latency**: 3-4 agentic loops = **10+ seconds** per query.
- **Cost**: every iteration = another LLM call. Tactic: use cheaper model for routine steps, expensive only for synthesis.
- **Reliability / complexity / overhead**: named as the 5 challenges scaling agentic RAG (Google Cloud 2025 ROI report: 52% of enterprises using GenAI run AI agents in production, 88% positive ROI).

### What Nom already has
- **`nom-intent` M8 slice-1** (shipped `800baea`): `NomIntent::Reject(Reason)` arm **IS the CRAG "weak retrieval" signal** in miniature. The evaluator has already been seeded.
- **MECE validator** (`c63a6a7`/`bcadcb3`/`4307c5a`): structurally a Self-RAG critique step — rejects compositions that fail validation.
- **Glass-box report** (M1): the "show your reasoning" output Self-RAG's critique step consumes.
- **`nom-extract` Annotator trait** (shipped `7caa41f` + `9928187`): `requires()` / `requirements_satisfied()` contract is **exactly the tool-description shape** ReAct agents need (see §3).

### Applicable to Nom
- **CRAG loop in `nom-intent`**: extend slice-1's `classify()` with a `retry_with_broader_candidates()` path when the LLM returns `Reject(UnknownSymbol)`. Natural slice-2 of M8.
- **Self-RAG critique step**: reuse the MECE validator as the critique — if an agent proposes a composition that fails MECE, the agent retries with the unmet axes named explicitly.

## 3. ReAct + Reflexion + LATS

### ReAct (Yao et al. 2023)
**Thought → Action → Observation** loop. Agent emits interleaved reasoning steps + tool calls + tool results. Repeats until it reaches an answer.

### Reflexion
Extends ReAct with **verbal self-critique grounded in external data**. Agent enumerates superfluous + missing aspects of its response → reflection → next attempt. Forced citation requirements make reflections constructive rather than hand-waving.

### LATS — Language Agent Tree Search (Zhou et al.)
Combines reflection + **Monte-Carlo Tree Search over reasoning traces**. Beats ReAct, Reflexion, and Tree-of-Thoughts on overall task performance. Explores branches speculatively; commits to the highest-evaluation branch.

### Tool-calling best practices
- **Don't provide 20+ tools** — confuses the agent. Group related functionality rather than one tool per operation.
- **Iterative LLM calls cost** — use cheap models for routine, expensive for synthesis.
- **Limit iterations** — hard cap at 4-5 steps with explicit "max_iterations" parameter.
- **Careful prompt engineering for action format** — model needs to know exactly how to format thoughts + actions so the parser can dispatch.

### Critical implication for Nom
**Nom has 30 crates**. Exposing each as a separate tool = immediate anti-pattern. The research literature is unanimous: **group by capability, not by module**. Likely grouping:

1. **Query** (retrieve nomtu): `nom-dict::find_word_v2`, `nom-dict::find_words_v2_by_kind`, `nom-concept::resolve_closure`, `nom-graph::detect_communities`, `nom-search::*` → ONE tool: `query(subject: &str, kind: Option<Kind>, depth: usize)`.
2. **Compose** (propose a Nom): `nom-intent::classify`, `nom-extract::parse_and_extract`, `nom-concept::MECE` → ONE tool: `compose(prose: &str, context: Vec<Uid>)`.
3. **Verify** (judge a Nom): `nom-verifier::*`, `nom-security::audit`, MECE validator → ONE tool: `verify(uid_or_draft)`.
4. **Render** (emit artifact): `nom-codegen`, `nom-llvm`, `nom-app::build`, `nom-media::render` → ONE tool: `render(uid, target)`.
5. **Report** (explain): `nom-cli::cmd_build_report`, glass-box + LayeredDreamReport → ONE tool: `explain(uid, depth)`.

**5 tools, not 30.** Each maps to multiple underlying crates.

## 4. Consolidated design implications for Nom

| Principle | Source | Nom instance |
|---|---|---|
| Group tools by capability, not module | ReAct best practices | 5 agent tools covering 30 crates |
| Don't pre-summarize; lazy at query time | LazyGraphRAG | Community summaries computed on demand; dict stays compile-time-deterministic |
| Critique is separate from generate | Self-RAG | MECE validator reused as critique step; glass-box report is the "show your work" artifact |
| Re-query on weak retrieval | CRAG | `NomIntent::Reject(Reason)` already shipped — wire retry loop |
| Hard-cap iterations | Production reports | 4-5 step budget in `nom-intent`; `Reject(IterationBudgetExhausted)` variant |
| Cheap model for routine, expensive for synthesis | ReAct cost tactics | Retrieval = cheap; final composition = expensive |
| Structured action format (parseable) | ReAct | Already solved: `NomIntent` enum is the bounded action space |
| Global + Local + hybrid search | GraphRAG DRIFT | `nom rag global`, `nom rag local`, `nom rag drift` subcommands |

## 5. Brainstorm input (for the next cycle)

The three approach shapes for the Graph-RAG mission, informed by the research:

### α — Thin `nom-rag` crate (new workspace member)
- `nom-rag` depends on `nom-dict` + `nom-graph` + `nom-intent`
- Exposes the 5 grouped tools above as a `trait AgentTool`
- Implements CRAG retrieve-then-evaluate loop
- **Cost**: one new crate + integration churn
- **Win**: clean seam, independently testable

### β — Extend `nom-intent` with `ReActStep` enum (compounds on shipped M8 slice-1)
- Add `pub enum ReActStep { Thought(String), Action(NomIntent), Observation(String), Answer(Uid) }`
- Extend `classify()` into `classify_with_react(prose, max_iters, llm_fn)` returning `Vec<ReActStep>` ending in `Answer` or `Reject`
- Grouped tools stay as functions in `nom-intent`
- **Cost**: ~1 new module, no new crate
- **Win**: compounds on what already ships; no dependency reshuffling

### γ — Per-crate retrieval handlers + coordinator in `nom-resolver`
- Each crate gets a `pub fn as_agent_tool() -> AgentTool` function
- `nom-resolver` hosts the loop + tool dispatcher
- **Cost**: 30 crates × boilerplate; violates "don't provide 20+ tools"
- **Win**: distributed — but this IS the anti-pattern the ReAct literature warns against

### Recommendation (per pre-authorized feedback note)
**β** — extend `nom-intent`. Reasons:
1. M8 slice-1 already ships the bounded-output discipline (Reject arm = CRAG signal).
2. The "5 tools not 30" principle means tools are FUNCTIONS, not crates — no distributed coordination needed.
3. Zero new Cargo.toml member; incremental wedge pattern continues.
4. Tests reuse the existing `LlmFn` closure stub machinery.

Next cycle: formal brainstorm-skill design round on approach β, then spec to `docs/superpowers/specs/2026-04-14-graph-rag-agentic-design.md`.

## Sources
- [Microsoft GraphRAG main site](https://microsoft.github.io/graphrag/)
- [Microsoft GraphRAG github](https://github.com/microsoft/graphrag)
- [GraphRAG dynamic community selection](https://www.microsoft.com/en-us/research/blog/graphrag-improving-global-search-via-dynamic-community-selection/)
- [Graph RAG Survey (ACM)](https://dl.acm.org/doi/10.1145/3777378)
- [AgenticRAG Survey (arxiv 2501.09136)](https://arxiv.org/abs/2501.09136)
- [AgenticRAG-Survey github](https://github.com/asinghcsu/AgenticRAG-Survey)
- [Weaviate: What Is Agentic RAG](https://weaviate.io/blog/what-is-agentic-rag)
- [IBM: What is a ReAct Agent](https://www.ibm.com/think/topics/react-agent)
- [LangChain: Reflection Agents](https://blog.langchain.com/reflection-agents/)
- [Agentic Design Patterns: ReAct, ReWOO, CodeAct, Beyond](https://capabl.in/blog/agentic-ai-design-patterns-react-rewoo-codeact-and-beyond)
