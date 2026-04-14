# 08 — Mission Checklog

Single moving snapshot. When something changes, this file is rewritten in
place — no append-only ledger, no per-cycle entries.

## Shipped (current code state)

- Workspace + ~30 crates including `nom-grammar`, `nom-dict`, `nom-concept`,
  `nom-parser`, `nom-llvm`, `nom-cli`, `nom-corpus`, `nom-lsp`, `nom-intent`,
  `nom-app`.
- Self-hosting lexer compiles end-to-end via the LLVM backend.
- Three-DB layout: `~/.nom/{concepts,entities,grammar}.sqlite` plus the
  artifact store at `~/.nom/store/<hash>/body.{bc,avif,...}`.
- `Dict { concepts, entities }` struct with per-tier connections; `open_dir
  / open_paths / open_in_memory` constructors.
- Five `entities`-tier free functions on `&Dict`: `upsert_entity`,
  `find_entity`, `find_entities_by_word`, `find_entities_by_kind`,
  `count_entities`.
- Per-tier specialised schemas: `CONCEPTS_SCHEMA_SQL` (DB1 only),
  `ENTITIES_SCHEMA_SQL` (DB2 only); cross-file FKs absent.
- Grammar registry shipped (`nom-grammar` crate) with five tables:
  `keywords` (45 rows), `clause_shapes` (46 rows), `kinds` (9 rows),
  `quality_names` (10 rows), `patterns` (10 rows).
- `nom grammar {init,seed,status}` CLI subcommands.
- Annotator-style staged pipeline (S1-S6) shipped.
- W4 strictness lane: A1/A2/A3/A4/A6 closed.
- Effect valence (boon/hazard) parsed and verified.
- MECE validator on agent-demo concept.
- LSP slice 1-6 shipped: stdio server, classify CLI, agentic-RAG markdown
  rendering, executeCommand handler, ReAct adapter trait with stub + MCP +
  NomCli concrete impls.

## In flight

- **Pattern migration** — distill every captured insight from the archived
  translation corpus into `grammar.sqlite.patterns` rows that meet the
  grammar-for-synthesis quality bar (parser-acceptable example_shape +
  build-stage-checkable hazards + uniquely matched intent). 10 of ~100-150
  founding rows shipped.
- **Dict-split S3b–S8** — port the remaining ~35 nom-dict functions to
  free functions on `&Dict`; delete legacy `NomDict` struct + `entries`
  table + `concepts` legacy table + V2/V3/V4/V5_SCHEMA_SQL constants in
  the same commit as the native replacement ships.
- **`.nomx` v1 + v2 merge** — single canonical source format; v1 vs v2
  parser-path distinction deleted.
- **`quality_names.metric_function`** — currently nullable awaiting `nom
  corpus register-axis` CLI.
- **Embedding index** — `nom corpus embed` populates per-kind embeddings
  for the resolver's semantic re-rank, replacing the alphabetical-smallest
  stub.
- **Real planner-in-Nom** — replaces stubbed planner.

## Planned

- Phase 6 — parser-in-Nom (Stage-1 self-host).
- Phase 7 — resolver + verifier in Nom (Stage-2).
- Phase 9 LSP MVP — full hover / completion / goto-def with embedding-driven
  candidate ranking.
- 100-repo corpus pilot — DB carries function/concept/entity rows with
  placeholder stubs for missing items; crashes fixed before advancing to
  500 repos.
- Mass corpus ingestion — top-N per ecosystem with stream-and-discard disk
  discipline.
- Aesthetic backends — image / audio / video / 3D / typography rendering as
  body_kind targets in the artifact store.

## Aspirational

- Phase 10 bootstrap — fixpoint Stage-0 → Stage-1 → Stage-2 → Stage-3
  byte-identical proof of self-hosting; parity track ≥99% IR + 100% runtime
  correctness; default-flip + 3-month grace + archive of host language.
- Phase 11 mathematics-as-language — algebraic laws + dimensional analysis
  on function decls; cross-domain composition verification.
- Phase 12 closure-level specialization — 70-95% per-platform binary
  reduction via joint multi-app × multi-platform optimization (bipartite
  min-cost assignment; benchmark-driven).
- Universal knowledge composition — closed kind set extends to scientific
  knowledge primitives composed via the same operators.

## Blockers / open

- Disk full has hit the build environment in the past. Pre-cycle discipline:
  check available disk before cargo invocations on Windows.
- Embedding index requires network access for corpus pulls; offline
  cycles can only progress on dict-side or pattern-side work.
