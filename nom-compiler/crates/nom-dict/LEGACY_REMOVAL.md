# Legacy `concepts` and `concept_members` Table Removal

Audit date: 2026-04-16

## Background

Two parallel table families exist in the codebase:

| Family | Tables | Home | Purpose |
|--------|--------|------|---------|
| **Legacy** | `concepts`, `concept_members` | single-file `NomDict.conn` (legacy `nom.sqlite`) AND split-file `concepts.sqlite` (DB1 via `Dict.concepts`) | Manual, LLM-facing groupings of `entries` rows by domain name |
| **Canonical** | `concept_defs` | `concepts.sqlite` (DB1 via `Dict.concepts`) AND legacy `nom.sqlite` via `V3_SCHEMA_ADDITIONS_SQL` | Parser-driven, one row per `.nom` concept file synced by `nom store sync` |

These are **distinct concerns**. `concept_defs` is NOT a replacement for `concepts`/`concept_members` — they serve different purposes. Removal of the legacy tables requires either:
1. Deciding that the LLM-facing grouping feature is dropped entirely, OR
2. Migrating `concept_members` to reference entity hashes (currently it references `entries.id`, but `entries` is itself a legacy table being migrated out)

---

## Schema Locations

### `lib.rs` (legacy `NomDict` single-file path)

**File:** `crates/nom-dict/src/lib.rs` lines 201–216

```sql
CREATE TABLE IF NOT EXISTS concepts (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    describe    TEXT,
    created_at  TEXT DEFAULT (datetime('now')),
    updated_at  TEXT
);

CREATE TABLE IF NOT EXISTS concept_members (
    concept_id  TEXT NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    entry_id    TEXT NOT NULL REFERENCES entries(id),   -- FK to entries
    added_at    TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (concept_id, entry_id)
);
```

Note: in `lib.rs`, `concept_members.entry_id` has a FOREIGN KEY to `entries(id)`.

### `dict.rs` (split-file `Dict` two-file path)

**File:** `crates/nom-dict/src/dict.rs` lines 39–56 (`CONCEPTS_SCHEMA_SQL`)

```sql
CREATE TABLE IF NOT EXISTS concepts ( ... );

CREATE TABLE IF NOT EXISTS concept_members (
    concept_id  TEXT NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    entry_id    TEXT NOT NULL,   -- NO FK: dangling hash reference per doc 22 §1
    ...
);
```

Note: in `dict.rs`, the FK is intentionally absent. `entry_id` is a dangling hash into `entities.sqlite`.

---

## All Call Sites

### `nom-dict/src/lib.rs` — `NomDict` methods (legacy struct)

| Method | SQL | Line | Can Delete? |
|--------|-----|------|-------------|
| `NomDict::upsert_concept` | `INSERT INTO concepts` | 1017 | Blocked — callers exist |
| `NomDict::get_concept_by_name` | `SELECT FROM concepts WHERE name` | 1036 | Blocked — callers exist |
| `NomDict::list_concepts` | `SELECT FROM concepts ORDER BY name` | 1050 | Blocked — callers exist |
| `NomDict::delete_concept` | `DELETE FROM concepts WHERE name` | 1062 | Blocked — callers exist |
| `NomDict::add_concept_member` | `INSERT OR IGNORE INTO concept_members` | 1071 | Blocked — callers exist |
| `NomDict::remove_concept_member` | `DELETE FROM concept_members WHERE concept_id AND entry_id` | 1080 | Blocked — callers exist |
| `NomDict::get_concept_members` | `JOIN concept_members ON entry_id` | 1088 | Blocked — callers exist |
| `NomDict::count_concept_members` | `SELECT COUNT(*) FROM concept_members` | 1107 | Blocked — callers exist |
| `NomDict::add_concept_members_by_filter` | `INSERT OR IGNORE INTO concept_members` (bulk) | 1119 | Blocked — callers exist |

### `nom-dict/src/dict.rs` — free functions on `Dict` (split-file path)

