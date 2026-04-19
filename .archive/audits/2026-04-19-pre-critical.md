# Nom Repository — Comprehensive End-to-End Audit Report

**Date:** 2026-04-19  
**Auditor:** Multi-agent audit (8 parallel deep-dive agents + manual verification)  
**HEAD:** `2da074836fd4a2f9b8ae8859d94199ea4dfc29a9`  
**Git status:** 131 unstaged files (mostly `.archive/` deletions + research markdown removals)  
**Scope:** Full audit of `nom-canvas/` (18 crates, 405 .rs files), `nom-compiler/` (29 crates, 138 .rs files), `examples/` (100 .nomx), `docs/` (15 .md), and reference repo inventory.

---

## Executive Summary

| Axis | Claimed | Actual | Verdict |
|------|---------|--------|---------|
| A · nom-compiler | 72% | ~55% | **Overstated**. Rust LLVM backend is real, but Nom-in-Nom self-host is aspirational. Type solver is stub. |
| B · Nom language | 95% | ~40% | **Massively overstated**. `.nomx` examples are prose containers, not executable code. |
| C · nom-canvas ↔ compiler | 95% | ~45% | **Overstated**. Dispatch/registry is real but all media backends are stubs. Renderer is library-real, integration-stub. |
| D · Overall platform | 100% | ~65% | **Overstated**. LSP is framing-only. Golden paths are 0% end-to-end. Foreign brands in public API. |

**Most critical finding:** The repository contains a genuinely impressive volume of well-structured Rust code (~10,500 tests, 0 clippy warnings, clean architecture), but the **claims of functional completeness in task.md and ROADMAP_TO_100.md are systematically inflated**. The codebase is a **sophisticated scaffold** — real data structures, real dispatch logic, real LLVM backend — but **virtually no end-to-end execution path works**. The renderer initializes wgpu then sits idle. The LSP parses Content-Length headers then returns hardcoded nulls. The compose backends write Y4M/WAV stubs but never call FFmpeg. The `.nomx` language tokenizes beautifully but has no evacluator.

---

## 1. A-Axis Audit: nom-compiler → 100%

### 1.1 Self-Hosting (A1, A7, B10) — ASPIRATIONAL

| Component | Claim | Reality |
|-----------|-------|---------|
| `lexer.nom` | "Frozen baseline" | **Real algorithm, uncompilable**. Uses tuple returns, string slicing, generic lists — features not in compiler. |
| `parser.nom` | "Parser-in-Nom" | **STUB**. Returns empty `SourceFile`. |
| `codegen.nom` | "CodeGen-in-Nom" | **STUB**. Returns empty `GeneratedSource`. |
| `bootstrap.rs` | "Fixpoint proof" | **Real hash-comparison logic**, but hashes come from Rust-built binaries, not Nom-built ones. |
| `check_fixpoint()` | "s2 == s3 byte-identical" | Compares SHA-256 strings. Real code. But **cannot be triggered** because Nom-built stages don't exist. |

**Verdict:** The bootstrap infrastructure is real, but the fixpoint proof is **theoretically possible, practically impossible** today. The Nom compiler cannot compile its own parser or codegen. The 72% claim for A-axis is plausible only if you count the Rust LLVM pipeline; if you count Nom-in-Nom self-hosting, it is closer to **30%**.

### 1.2 LLVM Pipeline (A11) — MIXED

| Component | Verdict |
|-----------|---------|
| `nom-llvm` crate (inkwell) | **REAL**. Produces verified `.bc` bitcode. Full expression + statement compilation. |
| `llvm_emit.rs` (IR text) | **REAL**. Emits valid LLVM IR text (`%0 = add i32 %a, %b`). |
| `native.rs` | **STUB**. Emits `0xC3` (x86 RET) per function. |
| `bitcode.rs` | **STUB**. One byte per function = name length. |
| `type_infer.rs` | **PARTIAL**. Real `TypeEnv`, but `solve()` is `self.env.clone()` — no unification. |
| `stages.rs` (S1–S6) | **REAL**. Full pipeline with 60+ tests. |

