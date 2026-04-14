# 01 — Mission

## Mission

Nom is a self-defined programming language for AI-assisted application
synthesis. The compiler determines author intent from `.nomx` source and
produces working applications by composing entries from a content-addressed
dictionary. Nom is defined entirely on its own terms — no foreign-language
names appear in its surface, no parallel formats coexist, no wrappers bridge
to legacy code.

## Current state

- Three SQLite tiers at `~/.nom/`: `concepts.sqlite` (DB1) +
  `entities.sqlite` (DB2) + `grammar.sqlite` (registry).
- Content-addressed artifact store at `~/.nom/store/<hash>/body.{bc,avif,...}`.
- Compiler crate workspace: ~30 crates including `nom-grammar`, `nom-dict`,
  `nom-concept`, `nom-parser`, `nom-llvm`, `nom-cli`, `nom-corpus`, `nom-lsp`,
  `nom-intent`, `nom-app`.
- Self-hosting lexer compiles end-to-end through the LLVM backend.
- `Dict { concepts, entities }` struct ships per-tier connections; legacy
  single-file path scheduled for deletion under no-legacy rule.
- Grammar registry seeds five tables (kinds, quality_names, keywords,
  clause_shapes, patterns) via `nom grammar seed`; native pattern catalog
  contains 10 founding rows with the migration of captured insights ongoing.

## Target state

- Compiler is itself authored in Nom; bootstrap fixpoint achieved.
- Dictionary holds at least one hundred million entities, populated from
  surveyed corpora through the ingest pipeline.
- Grammar registry holds the complete native pattern catalog (~100-150 rows)
  with every captured pattern preserved in Nom vocabulary and verified to be
  100% sufficient so the source documents can be deleted.
- Single `.nomx` source format (the v1 prose form and v2 typed-slot form
  merged) is the sole authoring surface.
- `~/.nom/` carries exactly three SQLite files plus the artifact store; no
  legacy schema remains.
- AI authoring loop (intent prose → resolver → dict lookup → composition →
  manifest emission → artifact build) runs end-to-end, deterministic and
  reproducible.

## Why

A language whose identity depends on being "like" some other language is
permanently bound to that other language's self-image. Nom's mission is to
study other languages externally for strengths and weaknesses, capture the
useful patterns in Nom's own vocabulary so the source need never be consulted
again, and present a self-contained compiler.
