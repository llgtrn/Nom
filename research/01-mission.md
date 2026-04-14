# 01 — Mission

## Mission

Nom is a self-defined programming language for AI-assisted application
synthesis. The compiler determines author intent from `.nomx` source and
produces working applications by composing entries from a content-addressed
dictionary. Nom is defined entirely on its own terms — no foreign-language
names appear in its surface, no parallel formats coexist, no wrappers
bridge to legacy code.

## Current state

Three SQLite tiers at `~/.nom/`:

- `concepts.sqlite` — DB1 — per-repo concept registry.
- `entities.sqlite` — DB2 — nomtu entities.
- `grammar.sqlite` — registry — language structural surface.

Plus a content-addressed artifact store at
`~/.nom/store/<hash>/body.{bc,avif,...}`.

The compiler workspace ships ~30 crates including `nom-grammar`,
`nom-dict`, `nom-concept`, `nom-parser`, `nom-llvm`, `nom-cli`, `nom-corpus`,
`nom-lsp`, `nom-intent`, `nom-app`. The self-hosting lexer compiles
end-to-end through the LLVM backend.

`nom-grammar` ships the schema for grammar.sqlite (six tables:
`schema_meta`, `keywords`, `clause_shapes`, `kinds`, `quality_names`,
`patterns`) and the connection / query API. It contains zero grammar
data — every row is the user's responsibility to populate via SQL or
future row-level CLI commands. After `nom grammar init`, the registry is
empty schema, ready to receive content.

`nom-dict` ships the `Dict { concepts, entities }` struct with per-tier
SQLite connections, the schema for both tiers, and a free-function API
for entity reads/writes. The legacy single-file `NomDict` struct, the
legacy `entries` table, the legacy `concepts` table, and the
V2/V3/V4/V5_SCHEMA_SQL constants are queued for deletion under the
no-legacy rule.

## Target state

- Compiler is itself authored in Nom; bootstrap fixpoint achieved.
- Dictionary holds at least one hundred million entities, populated from
  surveyed corpora through the ingest pipeline.
- Grammar registry holds the complete catalog of authoring patterns,
  keywords, clause shapes, kinds, and quality names — every row entered
  by the user (or downstream tooling) directly into the DB through a
  stable row-level surface, never through Rust source bundling.
- Single `.nomx` source format (the prose form and typed-slot form
  merged) is the sole authoring surface.
- `~/.nom/` carries exactly three SQLite files plus the artifact store;
  no legacy schema remains.
- AI authoring loop (intent prose → resolver → dict lookup →
  composition → manifest emission → artifact build) runs end-to-end,
  deterministic and reproducible.

## Why

A language whose identity depends on being "like" some other language is
permanently bound to that other language's self-image. Nom's mission is
to study other languages externally for strengths and weaknesses,
capture the useful patterns in Nom's own vocabulary so the source need
never be consulted again, and present a self-contained compiler whose
data lives in the canonical DB rather than in Rust source bundles.