### 1.3 Open Gaps (A-axis)

- [ ] Parser.nom compiles via rust-nomc
- [ ] Resolver.nom compiles
- [ ] Type checker.nom compiles
- [ ] Codegen.nom compiles
- [ ] s2 == s3 byte-identical attested
- [ ] IR → LLVM bitcode for all S1–S6 stages
- [ ] Native codegen beyond `0xC3`
- [ ] Type inference unification algorithm

---

## 2. B-Axis Audit: Nom Language → 100%

### 2.1 The `.nomx` Language — NOT A REAL PROGRAMMING LANGUAGE

**Brutal finding:** The 100 `.nomx` examples in `examples/` look like executable code but are **prose containers**.

```rust
// stages.rs — parse_define_that()
let body = rest
    .iter()
    .map(|t| format!("{t:?}"))
    .collect::<Vec<_>>()
    .join(" ");
```

The body of `define add_numbers that a + b` becomes the **string** `"Word(\"a\") Add Word(\"b\")"`. There is:
- **No AST** for `.nomx` expressions (the `nom-ast` crate only handles `.nom` syntax)
- **No evaluator** or interpreter
- **No LLVM lowering** for `define...that`
- **No type system** for `.nomx`
- **No function call resolution**
- **No parameter lists**, no return types

The `+` operator is never parsed as a binary operation. `fibonacci_loop a b n` is not a function call with arguments — it is an opaque token sequence.

### 2.2 The `.nom` Language — REAL BUT SEPARATE

The `.nom` language (used in `nom-compiler/examples/` and `stdlib/`) **is** a real language with:
- Classifiers (`system`, `flow`, `store`, `graph`, `agent`, etc.)
- Statements (`let`, `if`, `for`, `while`, `match`, `return`, `fn`, `struct`, `enum`)
- Expressions (binary ops, calls, closures, arrays, tuples)
- A real parser (`stages.rs` S1–S6)
- A real LLVM backend (`nom-llvm` via inkwell)

**But `.nom` and `.nomx` are completely separate.** `.nomx` is not a successor to `.nom`; it is a parallel, less-implemented surface.

### 2.3 Syntax Claims

| Claim | Verdict |
|-------|---------|
| "`define X that Y` replaces `fn X -> Y`" | **FALSE**. `migrate_typed_to_natural()` does string replacement, not semantic migration. |
| "English-only vocabulary" | **TRUE at lexer layer**. No Vietnamese tokens. |
| "Zero null by grammar" | **UNVERIFIABLE**. No formal grammar exists for `.nomx`. |
| "100 `.nomx` golden examples" | **TRUE** (100 files exist). But they are not executable. |
| "Round-trip byte-identity" | **FALSE**. No round-trip pipeline exists. |

### 2.4 Open Gaps (B-axis)

- [ ] `.nomx` parser produces executable AST (not token-debug strings)
- [ ] `.nomx` evaluator / interpreter
- [ ] `.nomx` → LLVM lowering
- [ ] Formal grammar for `.nomx` (BNF/EBNF)
- [ ] `define...that` integrated into S1–S6 pipeline
- [ ] Round-trip byte-identity on corpus
- [ ] 100 paradigm families (claimed 71, likely fewer real)

---

## 3. C-Axis Audit: nom-canvas ↔ Compiler Integration → 100%

### 3.1 Renderer — LIBRARY-REAL, INTEGRATION-STUB

| Component | Claim | Reality |
|-----------|-------|---------|
| wgpu init chain | ✅ FIXED | **REAL**. `Instance::new`, `create_surface`, `request_adapter`, `request_device`, `configure` all present. |
| `end_frame_render()` | ✅ FIXED | **REAL** function exists with `CommandEncoder`, `begin_render_pass`, `draw`, `submit`, `present`. |
| Actual pixels on screen | — | **NO**. The function is **called zero times** in any executable path. Event loop leaves wgpu idle. |
| Scene graph → GPU | — | **DISCONNECTED**. `Renderer::draw()` only increments counters. |
| Frosted glass | — | **STUB**. State machine exists, no shaders, no framebuffers. |
| WebGPU (WASM) | — | **STUB**. Explicitly labeled stub. |

