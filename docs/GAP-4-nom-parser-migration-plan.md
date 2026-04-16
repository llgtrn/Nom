# GAP-4: nom-parser Deletion Migration Plan

> **Date:** 2026-04-15  
> **Status:** Analysis complete — migration requires full-fidelity AST bridge  
> **Blocking:** The bridge from `nom-concept::PipelineOutput` → `nom_ast::SourceFile` only preserves declaration names with empty statement bodies. Full migration requires preserving statement bodies (flow chains, need statements, etc.) for downstream consumers (nom-llvm, nom-verifier, nom-planner).

---

## 1. Current State

### Two Different Syntaxes, Two Different Parsers

| Aspect | `nom-parser` (legacy) | `nom-concept` S1-S6 (current) |
|--------|----------------------|-------------------------------|
| **Input format** | Flow-style `.nom` files: `system auth\nneed hash::argon2 where security>0.9\nflow request->hash->store->response` | Prose-English `.nomx` files: `the function fetch_url is\n  intended to fetch the body of an https URL.\n  given a url of text, returns text.` |
| **Output** | `nom_ast::SourceFile { declarations: Vec<Declaration> }` with full statement bodies (FlowChain, NeedStmt, etc.) | `PipelineOutput::Nom(NomFile)` or `PipelineOutput::Nomtu(NomtuFile)` with concept/entity/composition ASTs |
| **Grammar** | Keyword-driven: `system`, `flow`, `store`, `need`, `require`, `effects`, `let`, `fn`, `if`, `for`, `while`, `match`, `struct`, `enum`, etc. | English prose: `the`, `is`, `intended to`, `given`, `returns`, `requires`, `benefit`, `hazard`, `uses`, `exposes`, `favor`, `@Kind matching` |
| **Purpose** | Imperative orchestration + general-purpose code inside declaration bodies | Dictionary entity/concept declaration for content-addressed store |

### The Bridge Gap

`crates/nom-cli/src/ast_bridge.rs` (66 lines) maps:
- `PipelineOutput::Nom` → `Declaration { classifier: Classifier::Nom, name: concept.name, statements: vec![] }`
- `NomtuItem::Entity` → `Declaration { classifier: Classifier::from_str(kind), name: ent.word, statements: vec![] }`
- `NomtuItem::Composition` → `Declaration { classifier: Classifier::System, name: comp.word, statements: vec![] }`

**Statements are always empty.** This is sufficient for `nom check`/`nom fmt`/`nom report` (which only need declaration names) but **insufficient for `nom build`/`nom run`/`nom-llvm`** (which need full statement bodies to generate code).

### The nom-llvm Self-Hosting Exception

The self-hosting lexer in `stdlib/self_host/lexer.nom` and related files are written in **flow-style syntax** (the `nom-parser` format), not `.nomx` prose. The `nom-llvm` crate's tests and its `compile_source_to_bc` function use `nom_parser::parse_source` to parse these files. This is the primary blocker for deleting `nom-parser`.

---

## 2. Dependency Map

### Production Dependencies (must migrate)

| Crate | Dependency Type | Call Sites | What It Does |
|-------|----------------|------------|-------------|
| **nom-cli** (`src/main.rs`) | `use nom_parser::parse_source` | 8 call sites (lines 2902, 3038, 3107, 3355, 3491, 3712, 3770, 4094) | `nom build`, `nom check`, `nom fmt`, `nom report`, `nom run`, `nom quality` — all parse flow-style `.nom` files |
| **nom-cli** (`src/fmt.rs`) | `use nom_parser::parse_source` | 1 call site (line 29) | `nom fmt` — canonical format for flow-style syntax |
| **nom-llvm** (`src/lib.rs`, `src/context.rs`) | `nom_parser::parse_source` | 3 call sites (lines 63, 267, 333) | LLVM compilation — parses flow-style source to generate bitcode |
| **nom-corpus** (`Cargo.toml`) | `nom-parser = { path = ... }` | Declared dependency | Declares but usage not yet confirmed in detail |

### Test Dependencies (can isolate or migrate)

