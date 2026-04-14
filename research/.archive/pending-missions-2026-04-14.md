# Pending Missions — 2026-04-14

## Current blockers

### Archive mission re-verification and labeling

- Status: Blocked
- Why it still matters: the archive is being used as an active documentation store, but several mission/status docs still read like verified current-state truth when they are actually mixed or stale.
- Live evidence: multiple archived docs still carry old `Last verified` banners or stale shipped/planned summaries; newer code truth now includes `nom-grammar`, `nom-intent`, `nom-lsp`, and a passing workspace build.
- Source docs:
  - `research/.archive/deferred/06-blueprint-for-building-novel.md`
  - `research/.archive/language-analysis/04-next-phases-plan.md`
  - `research/.archive/language-analysis/21-grammar-registry-design.md`
  - `research/.archive/motivation/01-world-language-survey.md`
  - `research/.archive/motivation/02-vietnamese-grammar-to-novel-syntax.md`
- Suggested next action: add explicit `current`, `historical`, or `needs re-verification` banners to the affected archive docs.

### GitNexus archive indexing

- Status: Blocked
- Why it still matters: without direct indexing of `.archive`, the repo cannot use one consistent GitNexus-first workflow to validate doc claims against code.
- Live evidence: refreshed GitNexus code index is healthy, but `research/.archive/**` does not appear in GitNexus file results because the directory is hidden.
- Source docs:
  - `research/.archive/gap-audit-2026-04-14.md`
- Suggested next action: decide whether to unhide the archive path or extend GitNexus indexing rules to include it.

## Repo-wide pending missions

### Corpus pilot and meaningful ingestion at scale

- Status: Pending
- Why it still matters: this unlocks real dictionary richness, better retrieval, and makes many deferred guarantees testable instead of purely theoretical.
- Live evidence: `nom-corpus` exists, `cmd_corpus_scan` and `cmd_corpus_ingest` are live, but tests and code comments still explicitly refer to the future point “when M6 corpus + real embeddings land”.
- Source docs:
  - `research/.archive/language-analysis/09-implementation-status-2026-04-13.md`
  - `research/.archive/language-analysis/10-external-repo-upgrade-plan.md`
  - `research/.archive/language-analysis/15-100-repo-ingestion-plan.md`
  - `research/.archive/deferred/08-vietnamese-oss-landscape.md`
- Suggested next action: define a concrete first ingestion slice and success metric, then run one bounded pilot instead of keeping corpus as a broad future bucket.

### Embedding / rerank resolver

- Status: Pending
- Why it still matters: current typed-slot and confidence syntax is ahead of the resolver; semantic selection is still weaker than the language surface suggests.
- Live evidence:
  - `nom-cli/src/store/resolve.rs` still calls itself a stub and contains `TODO: Phase 9 — replace with per-kind embedding index`
  - `nom-concept/src/lib.rs` and `nom-concept/src/closure.rs` explicitly note that the Phase-9 corpus-embedding resolver is what should enforce confidence thresholds
  - `nom-intent/src/dict_tools.rs` still points toward future full-text / embedding retrieval
- Source docs:
  - `research/.archive/language-analysis/07-keyed-similarity-syntax-proposal.md`
  - `research/.archive/language-analysis/09-implementation-status-2026-04-13.md`
  - `research/.archive/motivation/13-beyond-all-models.md`
- Suggested next action: define the per-kind embedding index shape and integrate it behind the existing typed-slot resolver path.

### Contract enforcement at flow edges

- Status: Pending
- Why it still matters: many “safety” and “correct composition” claims depend on contract compatibility, not just parser support and MECE checks.
- Live evidence:
  - current code has MECE validation, typed-slot parsing, and confidence syntax
  - current resolver still resolves mainly by kind/stub logic
  - archive docs repeatedly describe full contract checking at flow edges as not yet enforceable
- Source docs:
  - `research/.archive/language-analysis/02-fifty-language-analysis.md`
  - `research/.archive/motivation/01-world-language-survey.md`
  - `research/.archive/language-analysis/04-next-phases-plan.md`
