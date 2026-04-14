# 06 — Dict Design

## Three SQLite files, one filesystem tree

```
~/.nom/
  concepts.sqlite       (DB1)
  entities.sqlite       (DB2)
  grammar.sqlite        (registry)
  store/<hash>/body.{bc,avif,mp4,...}
```

## Concept

`concepts.sqlite` holds per-repo concept metadata. Tables (canonical only;
legacy scheduled for deletion):

- `concept_defs` — one row per concept declared in a `.nom` file. Columns:
  `name TEXT PK, repo_id TEXT, intent TEXT, index_into_db2 TEXT (JSON list
  of entity hashes the concept composes), exposes TEXT (JSON list of
  exposed names), acceptance TEXT (JSON list of predicates), objectives
  TEXT (JSON list of QualityName refs), src_path TEXT, src_hash TEXT,
  body_hash TEXT, created_at, updated_at`. The `index_into_db2` column is
  the load-bearing one — the manifest the compiler walks to materialize
  the concept's body.
- `required_axes` — per-scope MECE registry. Rows: `(axis, scope,
  cardinality, repo_id, registered_at)`. The MECE validator at composition
  time checks every concept's objectives covers every required-axis row
  for the scope.
- `dict_meta` — key-value freshness state. Used by the freshness-tracking
  layer to detect stale-dict situations.

## Entity

`entities.sqlite` holds nomtu entities (formerly mis-named "words"). Tables
(canonical only):

- `entities` — one row per nomtu hash. Columns: `hash TEXT PK, name TEXT
  (the entity's name), kind TEXT (one of nine closed kinds), signature
  TEXT, contracts TEXT (JSON), body_kind TEXT, body_size INTEGER,
  origin_ref TEXT (where the entity came from — corpus URI or authored
  path), bench_ids TEXT, authored_in TEXT (path to the .nomtu file that
  declared the entity, NULL if ingested), composed_of TEXT (JSON list of
  entity hashes this composes, NULL if atomic), created_at, updated_at`.
- `entry_scores`, `entry_meta`, `entry_signatures`, `entry_security_findings`,
  `entry_refs`, `entry_graph_edges`, `entry_translations` — per-entity
  side tables.
- `dict_meta` — freshness tracking, mirroring the concepts tier so each
  file tracks its own staleness independently.

## Grammar

`grammar.sqlite` is the language registry. Five tables, all populated from
native Nom content via `nom grammar seed`:

- `kinds` — the nine closed top-level kinds with description, allowed
  clauses (derived from `clause_shapes`), allowed @Kind refs.
- `quality_names` — quality axes referenced by `favor` clauses; cardinality
  + required-at-scope.
- `keywords` — every reserved token in `.nomx` source with role + kind
  scope where applicable.
- `clause_shapes` — per-kind grammar of which clauses each kind accepts,
  in canonical authoring order, required vs optional, with a parser-
  acceptable example shape.
- `patterns` — native authoring patterns. Each row: `pattern_id TEXT PK,
  intent TEXT, nom_kinds TEXT JSON, nom_clauses TEXT JSON, typed_slot_refs
  TEXT JSON, example_shape TEXT (parser-acceptable), hazards TEXT JSON,
  favors TEXT JSON, source_doc_refs TEXT JSON, created_at`.

Every row in every grammar table is Nom-native. Foreign-language names are
absent by invariant.

## Artifact store

Filesystem tree at `~/.nom/store/<hash>/body.{bc,avif,mp4,wav,svg,...}`.
Bytes are written exactly once per content hash; the tree never updates
in-place. Bodies of any size land here; the entity row in
`entities.sqlite` carries `body_size` and `body_kind` but not the bytes
themselves.

## Cross-file references

There are no SQLite-level foreign keys across files. Cross-tier joins (a
concept's `index_into_db2` referencing entity hashes; a `concept_members`
row referencing an entity hash) are stored as raw hash strings; the Rust
layer resolves them via the `Dict` struct holding both connections.

## Open invariants

- Hash-as-identity: `sha256(canonicalize(body, contract))` is the sole
  identifier. Two entities with the same canonicalization collapse to one
  row.
- Body-bytes invariant: bodies live in the artifact store, not the SQL
  schema. The `entries` legacy table's `body_bytes BLOB` column is part of
  the legacy path scheduled for deletion.
- Per-tier schemas: `concepts.sqlite` carries DB1 tables only;
  `entities.sqlite` carries DB2 tables only. The full-merged `V2_SCHEMA_SQL`
  + V3/V4/V5 additions are part of the legacy path scheduled for deletion.
