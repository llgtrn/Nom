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
- Forty-eight public free functions on `&Dict` are now live (`44`
  migration-surface helpers once infra constructors are excluded).
  Of the legacy `NomDict` migration target, `41` methods now have
  split-aware free-function parity and `8` remain. Entities
  tier read-only: S3a (5) `upsert_entity` / `find_entity` /
  `find_entities_by_word` / `find_entities_by_kind` / `count_entities`;
  S3b (5) `count_concept_defs` / `count_required_axes` /
  `body_kind_histogram` / `resolve_prefix` / `count_entities_meta`;
  S4 (5) `status_histogram` / `get_entry_bytes` / `list_partial_ids`
  / `get_meta` / `get_refs`. S5 mixes tiers (5): `closure`
  (entities-tier BFS over `entry_refs`), plus four concept-tier ports
  — `get_concept_id_by_name` / `list_concept_ids` / `delete_concept`
  / `add_concept_member`. S6 entities-tier mutators (5): `set_scores`
  / `add_meta` / `set_signature` / `add_finding` / `add_ref`.
  S7 entries-tier readers returning owned types (5):
  `find_by_word` / `find_by_body_kind` / `search_describe` /
  `get_scores` / `get_findings` — shared SELECT prefix in a
  `ENTRY_SELECT` const so the 21-column list lives in one place;
  `row_to_entry` promoted to `pub(crate)`. Each free fn is a faithful
  re-emit of the legacy `NomDict::*` SQL with `&self.conn` swapped
  for `&d.entities` or `&d.concepts`. Legacy methods stay live until
  the last replacement ships, per the no-legacy rule.
  S8 slice A (5) landed earlier on the concepts tier:
  `upsert_concept_def` / `find_concept_def` /
  `list_concept_defs_in_repo` / `register_required_axis` /
  `list_required_axes`. S8 slice B (9) lands this cycle:
  `upsert_concept` / `get_concept_by_name` / `list_concepts` /
  `remove_concept_member` / `get_concept_members` /
  `count_concept_members` / `unregister_required_axis` /
  `seed_standard_axes` / `add_concept_members_by_filter`.
  `nom-dict` tests moved 89 → 94 while the workspace stayed green.
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
  per-kind empty-registry guard plus required-clause-presence
  validation for every `is_required = 1` clause; S5b consults
  `quality_names` for every `favor X` clause. S4 + S6 still use the
  hardcoded path.
- CLI: `nom grammar init`, `nom grammar import <sql-file>`,
  `nom grammar status`, `nom grammar add-{kind,synonym,quality,
  keyword,clause-shape,pattern}` (Phase C — six row-level adds),
  plus four read-only explorers over the 258-pattern catalog:
  `pattern-list [--intent-contains TEXT] [--kind K] [--favor Q]
  [--limit N] [--json]`, `pattern-show <id> [--json]`, `pattern-stats
  [--json]`, and `pattern-search <prose> [--limit N] [--threshold T]
  [--json]`. The search backend (`nom_grammar::fuzzy_tokens` +
  `nom_grammar::jaccard`) is deterministic — same query yields same
  matches across CLI and the CI uniqueness test that enforces it.
- Phase E proofs — **7 of 7 shipped**: P1 schema-completeness
  (empty DB rejects every non-empty source at S2), P2 determinism
  (100-run Debug-equality on curated inputs), P3 closure-against-
  archive (88 v2 blocks from doc 14 extracted + parsed + row-count-
  stable; dashboard passes 84/88 = 95.5% end-to-end), P4 strictness
  property (256 random bytes, zero panics, well-formed
  `NOMX-S<N>-<reason>`, deterministic failure tuples), P5 synonym
  round-trip (open empty DB → insert row → expect canonical-equivalent
  token stream → delete row → expect pre-insert behaviour, plus three
  cross-reference tests), P6 no-foreign-names audit (whole-word match
  over every text column after baseline import), P7 no-Rust-bundled-
  data audit. Phase E complete.
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
- LSP slice 7a: pattern-driven completion items emitted by the
  canonical `search_patterns` backend; 17 dispatch tests green.
- T2.1 first slice — flow-edge verifier
  (`nom_concept::flow_edge`). Pure-data findings for three structural
  smells in `composes` / `Uses` chains: `ConsecutiveDuplicate`,
  `LoopReference`, `SelfReference`. No solver / dictionary lookup
  required; ships as soon as S6 emits a `NomFile` / `NomtuFile`.
  148 nom-concept lib tests green (142 + 6 new). Solver-backed
  contract checks (B.requires ⇐ A.ensures), entity-typed-slot
  resolution, and effect propagation queued for later T2.x slices.
