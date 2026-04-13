# 2026-04-13 — autonomous SDD session arc

**Mode**: pre-authorized `/loop` cron firing `/subagent-driven-development` against doc 09 path + research context. 28 commits from `654e981` (HEAD at session start) through `2a2c782`. This is the durable record so the next cycle picks up without rediscovering what's done.

## Path milestones shipped or scaffolded

| ID | Status | Commits | What it is |
|---|---|---|---|
| **M1** | ✅ | `41aedfe` | `nom build report` glass-box `ReportBundle` — per-slot resolution trace, alternatives, MECE outcome, provenance |
| **M2** | ✅ | `fb9c252` | `crates/nom-concept/src/acceptance.rs` — predicate preservation engine (text-hash diff + Jaccard reword detection) |
| **M4** | ✅ | `8aab409` | Three-tier recursive closure walker: `visit_entity_ref` descends through nested `kind="concept"` refs |
| **M5** | ✅ | `f0ae193`+`9b3f501`+`e28f69d`+`cc9641b`+`aa5e7c9` | Layered dreaming: `LayeredDreamReport { tier, label, leaf, child_reports, pareto_front }` + `--tier` CLI + recursion via ConceptGraph + Pareto over 3 axes (score, complete, low_partial_ratio) |
| **M7a** | ✅ | `bcadcb3`+`04df3dc` | `required_axes` SQLite registry + `check_mece_with_required_axes()` + `nom corpus register-axis`/`list-axes` CLI |
| **M7c** | ✅ | `4307c5a` | MECE violations surfaced in `LayeredDreamReport.me_collisions`/`ce_unmet`; 10-point score penalty per violation |
| **M3a** | ✅ | `49869ab`+`c7aea7a` | `crates/nom-locale/` new crate: `LocaleTag::parse` (BCP47) + `normalize_nfc` (UAX #15) + `LocalePack` + `builtin_packs()` + `nom locale list`/`validate` CLI |
| **M3c** | ✅ (infra) | `339a74a` (vocab)→`b3fe503` (reverted) + `e13c9e4` | `apply_locale()` lexical pass + `nom locale apply --write/--json` CLI. Vocab aliases reverted per user directive (fully English) |
| **M3b-minimal** | ✅ | `712ede4`+`567714a` | 30-pair baked confusable table (Cyrillic/Greek→Latin); `is_confusable` returns real `Confusable`/`DifferentSafe` instead of `Deferred` stub |
| **VN-vocab removal** | ✅ | `ecd0609`+`018d3c6`+`88ba715` | All 31 VN lexer arms + `is_vn_diacritic` predicate + `examples/agent_demo_vn/` deleted. `~17` VN tests removed from nom-concept. Verified clean by coherence audit |
| **M10a** | ✅ | `249fe62` | `self_host_parse_smoke.rs` — parse gate over all 7 `.nom` files in `stdlib/self_host/` + `examples/run_lexer.nom` |
| **M10b** | ✅ | `cef8425` | `run_lexer_bc_reproducibility_smoke.rs` — `run_lexer.bc` SHA-256 pinned at `085e8fa6...83296` as §10.3.1 byte-determinism gate (Linux-CI first-run verification) |
| **ci matrix fix** | ✅ | `f34e6f8` | Linux CI loop expanded 21 → 27 crates; adds `nom-concept`, `nom-locale`, `nom-config`, `nom-extract`, `nom-score`, `nom-runtime` |
| **coherence audit** | ✅ | `022db7c` | README + self_host/README + main.rs `M5b` → corrected; doc 09 test counts refreshed (nom-concept 77, nom-dict 29, nom-locale 25, nom-app 30) |
| **check-confusable CLI** | ✅ | `9649857`+`2a2c782` | `nom locale check-confusable <a> <b> [--json]` surfaces M3b-minimal to CLI; catches Cyrillic-paypal attack |

**Commits in session**: 28. **Test counts post-session**: nom-concept 77/77, nom-dict 29/29, nom-locale 25/25, nom-app 30/30, nom-media 50/50+3 ignored.

## Load-bearing user corrections (pinned feedback)

1. **"Vietnamese grammar style, English vocabulary"** — applies to Nom at every layer. `LocalePack.keyword_aliases` for vi-VN stays empty; lexer has zero VN tokens; `agent_demo_vn/` does not exist; no `is_vn_diacritic` predicate. Relapse signal: typing a Vietnamese character in any code path. See `feedback_vn_grammar_not_vocab.md`.
2. **"Full coherence sweep before continuing"** — not just VN; dead code, stale refs, duplicate logic, cargo drift, fixture drift. Completed 2026-04-13; verdict CLEAN across A–H categories.
3. **"Use GitNexus to query faster"** — CLAUDE.md assumes GitNexus MCP is wired. This session's tool set did NOT include GitNexus tools (`ToolSearch query: "gitnexus"` returned 0 matches). Fallback was grep + file reads via Explore subagent. For future cycles where MCP is live, use `gitnexus_query`/`gitnexus_context`/`gitnexus_detect_changes` instead.

## Milestones parked (deliberate) + why

| ID | Why parked |
|---|---|
| **M3b-full** (UTS #39 confusables.txt ~1.5 MB) | Days-scale but needs an external data file (`confusables.txt` from unicode.org). Autonomous loop shouldn't download; manual fetch + `include_str!` is the natural first step |
| **M6** (PyPI-100 corpus pilot) | Weeks-scale AND requires heavy network (PyPI + GitHub clone). Not safe in autonomous loop. Must be run manually with `nom corpus ingest pypi --top 100 --budget 50GB` after a human verifies disk budget |
| **M7b** (seed standard axes from corpus) | Depends on M6 corpus run |
| **M8** (intent transformer) | Quarters-scale; needs M6 populated dict |
| **M9** (per-kind embedding re-rank) | Weeks-scale; needs M6 corpus |
| **M10c** (compile-to-IR subset check for aspirational syntax) | Speculative; would need a subset-definition step first |
| **M10-full planner-in-Nom port** | Quarters-scale per doc 03 |
| **M15 parser-in-Nom** / **M16 LSP** / **M17 fixpoint bootstrap** | Each quarters-scale; M17 is the 1.0 cut |

## Honest % incomplete toward Nom 1.0

**~58%**, where 1.0 = "compiler is self-hosted (M17 fixpoint bootstrap passes) + Rust stage archived."

**Rationale**: 10 path milestones done (M1, M2, M4, M5 a+b+c, M7a, M7c, M3a, M3c, M3b-minimal, M10a, M10b) of 18 numbered milestones in doc 09. The remaining 8 (M3b-full, M6, M7b, M8, M9, M10-full, M15, M16, M17) include all the quarters-scale self-hosting work. Even with everything shipped, the hard part — **a real planner authored in Nom + a real parser authored in Nom + a fixpoint bootstrap** — sits in the quarters-scale bucket.

## Critical-path recommendation for next session

**1. Manual run**: `nom corpus ingest pypi --top 100` (M6). This single action unblocks M7b, M8, M9. Don't attempt in autonomous loop.

**2. After M6 lands**: M9 embedding re-rank (pure code once corpus is populated). That unblocks real slot resolution quality.

**3. Parallel-trackable** (no M6 dep): M3b-full (fetch confusables.txt + bundle), M10c (IR-subset gate for self_host/*.nom).

**4. Long tail**: M10/M15/M16/M17 are quarters each. Plan multi-month chunks.

## Pinned memory updates this session

- `feedback_vn_grammar_not_vocab.md` — rewritten: "vocabulary FULLY English" (stronger than previous "keep but don't extend")
- `project_nom_layered_architecture.md` — updated to reflect `ecd0609` VN removal
- `MEMORY.md` index entries refreshed

## Known workspace state at session end

- **HEAD**: `2a2c782` on origin/main
- **Clean tree**: yes (only `.remember/` untracked)
- **Windows build**: clean, 0 warnings
- **Linux CI**: will exercise all 27 Linux-testable crates (per `f34e6f8`)
- **Windows-gated smoke tests**: `layered_dream_smoke`, `locale_smoke`, `self_host_parse_smoke`, `run_lexer_bc_reproducibility_smoke`, `concept_status_smoke`, `store_add_media`, `media_import`, `corpus_axes_smoke` — all `#[cfg(not(windows))]`, run on Linux CI only
- **Known environmental failures on Windows**: `STATUS_DLL_NOT_FOUND 0xc0000135` across `nom-llvm`, `nom-corpus`, `mcp_smoke`, `phase4_acceptance`, `store_cli` — pre-existing, NOT caused by this session (verified by git stash test)
