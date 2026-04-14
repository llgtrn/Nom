# 10 — Bootstrap

The bootstrap protocol governs the transition from the host-language
compiler (Stage-0) to the Nom-authored compiler (Stage-1+). Two tracks
run in parallel until cutover; both must pass before the host compiler
archives.

## Concept

A self-hosting language compiles its own source. Stage-0 is the compiler
written in the host language. Stage-1 is the compiler authored in Nom,
compiled by Stage-0. Stage-2 is the same Nom source compiled by Stage-1.
Stage-3 is the same Nom source compiled by Stage-2.

If `Stage-2 == Stage-3` byte-identical, the language has reached fixpoint
— Stage-2 and beyond can rebuild themselves without ever touching the
host language again. This byte-identical equality is the proof of Nom as
a language. The same discipline applies to every self-hosting compiler
in long-standing language ecosystems.

## Two tracks

### Fixpoint track

1. Stage-0 (host-language compiler) compiles Stage-1 (Nom-authored
   compiler source) → produces Stage-1 binary.
2. Stage-1 binary compiles the same Nom source → produces Stage-2 binary.
3. Stage-2 binary compiles the same Nom source → produces Stage-3 binary.
4. Compare Stage-2 binary to Stage-3 binary byte-by-byte.
5. If identical: fixpoint reached. Record the proof tuple
   `(s1_hash, s2_hash, s3_hash, fixpoint_at_date,
   compiler_manifest_hash)` permanently in the dictionary.
6. If different: a non-determinism leaked into the compiler. Diagnose via
   per-pass hash comparison; fix; repeat from step 1.

### Parity track

1. Test corpus of N representative Nom programs (target N ≥ 100, drawn
   from the corpus ingestion phase).
2. Run every program through both Stage-0 and Stage-2.
3. Measure: IR equivalence (≥99% of programs produce identical IR after
   canonicalisation) AND runtime correctness (100% produce identical
   observable output).
4. Sustain ≥99% / 100% for 4 consecutive weeks.
5. Default flip — the `nom build` CLI now invokes Stage-2 by default;
   Stage-0 invocation requires an explicit flag.
6. Three-month grace period — both compilers ship for every release;
   incidents that recreate parity drops force a rollback.
7. Archive — host-language compiler source moves to `.archive/host-
   compiler/`; future maintenance is Nom-only.

## Per-pass invariants

The fixpoint requires every pass to be deterministic. Suspected leak
sources:

- Hash-map iteration order — replace with sorted iteration in any pass
  whose output is materialized.
- Time stamps in build artifacts — strip from canonicalised output.
- File-system traversal order — sort by canonical path.
- Integer overflow / underflow handling — use checked arithmetic in
  every codegen pass.

Any pass whose Stage-2 vs Stage-3 hash differs is the fix target. The
pass is rebuilt to be deterministic, the test corpus reruns, the fixpoint
attempt repeats.

## Cutover gate

The default flip from Stage-0 to Stage-2 is gated on:

- Fixpoint reached (Stage-2 == Stage-3 byte-identical).
- Parity track sustained for 4 weeks (≥99% IR + 100% runtime).
- All in-flight wedges either landed or queued in the registry as
  blocked-by-bootstrap.
- Mission-checklog updated to reflect Stage-2 as the canonical compiler.

After cutover, the proof tuple records permanently. The host-language
compiler enters the 3-month grace; after grace, archives.

## Why this matters

Nom is a self-defined language. The bootstrap fixpoint is the
operational proof of self-definition. Until fixpoint holds, Nom is a
language defined by its host-language compiler, not by itself. After
fixpoint, the language is self-sustaining and the host language can
disappear without losing the language.