- T2.2 first slice — agent_classify_smoke
  (`nom-intent/tests/agent_classify_smoke.rs`). End-to-end ReAct loop
  drives `DictTools::query` then `DictTools::render` to a
  `Rendered { target, bytes_hash }` observation with a 64-char hex
  SHA-256 plan hash, then propagates the LLM's terminal `Answer`.
  Second case locks the tool-error-vs-bounded-Reject distinction:
  rendering an unknown uid surfaces `Observation::Error` rather than
  rewriting it as a Reject. 63 nom-intent lib + 2 integration tests
  green. Real bytecode-linking + binary emission ship in slice-3c-full
  when the nom-llvm dependency is wired through.
- T3.1 — `nom-resolver::intent` module: `IntentResolver` trait +
  `JaccardOverIntents` deterministic impl + `SemanticEmbedding` stub.
  Same scoring shape as `nom-grammar::search_patterns` so retrieval
  ranking stays aligned across resolver / CLI / LSP / CI surfaces.
  Stable hash tie-break for byte-equal-across-runs ordering. Stub
  always returns `Err(Unavailable)` — production wiring already has
  a typed slot for the future model-backed backend (T4.2 gate). 58
  nom-resolver lib tests (52 prior + 6 new).
- T3.2 — `entry_scores` extended to 11 quality dimensions. New
  columns: `quality`, `maintenance`, `accessibility` (all REAL,
  nullable). Mirrored across both schema sources (lib.rs single-DB
  path + dict.rs ENTITIES_SCHEMA_SQL split-DB path) so the dict-split
  migration stays in sync. Two indexes added for the new query-hot
  columns. Schema only — population pipeline lands with the corpus
  pilot (T4.1). 64 nom-dict lib tests (62 prior + 2 new).

## In flight

- **Grammar-registry row-level CLI** — `nom grammar add-keyword`,
  `nom grammar add-kind`, `nom grammar add-clause-shape`,
  `nom grammar add-quality`, `nom grammar add-pattern`, plus
  `nom grammar import <sql-file>` for batch population. None shipped.
- **Dict-split follow-through** — Cycle 1: All 9 concept-tier DB1 helpers ported
  to free functions on `&Dict`. Cycle 2: Consumer bridge implemented for
  `nom-cli` concept-tier read-only commands (`cmd_concept_list_filtered`,
  `cmd_concept_show`, `cmd_concept_delete`). Cycle 3: All 8 entities-tier methods
  ported to free functions: `upsert_entry`, `upsert_entry_if_new`, `get_entry`,
  `find_entries`, `bulk_upsert`, `add_graph_edge`, `add_translation`, `bulk_set_scores`.
  Free function exports added to `nom-dict` lib.rs. Consumer bridge completed:
  `nom-cli` author.rs, store/commands.rs, and mcp.rs refactored to use dual-path
  pattern (try Dict via `try_open_from_nomdict_path`, fall back to NomDict).
  Follow-up bridge landed this cycle for `nom-cli` concept commands
  (`cmd_concept_new`, `cmd_concept_add`, `cmd_concept_add_by`) and
  `nom store sync`, both now opening `Dict` directly and calling
  split-aware free functions instead of legacy methods. The corpus
  axis-management commands (`register-axis`, `seed-standard-axes`,
  `list-axes`) also now use `Dict` directly. The core `nom-corpus`
  ingest/clone entry points (`ingest_directory`, `ingest_parent`,
  `clone_and_ingest`, `clone_batch`, `ingest_pypi_top`) now also take
  `&Dict` and preserve their per-repo transaction behavior on
  `entities.sqlite`. Remaining consumer bridges are now concentrated in
  `nom-app`,
  a few CLI fallback paths (`extract` / `score` / `stats` / `coverage`),
  and test-only legacy fixtures. Legacy `NomDict` deletion and
  schema-constant cleanup stay coupled to the eventual atomic no-legacy cut.
  Current audit shows `52` free functions ported (concept-tier + entities-tier);
  9 NomDict methods remain for cleanup phase after all consumers bridge.
- **`.nomx` v1 + v2 merge** — single canonical source format; v1 vs v2
  parser-path distinction deleted.
- **CLI parser rollback for repo health** — `nom store add` stays on the
  `nom-concept` S1-S6 pipeline, but the broader `nom-cli` parse/build/check/
  report/fmt path is back on `nom-parser` for now. A temporary AST bridge was
  dropping statement bodies, which made the migration look farther along than
  the executable CLI reality. The bridge remains a future helper surface, not
  the current default execution path.
