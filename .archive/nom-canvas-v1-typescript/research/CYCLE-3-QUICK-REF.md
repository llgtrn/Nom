# Quick Reference: Cycle 3 Entities-Tier Methods

| # | Method | Location | Risk | Consumers | Mutates | Status |
|---|--------|----------|------|-----------|---------|--------|
| 1 | `upsert_entry` | L395 | 🔴 HIGH | nom-app(20), nom-corpus(1) | ✅ | Needs impact analysis |
| 2 | `upsert_entry_if_new` | L454 | 🔴 HIGH | nom-cli(1), nom-corpus(1) | ✅ | Needs impact analysis |
| 3 | `get_entry` | L620 | 🟡 MED | nom-app(3) | ❌ | Ready to port |
| 4 | `find_entries` | L690 | 🔴 HIGH | nom-cli(2), nom-dict(1) | ❌ | Needs impact analysis |
| 5 | `bulk_upsert` | L898 | 🟡 MED | —none— | ✅ | Ready to port |
| 6 | `add_graph_edge` | L585 | 🟢 LOW | —none— | ✅ | Ready to port |
| 7 | `add_translation` | L600 | 🟢 LOW | —none— | ✅ | Ready to port |
| 8 | `bulk_set_scores` | L1357 | 🟢 LOW | —none— | ✅ | Ready to port |

## Risk-Based Porting Order

**Phase 1 (Low-Risk):** add_graph_edge, add_translation, bulk_set_scores  
→ *Can be ported without consumer bridging*

**Phase 2 (Medium-Risk):** get_entry, bulk_upsert  
→ *Port + test startup/transaction semantics*

**Phase 3 (High-Risk):** upsert_entry, upsert_entry_if_new, find_entries  
→ **REQUIRES:** `gitnexus_impact()` analysis before porting

## Design Constraints

- ✅ All 8 methods must be ported to `dict.rs` as free functions on `&Dict`
- ✅ Legacy `NomDict` methods must remain live (no deletion until consumers bridged)
- ✅ Selective COALESCE merging in `upsert_entry` is **critical**—preserve exactly
- ✅ `upsert_entry_if_new` dedup semantics are **critical for corpus ingest**
- ✅ `find_entries` dynamic SQL ordering must not change

## Impact Analysis Commands (Before Porting Tier 3)

```bash
# For high-risk methods, run these before porting:
gitnexus_impact({target: "upsert_entry", direction: "upstream"})
gitnexus_impact({target: "upsert_entry_if_new", direction: "upstream"})
gitnexus_impact({target: "find_entries", direction: "upstream"})
```

## Validation After Each Batch

```bash
cargo test -p nom-dict --quiet
cargo test -p nom-cli --quiet
cargo check --workspace --message-format short
```

---

**Full Spec:** [CYCLE-3-MIGRATION-SPEC.md](CYCLE-3-MIGRATION-SPEC.md)  
**Session Notes:** [/memories/session/cycle-3-methods.md](/memories/session/cycle-3-methods.md)
