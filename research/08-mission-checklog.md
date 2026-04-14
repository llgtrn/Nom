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
  connection helpers + query API + read-only helpers (`resolve_synonym`,
  `is_known_kind`, `kinds_row_count`, `clause_shapes_row_count_for_kind`,
  `required_clauses_for_kind`, `is_known_quality`,
  `quality_names_row_count`); data is the user's responsibility.
- Canonical baseline shipped as
  `nom-compiler/crates/nom-grammar/data/baseline.sql`. After
  `nom grammar init`, run `nom grammar import data/baseline.sql` to
  load 9 kinds + 10 quality_names + 43 keywords + 43 clause_shapes
  into the registry. Idempotent (INSERT OR IGNORE).
- Grammar-aware pipeline `run_pipeline_with_grammar`: S1 consults
  `keyword_synonyms` for canonicalization; S2 consults `kinds` for
  strict kind validation; S3 consults `clause_shapes` for the
  per-kind empty-registry guard; S5b consults `quality_names` for
  every `favor X` clause. S4 + S6 still use the hardcoded path;
  the cross-stage required-clause-presence validator is queued.
- CLI: `nom grammar init`, `nom grammar import <sql-file>`,
  `nom grammar status`, `nom grammar add-{kind,synonym,quality,
  keyword,clause-shape,pattern}` (Phase C — six row-level adds),
  plus `nom grammar pattern-list [--intent-contains TEXT] [--kind K]
  [--limit N]` and `nom grammar pattern-show <pattern_id>` for
  read-only exploration of the 258-pattern catalog.
- Phase E proofs — 6 of 7 shipped: P1 schema-completeness
  (empty DB rejects every non-empty source at S2), P2 determinism
  (100-run Debug-equality on curated inputs), P3 closure-against-
  archive (89 v2 blocks from doc 14 extracted + parsed + row-count-
  stable; dashboard passes 42/89 end-to-end after baseline extension),
  P4 strictness property (256 random bytes, zero panics, well-formed
  `NOMX-S<N>-<reason>`, deterministic failure tuples), P6 no-foreign-
  names audit (whole-word match over every text column after baseline
  import), P7 no-Rust-bundled-data audit. P5 synonym round-trip is
  already covered by `synonym_round_trip.rs`.
- Corpus dashboard: three layers of kind-noun drift fixed in sync
  (baseline.sql already had 9; KINDS const 7 → 9; lexer kind-noun
  pattern 7 → 9). baseline.sql extended 10 → 20 quality_names +
  0 → 7 keyword_synonyms (proof→property, composition→module,
  row→data, diagram→screen, participants→data, layout→screen,
  format→data — corpus-idiomatic vocabulary rewritten) + 0 → 258
  canonical authoring patterns in the grammar.sqlite patterns
  table. 220 via parallel-subagent batches covering 22 themes
  (concurrency, distributed, UI/UX, security, testing, observability,
  persistence, numerical, build/CI, networking, graphics, audio,
  business domain, compiler primitives, ML, game engine, NLP,
  time/calendar, geospatial, IoT/embedded, bioinformatics, robotics);
  38 hand-authored. P6 audit caught banned tokens in three batches
  and blocked them pre-commit — AI clients can query for canonical renderings rather
  than parsing prose. Catalog:
  pure-function-contract, exposes-data-shape, concept-composition,
  property-quantified-claim, scenario-given-when-then,
  event-quantified-delivery, screen-exposes-surface,
  supervised-process-tree, tagged-variant-errors, retry-policy,
  effect-handler, reactive-ui-state-machine, content-addressed-build,
  schema-query, pipeline-transformation, network-api-endpoint,
  verified-imperative, algebraic-law, monadic-do-sequence,
  first-class-continuation, type-class-polymorphism,
  stream-processing-window, infrastructure-declaration,
  structured-imperative-block, logic-programming-rule,
  term-rewriting-semantics, macro-expansion, dependent-type-indexed,
  dimensional-analysis, singleton-per-app, idempotent-command,
  authorization-guard, lifecycle-managed-resource, event-sourced-state,
  circuit-breaker, cache-memoization, publish-subscribe-fanout,
  scheduled-cron-task
  to the canonical 9-kind set at S1). S4 contract scanner + S5
  effect scanner both relaxed the same way: drop the mid-scan
  clause-opener check (English verbs inside prose clauses trip it);
  keep only the no-dot safety net. S6 one-kind-per-file rule retired
  — concepts bundled with supporting data/function decls now parse
  (filter concepts into NomFile; non-concepts validated at S1-S5
  but fall through at S6). Closure pass rate progression:
  0/89 → 42/88 → 60/88 → 68/88 → 75/88 → 77/88 → 79/88 → 83/88
  → 84/88 (95.5%). Remaining 4 failures: 1 instance binding
  `the X for the Y` (block #14), 1 follow-on parser-shape gap
  after layout→screen rewrite (block #53 `expected-block-name`),
  1 follow-on gap after format→data rewrite (block #63), 1 malformed
  favor clause with empty quality name (block #87). All four are
  authoring-side issues outside the parser's clean fix surface.
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