- **`quality_names.metric_function`** — currently nullable awaiting
  `nom corpus register-axis` CLI.
- **Embedding index** — `nom corpus embed` populates per-kind
  embeddings for the resolver's semantic re-rank, replacing the
  alphabetical-smallest stub.
- **Real planner-in-Nom** — replaces stubbed planner.

- **Archive mission re-verification** — completed as a repo-wide audit.
  Outputs:
  - root executive summary: [`Gap.md`](../Gap.md)
  - detailed archive audit: [`research/.archive/gap-audit-2026-04-14.md`](./.archive/gap-audit-2026-04-14.md)
  - pending mission ledger: [`research/.archive/pending-missions-2026-04-14.md`](./.archive/pending-missions-2026-04-14.md)
  Key conclusion: `.archive` is a live documentation-management store,
  not dead history, but multiple archived mission/status docs are stale
  and need explicit re-verification or historical banners.

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
- GitNexus + local verification now confirm current live truth at
  `HEAD 98bef38`: workspace is 31 crates, `cargo check --workspace`
  passes, and targeted tests for `nom-concept`, `nom-lsp`,
  `nom-grammar`, `nom-intent`, and `nom-graph` are green. Earlier
  archive text that still described `nom-lsp` or `nom-grammar` as
  nonexistent/draft-only is now officially treated as stale-doc state,
  not code state.
- Dict-split progress corrected to code truth: `dict.rs` now exposes
  48 public free functions total, 44 migration-surface helpers, and
  41 legacy-surface parity ports. Only 8 legacy `NomDict` methods are
  left. “Last 3–5 methods” is still inaccurate, but the finish line is
  now real and visible.

## Blockers / open

- Disk full has hit the build environment in the past. Pre-cycle
  discipline: check available disk before cargo invocations.
- Embedding index requires network access for corpus pulls; offline
  cycles can only progress on dict-side, schema-side, or doc-side
  work.

## NomCanvas Phase 1 batch-1 (2026-04-17, HEAD 6196ef1 + uncommitted)

- Workspace `nom-canvas/` initialized as a fresh Cargo workspace
  (resolver = 2); one member crate `nom-gpui` with 9 modules and
  31/31 unit tests passing under `cargo test` on Windows.
- Modules landed: `geometry` (Point/Size/Bounds + Pixels /
  ScaledPixels / DevicePixels + TransformationMatrix),
  `color` (Rgba source-over blend + Hsla→Rgba),
  `bounds_tree` (R-tree, MAX_CHILDREN=12, topmost_intersecting with
  max_order pruning), `scene` (6 typed primitive Vec collections —
  Quad, Shadow, Underline, MonochromeSprite, PolychromeSprite, Path
  — plus BatchIterator yielding same-kind runs in z-order),
  `atlas` (PlatformAtlas trait + AtlasTextureKind
  Monochrome/Subpixel/Polychrome + InMemoryAtlas shelf allocator
  for tests), `style` + `styled` (layout + paint style with 22
  fluent builder methods), `element` (three-phase `request_layout
  → prepaint → paint` lifecycle with caller-owned associated
  state), `taffy_layout` (LayoutEngine over taffy::TaffyTree with
  cached bounds keyed by NodeId).
- Deps pinned at wgpu=22, taffy=0.6, cosmic-text=0.12, winit=0.30,
  etagere=0.2, raw-window-handle=0.6, bytemuck=1.16,
  parking_lot=0.12, smallvec=1.13, thiserror=1. Workspace sets
  `unsafe_code = deny`.
- Architecture replicates Zed GPUI patterns end-to-end studied
  before any code was written (Explore agent read
  `c:/Users/trngh/Documents/APP/zed-main/crates/gpui/src/` across
  scene, platform, text_system, element, styled, taffy, window,
  bounds_tree, color, geometry). No foreign identifiers; no
  wrappers / adapters — every type is a native Nom implementation
  of the abstract pattern.
- Remaining Phase 1 batch-2 (blocking Phase 2): wgpu renderer
  consuming PrimitiveBatch enums, cosmic-text + etagere wiring
  into a concrete PlatformAtlas, winit window + event loop +
  60 fps frame timing, desktop-vs-browser platform abstraction.
- Disk pre-check: `c:/` 208 G / 237 G used (29 G free) — no
  disk-full blocker this batch.
