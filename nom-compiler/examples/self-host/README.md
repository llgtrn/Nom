# Self-Hosting Parser (.nomx)

These files define the Nom parser written in Nom itself (GAP-9).
They are the first step toward the bootstrap fixpoint (GAP-10).

## Files
- `tokenizer.nomx` — Token stream functions
- `parser.nomx` — Entity declaration parser functions
- `pipeline.nomx` — Module composition + concept declaration

## Status
Stage-1: These .nomx files can be compiled through the S1-S6 pipeline
to produce entity rows in the dictionary. They don't yet produce
executable code — that requires GAP-5b LLVM linking for .nomx sources.

## Bootstrap Path
1. These .nomx files -> S1-S6 -> entity rows (Stage-1) - possible now
2. Entity rows -> LLVM compile -> Stage-2 binary (needs GAP-5b wiring)
3. Stage-2 binary compiles same .nomx -> Stage-3 binary
4. Fixpoint: hash(Stage-2) == hash(Stage-3) = PROOF