**Verdict:** The wgpu quad pipeline will compile and run if manually invoked, but **no code in the repository invokes it**. The window opens, initializes wgpu, then renders nothing.

### 3.2 Compose Backends — MOSTLY STUB

| Backend | Claim | Reality |
|---------|-------|---------|
| Video (MP4/WebM) | Remotion pattern | **Y4M real, MP4/WebM stub**. No FFmpeg integration. |
| Audio (FLAC/Ogg) | rodio pattern | **WAV real, FLAC/Ogg stub**. No rodio playback. |
| Image | 200+ model registry | **STUB**. Raw bytes into store, no encoding. |
| Document | typst pattern | **STUB**. |
| Data extract | XY-Cut++ | **STUB**. |
| Web app | ToolJet + Dify | **STUB**. |
| Native screen | LLVM codegen | **STUB**. |
| Mobile screen | iOS/Android | **STUB**. |

The `dispatch.rs` registry and `HybridResolver` 3-tier fallback are **real and tested**. But every concrete backend implementation is either a minimal format writer (Y4M, WAV) or a text placeholder.

### 3.3 LSP — FRAMING REAL, METHODS STUB

| Layer | Verdict |
|-------|---------|
| Content-Length framing | **REAL** (`lsp_loop.rs`, `lsp_async.rs`) |
| JSON-RPC parsing | **Real string regex, no serde** |
| Method dispatch table | **Real table, all handlers return hardcoded fakes** |
| `textDocument/hover` | **STUB** → `"**Nom kind**"` |
| `textDocument/completion` | **STUB** → `[]` or grammar cache keywords |
| `textDocument/definition` | **STUB** → `null` |
| `workspace/rename` | **STUB** → in-memory Vec, no source editing |
| Actual stdio loop running | **NO**. No subprocess spawning, no running server. |

### 3.4 DB-Driven Architecture — INFRA REAL, UI UNPROVEN

| Component | Verdict |
|-----------|---------|
| `nomdict.db` exists | **YES** (~68 KB) |
| `SqliteDictReader` | **REAL** (feature-gated) |
| `DictWriter` | **REAL** (insert/promote) |
| `SharedState` | **REAL** (LRU cache, reader pool) |
| `can_wire()` grammar validation | **REAL** |
| NodePalette live DB query | **ARCHITECTURALLY REAL, OPERATIONALLY UNPROVEN**. Method exists, but all call sites use `StubDictReader`. |
| LibraryPanel live DB query | Same as above. |

### 3.5 Open Gaps (C-axis)

- [ ] Renderer actually presents frames in event loop
- [ ] GPU scene → FFmpeg real encode
- [ ] Real rodio/symphonia playback
- [ ] Image PNG/JPEG/WebP encoding
- [ ] LSP handlers return real analysis (not hardcoded stubs)
- [ ] LSP stdio loop spawned and running
- [ ] End-to-end test: DB → palette renders real nodes
- [ ] wasm-bindgen build config
- [ ] WebGPU renderer variant

---

## 4. D-Axis Audit: Overall Platform → 100%

### 4.1 Reference Repo Parity — PARTIAL

