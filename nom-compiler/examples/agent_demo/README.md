# agent_demo

The smallest demonstration that Nom is built for AI-agent composition.

An LLM asked to compose an agent in Nom can write the `.nom` directly. The
compiler verifies the contracts compose. The resolver maps prose hints to
verified `.nomtu` entries. The result is an agent whose tool boundaries are
auditable, reproducible, and content-addressed.

---

## 1. What this is

This example defines a minimal safe agent (`agent.nom`) that composes six
tools — file I/O, web access, shell execution — under a safety policy
(`policy/safety.nom`). The tools are declared as verified contracts in
`.nomtu` files (`tools/`). No real implementations exist: these are
contract-only stubs. That is intentional and documented below.

The key insight: **the contracts ARE the interface**. An LLM composing an
agent in Nom cannot hallucinate a tool that doesn't exist — the build
refuses to resolve a word it cannot find in the dictionary. Prose `matching
"..."` clauses get pinned to actual hashes after the first `--write-locks`
run. From that point, the agent definition is content-addressed: the same
source always builds the same closure.

---

## 2. The verified-tools insight

Every tool in `tools/` has:

- `requires` clauses: safety preconditions the runtime must enforce.
- `ensures` clauses: result guarantees the tool must honour.

Example from `tools/file_tools.nomtu`:

```
the function read_file is
  given a path of text, returns the file contents as text.
  requires the path is within the allowed workspace root.
  ensures the contents reflect the file at the time of the read.
```

When `agent.nom` references `read_file`, the build walks the closure and
surfaces every `requires`/`ensures` clause that transitively applies. The
agent author sees the full contract surface — not just the function name.

**Hallucination boundary**: the LLM can SUGGEST tool composition by writing
`the function read_file matching "read text from a workspace path"`. The
build accepts the prose hint for the first run. On `--write-locks`, the
resolver pins the actual hash: `the function read_file@<64-hex>`. After
that, the prose hint is locked — the build reproduces identically with no
resolver step needed.

**Word naming**: `read_file`, `write_file`, `list_dir` are canonical
primitives — they do not use the full feature-stack naming convention
(`read_file_text_utf8_strict`, etc.) because there is only one variant of
each in this demo. Feature-stack names would appear when the dictionary
contains specialised variants that differ in encoding, error handling, or
performance profile (doc 08 §6.5).

**Dream-objective ranking**: `agent.nom` ends with `favor security then
composability then speed.` The objectives are order-significant (doc 08
§6.2). The parser preserves this order in `ConceptDecl.objectives`. When
the MECE validator ships (Phase 8), it will verify the objectives are
non-overlapping and ranked consistently across the closure.

---

## 3. Pipeline you can run today

```sh
cd nom-compiler
cargo build -p nom-cli
./target/debug/nom store sync examples/agent_demo
./target/debug/nom build status examples/agent_demo
./target/debug/nom build status examples/agent_demo --write-locks
```

After `--write-locks`:

- `agent.nom` is rewritten so each `the function <name> matching "..."` becomes
  `the function <name>@<64-hex>`.
- `policy/safety.nom` is similarly rewritten for its one reference.
- Re-running sync + status returns a clean result (all words pinned, nothing
  left to resolve).

---

## 4. What the demo shows

- **Contract composition**: six tools, each with at least one `requires`
  clause, compose under one concept. The build walks the full closure and
  surfaces all contracts.
- **Prose-to-hash resolution**: `matching "..."` hints resolve to actual
  dictionary hashes. The resolver is the bridge between what an LLM writes
  and what the compiler verifies.
- **Verified tool boundaries**: after `--write-locks`, every tool reference
  in the agent source is a 64-hex content address. The agent definition is
  reproducible — same source, same DB, same hashes.
- **Safety policy composition**: `agent.nom` includes `the concept
  agent_safety_policy` as a dependency. The safety constraints (workspace
  root, allowed-commands list, https-only) are part of the agent's closure,
  not an afterthought.
- **Dream-objectives ranking**: `favor security then composability then
  speed` is parsed and stored in order. The ranking is available to the
  build report and future optimisers without re-parsing the source.
- **MECE-ready objectives**: the three objectives are typed even though the
  MECE validator is not yet shipped.

---

## 5. What is NOT shown

- **No LLM in the loop** (yet). Phase 9 will host the LLM-author flow under
  `nom author`. In this demo, the agent source is hand-written to show what
  an LLM would produce.
- **No actual tool implementations**. `read_file`, `write_file`, etc. are
  contracts only. Function bodies will land via corpus extraction (motivation
  10 §C). Until then, the contracts define the interface; nothing executes.
- **No runtime sandboxing**. `requires` clauses are documentation-grade
  until the verifier exists (Phase 8). The compiler checks contract
  *presence* but does not enforce them at runtime.
- **No MECE validation**. Objectives are ranked and stored. The validator
  that checks they are non-overlapping is Phase 8 scope.

---

## 6. Why this is the killer app (motivation 16 §5)

From `research/motivation/16-competitive-analysis-and-roadmap.md` §5:

```
CANDIDATE 1: "The language AI agents program in"
    WHY: AI agents need to compose verified actions (tool calls).
         .nomtu = verified tools with contracts.
         .nom = agent plans composed from tools.
         Glass box = auditable agent decisions.
    TIMING: perfect (2026 AI agent boom)
    RISK: agents may just use Python/TypeScript

STRONGEST: Candidate 1 — AI agent composition.
    Because: no other language is designed for this.
    Because: the timing is perfect (agents are the next wave).
    Because: it showcases every Nom advantage naturally.
```

This demo makes those claims concrete:

- `.nomtu` = the six tool files in `tools/`. Each has a contract. Each is
  content-addressed. An LLM can reference any of them by prose hint; the
  build pins the hash.
- `.nom` = `agent.nom`. The agent plan is a concept that composes tools and
  a safety policy. The plan is the source of truth; the build report is the
  audit trail.
- Glass box = after `--write-locks`, every dependency in the agent is a
  content address. The entire closure is reproducible and inspectable. There
  are no hidden side-effects, no version ranges, no "it worked on my
  machine".

No other language gives an LLM a build system that refuses hallucinated
tool names, pins verified contracts, and surfaces the full safety boundary
in one report. That is the Nom proposition.

---

## Files

```
agent_demo/
  agent.nom               root concept: minimal_safe_agent
  policy/
    safety.nom            concept: agent_safety_policy
  tools/
    file_tools.nomtu      3 entities: read_file, write_file, list_dir
    web_tools.nomtu       2 entities: fetch_url, search_web
    shell_tools.nomtu     1 entity:  run_command
```

End-to-end test: `crates/nom-cli/tests/agent_demo_e2e.rs`.
