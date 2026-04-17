# Gap

## Current Code Truth

- Audit date: 2026-04-14
- Repo head verified by GitNexus and local git: `98bef38`
- GitNexus refresh result: `5,856 nodes`, `14,682 edges`, `267 clusters`, `300 flows`
- Workspace crates in `nom-compiler/Cargo.toml`: `31`
- `cargo check --workspace --message-format short`: passes
- Targeted tests pass for:
  - `nom-concept`
  - `nom-lsp`
  - `nom-grammar`
  - `nom-intent`
  - `nom-graph`

## Main Gaps

### 1. Archive mission verification debt

The archive contains many valuable docs, but several mission/status docs were archived without being fully re-verified against the current codebase.

Most important stale claim families:

- docs that still say `nom-lsp` does not exist or Phase 9 is `0% code started`
- docs that still present `nom-grammar` as draft-only
- docs that still describe the repo as `28` crates or present older "last verified" heads
- docs that still understate `nom-translate` as mostly scaffold-only

### 2. `.archive` is useful, but the repo does not say that clearly

`research/.archive` is functioning as a real documentation-management store, not dead history. It contains design references, mission ledgers, session summaries, and implementation-roadmap docs that are still being used.

The gap is not whether `.archive` should exist. The gap is that the repo still does not explicitly define:

- which archive docs are current references
- which are historical snapshots
- which need re-verification banners

### 3. GitNexus cannot directly audit `.archive`

GitNexus is currently good for live code truth, but it does not index `research/.archive/**` because the path is hidden/dot-prefixed.

So archive verification must currently be mixed-mode:

- GitNexus for code truth
- local markdown audit for archive-doc truth

### 4. The next repo-wide missions are real and still open

The most important still-open missions are:

- corpus pilot / meaningful ingestion at scale
- embedding / rerank resolver replacing the current stub path
- verifier-at-flow-edge / contract enforcement beyond the current partial checks
- planner-in-Nom
- parser-in-Nom
- fixpoint bootstrap
- fuller LSP / authoring-protocol expansion
- dict-split follow-through and terminology cleanup

## Mission Value Summary

### Still valuable

- `language-analysis/10-external-repo-upgrade-plan.md`
- `language-analysis/13-nomx-strictness-plan.md`
- `language-analysis/20-session-summary-2026-04-14.md`
- `language-analysis/22-dict-split-migration-plan.md`
- `language-analysis/09-implementation-status-2026-04-13.md` as a strong evidence spine, though not fully current in every banner

### Valuable but partially stale

- `deferred/06-blueprint-for-building-novel.md`
- `language-analysis/04-next-phases-plan.md`
- `language-analysis/21-grammar-registry-design.md`
- `language-analysis/02-fifty-language-analysis.md`
- `motivation/01-world-language-survey.md`
- `motivation/02-vietnamese-grammar-to-novel-syntax.md`

### Superseded themes

- Vietnamese tokens as real source vocabulary
- locale packs as keyword-translation machinery instead of grammar-style infrastructure
- any mission framing that treats `nom-lsp`, `nom-grammar`, `nom-intent`, or `nom-graph` as nonexistent

## Outputs

- Detailed archive audit: [`research/.archive/gap-audit-2026-04-14.md`](research/.archive/gap-audit-2026-04-14.md)
- Pending mission ledger: [`research/.archive/pending-missions-2026-04-14.md`](research/.archive/pending-missions-2026-04-14.md)
