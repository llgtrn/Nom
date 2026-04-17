# 19 — Deferred design decisions (2026-04-14 resolution)

**Date:** 2026-04-14
**Purpose:** Close the two remaining "design deferred" rows in [doc 16](16-nomx-syntax-gap-backlog.md) by writing down the chosen direction. These aren't grammar wedges (no new tokens) and aren't code wedges (no new compiler logic); they're architectural calls that unblock future work when it lands.

> **Status 2026-04-14:** Both decisions below are resolutions, not implementations. Doc 16 rows #4 and #15 flip from 🧠 design-Q-open to ✅ design-Q-resolved. The actual code + grammar work these decisions imply stays queued (new wedges where noted).

---

## D1 — Path / file subkinds vs generic `@Data`

**Resolves:** doc 16 row #4.

### The question

Translations of I/O-heavy code (`render_template`, `Cipher_RC4_set_key`, `build-dex.sh`) repeatedly referenced filesystem paths, text-mode file handles, byte blobs, stdin/stdout streams, and process arguments. The current typed-slot kind set only has `@Data`, which is too coarse — every one of those concepts would land under the same kind and become ambiguous at the resolver layer.

### The decision

**Keep `@Data` as a single kind. Do NOT split into subkinds at the kind-set level.** Specificity comes from **feature-stack words** (see `MEMORY.md` §"Word naming — feature-stack convention"), not from a wider kind taxonomy.

Concretely:

- `the @Data matching "filesystem path"` resolves against `words_v2` rows whose word is in the `path_*` feature family (`path_file`, `path_dir`, `path_archive`, …).
- `the @Data matching "UTF-8 text file handle"` resolves against the `file_text_*` family.
- `the @Data matching "raw byte buffer"` resolves against `buffer_bytes_*`.
- `the @Data matching "process argv"` resolves against `argv_*` / `args_*`.

The kind noun stays one word (`data`) so the closed `KINDS` set (doc 08 §8.1) remains seven items.

### Why

1. **Kind additions are versioned.** Adding `@Path` + `@File` + `@Bytes` + `@Stream` to KINDS bumps the language's minor version and forces every tool that reads the AST (nom-lsp, nom-ext, nom-extract, editors, linters) to learn the new variants. Feature-stack refinement is data-only — no tool churn.

2. **The resolver already embeds matching strings.** Phase 9's per-kind embedding index (doc 08 §5.3) lets `@Data matching "filesystem path"` pick the right family deterministically from the corpus — the `matching` clause IS the subkind marker, without polluting the grammar.

3. **Doc 08 intentionally picked 7 kinds, not 70.** The closed kind set is a structural invariant; splitting it weakens the core language to solve a vocabulary problem.

### Consequence for translations

Doc 14's translations that referenced paths/files/streams keep `@Data` in their v2 forms; the `matching` phrase tightens to the specific subkind via feature-stack names. This is an authoring-guide tightening, not a grammar change — log as **doc 17 §I14** on a later cycle.

### Relation to other work

- W7 placeholder rows (doc 15 §2) — placeholders for `@Data` subkinds land as real `words_v2` rows with synthetic `path:` / `bytes:` feature prefixes, then get upgraded when a real corpus entry arrives.
- W9 `@Union` kind (doc 16 row #5) — unchanged by this decision. Sum-return types are orthogonal to data-shape specificity.

---

## D2 — Callback closures

**Resolves:** doc 16 row #15.

### The question

TypeScript's `changeBySelectedLine(state, (from, to, changes) => { ... })` and Rust's `L.iter().find(|x| ...)` pass anonymous inline callables to a higher-order function. Nom's `.nomx` has no closure syntax today; the best translation (#6 `indentMore`) punts the callback into prose. What's the canonical Nom form?

### The decision

**Nom does NOT have inline closure syntax. Higher-order callbacks get lifted into named `the function X is …` entities, then referenced by name in the caller's `uses` clause.** This is the authoring rule; the parser doesn't need a new grammar form.

Concretely:

- `indentMore` gets rewritten as:
  ```nomx
  the function push_indent_change is
    intended to append an insert-indent change for a selected line's range.

    uses the @Function matching "push editor change" with at-least 0.85 confidence.

  the function indent_more is
    intended to insert one indent unit at every selected line
    unless the editor is read-only.

    uses the @Function matching "change by selected line" with at-least 0.85 confidence.
    uses the function push_indent_change.
  ```

- The resolver treats `uses the function X` as an ordinary v1 entity ref (kind=function, word=X). The `push_indent_change` helper lands as a peer entity in the same `.nomtu` file.

### Why

1. **Every callable gets a name, a contract, and an origin.** Inline closures are anonymous; they can't carry `requires` / `ensures`, can't be found by `nom find`, can't be benchmarked per doc 04 §5.16. Lifting them to named entities preserves all those properties.

2. **Feature-stack naming fills the "but they'd need distinct names" worry.** A callback for `find_first_rust_file` and a callback for `find_first_python_file` end up as `filter_extension_rs` / `filter_extension_py` — two DB2 rows, one feature stack, no namespace pollution.

3. **Removes a large grammar surface.** Closure syntax would require lexer changes (bracketing for closure bodies), parser changes (nested block detection), type inference (captured variables), and a whole new AST node shape. None of it is needed.

4. **Matches doc 17 §I8** (pipelines → named intermediate values). Anonymous callbacks are the same anti-pattern as nested function calls; the resolution is the same (name the value).

### The edge case: trivial `|x| x.foo` accessors

For truly trivial field-access callbacks — `L.iter().find(|x| x.is_valid)` — translating forces an odd-looking named helper `is_valid_predicate`. This is acceptable authoring cost; the alternative is opening the grammar for closures. If empirical pain surfaces in 50+ translations, revisit — until then, the cost is small and the simplicity gain is large.

### Relation to other work

- **Doc 17 §I8** already pins the "name every intermediate" rule for pipelines. Callbacks are an instance of that rule; no new idiom number needed.
- **Doc 16 row #11** (lifetime annotations) stays deferred — that's the borrow-model work; closures don't need to lift it.

---

## Closure status (doc 16 rollup)

After this doc:

- Row #4  — ✅ D1 above (stay with `@Data`; feature-stack subkinds)
- Row #15 — ✅ D2 above (lift callbacks to named entities)

**0 design-deferred rows remain** in doc 16. Open rows are:

- 12 W-wedges queued (W5 / W6 / W9 / W10 / W11-W18 grammar additions)
- 2 smoke tests (minor coverage)
- 1 blocked (row #11 lifetime annotations, blocked on borrow-model)

The gap backlog is now all actionable work — no open design questions remain.
