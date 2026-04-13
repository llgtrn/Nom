# 09 — Implementation Status as of 2026-04-13

**Snapshot date**: 2026-04-13. **HEAD**: `fa1ba8a` (origin/main). **Session arc**: 24 commits past `1ae2136` (research docs 07+08 landed).

This doc maps what's actually shipped vs aspirational across all 13 prior research/motivation documents. Future cycles can reference this as the canonical status before re-discovering state.

## End-to-end pipeline that runs today

```
.md drafts                        author writes prose intent
   │
   │  nom author translate (existing)
   ▼
.nom + .nomtu source              hand-edited or AI-emitted; per doc 08 §3
   │
   │  nom store sync <repo>       walks files → parses → writes DB1+DB2 rows
   ▼
SQLite (concept_defs + words_v2)  per doc 08 §2; additive on existing entries table
   │
   │  nom build status <repo> [--write-locks]
   ▼
ConceptGraph + closure walk        DFS post-order with cycle detection (doc 08 §4.3)
   │
   │  resolve_closure              kind-or-word lookup; alphabetical-smallest tiebreak
   │                                (Phase-9 corpus-embedding re-rank pending)
   ▼
ResolvedRefs + UnresolvedRefs      doc 08 §5 per-kind embedding-index discipline
   │
   │  check_mece                   per doc 08 §9.2 — Mutually-Exclusive axis check
   │                                (Collectively-Exhaustive deferred to Phase-9 corpus)
   ▼
MECE-validated objectives          exit 1 on collision; stub note for CE
   │
   │  nom build manifest <repo>    JSON RepoManifest with full closure + objectives
   │                                + acceptance + effects + typed_slot + threshold
   ▼
Phase-5 planner handoff (pending)
```

Concrete examples that exercise this pipeline:

- [`examples/concept_demo/`](../../nom-compiler/examples/concept_demo/) — minimal end-to-end (1 root concept + 1 nested concept + 1 module with 2 entities + 1 composition).
- [`examples/agent_demo/`](../../nom-compiler/examples/agent_demo/) — motivation 16 STRONGEST candidate (AI-agent composition: 6 tools + safety policy + intentional MECE collision).
- [`examples/agent_demo_vn/`](../../nom-compiler/examples/agent_demo_vn/) — Vietnamese keyword aliases applied to the same demo (motivation 02 phase 1; user later clarified vocabulary stays English).

All three have e2e tests at `crates/nom-cli/tests/{concept_demo,agent_demo,agent_demo_vn,build_manifest}_e2e.rs` (all `#[cfg(not(windows))]` because the `nom` binary links LLVM-C.dll for compile commands; metadata pipeline runs anywhere).

## Per-doc status

### `02-fifty-language-analysis.md`

**Status**: 100% as docs (440 lines). Survey of 47 design failures across 40+ languages with mapped prevention in Nom architecture.

**Implementation derivative**: many of the "Nom prevents this" claims rely on infrastructure not shipped yet (real planner, real codegen, runtime sandboxing). The CLAIMS are documented; the BACKING CODE is partial. `body_kind` invariant 15 is enforced (commit `540620d`/`6c336b4`); other claims await Phase-5+ code.

### `03-self-hosting-roadmap.md`

**Status**: docs at 100%, code at 40%. 7 phases: Phase 4 (DIDS) ✅ shipped. Phase 5 (planner-in-Nom) — 0% (multi-week, parked). Phases 6–10 — 0%.

### `04-next-phases-plan.md`

**Status**: docs at 100% (2540 lines, the meta-plan). Code: §4.4.6 body_kind invariant enforced (commits 540620d/6c336b4); §10.3.1 toolchain pin verified (29f5f1d/bc89d8a/96267df); §5.16.13 codec roadmap order #1 PNG path partly addressed via the AVIF milestone.

### `04-next-phases-plan-review.md`

**Status**: docs at 100% (142 lines, risk + review). Risk #1 toolchain unblocked + CI-enforced.

### `05-natural-language-syntax.md`

**Status**: docs at 100% (.nomx v1 grammar shipped; doc has 382 lines including §10 implementation status with 34/34 parser tests at the time). The `.nomx v1` parser at `crates/nom-parser/src/nomx.rs` predates the doc-08 layered work.

**Note**: doc 08's `.nom`/`.nomtu` parser at `crates/nom-concept/src/lib.rs` is a SEPARATE parser that handles the doc-08 layered architecture, NOT a successor to the `.nomx v1` parser. They serve different scopes.

### `06-nomx-keyword-set.md`

**Status**: docs at 100% (vocabulary draft companion to 05). Implementation extends the doc 08 closed kind set with: `function|module|concept|screen|data|event|media`. Keyword set extended in cycles for typed-slot syntax (`@Function`, `with at-least N confidence`).

### `07-keyed-similarity-syntax-proposal.md`

**Status**: docs at 100% (.nomx v2 keyed syntax). **Code at ~98%** as of HEAD `fa1ba8a`:

