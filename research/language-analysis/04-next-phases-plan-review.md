# Adversarial review of 04-next-phases-plan.md

**Reviewer:** Claude (opus-4-6-1m), with critic subagent dispatch and 6 empirical probe iterations.
**Date:** 2026-04-12.
**Plan reviewed:** [04-next-phases-plan.md](./04-next-phases-plan.md), 2351 lines.
**Method:** one adversarial pass (subagent), then five iterations of empirical verification against the shipped nom-compiler workspace and the plan text itself.

This document is a snapshot of the review — the plan has since been patched for Risk #2 and Risk #1 prerequisites (see commit following this review).

## Verdict

Directionally right, systemically overambitious as one phase, and underspecified on the load-bearing claims. The DIDS framing (Phase 4) is intellectually coherent and is real code on `main`. The critical path (5.0 → 6 → 7 → 10) is plausible in ~60 weeks. Phase 5's §5.11–§5.19 sub-scope is collectively ~10× the core and stapled into one phase. Ship as REVISE: carve the sub-scope into named phases with independent go/no-go gates, pin the load-bearing invariants, re-estimate horizons.

## Scoreboard

| Risk | Initial severity | Post-evidence verdict | Fix size |
|------|------------------|----------------------|----------|
| #1 LLVM fixpoint unachievable | CRITICAL | **Refined**: LLVM 18 pinned via `inkwell` ✅. Rust toolchain pinned via `rust-toolchain.toml = "1.94.1"` ✅ (landed 2026-04-12). `-g` (debug info) determinism still untested; `SOURCE_DATE_EPOCH` + `llvm.ident` stripping + PDB/COFF timestamp-zero still to be wired in the build driver. | Remaining: debug-info determinism probe |
| #2 Canonicalizer evolution breaks hash-as-syntax-token | CRITICAL | **Internal plan/code contradiction identified and resolved** in this commit: canonical.rs comment and §5.10.1 now align on version-scoped pins + mandatory `SupersededBy` sweep. | Plan edit only |
| #3 License propagation unhandled at mass-corpus scale | CRITICAL | **Fully descoped by user 2026-04-12.** License column removed from `dictionary/seed.sql`, `SecurityConfig.allowed_licenses` field removed, resolver examples updated. License tracking is not part of the language. | — (done) |
| #4 §5 is six subsystems, not one phase | MAJOR | **Confirmed**: six new workspace crates (`nom-ux`, `nom-media`, `nom-corpus`, `nom-bench`, `nom-app`, `nom-flow`) from zero, ~12–18 kLOC scaffolding buried inside "§5.0 10 weeks." | +~3 weeks hidden scaffolding |
| #5 No diagnostics/DWARF/panic-unwind story | MAJOR | **Refined**: `nom-diagnostics` crate exists (269 LOC) ✅. DWARF emission absent ❌. Unwinding absent (panic is abort-only via `nom_panic` stub) ❌. | ~1 week DIBuilder + 1 decision on unwinding |

## Evidence summary per risk

### Risk #1 — LLVM fixpoint

