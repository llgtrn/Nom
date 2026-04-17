# Cycle 3: 8 Entities-Tier Methods for Dict-Split Migration

**Status:** Exploration & Impact Analysis Ready  
**Date:** 2026-04-15  
**Goal:** Document all 8 methods that need NomDict → Dict free-function porting  

---

## Executive Summary

The following **8 methods** from `NomDict` must be ported to free functions on `&Dict` in Cycle 3:

| # | Method | Location | Risk | Current Consumers |
|---|--------|----------|------|-------------------|
| 1 | `upsert_entry` | lib.rs:395 | 🔴 HIGH | nom-app (20+), nom-corpus (1) |
| 2 | `upsert_entry_if_new` | lib.rs:454 | 🔴 HIGH | nom-cli (1), nom-corpus (1) |
| 3 | `get_entry` | lib.rs:620 | 🟡 MEDIUM | nom-app (3) |
| 4 | `find_entries` | lib.rs:690 | 🔴 HIGH | nom-cli (2), nom-dict (self) |
| 5 | `bulk_upsert` | lib.rs:898 | 🟡 MEDIUM | (unused) |
| 6 | `add_graph_edge` | lib.rs:585 | 🟢 LOW | (unused) |
| 7 | `add_translation` | lib.rs:600 | 🟢 LOW | (unused) |
| 8 | `bulk_set_scores` | lib.rs:1357 | 🟢 LOW | (unused) |

**High-Risk Methods Require GitNexus Impact Analysis Before Porting.**

---

## Detailed Method Specifications

### 1. upsert_entry — 🔴 HIGH RISK

