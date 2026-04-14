# 06 — Dict Design

## Three SQLite files, one filesystem tree

```
~/.nom/
  concepts.sqlite       (DB1)
  entities.sqlite       (DB2)
  grammar.sqlite        (registry — language structural surface)
  store/<hash>/body.{bc,avif,mp4,wav,svg,...}
```

All three SQLite files are populated separately from the Rust binary —
the binary ships only schema + connection helpers + query API. Data is
the user's responsibility (via SQL, row-level CLI, or downstream
tooling).

## Concept (DB1)

`concepts.sqlite` holds per-repo concept metadata. Tables (canonical
only; legacy scheduled for deletion):

- `concept_defs` — one row per concept declared in a `.nom` file.
  Columns: `name TEXT PK, repo_id TEXT, intent TEXT, index_into_db2
  TEXT (JSON list of entity hashes the concept composes), exposes TEXT,
  acceptance TEXT, objectives TEXT, src_path TEXT, src_hash TEXT,
  body_hash TEXT, created_at, updated_at`. The `index_into_db2` column
  is the manifest the compiler walks to materialize the concept's body.
- `required_axes` — per-scope MECE registry. Columns: `(axis, scope,
  cardinality, repo_id, registered_at)`.
- `dict_meta` — key-value freshness state.

Legacy tables `concepts` and `concept_members` remain temporarily and
delete in the dict-split S8 commit per the no-legacy rule.

## Entity (DB2)

`entities.sqlite` holds nomtu entities. Tables (canonical only):

- `entities` — one row per nomtu hash. Columns: `hash TEXT PK, name
  TEXT, kind TEXT, signature TEXT, contracts TEXT, body_kind TEXT,
  body_size INTEGER, origin_ref TEXT, bench_ids TEXT, authored_in
  TEXT, composed_of TEXT, created_at, updated_at`.
- `entry_scores`, `entry_meta`, `entry_signatures`,
  `entry_security_findings`, `entry_refs`, `entry_graph_edges`,
  `entry_translations` — per-entity side tables.
- `dict_meta` — freshness tracking, mirroring the concepts tier so
  each file tracks its own staleness independently.

Legacy table `entries` remains temporarily and deletes in the dict-
split S8 commit.

## Grammar (registry)

`grammar.sqlite` is the language registry. Six tables, all with
schema-only support in Rust; data is user-populated:

- `schema_meta` — `(key, value)` pairs stamping the schema version
- `keywords` — every reserved token in `.nomx` source: `(token PK,
  role, kind_scope, source_ref, shipped_commit, notes)`
- `clause_shapes` — per-kind grammar of which clauses each kind
  accepts: `(kind, clause_name, is_required, one_of_group, position,
  grammar_shape, min_occurrences, max_occurrences, source_ref, notes)`
  PK on `(kind, clause_name, position)`
- `kinds` — the closed top-level kinds: `(name PK, description,
  allowed_clauses, allowed_refs, shipped_commit, notes)`
- `quality_names` — quality axes referenced by `favor` clauses:
  `(name PK, axis, metric_function nullable, cardinality, required_at,
  source_ref, notes)`
- `patterns` — native authoring patterns: `(pattern_id PK, intent,
  nom_kinds, nom_clauses, typed_slot_refs, example_shape, hazards,
  favors, source_doc_refs, created_at)`

After `nom grammar init`, every table is empty. The user populates rows
through SQL, row-level CLI commands (planned: `nom grammar add-keyword`,
`nom grammar add-pattern`, etc.), or batch SQL imports.

## Artifact store

Filesystem tree at `~/.nom/store/<hash>/body.{bc,avif,mp4,wav,svg,...}`.
Bytes are written exactly once per content hash; the tree never updates
in place. Bodies of any size land here; the entity row in
`entities.sqlite` carries `body_size` and `body_kind` but not the
bytes.

## Cross-file references

There are no SQLite-level foreign keys across files. Cross-tier joins
(a concept's `index_into_db2` referencing entity hashes) are stored as
raw hash strings; the Rust layer resolves them via the `Dict` struct
holding both connections.

## Open invariants

- Hash-as-identity: `sha256(canonicalize(body, contract))` is the sole
  identifier. Two entities with the same canonicalization collapse to
  one row.
- Body bytes live in the artifact store, not in the SQL schema.
- Per-tier schemas: `concepts.sqlite` carries DB1 tables only;
  `entities.sqlite` carries DB2 tables only; `grammar.sqlite` carries
  registry tables only.
- Grammar data lives in the DB, never in Rust source. The
  `nom-grammar` crate is awareness-only (schema + queries).