- [nom-llvm/Cargo.toml:11](../../nom-compiler/crates/nom-llvm/Cargo.toml#L11) pins `inkwell = { version = "0.5", features = ["llvm18-0"] }` — LLVM major-version is already pinned.
- 2026-04-12: `rust-toolchain.toml` with `channel = "1.94.1"` landed and verified — `cargo --version` / `rustc --version` both resolve to 1.94.1 and `cargo check --workspace` passes under the pinned toolchain. Risk #1b is now closed on the Rust-toolchain side; remaining Risk #1 prerequisites (build-driver-level debug-info determinism flags) are separate.
- 2026-04-13: second verification (after Risk #1 loop prompt re-fired) — `rustup show` inside nom-compiler/ reports `active because: overridden by rust-toolchain.toml`, active toolchain `1.94.1-x86_64-pc-windows-msvc`. `rustc --version` = `1.94.1 (e408947bf 2026-03-25)`. Pin is stable on the box; earlier failure was transient. First determinism probe (`compile_nom_to_bc_is_deterministic`) landed in nom-corpus tests — compiles `fn id(x:i64) -> i64 { x }` twice and asserts byte-equality; Windows-ignored due to pre-existing LLVM DLL load issue, runs on Linux CI.
- Two back-to-back builds of [examples/run_lexer.nom](../../nom-compiler/examples/run_lexer.nom) produce byte-identical IR today (md5 `bf5efa8dbe6cc3982d3fd9f0e65cada7`, 335 lines). This is necessary but not sufficient for `s2==s3`: different compiler binaries producing the same output is the real test.
- `-g` (debug info) determinism is untested. Debug info is where most non-determinism hides (paths, timestamps, compilation-unit ordering).
- Plan text at §10.3.1 line 2052 now carries an explicit prerequisites block: LLVM pinned, Rust toolchain pinned, `SOURCE_DATE_EPOCH` set, `llvm.ident` stripped, PDB/COFF timestamps zeroed. These are ABI-level facts, not aspirations.

### Risk #2 — canonicalizer evolution (RESOLVED)

- [canonical.rs:29-31](../../nom-compiler/crates/nom-types/src/canonical.rs#L29-L31) originally said *"Adding new variants appends — never reorder, never recycle"*.
- §5.10.1 originally said *"sugar gets desugared, operator associativity gets fixed"* — which is NOT append-only at the tag-stream level.
- These two statements contradicted each other.
- Resolution chosen 2026-04-12 (option b): append-only **within a canonicalizer version**; cross-version changes rehash via mandatory `SupersededBy` sweep. Source hash pins are optionally version-prefixed (`#<canon_v>:<hash>@<word>`); unprefixed pins resolve via `SupersededBy` to the head; prefixed pins freeze.
- Code comment and plan §5.10.1 both updated in this commit.
- Proof-of-bootstrap tuple (§10.3.1) should now record canonicalizer version in use (future edit — not in this commit).

### Risk #3 — license propagation (DESCOPED + REMOVED)

- 2026-04-12: license tracking removed from the language entirely per user direction ("no need to care about license, delete entire column have license, and bypass all kind").
- `dictionary/seed.sql`: `license` column dropped from the legacy `nomtu` schema — column list in INSERTs and corresponding value removed from all 70 rows.
- [nom-security/src/lib.rs](../../nom-compiler/crates/nom-security/src/lib.rs): `SecurityConfig.allowed_licenses` field + default removed; module docstring line *"License compatibility and supply-chain provenance"* shortened to *"Supply-chain provenance."*
- [nom-resolver/src/lib.rs](../../nom-compiler/crates/nom-resolver/src/lib.rs): `license=MIT` examples in docstring + 2 tests replaced with `source=stdlib` (parser grammar was generic; no behavioral change).
- Output license of translated `body_nom` remains unstated — intentionally, since license tracking is not part of the language.

### Risk #4 — §5 scope

- Workspace at [Cargo.toml:1-24](../../nom-compiler/Cargo.toml#L1-L24) has 20 crates. Six promised-for-§5 crates (`nom-ux`, `nom-media`, `nom-corpus`, `nom-bench`, `nom-app`, `nom-flow`) do not exist.
- At the workspace's current per-crate average LOC, adding six seed crates is ~12–18 kLOC of pure Rust scaffolding. Phase 4's total commit count was 13 for DIDS (Task A1-A3, B1-B4, C1-C3, D1-D3).
- Plan edit recommendation (not yet applied): split §5.0 into §5.0a (scaffolding, ~2–3 weeks), §5.0b (body-only translator @ 100M, ~10 weeks), §5.0c (integration test, ~1 week). Honest Phase 5.0 horizon: ~13–14 weeks.

### Risk #5 — diagnostics / DWARF / unwinding

- `nom-diagnostics` crate exists at 269 LOC — my initial "no diagnostic rendering system" claim was wrong. Retracted.
- DWARF emission: zero hits for `DIBuilder|debug_info|DW_TAG|DILocation` in nom-compiler's own code. [examples/run_lexer.ll](../../nom-compiler/examples/run_lexer.ll) contains zero `!dbg`/`!DILocation`/`!DISubprogram` directives. Debug info is genuinely absent.
- Panic runtime: [nom-llvm/src/runtime.rs:78-81](../../nom-compiler/crates/nom-llvm/src/runtime.rs#L78-L81) declares `nom_panic(msg: *const i8, msg_len: i64) -> void` as an extern hook. No landingpad, no `eh_personality`, no unwinding — panic-on-crash is abort-only today.
- Recommended Phase 10 prerequisite: wire inkwell's `DIBuilder` (~1 week) and explicitly declare `panic = "abort"` in the compiler's own crate OR ship an unwinding runtime.

## Exit-criteria audit

Nine of sixteen evaluated sub-phases had weak or absent falsifiable done-conditions at the time of review. Strongest: Phase 4 (`test_phase4_closure_demo` green — passes today), Phase 5.0 core (`1M-entry scale benchmark, p50/p95/p99 committed to benches/`), Phase 12 (`≥70% binary size reduction on reference Node-app closure`). Weakest: §5.12 apps, §5.13 bench, §5.14 flow, §5.15 joint optimization, §5.18 aesthetic, §5.19 AI loop — most of these ship as commands rather than as gated criteria.

## Open plan edits not yet applied

1. §5.17 should name the output license of translated `body_nom` explicitly. *Descoped by user.*
2. §5 split into §5a-§5f with per-subsystem horizons. Needs user authoring.
3. ~~Phase 10 prerequisite list should add "DIBuilder wiring" and "panic-unwinding-or-abort decision."~~ **Applied 2026-04-13**: both prerequisites now listed in §10.3.1 with near-term defaults (`-C debuginfo=0` + `panic = "abort"`) and the longer-term DIBuilder + unwinding paths documented.
4. §10.3.2 parity track should be elevated in plan narrative from "housekeeping" to "real proof"; byte-fixpoint framed as aesthetic + pinned-toolchain-dependent.
5. ~~Proof-of-bootstrap tuple (§10.3.1 last paragraph) should include `canonicalizer_version`.~~ **Applied 2026-04-13**: tuple now carries `canonicalizer_version`, `rust_toolchain_channel`, and `llvm_major_version` alongside the original five fields.
6. Dependency-ordering fixes: Phase 9 LSP should precede or overlap §5.17 (since `nom check --audit` depends on it). §5.10 lifecycle should defer to after Phase 7 (canonicalizer is owned by Rust parser until Phase 7 ships Nom parser).

## Strengths (for balance — the plan has these)

- **Content-addressed dict as identity foundation** is a genuinely strong architectural bet. Phase 4 shipped it end-to-end. The hash-as-syntax-token idea is coherent if version-scoping holds.
- **Explicit separation of resolution vs closure walking** (§5.4 vs `nom build`) is well thought through: `SupersededBy` is followed by the former but not the latter. This prevents transitive-rebind bugs.
- **`SupersededBy` as the primary evolution mechanism** (not version numbers) is elegant and matches actual codebase evolution patterns better than semver.
- **Canonicalizer is already shipped, tested, and documented** (`canonical.rs`, 265 LOC, tag-based, span-stripping, declaration-order-preserving). Many compiler projects at Nom's age still debate canonicalization.
- **Partial/Complete lifecycle** (§5.17.4) is a rare piece of ingestion design that acknowledges the reality that 80%+ of translated code will start imperfect — rather than pretending everything will land Complete.
- **Parity track + fixpoint track as parallel proofs** (§10.3) is sophisticated. Many self-hosting projects only define one; Nom's dual-track is strictly stronger.

## Architectural shift: dict is a compiled-artifact cache (2026-04-12)

User direction: "the one in code db is not nom language, translate directly into .bc, AV1 AAC FLAC AVIF. one that have nom language is .nom only."

Captured in the plan as new **§4.4.6 "Body-as-compiled-artifact"** with three new invariants:
- **Inv 15** — Body is bytes (`.bc` or canonical-format bytes), never Nom source.
- **Inv 16** — `.nom` is the only Nom source form; dict never stores Nom AST.
- **Inv 17** — One canonical format per modality: image→AVIF, video→AV1, audio lossy→AAC, audio lossless→FLAC, code→`.bc`. Alternatives are `Specializes` variants.

**Effects on earlier sections:**
- §5.11 and §5.16 bodies are now bytes, not declarative Nom. Their kind/edge/metadata models survive as analysis layers on top of compiled bodies. Callouts at the top of both sections mark this.
- §5.16.11 Tier 2 (pure-Nom codec rewrite) is **removed**. Codec bodies are `.bc` forever.
- §5.2 equivalence gate shifts from "source→Nom→retranslate" to "source→`.bc`→recompile + byte-compare (modulo LLVM pins)" or "decode→re-encode within tolerance" for media.
- §5.10 canonicalizer operates on `.bc` (normalization passes) or canonical-format bytes (re-mux at fixed settings) — not on Nom AST, which doesn't exist in the dict.
- §5.17.4 Partial/Complete semantics tightens: the "immature translator" category disappears because there are no translators, only existing mature compilers.
- §10.3.1 fixpoint proof shape unchanged; inputs shift to compare compiler OUTPUT `.bc` across stages.

**What it simplifies:**
- Risk #2 (canonicalizer evolution) loses most of its bite — canonicalization is now LLVM-opt-pass normalization, a well-understood problem with deterministic tooling.
- Translator-correctness risk evaporates: upstream compilers already produce correct `.bc`.
- Codec roadmap simplifies: no long-tail pure-Nom rewrites.

**What it loses (honestly):**
- Cross-language semantic hash-dedup: Rust `sha256` and C `sha256()` won't hash-collide even though they implement the same algorithm. Equivalence is recorded as `ContractMatches` edges, not hash collisions.
- AI-mediated body inspection: AI reads signatures/contracts/edges/metadata, not bodies. For behavior inspection it must decompile `.bc` or have ingestion-metadata pointers.
- §5.18 "aesthetic as programming": generative-media composition happens in `.nom` code calling into codec `.bc`, not across dict bodies themselves.

**New sensitive point**: hash stability across LLVM upgrades. Mitigation already named — pin LLVM version (same as §10.3.1 prerequisites), ship `nom store recompile` sweep on major LLVM bump.

---

## Deep plan: media & UX compilation pipeline (follow-up, 2026-04-12)

Deep-planned in response to the user question *"from a Nom body in the DB, can the compiler emit AV1 / AAC / FLAC / AVIF for media and compile the UX UI?"* Answer: yes, architecturally supported; now concretely specified in §5.11.6 and §5.16.11-13 of the plan. Summary:

**Media compilation (§5.16.11-13 added):**
- Two-tier codec residency: Tier 1 FFI-binding nomtu wraps mature C/Rust libraries (rav1e / dav1d / fdk-aac / libFLAC / libavif); Tier 2 pure-Nom rewrites arrive in Phase 12+.
- `nom media render <hash> --target <codec> --out <file>` resolves codec+container via a dict query, walks the `Encodes` + `ContainedIn` + `Requires` closure, links against `linker_requires` libs, emits bytes.
- Ten-codec landing roadmap, one PR per codec, each gated by §5.2 round-trip equivalence: PNG → FLAC → JPEG → Opus → AVIF → AV1 → AAC → WebM → MP4 → HEVC (decode only).
- Shared muxer nomtu (`muxer_isobmff`, `muxer_matroska`) compose video+audio tracks.
- Budget raised from ~27 kLOC to ~32 kLOC after factoring in FFI wrappers + muxers + equivalence-gate harness.

**UX compilation (§5.11.6 added):**
- `nom app build <screen_hash> --target web|desktop|mobile` replaces each `ui_runtime_launch` in the closure with its `Specializes`-edge variant (`dioxus_web` / `dioxus_desktop` / `dioxus_mobile`), emits per-platform artifacts (`.wasm` + `index.html`, native `.exe`/`.app`, `.apk`/`.ipa`).
- No per-target source branching; `Specializes` edges encapsulate runtime differences.
- Asset bundling via `MediaUnit` leaves in the closure.
- Cross-target parity measured per commit via §5.13 benchmark runs; parity failures raise `NOM-U03`.

**Relationship to existing §5.16 / §5.11 sections:** the new subsections extend but don't replace. §5.16.1-10 still specifies the kind/edge model; §5.16.11-13 specifies the compile path. §5.11.1-5 still specifies ingestion + extraction; §5.11.6 specifies the build-out path.

**Interaction with open items:**
- LLVM DWARF (Risk #5) is unchanged; codec-compiled binaries need DWARF for debuggability — independent track, not blocked.
- Risk #4 (§5 scope) remains MAJOR; the codec roadmap adds workload, doesn't reduce it.
- Risk #1 (Rust toolchain pin) remains blocked on rustup.