| Function | SQL tables used | Line | Can Delete? |
|----------|----------------|------|-------------|
| `get_concept_id_by_name` | `SELECT FROM concepts WHERE name` | 711 | Blocked — callers exist |
| `list_concept_ids` | `SELECT FROM concepts ORDER BY name` | 726 | Blocked — callers exist |
| `delete_concept` | `DELETE FROM concepts WHERE name` | 740 | Blocked — callers exist |
| `add_concept_member` | `INSERT OR IGNORE INTO concept_members` | 753 | Blocked — callers exist |
| `upsert_concept` | `INSERT INTO concepts ... ON CONFLICT` | 929 | Blocked — callers exist |
| `get_concept_by_name` | `SELECT FROM concepts WHERE name` | 950 | Blocked — callers exist |
| `list_concepts` | `SELECT FROM concepts ORDER BY name` | 973 | Blocked — callers exist |
| `remove_concept_member` | `DELETE FROM concept_members WHERE` | 993 | Blocked — callers exist |
| `get_concept_members` | `SELECT entry_id FROM concept_members` then cross-queries `entries` | 1008 | MIGRATION BLOCKED (see below) |
| `count_concept_members` | `SELECT COUNT(*) FROM concept_members` | 1038 | Blocked — callers exist |
| `add_concept_members_by_filter` | reads `entries`, writes `concept_members` | 1091 | MIGRATION BLOCKED (see below) |

### `nom-dict/src/lib.rs` — exports

**File:** `crates/nom-dict/src/lib.rs` lines 20, 31, 45, 210–216

Re-exports from `dict.rs`: `add_concept_members_by_filter`, `count_concept_members`, `get_concept_members`.
Schema constant `V2_SCHEMA_SQL` (line 210–216) contains both `concepts` and `concept_members` table DDL.

### `nom-cli/src/concept.rs` — CLI subcommands

**File:** `crates/nom-cli/src/concept.rs`

| Handler | Functions used | Purpose |
|---------|---------------|---------|
| `cmd_concept_new` | `upsert_concept`, `upsert_entry` | `nom concept new <name>` |
| `cmd_concept_add` | `get_concept_by_name`, `add_concept_member` | `nom concept add <concept> <entry>` |
| `cmd_concept_add_by` | `get_concept_by_name`, `add_concept_members_by_filter`, `add_concept_member` | `nom concept add-by` |
| `cmd_concept_list_filtered` | `list_concepts`, `count_concept_members` | `nom concept list [--empty]` |
| `cmd_concept_show` | `get_concept_by_name`, `get_concept_members` | `nom concept show <name>` |
| `cmd_concept_delete` | `get_concept_by_name`, `delete_concept` | `nom concept delete <name>` |

### `nom-cli/src/mcp.rs` — MCP tool handlers

**File:** `crates/nom-cli/src/mcp.rs` lines 17–18, 267, 410–468

| MCP Tool | Functions used |
|----------|---------------|
| `list_concepts` | `list_concepts`, `count_concept_members` |
| `get_concept` | `get_concept_by_name`, `get_concept_members` |

These are exposed as named MCP tools to LLM clients (line 175, 191).

### `nom-cli/src/author.rs` — authoring pipeline

**File:** `crates/nom-cli/src/author.rs` lines 239, 262, 312, 849, 880

- `upsert_concept` and `add_concept_member` called during `nom author commit` flow to group a new entry under its parent concept
- `list_concepts` used in an internal test helper at line 880

---

## Migration Blockers

### Blocker 1: `get_concept_members` and `add_concept_members_by_filter` — cross-table dependency

**File:** `crates/nom-dict/src/dict.rs` lines 1004–1007 and 1087–1090

These functions bridge `concept_members.entry_id` to the legacy `entries` table. The `entities` table (DB2-v2, the replacement for `entries`) uses `hash` as its PK, not `id`. Until `concept_members.entry_id` is migrated to reference `entities.hash` values, these functions cannot be moved or the tables dropped without losing membership resolution.

**Resolution path:** Extend `concept_members` with an `entity_hash TEXT` column (or rename `entry_id` → `entity_hash` in a migration), and update both functions to join against `entities` instead of `entries`.

### Blocker 2: `NomDict` methods on the legacy single-file path

**File:** `crates/nom-dict/src/lib.rs`

