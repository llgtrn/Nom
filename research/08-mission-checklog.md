# 08 — Mission Checklog

Single moving snapshot. Rewritten in place every cycle that changes
anything load-bearing.

## Shipped (current code state)

- Workspace + ~30 crates including `nom-grammar`, `nom-dict`,
  `nom-concept`, `nom-parser`, `nom-llvm`, `nom-cli`, `nom-corpus`,
  `nom-lsp`, `nom-intent`, `nom-app`.
- Self-hosting lexer compiles end-to-end through the LLVM backend.
- Three-DB layout under `~/.nom/`: `concepts.sqlite` (DB1) +
  `entities.sqlite` (DB2) + `grammar.sqlite` (registry) plus the
  artifact store at `~/.nom/store/<hash>/body.{bc,avif,...}`.
- `Dict { concepts, entities }` struct with per-tier connections.
  Constructors: `open_dir`, `open_paths`, `open_in_memory`.
- Five `entities`-tier free functions on `&Dict`: `upsert_entity`,
  `find_entity`, `find_entities_by_word`, `find_entities_by_kind`,
  `count_entities`.
- Per-tier specialised schemas: `CONCEPTS_SCHEMA_SQL` and
  `ENTITIES_SCHEMA_SQL`; cross-file foreign keys absent per the
  no-cross-file-FK invariant.
- `grammar.sqlite` schema for seven tables: `schema_meta`,
  `keywords`, `keyword_synonyms`, `clause_shapes`, `kinds`,
  `quality_names`, `patterns`. The Rust crate ships only the schema +
  connection helpers + query API + read-only `resolve_synonym`
  helper; data is the user's responsibility.
- CLI: `nom grammar init` (creates the file + applies schema),
  `nom grammar status` (counts rows per table; supports `--json`).
- Annotator-style staged pipeline (S1–S6) shipped.
- Strictness lane: A1/A2/A3/A4/A6 closed.
- Effect valence (boon / hazard) parsed and verified.
- MECE validator on the agent-demo concept.
- LSP slices 1–6: stdio server, classify CLI, agentic-RAG markdown
  rendering, executeCommand handler, ReAct adapter trait with stub +
  MCP + NomCli concrete impls.

## In flight

- **Grammar-registry row-level CLI** — `nom grammar add-keyword`,
  `nom grammar add-kind`, `nom grammar add-clause-shape`,
  `nom grammar add-quality`, `nom grammar add-pattern`, plus
  `nom grammar import <sql-file>` for batch population. None shipped.
- **Dict-split S3b–S8** — port the remaining ~35 nom-dict functions
  to free functions on `&Dict`; delete legacy `NomDict` struct, legacy
  `entries` table, legacy `concepts` + `concept_members` tables, and
  V2/V3/V4/V5_SCHEMA_SQL constants in the same commit as native
  replacements ship.
- **`.nomx` v1 + v2 merge** — single canonical source format; v1 vs v2
  parser-path distinction deleted.
- **`quality_names.metric_function`** — currently nullable awaiting
  `nom corpus register-axis` CLI.
- **Embedding index** — `nom corpus embed` populates per-kind
  embeddings for the resolver's semantic re-rank, replacing the
  alphabetical-smallest stub.
- **Real planner-in-Nom** — replaces stubbed planner.

## Planned

- Phase 6 — parser-in-Nom (Stage-1 self-host).
- Phase 7 — resolver + verifier in Nom (Stage-2).
- Phase 9 LSP MVP — full hover / completion / goto-def with
  embedding-driven candidate ranking.
- 100-repo corpus pilot — DB carries function/concept/entity rows
  with placeholder stubs for missing items; crashes fixed before
  advancing to 500 repos.
- Mass corpus ingestion — top-N per ecosystem with stream-and-discard
  disk discipline.
- Aesthetic backends — image / audio / video / 3D / typography
  rendering as `body_kind` targets in the artifact store.

## Aspirational

- Phase 10 bootstrap — fixpoint Stage-0 → Stage-1 → Stage-2 →
  Stage-3 byte-identical proof of self-hosting; parity track ≥99%
  IR + 100% runtime correctness; default-flip + 3-month grace +
  archive of host language.
- Phase 11 mathematics-as-language — algebraic laws + dimensional
  analysis on function decls; cross-domain composition verification.
- Phase 12 closure-level specialization — 70-95% per-platform binary
  reduction via joint multi-app × multi-platform optimization
  (bipartite min-cost assignment; benchmark-driven).
- Universal knowledge composition — closed kind set extends to cover
  scientific knowledge primitives composed via the same operators.

## Recent corrections (current snapshot, no per-cycle log)

- `nom-grammar` is awareness-only: schema + connection + query API.
  Zero grammar data inside Rust. Earlier cycles incorrectly bundled
  `KINDS_SEED`, `KEYWORDS_SEED`, `PATTERNS_SEED` const arrays + a
  `seed_all()` function + a `nom grammar seed` CLI subcommand. All
  removed; ~1000 lines of source-bundled data deleted. Grammar.sqlite
  starts empty; the user populates it.
- 35 historical research docs archived under `research/.archive/`;
  10 canonical docs at `research/` root capture mission state per
  the 10-doc ceiling rule.
- Foreign-language names absent from every doc, every commit message
  going forward, every DB row.

## Blockers / open

- Disk full has hit the build environment in the past. Pre-cycle
  discipline: check available disk before cargo invocations.
- Embedding index requires network access for corpus pulls; offline
  cycles can only progress on dict-side, schema-side, or doc-side
  work.