**LOCATION:**  
File: `nom-compiler/crates/nom-dict/src/lib.rs`  
Lines: [395–444](nom-compiler/crates/nom-dict/src/lib.rs#L395)

**SIGNATURE:**  
```rust
pub fn upsert_entry(&self, entry: &Entry) -> Result<String>
```

**BEHAVIOR:**  
- `INSERT INTO entries (21 columns) ... ON CONFLICT(id) DO UPDATE SET ...`
- Selective field-merge strategy using `COALESCE()` for optional fields:
  - `word, variant, kind, language, body, body_nom, body_kind, body_bytes, status, is_canonical` — replaced unconditionally
  - `describe, concept, translation_score, deprecated_by` — only updated if excluded value is NOT `NULL`
- Returns inserted/updated entry `id` string

**MUTABILITY:**  
✅ **MUTATES DATABASE** — write operation with timestamp update

**CONSUMERS:**  
- **nom-app** (`src/lib.rs`):  
  - Lines: 1598–1599, 1680–1683, 1738–1744, 1781, 1803, 1825, 1926, 1929–1930, 1973–1975, 2029–2030, 2033, 2108–2111, 2175–2178, 2606, 2717
  - **Context:** Integration tests + app initialization code (19 call sites)
  
- **nom-corpus** (`src/lib.rs`):
  - Line: 1331
  - **Context:** Corpus ingest/seeding

**COMPLEXITY:**  
⭐⭐⭐ **COMPLEX** — Selective COALESCE merging logic across 21 columns must be preserved exactly

**MIGRATION CONCERNS:**  
- ❌ Highest call-site density in nom-app
- ❌ Selective COALESCE logic is subtle and easy to break
- ⚠️ Cross-tier: affects entries with concept references
- **Action:** Run `gitnexus_impact({target: "upsert_entry", direction: "upstream"})` first

---

### 2. upsert_entry_if_new — 🔴 HIGH RISK

**LOCATION:**  
File: `nom-compiler/crates/nom-dict/src/lib.rs`  
Lines: [454–488](nom-compiler/crates/nom-dict/src/lib.rs#L454)

**SIGNATURE:**  
```rust
pub fn upsert_entry_if_new(&self, entry: &Entry) -> Result<bool>
```

**BEHAVIOR:**  
- `INSERT OR IGNORE INTO entries` (21 columns same as `upsert_entry`)
- No UPDATE component: existence check only, no row modification if id exists
- Returns `true` (newly inserted), `false` (skipped, id existed)
- **Designed for:** Corpus deduplication without SELECT overhead

**MUTABILITY:**  
✅ **MUTATES DATABASE** — conditional insert based on primary key uniqueness

**CONSUMERS:**  
- **nom-cli** (`src/author.rs`):
  - Line: 294
  - **Context:** Author command flow (single site)
  
- **nom-corpus** (`src/lib.rs`):
  - Line: 479
  - **Context:** Corpus ingest loop (critical dedup path)

**COMPLEXITY:**  
⭐⭐ **STRAIGHTFORWARD** — `INSERT OR IGNORE` is simple, but **dedup semantics are critical**

**MIGRATION CONCERNS:**  
- ⚠️ Corpus ingest depends on **exact dedup behavior**
- ⚠️ Only 2 callers, but **high-leverage use case** (seeding)
- ❌ Breaking this in migration would corrupt corpus ingest pipeline
- **Action:** Run `gitnexus_impact({target: "upsert_entry_if_new", direction: "upstream"})` first

---

### 3. get_entry — 🟡 MEDIUM RISK

**LOCATION:**  
File: `nom-compiler/crates/nom-dict/src/lib.rs`  
Lines: [620–632](nom-compiler/crates/nom-dict/src/lib.rs#L620)

**SIGNATURE:**  
```rust
pub fn get_entry(&self, id: &str) -> Result<Option<Entry>>
```

**BEHAVIOR:**  
- `SELECT (21 columns) FROM entries WHERE id = ?1`
- Uses `row_to_entry()` helper to reconstruct `Entry` struct
- Returns `Some(Entry)` or `None`
- Handles enum deserialization for `status` and `kind` fields

**MUTABILITY:**  
✅ **READ-ONLY** — no database modifications

**CONSUMERS:**  
- **nom-app** (`src/lib.rs`):
  - Lines: 340, 389, 685
  - **Context:** Startup checks (heartbeat logic, root entry validation) — **startup-critical**

**COMPLEXITY:**  
⭐ **SIMPLE** — Straightforward SELECT → struct mapping, reuses existing helper

**MIGRATION CONCERNS:**  
- ✅ Read-only, low complexity
- ⚠️ Startup-critical: root entry checks must work immediately
- ⚠️ Enum deserialization (status/kind) must preserve all variants
- **Action:** Validate nom-app startup flow after porting

---

### 4. find_entries — 🔴 HIGH RISK

**LOCATION:**  
File: `nom-compiler/crates/nom-dict/src/lib.rs`  
Lines: [690–750](nom-compiler/crates/nom-dict/src/lib.rs#L690)

**SIGNATURE:**  
```rust
pub fn find_entries(&self, f: &EntryFilter) -> Result<Vec<Entry>>
```

**BEHAVIOR:**  
Dynamic SQL query builder:
```
SELECT (21 columns)
FROM entries
WHERE 1=1
  [AND body_kind = ?]      if filter.body_kind present
  [AND language = ?]       if filter.language present
  [AND status = ?]         if filter.status present
  [AND kind = ?]           if filter.kind present
ORDER BY id
LIMIT {filter.limit}  (default: 50)
```
- Empty `EntryFilter` returns first N entries
- All filters are optional AND clauses
- Results ordered by id for determinism

**MUTABILITY:**  
✅ **READ-ONLY** — no database modifications

**CONSUMERS:**  
- **nom-cli** (`src/store/commands.rs`):
  - Line: 620
  - **Context:** `dict list` command (entry listing with filters)
  
- **nom-cli** (`src/mcp.rs`):
  - Line: 282
  - **Context:** MCP query tool (for LLM entry search)
  
- **nom-dict** (`src/lib.rs`):
  - Line: 1057
  - **Context:** Internal method calls this method; requires refactoring

**COMPLEXITY:**  
⭐⭐⭐ **COMPLEX** — Dynamic SQL string building is fragile to reorder; must preserve AND clause sequence

**MIGRATION CONCERNS:**  
- ❌ Dynamic SQL ordering is brittle; any reorder breaks callers
- ⚠️ Internal self-reference (line 1057) needs refactoring when ported to free function
- ⚠️ 2 external callers (nom-cli commands) that depend on filter order
- **Action:** Run `gitnexus_impact({target: "find_entries", direction: "upstream"})` first

---

### 5. bulk_upsert — 🟡 MEDIUM RISK

**LOCATION:**  
File: `nom-compiler/crates/nom-dict/src/lib.rs`  
Lines: [898–942](nom-compiler/crates/nom-dict/src/lib.rs#L898)

**SIGNATURE:**  
```rust
pub fn bulk_upsert(&self, entries: &[Entry]) -> Result<usize>
```

**BEHAVIOR:**  
- Transaction-wrapped `INSERT OR IGNORE` batch operation
- Loop: `for e in entries: stmt.execute(params![...])`
- Returns count of rows actually inserted
- Similar to `upsert_entry_if_new`, but batched for efficiency
- **Designed for:** Corpus load with many duplicate ids expected

**MUTABILITY:**  
✅ **MUTATES DATABASE** — batch conditional insert in transaction

**CONSUMERS:**  
- **None currently in codebase**
- Likely reserved for future corpus bulk-load optimization

**COMPLEXITY:**  
⭐⭐ **MODERATE** — Transaction + prepared statement loop, straightforward logic

**MIGRATION CONCERNS:**  
- ✅ No current callers = low immediate risk
- ⚠️ Transaction semantics must be exact (all-or-nothing rollback)
- ⚠️ Return value (insert count) is important if re-enabled in future
- **Action:** Port with confidence; test transaction behavior

---

### 6. add_graph_edge — 🟢 LOW RISK

**LOCATION:**  
File: `nom-compiler/crates/nom-dict/src/lib.rs`  
Lines: [585–594](nom-compiler/crates/nom-dict/src/lib.rs#L585)

**SIGNATURE:**  
```rust
pub fn add_graph_edge(&self, edge: &GraphEdge) -> Result<()>
```

**BEHAVIOR:**  
- `INSERT INTO entry_graph_edges (from_id, to_id, edge_type, confidence) VALUES (?1, ?2, ?3, ?4)`
- Creates directed graph edge linking two entries
- No UPDATE on conflict; inserts only
- Returns `Ok(())` on success

**MUTABILITY:**  
✅ **MUTATES DATABASE** — writes edge rows

**CONSUMERS:**  
- **None currently in codebase**
- Appears to be a placeholder API for future graph operations

**COMPLEXITY:**  
⭐ **SIMPLE** — Straightforward `INSERT`; no conditional logic

**MIGRATION CONCERNS:**  
- ✅ No current callers = safe to port
- ✅ Simple, isolated logic
- ⚠️ Consider whether this belongs in entities-tier vs. a separate graph-tier
- **Action:** Port freely; may be a future integration point

---

### 7. add_translation — 🟢 LOW RISK

**LOCATION:**  
File: `nom-compiler/crates/nom-dict/src/lib.rs`  
Lines: [600–615](nom-compiler/crates/nom-dict/src/lib.rs#L600)

**SIGNATURE:**  
```rust
pub fn add_translation(&self, t: &Translation) -> Result<()>
```

**BEHAVIOR:**  
- `INSERT OR IGNORE INTO entry_translations (id, target_language, body, confidence, translator_version, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)`
- Appends a translation for an entry in a target language
- Unique constraint: `(id, target_language, translator_version)` enforced by `INSERT OR IGNORE`
- No update on duplicate key—just skips silently
- Returns `Ok(())` on success

**MUTABILITY:**  
✅ **MUTATES DATABASE** — conditional insert of translation rows

**CONSUMERS:**  
- **None currently in codebase**
- Placeholder for future i18n/translation pipeline

**COMPLEXITY:**  
⭐ **SIMPLE** — Straightforward `INSERT OR IGNORE`; no logic

**MIGRATION CONCERNS:**  
- ✅ No current callers = safe to port
- ✅ Clean, isolated logic
- ⚠️ Unique key constraint must be preserved
- **Action:** Port freely; document for future i18n work

---

### 8. bulk_set_scores — 🟢 LOW RISK

**LOCATION:**  
File: `nom-compiler/crates/nom-dict/src/lib.rs`  
Lines: [1357–1387](nom-compiler/crates/nom-dict/src/lib.rs#L1357)

**SIGNATURE:**  
```rust
pub fn bulk_set_scores(&self, scores: &[EntryScores]) -> Result<()>
```

**BEHAVIOR:**  
- Transaction-wrapped `INSERT OR REPLACE` batch operation
- Loop: `for s in scores: stmt.execute(params![...])`
- Sets 9 quality score fields:
  - `security, reliability, performance, readability, testability, portability, composability, maturity, overall_score`
- Returns `Ok(())` on success
- **Designed for:** Bulk analytics/scoring pipeline

**MUTABILITY:**  
✅ **MUTATES DATABASE** — batch replace score rows in transaction

**CONSUMERS:**  
- **None currently in codebase**
- Placeholder for future analytics/scoring infrastructure

**COMPLEXITY:**  
⭐⭐ **MODERATE** — Transaction + prepared statement loop, straightforward replace logic

**MIGRATION CONCERNS:**  
- ✅ No current callers = safe to port
- ✅ Transaction semantics are straightforward (replace is idempotent)
- ⚠️ 9 score fields must be mapped correctly from `EntryScores` struct
- **Action:** Port freely; verify field mapping once enabled

---

## Risk Profile & Migration Order

### Tier 1: No-Risk Batch (Port First)
These have **no current consumers** and pose **no immediate risk**:
- ✅ add_graph_edge
- ✅ add_translation
- ✅ bulk_set_scores

**Why First:** Immediate confidence that porting is safe; no consumer surfacing needed.

### Tier 2: Medium-Risk (Port with Validation)
These have **straightforward logic** but **require startup/transaction verification**:
- ⚠️ get_entry (startup-critical)
- ⚠️ bulk_upsert (transaction semantics)

**Why Second:** Port, then test nom-app startup flow and future corpus ops.

### Tier 3: High-Risk (GitNexus Analysis Required)
These have **complex semantics** and **multiple high-impact consumers**:
- 🔴 **upsert_entry** (20+ call sites, selective COALESCE merging)
- 🔴 **upsert_entry_if_new** (corpus dedup critical path)
- 🔴 **find_entries** (dynamic SQL, internal self-ref)

**Why Last:** Requires blast-radius analysis to avoid breaking nom-app/nom-corpus.

---

## GitNexus Impact Analysis Checklist

Before porting **each high-risk method**, run:

```bash
# Run impact analysis in this order:
gitnexus_impact({target: "upsert_entry", direction: "upstream"})
gitnexus_impact({target: "upsert_entry_if_new", direction: "upstream"})
gitnexus_impact({target: "find_entries", direction: "upstream"})
```

For each, verify:
- ✅ Direct callers (depth=1) and count
- ✅ Transitive callers (depth=2+) and risk level
- ✅ Any cross-tier dependencies (concept, graph, intent)
- ✅ Which call sites are tests vs. runtime code

---

## Dual-Path Strategy (Per Cycle 3 Plan)

For **high-risk methods**, follow this sequence:

1. **Port to free function on `&Dict`** (in `dict.rs`)
   - Add: `pub fn upsert_entry(dict: &Dict, entry: &Entry) -> Result<String>`
   - Preserve all logic exactly as-is

2. **Keep `NomDict` method live** (per "no legacy deletion" rule)
   - Both live in parallel during consumer migration

3. **Add bridging tests** for each new free function
   - Test semantics match NomDict version

4. **Gradually migrate consumers** (highest-impact first)
   - nom-cli → use &Dict version
   - nom-app → use &Dict version
   - nom-corpus → use &Dict version

5. **Only remove `NomDict` method** when **all consumers** are bridged
   - Final cleanup: delete legacy NomDict impl
   - Update mission checklog

---

## Expected Effort Estimate

| Task | Tier | Time | Notes |
|------|------|------|-------|
| Impact analysis | High | 1–2h | GitNexus queries + result review |
| Port Tier 1 (3 methods) | Low | 30m | No consumers to validate |
| Port Tier 2 (2 methods) | Medium | 1h | Add startup + transaction tests |
| Port Tier 3 (3 methods) | High | 2–3h | Each requires careful semantics verification |
| Consumer migration (nom-cli) | Medium | 1–2h | Bridge 2–3 call sites |
| Consumer migration (nom-app) | High | 2–3h | 20+ call sites, careful refactoring |
| Consumer migration (nom-corpus) | Medium | 1h | 2 call sites, dedup path |
| Final legacy cleanup | Low | 30m | Delete NomDict, update checklog |
| **Total** | — | **9–13h** | **Spread across 1–2 development cycles** |

---

## Test Strategy

For each method ported:

1. **Existence test:** Method compiles and is callable
2. **Semantics test:** Output matches NomDict version on same input
3. **Consumer test:** Each caller still works after porting
4. **Integration test:** nom-cli, nom-app, nom-corpus tests pass

Run after each batch:
```bash
cargo test -p nom-dict --quiet
cargo test -p nom-cli --quiet
cargo check --workspace --message-format short
```

---

## Success Criteria (End of Cycle 3)

- ✅ All 8 methods ported to `&Dict` free functions in `dict.rs`
- ✅ GitNexus impact analysis completed for high-risk 3
- ✅ Tier 1 (low-risk) methods fully ported and tested
- ✅ Tier 2 (medium-risk) methods ported + startup/transaction tests passing
- ✅ Tier 3 (high-risk) methods ported + consumer validation in progress
- ✅ nom-cli fully bridged off `NomDict` for these methods
- ✅ nom-app and nom-corpus bridging 75%+ complete
- ✅ Mission checklog updated with progress
- ✅ Workspace tests passing, no regressions

---

## Next Steps

1. **Immediate:** Document this spec in `research/CYCLE-3-MIGRATION-SPEC.md`
2. **Prepare:** Set up GitNexus analysis environment
3. **Execute:** Start with Tier 1 (low-risk) porting batch
4. **Validate:** Run corpus and nom-app integration tests
5. **Progress:** Move to Tier 2, then Tier 3 as consumer migration advances