- Suggested next action: specify the verifier rules for `pre/post/effects` compatibility and make mismatch behavior explicit in the build pipeline.

### Planner-in-Nom

- Status: Pending
- Why it still matters: this is part of the real self-hosting path rather than just scaffold parity.
- Live evidence:
  - `stdlib/self_host/planner.nom` exists
  - self-host parse/pipeline/parity tests exist
  - docs still describe the real `nom plan <manifest.json>` port as unfinished
- Source docs:
  - `research/.archive/language-analysis/03-self-hosting-roadmap.md`
  - `research/.archive/language-analysis/04-next-phases-plan.md`
  - `research/.archive/language-analysis/09-implementation-status-2026-04-13.md`
- Suggested next action: implement one real planner slice in Nom and route a real CLI/code path through it.

### Parser-in-Nom

- Status: Pending
- Why it still matters: parser scaffold parity is not the same as a real parser-in-Nom milestone.
- Live evidence:
  - `stdlib/self_host/parser.nom` exists
  - parser self-host tests and pipeline tests exist
  - docs still describe parser-in-Nom as scaffolded / not semantically complete
- Source docs:
  - `research/.archive/language-analysis/03-self-hosting-roadmap.md`
  - `research/.archive/language-analysis/04-next-phases-plan.md`
  - `research/.archive/language-analysis/09-implementation-status-2026-04-13.md`
- Suggested next action: move from scaffold/parity gates to real token-consumption and AST-equivalence milestones.

### Fixpoint bootstrap

- Status: Blocked
- Why it still matters: this remains the real 1.0 proof gate in the roadmap.
- Live evidence:
  - toolchain pin and determinism-related groundwork exist
  - self-host scaffolds and parity tests exist
  - no evidence of an actual Stage2 == Stage3 fixpoint attempt or proof tuple recording
- Source docs:
  - `research/.archive/language-analysis/03-self-hosting-roadmap.md`
  - `research/.archive/language-analysis/04-next-phases-plan.md`
  - `research/.archive/language-analysis/09-implementation-status-2026-04-13.md`
- Suggested next action: keep this blocked behind real planner-in-Nom and parser-in-Nom progress; do not present it as near-term completion.

### LSP / Authoring Protocol expansion

- Status: Pending
- Why it still matters: `nom-lsp` exists, but the full authoring-protocol story is still larger than the currently shipped slices.
- Live evidence:
  - `nom-lsp` crate exists and tests pass
  - archive docs still frame broader LSP/authoring-protocol scope as open work beyond the initial slices
- Source docs:
  - `research/.archive/language-analysis/04-next-phases-plan.md`
  - `research/.archive/language-analysis/09-implementation-status-2026-04-13.md`
  - `research/.archive/language-analysis/10-external-repo-upgrade-plan.md`
- Suggested next action: define the next MVP boundary explicitly: hover/definition/completion-only, or the broader Authoring Protocol transport and agent hooks.

### Dict-split follow-through and naming cleanup

- Status: Pending
- Why it still matters: the split has landed in part, but the migration and terminology are still in flight.
- Live evidence:
  - archive docs explicitly keep later dict-split stages queued
  - code now contains split-aware surfaces alongside legacy naming
- Source docs:
  - `research/.archive/language-analysis/22-dict-split-migration-plan.md`
  - `research/.archive/language-analysis/09-implementation-status-2026-04-13.md`
  - `research/.archive/language-analysis/10-external-repo-upgrade-plan.md`
- Suggested next action: decide the final naming (`words` vs `entities`) and finish the queued migration stages before more docs assume the split is complete.

## Archive-doc missions still valuable

### Strictness lane and staged parser discipline