| Crate | Test Files | Call Sites |
|-------|-----------|------------|
| **nom-cli** | `self_host_pipeline.rs`, `self_host_smoke.rs`, `self_host_ast.rs`, `self_host_codegen.rs`, `self_host_parser.rs`, `self_host_verifier.rs`, `self_host_planner.rs`, `self_host_parse_smoke.rs`, `parser_subset_probe.rs` | 10 call sites |
| **nom-llvm** | `tuples.rs`, `strings.rs`, `lists.rs`, `enums.rs`, `builtins.rs` | 5 call sites |

### Self-Host Stdlib Files (flow-style syntax, not .nomx)

| File | Content |
|------|---------|
| `stdlib/prelude.nom` | Pre-declared types for self-host |
| `stdlib/self_host/lexer.nom` | Nom-in-Nom lexer implementation (flow-style) |
| `stdlib/self_host/parser.nom` | Nom-in-Nom parser scaffold (flow-style, returns empty SourceFile) |
| `stdlib/self_host/ast.nom` | AST type definitions (flow-style) |
| `stdlib/self_host/codegen.nom` | Codegen scaffold (flow-style) |
| `stdlib/self_host/verifier.nom` | Verifier scaffold (flow-style) |
| `stdlib/self_host/planner.nom` | Planner scaffold (flow-style) |

These 7 files use flow-style syntax (`nom struct TokenStream { ... }`, `fn nom_parse(tokens: TokenStream) -> SourceFile { ... }`). They are **NOT `.nomx` prose files**. They cannot be parsed by `nom-concept`'s S1-S6 pipeline without rewriting them in prose syntax — which would defeat the purpose of self-hosting (the lexer is supposed to be able to parse itself).

---

## 3. The Fundamental Problem

**`nom-parser` and `nom-concept` parse different languages.**

- `nom-parser` parses **flow-style syntax**: imperative declarations with `need`, `require`, `effects`, `flow`, `let`, `fn`, `struct`, `enum`, `if`, `for`, `while`, `match`, etc.
- `nom-concept` S1-S6 parses **prose-English syntax**: `the function X is intended to ... given ... returns ... requires ... benefit ... hazard ...`

You cannot replace `nom_parser::parse_source` with `nom_concept::stages::run_pipeline` for flow-style files. The two parsers accept different inputs and produce different outputs.

**The `ast_bridge.rs` is not a replacement for `nom-parser`.** It bridges the `.nomx` → `SourceFile` gap for validation purposes, but the bridge produces **skeleton Declarations with empty bodies**. Downstream consumers (nom-llvm, nom-verifier) need the full statement bodies.

---

## 4. Migration Strategy

### Phase 1: Isolate nom-parser to Self-Host Tests Only (Week 1-2)

**Goal:** Remove `nom-parser` from all production CLI paths, keeping it only for self-host regression tests.

**Steps:**
1. **`nom-llvm` self-host path:** Add a feature flag `self-host` to `nom-llvm` that conditionally compiles the `nom-parser` dev-dependency tests. Production `nom-llvm::compile_source_to_bc` should use an alternative parse path (either direct token-stream parsing or a dedicated flow-style parser module).

2. **`nom-cli` production commands:** For commands that currently call `parse_source` (`nom build`, `nom check`, `nom fmt`, `nom report`, `nom run`, `nom quality`), determine whether they should:
   - (a) Use the `nom-concept` S1-S6 pipeline (for `.nomx` files), OR
   - (b) Use a dedicated flow-style parser (for `.nom` files), OR
   - (c) Be deprecated entirely if flow-style is no longer the target format

3. **Create a minimal flow-style parser module:** Extract just the flow-style parsing needed by self-host tests into a small, isolated module. This could be a new crate `nom-flow-parser` or a module within `nom-concept`. It should be much smaller than the full 3851-line `nom-parser`.

4. **Move all self-host tests** behind a `#[cfg(feature = "self-host")]` gate so they don't block the main build.

### Phase 2: Bridge Full Fidelity (Week 2-4)