The legacy `NomDict` struct still has all concept/concept_members methods. All callers in `nom-cli/src/concept.rs`, `nom-cli/src/mcp.rs`, and `nom-cli/src/author.rs` currently call the **free function** versions from `dict.rs` (they pass `&Dict`, not `&NomDict`), so the `NomDict` methods on `lib.rs` appear to be dead code after the split-file migration. However, the schema DDL in `V2_SCHEMA_SQL` still creates the tables on the legacy path.

**Resolution path:** Confirm no test or binary opens a `NomDict` and calls concept methods directly, then delete the `NomDict` methods and remove the `concepts`/`concept_members` DDL from `V2_SCHEMA_SQL`.

### Blocker 3: `nom concept` CLI subcommands

If `concepts`/`concept_members` are deleted, the entire `nom concept` subcommand family (`new`, `add`, `add-by`, `list`, `show`, `delete`) and both MCP tools (`list_concepts`, `get_concept`) stop working. Either:
- (a) The feature is intentionally dropped (delete `concept.rs`, remove MCP registrations), OR
- (b) The feature is migrated to use `concept_defs` + a new membership table that references `entities.hash`

---

## Removal Priority Table

| Item | Priority | Status | Blocker |
|------|----------|--------|---------|
| `NomDict::upsert_concept` | Low | Can delete after confirming no direct callers of `NomDict` method | Blocker 2 |
| `NomDict::get_concept_by_name` | Low | Same as above | Blocker 2 |
| `NomDict::list_concepts` | Low | Same as above | Blocker 2 |
| `NomDict::delete_concept` | Low | Same as above | Blocker 2 |
| `NomDict::add_concept_member` | Low | Same as above | Blocker 2 |
| `NomDict::remove_concept_member` | Low | Same as above | Blocker 2 |
| `NomDict::get_concept_members` | Low | Same as above | Blocker 2 |
| `NomDict::count_concept_members` | Low | Same as above | Blocker 2 |
| `NomDict::add_concept_members_by_filter` | Low | Same as above | Blocker 2 |
| `concepts`/`concept_members` DDL in `V2_SCHEMA_SQL` | Low | Needs NomDict methods gone first | Blocker 2 |
| `dict.rs::get_concept_members` | Medium | BLOCKED on `entries`→`entities` migration | Blocker 1 |
| `dict.rs::add_concept_members_by_filter` | Medium | BLOCKED on `entries`→`entities` migration | Blocker 1 |
| `dict.rs::upsert_concept` et al. (non-member functions) | Medium | Blocked on CLI/MCP decision | Blocker 3 |
| `nom-cli/src/concept.rs` (all handlers) | Medium | Decision required: drop or migrate feature | Blocker 3 |
| MCP tools `list_concepts`, `get_concept` | Medium | Decision required: drop or migrate feature | Blocker 3 |
| `nom-cli/src/author.rs` concept writes | Medium | Decision required | Blocker 3 |
| `concepts`/`concept_members` DDL in `CONCEPTS_SCHEMA_SQL` | Last | After all callers gone | Blockers 1+2+3 |

---

## Recommended Sequence

1. **Decide on feature intent** (Blocker 3): should `nom concept` and the MCP grouping tools survive into the split-file era? If yes, map to `concept_defs` + a new `concept_members_v2` that references `entities.hash`. If no, delete `nom-cli/src/concept.rs` and remove the MCP tool registrations.

2. **Migrate `get_concept_members` and `add_concept_members_by_filter`** (Blocker 1): extend `concept_members` with `entity_hash`, populate it during `nom store sync`, update the two functions.

3. **Audit and delete `NomDict` methods** (Blocker 2): grep for direct `NomDict` usage of concept methods in tests; if none found, delete from `lib.rs` and strip DDL from `V2_SCHEMA_SQL`.

4. **Drop tables** (final step): once all callers are gone, remove the `CREATE TABLE` statements from both `CONCEPTS_SCHEMA_SQL` (`dict.rs`) and `V2_SCHEMA_SQL` (`lib.rs`), and add `DROP TABLE IF EXISTS concepts; DROP TABLE IF EXISTS concept_members;` to a migration guard in `init_schema`.