- Status: Pending
- Why it still matters: the current `nom-concept` stage pipeline, strictness checks, and synonym tests show this lane is still alive and compounding.
- Live evidence:
  - `nom-concept/src/stages.rs`
  - `nom-concept/src/strict.rs`
  - `nom-concept/tests/synonym_round_trip.rs`
  - `cargo test -p nom-concept --quiet` passes
- Source docs:
  - `research/.archive/language-analysis/13-nomx-strictness-plan.md`
  - `research/.archive/language-analysis/18-w4-a4-annotator-pipeline-design.md`
  - `research/.archive/language-analysis/20-session-summary-2026-04-14.md`
- Suggested next action: continue the remaining strictness wedges and refresh stale banners around what is already shipped.

### External-repo upgrade / graph-rag / translation lane

- Status: Pending
- Why it still matters: the codebase now really has `nom-graph`, `nom-intent`, `nom-translate`, and `nom-grammar`, so this lane is no longer speculative.
- Live evidence:
  - `cmd_graph` is live
  - `nom-graph` tests pass
  - `translate()` is real and CLI-wired
  - `nom-intent` tests pass
- Source docs:
  - `research/.archive/language-analysis/10-external-repo-upgrade-plan.md`
  - `research/.archive/language-analysis/11-graph-rag-agentic-rag-research.md`
  - `research/.archive/language-analysis/14-nom-translation-examples.md`
- Suggested next action: split the lane into “already shipped foundations” vs “still-open upgrades” so future planning stops treating the whole bundle as one status.

### Self-hosting and bootstrap mission

- Status: Pending
- Why it still matters: the self-host scaffolds are real and actively tested; this is still a live mission, not dead roadmap text.
- Live evidence:
  - `stdlib/self_host/*.nom` scaffolds exist
  - self-host parse, parity, and pipeline tests exist
  - fixpoint-related comments and tests are present in runtime/corpus/cli code
- Source docs:
  - `research/.archive/language-analysis/03-self-hosting-roadmap.md`
  - `research/.archive/language-analysis/04-next-phases-plan.md`
  - `research/.archive/language-analysis/09-implementation-status-2026-04-13.md`
- Suggested next action: keep as a first-class active mission, but split scaffold parity from real semantic self-host completion.

### Algebraic laws / mathematical verification / connector model

- Status: Pending
- Why it still matters: these docs still contain useful high-level design pressure for future verification work, even though the implementation is not there.
- Live evidence:
  - no `connector` kind in current code
  - no `laws` / `dimensions` fields on entries
  - docs still correctly frame these as unimplemented
- Source docs:
  - `research/.archive/deferred/11-mathematics-as-language.md`
  - `research/.archive/deferred/12-universal-knowledge-composition.md`
- Suggested next action: keep as architecture references, not near-term shipping promises, until corpus/verifier foundations are farther along.

## Stale archive claims that need re-verification or rewording

### Grammar registry is still labeled draft in one doc

- Status: Shipped but doc stale
- Why it still matters: the grammar-registry mission is now live enough that “draft only” wording is actively misleading.
- Live evidence:
  - `cmd_grammar_init`, `cmd_grammar_import`, and `cmd_grammar_status` are wired in `nom-cli/src/main.rs`
  - `cargo test -p nom-grammar --quiet` passes
  - `cargo test -p nom-grammar -- --list` shows schema/import/audit coverage
- Source docs:
  - `research/.archive/language-analysis/21-grammar-registry-design.md`
  - `research/.archive/language-analysis/10-external-repo-upgrade-plan.md`
  - `research/.archive/language-analysis/09-implementation-status-2026-04-13.md`
- Suggested next action: change the top banner from “draft design” to “implemented foundation; further expansion pending”.

### LSP is still described as nonexistent in older roadmap text

- Status: Shipped but doc stale
- Why it still matters: this distorts the current mission map and makes the repo look further behind than it is.
- Live evidence:
  - `nom-lsp` crate exists
  - GitNexus indexes `nom-lsp`
  - `cargo test -p nom-lsp --quiet` passes
- Source docs:
  - `research/.archive/language-analysis/04-next-phases-plan.md`