**Goal:** Make `ast_bridge.rs` produce full `SourceFile` with statement bodies from `PipelineOutput`.

**Steps:**
1. Extend `EntityDecl` and `CompositionDecl` in `nom-concept` to capture statement bodies (flow chains, contract clauses, effect clauses, etc.) — they already capture contracts and effects, but not imperative statements.

2. Add imperative statement types to the `.nomx` grammar (if not already present): `let`, `fn`, `if`, `for`, `while`, `match`, `return`, `struct`, `enum`.

3. Extend `ast_bridge.rs` to map captured statement bodies into `nom_ast::Statement` variants.

4. Test that `nom-llvm` can compile from the bridge output.

### Phase 3: Delete nom-parser (Week 4-6)

**Steps:**
1. After Phase 1 + Phase 2 are complete, remove `nom-parser` from workspace `members`.
2. Delete the `nom-parser` crate directory.
3. Remove `nom-parser` from all `Cargo.toml` dependencies.
4. Update `TESTING.md` to remove `nom-parser` references.
5. Update `examples/README.md` to remove `nom-parser` references.
6. Run `cargo test --workspace` to verify.

---

## 5. Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Self-host tests break | Blocks regression testing of Nom-in-Nom pipeline | Gate behind feature flag; migrate to new parser |
| `nom-llvm` production path breaks | Cannot compile flow-style `.nom` files to LLVM | Add dedicated flow-style parser or migrate to `.nomx` |
| `nom build`/`nom check`/`nom fmt` break | Core CLI commands broken | These commands should migrate to `nom-concept` pipeline for `.nomx` files |
| Backward compatibility break | Existing `.nom` flow-style files can't be compiled | Deprecate flow-style with clear migration path to `.nomx` |

### What Breaks If We Delete nom-parser Today

| Crate | Impact |
|-------|--------|
| `nom-cli` | `nom build`, `nom check`, `nom fmt`, `nom report`, `nom run`, `nom quality` — **HIGH** — all call `parse_source` |
| `nom-llvm` | `compile_source_to_bc` + all 5 integration tests — **HIGH** — uses `parse_source` |
| Self-host pipeline | All 10 self-host tests — **MEDIUM** — test-only, but important for regression |

---

## 6. Recommended Immediate Action (Unblocked Today)

**Do NOT delete `nom-parser` yet.** The production CLI paths depend on it for flow-style parsing, and the self-host tests depend on it for regression testing.

**What CAN be done today (already done in this session):**
- ✅ Cleaned up `NomDict` doc-comment breadcrumbs (GAP-1)
- ✅ Removed unused `nom-score` dependency from `nom-planner`
- ✅ Updated `TESTING.md` nom-parser references

**What should happen next:**
1. Decide whether flow-style syntax is still a target format or should be deprecated
2. If deprecated: migrate all flow-style self-host files to `.nomx` prose syntax
3. If kept: extract a minimal flow-style parser module, isolate behind feature flag
4. Then proceed with Phase 1 → Phase 2 → Phase 3

---

## 7. Estimated Effort

| Phase | Effort | Notes |
|-------|--------|-------|
| Phase 1 (isolate) | 1-2 weeks | Feature flags, conditional compilation, test isolation |
| Phase 2 (bridge) | 2-4 weeks | Full-fidelity AST bridge is substantial work |
| Phase 3 (delete) | 1-2 days | Straightforward deletion after Phase 1+2 |
| **Total** | **3-6 weeks** | Depends on Phase 2 scope and whether flow-style is deprecated |

---

## 8. Open Questions

1. **Is flow-style syntax still a target format?** If no, the migration becomes much simpler: rewrite self-host files to `.nomx`, delete `nom-parser`, done.
2. **Does `nom-corpus` actually use `nom-parser`?** It declares the dependency but usage was not confirmed in detail.
3. **Should the `nom run` and `nom run-llvm` commands be preserved?** They parse flow-style files and execute them via `lli` or native binary.
4. **What is the long-term format for `.nom` files?** The state machine report distinguishes `.nom` (Tier-2 concept file) from `.nomx` (authored prose source). Are these converging?