| Repo | Pattern | In Code? | Real or Stub? |
|------|---------|----------|---------------|
| Zed | GPUI scene/shell | Yes | Renderer library-real, integration-stub |
| AFFiNE | 73 tokens, frosted glass | Yes | Tokens real, frosted glass stub |
| rowboat | Chat sidebar, deep-think | Yes | UI models real, no LLM integration |
| ComfyUI | 4-tier cache, Kahn, cancel | Yes | Cache real, cancel real, Kahn real |
| GitNexus | Confidence edges, NomtuRef | Yes | Real |
| dify | TypedNode, NodeEvent | Yes | Real types |
| n8n | AST sandbox, sanitizers | Partial | Sandbox exists, not wired to execution |
| LlamaIndex | RRF, cosine, BFS | Yes | Real algorithms |
| Haystack | Pipeline composition | Partial | `ComponentPipeline` stubbed |
| ToolJet | 55-widget registry | Partial | 51 kinds seeded, not all wired |
| yara-x | Sealed trait linter | Yes | `NamingLinter` real |
| typst | comemo memoization | Yes | `nom-memoize` real |
| WrenAI | MDL semantic layer | Partial | `semantic.rs` exists, stubbed |
| 9router | 3-tier fallback | Yes | `ProviderRouter` real |
| Refly | SkillRouter | Yes | Real |
| Remotion | GPU→frame→FFmpeg | No | Y4M only, no FFmpeg |
| Open-Higgsfield | 200+ model dispatch | No | Stub only |
| ArcReel | 5-phase orchestration | Partial | `StoryboardPhase` exists, stubbed |
| waoowaoo | 4-phase cinematography | No | Not in code |
| opendataloader | XY-Cut++ | No | Stub only |
| excalidraw | Hit-test, selection | Yes | Real |

### 4.2 Foreign Identities in Public API — VIOLATION

**ROADMAP D6 claims:** `[x] Zero foreign identities in public API`

**Finding:** FALSE. Multiple foreign brands exist as public module names, type names, and exports:

| Foreign Brand | Public API Impact |
|---------------|-------------------|
| `affine` | `affine_tokens` module, `AffineToken`, `AffineTokenSet`, `--affine-` CSS prefix |
| `n8n` | `n8n_workflow` module, `WorkflowNode`, `WorkflowGraph` |
| `sherlock` | `sherlock` + `sherlock_native` modules, `SherlockAdapter`, `SherlockResult`, `SherlockSite`, `SherlockStatus` |
| `haystack` | `haystack_pipeline` module, `HaystackComponent`, `ComponentPipeline` |
| `ffmpeg` | `FfmpegConfig`, `FfmpegEncoder` |
| `tooljet` | `CorpusSource::ToolJet` |
| `openai` / `anthropic` | Credential store strings (public API surface) |

**The `NamingLinter` (`nom-lint/src/rules/naming.rs`) only bans 4 prefixes:** `affine`, `figma`, `notion`, `linear`. It does **not** ban `n8n`, `sherlock`, `haystack`, `ffmpeg`, `tooljet`, `openai`, `anthropic`, `claude`, `zed`, `remotion`, `comfy`, `dify`, `higgsfield`, `bolt`, `rowboat`, `llamaindex`, `refly`.

### 4.3 Golden Path Tests — 0% END-TO-END

**Claim:** 35–40 "golden path" end-to-end integration tests.

**Finding:** All 40 tests in `golden_paths.rs` are **in-memory unit tests**:
- Data structure construction (~22)
- Algorithm logic (~10)
- Stubbed pipeline wiring (~6)
- Math transforms (~2)

**None test:** pixel rendering, DB queries, file I/O, process spawning, HTTP requests, or actual compose pipelines.

The only tests approaching end-to-end are 4 API integration tests in `nom-cli/tests/api_integration.rs` that exercise the axum router (HTTP 200/422), but these do not test compose execution.

### 4.4 Open Gaps (D-axis)

- [ ] Remove/rename all foreign-brand public identifiers
- [ ] Expand `BANNED_PREFIXES` to cover all reference repo names
- [ ] Real end-to-end golden path tests (pixel, DB, file, network)
- [ ] `cargo build --workspace --release` on Windows/Linux/macOS
- [ ] CI green on PR
- [ ] Settings panel full-screen overlay
- [ ] Theme toggle `Cmd/Ctrl+K T`
- [ ] API reference (`cargo doc --no-deps`)
- [ ] Video compose demo: prose → 10s MP4
- [ ] Document compose demo: prose → PDF
- [ ] Web compose demo: spec → web app