- ✅ `@Kind` sigil token + parser (`Tok::AtKind`, `parse_entity_ref_after_the` — commit c9d1835)
- ✅ `EntityRef.typed_slot` AST flag + serde round-trip
- ✅ Resolver branches on typed_slot → `find_words_v2_by_kind` with alphabetical tiebreak (commit c405d2a)
- ✅ `with at-least 0.85 confidence` threshold syntax (commit 97c836f, closes §6.3)
- ✅ Manifest carries typed_slot + threshold through serde
- ✅ Per-slot top-K diagnostic in `nom build status` (doc 07 §3.3 — commit 853e70b): when typed-slot has N>1 candidates, prints `slot @Kind matching "..."` header + resolved + alternatives list
- ⏳ Phase-9 corpus embedding semantic re-rank — not yet built; stub uses alphabetical pick

**Three §6 open questions all resolved**:
1. Kind sigil — shipped (user reversed doc-08 §8.1's prose-only decision)
2. Lock storage — typed slots NOT written back per §3.5; v1 refs still get `name@hash` writeback
3. Threshold authoring — option (c) per-slot inline shipped (`with at-least N confidence`)

### `08-layered-concept-component-architecture.md`

**Status**: docs at 100% (the architecture proposal). **Code at ~55%**:

- ✅ Three-tier model: atoms (DB2 rows) / `.nomtu` modules (multi-entity files) / `.nom` concepts (multi-concept files; dictionary-relative index)
- ✅ Two databases: DB1 (`concept_defs`) + DB2 (`words_v2`), additive to existing `entries` table (no breaking change)
- ✅ Lock writeback — source IS the lock for v1 refs (doc 08 §8.2)
- ✅ `nom store sync` + `nom build status [--write-locks]` + `nom build manifest`
- ✅ Concept-graph closure walker with cycle detection
- ✅ MECE validator (§9.2) — ME-collision check shipped; CE check stub-only (corpus-required-axis-set deferred)
- ⏳ Layered dreaming (§9) — only single-tier `nom app dream` exists; concept-tier + module-tier dreaming pending
- ⏳ Three-tier compiler (recursive ingest of bytes per §4.3) — pending Phase 5/6
- ⏳ Acceptance-predicate preservation engine (§9.1) — preserve/no-mislead discipline pending
- ⏳ Cascade through dream (§9.3) — pending
- ⏳ Singleton enforcement via MECE — design shipped as part of MECE work; corpus required-axis registration deferred

### Motivation 01 — `01-world-language-survey.md`

**Status**: docs at 100% (parallel to language-analysis 02 from the motivation lens). No direct code.

### Motivation 02 — `02-vietnamese-grammar-to-novel-syntax.md`

**Status**: docs at 100%. **Code partial**:

- ✅ Mandatory classifiers (§3) — `the function`, `the concept`, etc. shipped via doc 08 closed kind set
- ✅ Modifier follows head (§2) — `<word> matching "..."` clauses + `<ref> with at-least N confidence`
- ✅ Effect valence (§9 — genuinely novel per motivation 10 §E #4) — `benefit`/`hazard` keywords on entities and compositions (commit c9d1835), surfaced in manifest (eeb1e23). **English-only vocabulary** per user clarification; Vietnamese loanwords explicitly out of scope.
- ✅ Vietnamese keyword aliases (locale pack) — ASCII (4b04b1d) + diacritic (5b59f82) shipped; user later clarified to keep but not extend (vocabulary stays English; Vietnamese inspires GRAMMAR style not vocabulary).
- ⏳ Aspect markers (§8 — `verified`/`active`/`deferred` state) — 0%; low semantic value without runtime
- ⏳ Topic-comment block discipline (§7) — implicit via current `{}`-free declarations; explicit `{...}` topic markers not yet formalized
- ⏳ Reduplication parameterized variants (§12) — 0%
- ⏳ Sino-Vietnamese morphemes as core kinds (§11) — implicit via the closed kind set; no `~200-500 root vocabulary` registered

### Motivation 10 — `10-how-novel-replaces-everything.md`

**Status**: docs at 100% (universal-replacement strategy). **Code: aspirational at scale**.

- §B "Universal replacement" — long-game; the dictionary is empty modulo the demo nomtu files
- §C "NovelOS dictionary extraction" — `nom-corpus` crate exists with `ingest pypi/github/repo` skeletons (per memory) but mass corpus ingestion is a Phase-5+ activity
- §E "What Novel uniquely contributes" — 6 items listed; effect valence (§E#4) is the one item shipped as code; others (semantic contract extraction, scored composition with provenance, glass-box reports, Vietnamese 4-layer disambiguation cascade) are AST-level only

### Motivation 13 — `13-beyond-all-models.md`

**Status**: docs at 100% (hybrid AI/Nom architecture). **No direct code**; the framing informs design but no Phase-9 LLM-author-loop exists yet.

### Motivation 16 — `16-competitive-analysis-and-roadmap.md`

**Status**: docs at 100% (honest competitive position + roadmap). **Killer-app foundation: ~50%**.

- §5 STRONGEST candidate (AI-agent composition) — `examples/agent_demo/` ships as the minimal demonstration
- §6 build sequence Phases 0-1 — substantially landed (parser + tooling + LLVM backend in main + first useful CLI)
- Phase 2-4 (proving fast/scales/lasts) — pending; needs corpus-fed dictionary at scale + benchmark integration

## Architecture decisions made (with refs)

| Decision | Rationale | Commit |
|---|---|---|
| Three tiers above artifact store | Authors reason in concept-level groupings (doc 08 §1) | `1ae2136` (doc) + entire arc |
| DB1+DB2 additive (don't migrate `entries`) | Preserve existing 1715-line nom-dict + 1847-line nom-resolver | `aaa914d` |
| Hand-rolled parsers (no nom/pest/lalrpop) | Determinism + small dep surface per §10.3.1 fixpoint | `05ee1b6` + `d9425ba` |
| `@Kind` sigil for typed-slot refs | User explicit reversal of doc 08 §8.1 (had rejected sigils) per "syntax exactly like doc 07" | `c9d1835` |
| Source-as-lock for v1 refs; no writeback for typed-slot refs | Doc 08 §8.2 + doc 07 §3.5 | `a04b91e` (v1) + `c405d2a` (v2) |
| Stub resolver: alphabetical-smallest hash tiebreak | Determinism per §10.3.1; Phase-9 corpus replaces with semantic re-rank | `bf95c2c` + `c405d2a` |
| MECE exit-1 on ME collision | Validator does real work, not advisory; agent_demo intentionally collides to prove it | `c63a6a7` |
| Effect valence keywords English-only | User clarification: vocabulary stays English; Vietnamese inspires GRAMMAR style | `c9d1835` (with VN-loanword strip) |
| Confidence threshold per-slot inline | Doc 07 §6.3 option (c) | `97c836f` |

## What's parked + why

- **Phase-5 planner-in-Nom port** — multi-week scope; manifest handoff exists but the planner consuming it doesn't.
- **Phase-9 corpus-embedding resolver** — needs corpus pipeline + index infrastructure; multi-week.
- **Aspect markers** (motivation 02 §8) — low semantic value without runtime to interpret `active`/`verified`/`deferred`.
- **Topic-comment formalization** (motivation 02 §7) — current parser already accepts the implicit form; explicit `{...}` markers add complexity without immediate payoff.
- **Vietnamese-character function names** — explicit exclusion in lexer (function names stay ASCII per `expect_word` enforcement); diacritic-in-identifier scope is bigger than diacritic-in-keyword.
- **MECE CE-check** (collectively-exhaustive) — needs corpus-side required-axis registry per composition layer.
- ~~**`store.rs` module split**~~ — DONE (commit fa1ba8a). 1814 → 5 files: `mod.rs` 957 + `add_media.rs` 112 + `sync.rs` 279 + `resolve.rs` 322 + `materialize.rs` 172. mod.rs further split into `commands.rs` is the remaining hygiene work.

## Open questions still unresolved

1. **Doc 07 vs doc 08 syntax tension on the `the NOUN` form** — doc 08 §8.1 chose prose, user reversed for typed slots. Both forms now coexist via `EntityRef.typed_slot`. Long-term: should `the NOUN` form be deprecated in favor of `@Kind`, or kept as the v1 path forever?
2. **Tier-2 override scope** (doc 08 Q5) — deferred per user "I don't understand"; revisit when concrete use case arises.
3. **Aspect marker semantics** — `verified`/`active`/`deferred` are runtime state. What's the runtime that interprets them? Phase-9+ work.
4. **Singleton CE registry** — where does the corpus declare "exactly_one_per_app" axes (database, auth_provider, …)? Phase-9 corpus design decision.
5. **Multi-locale demo strategy** — `agent_demo_vn` exists as inert locale-pack code per user "keep but don't extend." Should it be kept, archived, or deleted?

## Test totals (HEAD `97c836f`)

- `nom-concept`: 76 passed
- `nom-dict`: 24 passed
- `nom-media`: 50 + 3 ignored on Windows (3 PSNR tests gated; CI runs Linux)
- `nom-cli` integration: gated `#[cfg(not(windows))]` on Windows dev box; runs in Linux CI
- Workspace builds clean except 1 pre-existing `dead_code` warning on `ResolvedRef::kind` in `nom-cli/src/store.rs`

## Branch state

- Local HEAD: `fa1ba8a`
- origin/main: `fa1ba8a` (in sync — every cycle pushes)
- 24 commits past `1ae2136` this session
- Working tree: only auto-stamped files (Cargo.lock, AGENTS.md, CLAUDE.md from gitnexus PostToolUse hook); nothing intentional uncommitted

---

This doc is a snapshot. Next session should re-verify against `git log --oneline` and the actual test runs before relying on the percentages.
