# Nom — Implementation Plan

**This file is deprecated.** The implementation plan has been unified into three canonical documents:

| Need | Go Here |
|------|---------|
| Axis completion %, checkbox tracking, 100% criteria | `ROADMAP_TO_100.md` |
| Iteration history, what was built each wave | `nom_state_machine_report.md` |
| Test counts, open items, current wave status | `task.md` |

**Why:** The previous 5-document tracking system (ROADMAP + state machine + task + implementation_plan + audit report) became mutually contradictory. Test counts varied by 1,000+, axis percentages were inflated, and audit findings went stale before fixes landed.

**Single Source of Truth rules:** See `AGENTS.md` → "Single Source of Truth" section.

---

*Last updated: 2026-04-19 | Wave AF launched (16-repo parallel audit) | HEAD `2da0748`*

## Current Wave: AF — Massive Parallel Repo Pattern Audit

16 subagents analyzing 12 untapped high-value + 4 future-wave repos.
Reports: `.archive/audits/2026-04-19-{repo}-pattern.md`

See `task.md`, `ROADMAP_TO_100.md`, `nom_state_machine_report.md` for live tracking.

## Structural Audit Findings (2026-04-19)

**5 duplicate crate names** across workspaces (completely different code):
- `nom-cli` / `nom-graph` / `nom-media` / `nom-ux` / `nom-intent` exist in BOTH nom-canvas and nom-compiler
- Blocks workspace unification
- **Action:** Rename canvas versions to `nom-canvas-*`

**God crates needing split:**
| Crate | Lines | Split Into |
|---|---|---|
| `nom-compose` | 28,607 | orchestrator + backends + output |
| `nom-canvas-core` | 16,902 | render + input + viewport |
| `nom-blocks` | 11,672 | core + tree + registry |
| `nom-graph` (canvas) | 15,542 | execution + rag + infra |
| `nom-concept` (compiler) | ~13,000 | lexer + parser + ir + validate + bootstrap |

**Stale artifacts:** `nom-canvas/nom-canvas/` (delete), `nom-canvas/tests/` (delete)

**Cross-workspace coupling:** ONE-WAY (canvas → bridge → compiler). Clean. Keep two workspaces.