---

## 5. Test Quality Assessment

### 5.1 Quantity vs Quality

| Metric | Claimed | Actual |
|--------|---------|--------|
| Total tests | 10,436 | **~10,466 `#[test]` attributes found** |
| Inline test modules | — | **384+ files** |
| Test inflation | — | **~6.8× estimated** (many simple constructors/assert_eq) |

### 5.2 Test Distribution

| Crate | Claimed | Honest Assessment |
|-------|---------|-------------------|
| nom-compose | 685+ | Many stub-validation tests (assert backend returns placeholder) |
| nom-concept | ~178 | Mostly stage pipeline tests (real) + translation tests |
| nom-panels | 600+ | UI model construction tests |
| nom-canvas-tests | 40 | 0% end-to-end (see §4.3) |

### 5.3 What's Missing

- **Visual regression tests:** No screenshot comparisons
- **GPU render tests:** No framebuffer reads
- **DB integration tests:** Only in-memory SQLite in tests
- **Media encoding tests:** No actual encoded bytes verified against fixtures
- **Network tests:** Only router deserialization tests
- **LSP integration tests:** No running server tests

---

## 6. Reference Repo Inventory

### 6.1 Existing in `C:\Users\trngh\Documents\APP\Accelworld\upstreams\`

Confirmed present: `zed-main`, `AFFiNE-canary`, `rowboat-main`, `candle`, `qdrant`, `wasmtime`, `polars`, `llamaindex`/`llama_index-main`, `haystack`, `yara-x`, `typst-main` (in `services/other5/`), `refly-main`, `remotion-main`, `segment-anything`, `ultralytics`, `unilm`, `screenshot-to-code`, `gpt-engineer`, `donut`, `AnimateDiff`, `stable-video-diffusion`, `bolt.new-main`, `deer-flow-main`, `agentscope-main`, `wrenai`, `wgpu`, `cosmic-text` (implied by deps), `taffy` (implied by deps).

### 6.2 Existing in `C:\Users\trngh\Documents\APP\Accelworld\services\`

Confirmed present: `ComfyUI-master` (in `other2/`), `dify-main` (in `other4/`), `n8n-master` (in `other2/`), `ArcReel-main` (in `other4/`), `waoowaoo` (in `media/`), `Open-Higgsfield-AI-main` (in `media/`), `ToolJet-develop` (in root of upstreams), `9router-master` (in `other4/`).

### 6.3 Read End-to-End Before Writing?

**Cannot verify.** The codebase has detailed comments citing specific files from reference repos (e.g., `Pattern: LlamaIndex output_parser.py`), suggesting source reading occurred. However, many adopted patterns are surface-level (type names, module names) rather than deep semantic adoption.

---

## 7. Code That Should Be Native Nom Language

### 7.1 Currently Rust-Only (Should Eventually Be Nom)

Per the self-hosting roadmap, these should eventually be rewritten in `.nom`:

| Component | Current Language | Target Language |
|-----------|-----------------|-----------------|
| Lexer | Rust (`nom-concept/src/stages.rs` S1) | `.nom` (`stdlib/self_host/lexer.nom`) |
| Parser | Rust (`stages.rs` S2–S6) | `.nom` (`stdlib/self_host/parser.nom`) |
| Type checker | Rust (`type_infer.rs`) | `.nom` |
| LLVM codegen | Rust (`nom-llvm/`) | `.nom` → LLVM IR |
| Native codegen | Rust (`native.rs`) | `.nom` |

### 7.2 Currently Nom-But-Not-Executable

The `.nomx` examples should be made executable by:
1. Building a real parser for `define...that` that produces an AST (not token-debug strings)
2. Adding an evaluator or lowering to the existing `.nom` AST
3. Integrating `.nomx` into the S1–S6 pipeline (currently `define` at top level is a "kindless block" error)

### 7.3 Hybrid Composer Glue