- Suggested next action: replace “does not yet exist” with “foundation shipped; broader protocol/mvp still pending”.

### `nom-intent` now exists and has real test coverage

- Status: Shipped but doc stale
- Why it still matters: older docs that treat intent resolution as absent obscure the actual current starting point.
- Live evidence:
  - `nom-intent` crate exists
  - `cargo test -p nom-intent --quiet` passes
  - live code includes classifier, adapters, and dict-tool surfaces
- Source docs:
  - `research/.archive/deferred/05-beyond-transformers-and-vietnamese-efficiency.md`
  - `research/.archive/language-analysis/10-external-repo-upgrade-plan.md`
  - `research/.archive/language-analysis/09-implementation-status-2026-04-13.md`
- Suggested next action: distinguish “bounded-intent slice shipped” from “full intent-resolution system pending”.

### `nom-graph` is live and should stop being treated as invisible

- Status: Shipped but doc stale
- Why it still matters: graph capabilities are already part of the live system and should not be treated as an afterthought.
- Live evidence:
  - `cmd_graph` is live in `nom-cli`
  - `nom-graph` tests pass
  - GitNexus shows `cmd_graph` calling graph construction and community-detection methods
- Source docs:
  - `research/.archive/language-analysis/09-implementation-status-2026-04-13.md`
  - older docs that still under-mention `nom-graph`
- Suggested next action: promote `nom-graph` to first-class status in any future current-state architecture doc.

### `nom-translate` is more than a scaffold

- Status: Shipped but doc stale
- Why it still matters: it already has operational code and tests, even if the larger translation mission remains unfinished.
- Live evidence:
  - `translate()` dispatches to C/C++/Python/JS/Go implementations
  - `cmd_translate` calls into it
- Source docs:
  - `research/.archive/language-analysis/02-fifty-language-analysis.md`
- Suggested next action: reword older “scaffold” claims to “implemented slice, broader translation mission still pending”.

## Superseded / historical missions

### Vietnamese tokens as source vocabulary

- Status: Superseded
- Why it still matters: this is the most common way older mission framing can mislead current planning.
- Live evidence:
  - multiple archive docs themselves already acknowledge the English-only directive
  - current locale work is grammar-style infrastructure, not source-token replacement
- Source docs:
  - `research/.archive/deferred/07-vietnamese-flexibility-and-language-mixing.md`
  - `research/.archive/deferred/08-vietnamese-oss-landscape.md`
  - `research/.archive/deferred/09-vietnamese-lingua-franca-global-vocabulary.md`
  - `research/.archive/motivation/02-vietnamese-grammar-to-novel-syntax.md`
- Suggested next action: keep the linguistic insights, but remove this theme from any active compiler mission list.

### Locale packs as keyword-translation machinery

- Status: Superseded
- Why it still matters: current locale infrastructure is still valuable, but not for reintroducing vocabulary aliases as the main goal.
- Live evidence:
  - `apply_locale` is live
  - archive notes and current code frame locale as grammar-style / normalization infrastructure instead
- Source docs:
  - `research/.archive/deferred/07-vietnamese-flexibility-and-language-mixing.md`
  - `research/.archive/deferred/09-vietnamese-lingua-franca-global-vocabulary.md`
- Suggested next action: keep locale work on normalization, confusables, and grammar-style adaptation only.

## Recommended execution order

1. Add explicit re-verification / historical banners to the stale archive docs.
2. Decide whether `.archive` should become GitNexus-indexable.
3. Continue corpus pilot / ingestion work.
4. Land embedding/rerank resolver work on top of the current typed-slot path.
5. Push verifier-at-flow-edge work so more high-level guarantees become real.
6. Advance planner-in-Nom and parser-in-Nom before talking about fixpoint bootstrap as near-term.
7. Keep `nom-lsp`, `nom-intent`, `nom-grammar`, `nom-graph`, and `nom-translate` out of the “future only” bucket in all roadmap summaries.
