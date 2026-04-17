# 03 — Architecture

## Three tiers above the artifact store

- **Tier 0 — atomic entities.** Each entity is one row in
  `entities.sqlite.entities` carrying `(hash, name, kind, signature,
  contracts, body_kind, body_size, origin_ref, bench_ids, authored_in,
  composed_of)`. The hash is the canonical sha256 of the entity's body and
  contract. Bodies live in the artifact store at
  `~/.nom/store/<hash>/body.{bc,avif,...}`.
- **Tier 1 — modules.** A `.nomtu` file declares a small-scope group of
  entities plus optional composition expressions binding them. The compiler
  ingests the per-entity bytes and the composition manifest to produce the
  module body, also a Tier-0 entity row.
- **Tier 2 — concepts.** A `.nom` file declares a big-scope concept whose
  body is the dictionary-relative index over Tier-0 entities. Each concept is
  one row in `concepts.sqlite.concept_defs` carrying `(name, repo_id, intent,
  index_into_db2, exposes, acceptance, objectives, src_path, src_hash,
  body_hash)`. The root `app.nom` is itself a concept whose index composes
  the rest.

## Three SQLite files at `~/.nom/`

- `concepts.sqlite` — DB1: per-repo concept registry. Tables: `concepts`
  (legacy, scheduled for deletion), `concept_members` (legacy), `concept_defs`
  (canonical), `required_axes` (per-scope MECE registry), `dict_meta`.
- `entities.sqlite` — DB2: nomtu entities. Tables: `entries` (legacy,
  scheduled for deletion), `entry_scores`, `entry_meta`, `entry_signatures`,
  `entry_security_findings`, `entry_refs`, `entry_graph_edges`,
  `entry_translations`, `entities` (canonical), `dict_meta`.
- `grammar.sqlite` — language registry. Tables: `schema_meta`,
  `keywords`, `keyword_synonyms`, `clause_shapes`, `kinds`,
  `quality_names`, `patterns`. AI clients query this to determine
  intent → synthesis without reading any markdown. Canonical baseline
  ships at `nom-compiler/crates/nom-grammar/data/baseline.sql`
  (9 kinds + 20 quality_names + 43 keywords + 7 keyword_synonyms +
  43 clause_shapes + 258 patterns).

The artifact store at `~/.nom/store/<hash>/` is a filesystem tree (not
SQLite). Each leaf is the compiled body for one entity, content-addressed.

## Closed kind set (9 nouns)

`function`, `module`, `concept`, `screen`, `data`, `event`, `media`,
`property`, `scenario`. Each kind has its own clause-shape grammar (see
`grammar.sqlite.clause_shapes`). New kinds require a wedge.

## Edge types (28 variants)

`Calls`, `Imports`, `Uses`, `Specializes`, `BindsTo`, `Triggers`, `Reads`,
`Writes`, `NavigatesTo`, `RunsOn`, `HasFlowArtifact`, `FlowsTo`, `Encodes`,
`ContainedIn`, `UsesColor`, `UsesPalette`, `Derives`, `EmbeddedGlyph`,
`Frame`, `RendersOn`, `Styles`, `Constrains`, `Recommends`, `InteractsWith`,
`TransitionsTo`, plus a few reserved. Edges land in
`entities.sqlite.entry_graph_edges`.

## Hash-as-identity invariant

Every entity is uniquely identified by `sha256(canonicalize(body, contract))`.
Two entities with the same canonicalization collapse to one row. The
artifact store is content-addressed off the same hash.

## No cross-file foreign keys

Per the dict-split design, references between SQLite files are stored as raw
hash strings; the Rust layer resolves them. SQLite cannot enforce foreign
keys across attached databases, so cross-file integrity is a runtime
contract, not a schema constraint.

## No legacy after new

When new code or new schema replaces old, the old is deleted in the same
commit per the no-legacy rule. The `entries` table, `concepts` table, the
single-file `NomDict` struct, and the V2/V3/V4/V5_SCHEMA_SQL constants are
all queued for deletion as part of the in-flight dict-split S3b–S8 work.