Per `2026-04-18-hybrid-compose-design.md`, AI-generated glue should be `.nomx`, not Rust or JS. Currently:
- `AiGlueOrchestrator` generates `.nomx` strings (real)
- But `execute_blueprint()` does not execute them (stub)
- The sandbox is labeled as JS AST execution, but no JS interpreter exists

**Recommendation:** The sandbox should execute `.nomx` via the Nom evaluator, not JS. Remove JS AST references.

---

## 8. Recommendations

### Immediate (Critical Path)

1. **Fix the renderer event loop.** Call `end_frame_render()` from `RedrawRequested`. This is the single biggest blocker for all C-axis and D-axis demos.
2. **Integrate scene graph → GPU.** `Renderer::draw()` must upload quads to wgpu buffers and submit them.
3. **Make `.nomx` executable.** Either extend the `.nom` parser to handle `define...that`, or build a small evaluator for `.nomx` that desugars to `.nom` AST.
4. **Remove/rename foreign brands.** `affine_tokens` → `design_tokens`, `n8n_workflow` → `workflow_runner`, `sherlock` → `osint_adapter`, etc.
5. **Expand `BANNED_PREFIXES`** to all foreign brands and add a CI check.

### Short-Term (Next 10 Waves)

6. **Real FFmpeg integration.** `FfmpegEncoder` should spawn `ffmpeg` and write real MP4/WebM.
7. **Real LSP method handlers.** Replace hardcoded stubs with actual source analysis.
8. **Real end-to-end golden path tests.** At minimum: open file → tokenize → highlight → compile → artifact.
9. **Nom-in-Nom parser.** Complete `parser.nom` with a real recursive descent parser using only compilable Nom features.
10. **Fix `solve()` in `type_infer.rs`.** Implement unification.

### Medium-Term (To 100%)

11. **Self-hosting fixpoint proof.** Get `parser.nom` and `codegen.nom` to compile via rust-nomc, then bootstrap.
12. **Real media backends.** Image encoding, audio playback, document generation.
13. **Wasm target.** `wasm-bindgen` + WebGPU variant.
14. **Full settings panel.** Theme toggle, keybindings, extensions.
15. **Polars integration.** Replace row-returning `data_query` with lazy frames.

---

## Appendix: Honest Progress Table

| # | Deliverable | Claimed | Actual | Gap |
|---|-------------|---------|--------|-----|
| 1 | 10,436 tests | ✅ | ✅ ~10,466 | 0% |
| 2 | 0 clippy warnings | ✅ | ✅ | 0% |
| 3 | wgpu init | ✅ | ✅ Real | 0% |
| 4 | wgpu render pass | ✅ | ✅ Real function | 0% (but uncalled) |
| 5 | Actual pixels | ✅ | ❌ No | 100% |
| 6 | BackendKind deleted | ✅ | ✅ | 0% |
| 7 | GrammarKind.status | ✅ | ✅ | 0% |
| 8 | 100 .nomx examples | ✅ | ✅ | 0% |
| 9 | Executable .nomx | ❌ | ❌ No | 100% |
| 10 | Nom-in-Nom parser | ❌ | ❌ Stub | 100% |
| 11 | Nom-in-Nom codegen | ❌ | ❌ Stub | 100% |
| 12 | Bootstrap fixpoint | ❌ | ❌ Unreachable | 100% |
| 13 | Real video encode | ❌ | ❌ Y4M only | ~80% |
| 14 | Real audio playback | ❌ | ❌ WAV only | ~80% |
| 15 | LSP real methods | ❌ | ❌ Stubs | 100% |
| 16 | Golden paths end-to-end | ❌ | ❌ 0% | 100% |
| 17 | Zero foreign brands | ✅ | ❌ 7+ brands | ~60% |
| 18 | Wasm target | ❌ | ❌ Stub | 100% |
| 19 | Settings panel | ❌ | ❌ Missing | 100% |
| 20 | Theme toggle | ❌ | ❌ Missing | 100% |

---

*Report compiled from 8 parallel agent audits + manual verification. All claims cross-referenced against source code, not tracker files alone.*
