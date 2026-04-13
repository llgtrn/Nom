# 17 — Nom authoring idioms

**Date:** 2026-04-14
**Purpose:** Canonical authoring rules for `.nomx` sources. Each section resolves one or more rows from [doc 16](./16-nomx-syntax-gap-backlog.md) whose destination was "authoring-guide entry". These rules are conventions, not grammar — the parser doesn't enforce them, but they keep Nom source legible across authors.

> **Status 2026-04-14:** First 5 idioms written in this cycle (closes doc 16 rows #20, #24, #25, #34, #35). Subsequent cycles append more idioms here and flip the corresponding doc 16 rows to ✅.

---

## I1. `perhaps … nothing` for optional values

**Resolves:** doc 16 row #20.

Where another language would write `Option<T>`, `T?`, `Maybe T`, or `T | null`, Nom uses the prose form **`perhaps <T>`** for the type and **`nothing`** for the absent value:

```nomx
the function get_python_source is
  intended to return the source text perhaps nothing.

the result is perhaps the text.
when x is nothing, get_python_source returns nothing.
```

**Rules:**

- `perhaps X` is the optional-type phrase. The next noun is the contained type.
- `nothing` is the sentinel absent value. It is a reserved keyword.
- `is nothing` is a first-class predicate; always check explicitly (`when x is nothing, ...`).
- Never mix with `or nothing` (that would read as sum-type; keep `perhaps` as the single phrasing).

**Anti-pattern:**

```nomx
# Bad — mixes sum-type phrasing with optional concept:
define foo returns text or nothing.

# Good — `perhaps` form:
define foo returns perhaps text.
```

---

## I2. Exit-code vocabulary: `success`, `failure`, `code N`

**Resolves:** doc 16 row #24.

Nom programs signal process-exit status with three fixed phrases — no integer literal conventions, no `EXIT_FAILURE` constant needed:

- `returns success` — process exits 0.
- `returns failure` — process exits 1. Use for generic errors.
- `returns code <N>` — process exits with the given integer. Reserve for cases where the specific code is load-bearing (Unix conventions, shell scripting).

```nomx
define main that takes argc and argv, returns an exit_code.
  when argc is zero, main returns failure.
  when arguments are valid, main returns success.
  when the daemon failed to start, main returns code 111.
```

**Rationale:** Shell scripts, CI pipelines, and POSIX tools all key on a small set of integer exit codes. Giving Nom fixed names for 0, 1, and "explicit N" keeps translated code portable without paying for a constants table.

---

## I3. `text-sprintf` — prose string composition

**Resolves:** doc 16 row #25.

Where C uses `sprintf`, Python `f"..."`, Rust `format!`, and JS template literals, Nom uses the **text-of / followed-by** idiom:

```nomx
the greeting is the text of "hello, " followed by name followed by "!".
the url is "https://" followed by host followed by ":" followed by port.
```

**Rules:**

- `the text of <literal>` starts a prose-formatted string.
- `followed by <expr>` chains further pieces; evaluates `<expr>` to text and concatenates.
- Never introduce `"{name}"`-style interpolation — the goal is to keep the language readable out loud.

**For a large number of pieces**, break the phrase across multiple lines:

```nomx
the banner is the text of "WARNING: binary '"
  followed by filename
  followed by "' is deprecated; use '"
  followed by replacement
  followed by "' instead.".
```

**Anti-pattern:**

```nomx
# Bad — opaque interpolation:
the banner is "WARNING: binary '{filename}' is deprecated.".

# Good — prose concatenation:
the banner is the text of "WARNING: binary '"
  followed by filename
  followed by "' is deprecated.".
```

A future wedge (W5 format-string interpolation per doc 16) may add a more compact form; until then, `followed by` is canonical.

---

## I4. Non-ASCII string literals are verbatim UTF-8

**Resolves:** doc 16 row #34.

Identifiers (the names of functions, words, concepts) are restricted to ASCII per the current lexer (`is_word_start_char`). But **string literals inside double quotes are UTF-8 verbatim** — no escape-mangling, no normalization, no re-encoding:

```nomx
the author_name is "Blaž Hrastnik".
the greeting_japanese is "こんにちは".
the math_char is "π".
```

**Rules:**

- `"..."` content is raw UTF-8 bytes; the lexer preserves them unchanged.
- No Unicode escape sequence is supported; paste the actual character.
- Identifiers remain ASCII-only — if you need to name a function by a non-ASCII concept, transliterate (`pi_constant`, not `π_constant`).
- The `matching "..."` clause follows the same rule — a typed-slot ref can match prose in any language: `the @Function matching "こんにちは greeting" with at-least 0.85 confidence`.

**Smoke test shipped (doc 16 row #34):** `ct11_utf8_string_literals_verbatim` in nom-concept parses a concept with `こんにちは`, `Blaž Hrastnik`, and `π` inside `matching "..."` clauses and asserts each string survives byte-identical through serde-JSON round-trip. Scope note: only string literals are covered — intent-prose outside quotes currently relies on ASCII because the lexer's fallthrough branch treats non-ASCII chars as single-char `Word` tokens and the prose collector may reshape them. Lift that under a future wedge if non-ASCII intent prose becomes a real authoring need.

---

## I5. TOML / external-config hyphen keys map to underscore identifiers

**Resolves:** doc 16 row #35.

When translating configs from TOML (`default-theme`), CSS (`font-size`), Kubernetes YAML (`api-version`), HTTP headers (`content-type`), etc., Nom identifiers use underscore substitution:

```
default-theme  →  default_theme
font-size      →  font_size
api-version    →  api_version
content-type   →  content_type
```

**Rules:**

- Hyphen is NOT a valid identifier character in Nom (the lexer's `is_word_continue_char` accepts `alnum | _`; hyphen terminates a word).
- When translating config keys, replace each hyphen with underscore. The mapping is purely mechanical.
- **Round-trip emission** (e.g., `nom -> toml`): the Nom emitter should know the target config format's key-case convention and reverse the mapping (underscores → hyphens). Never require humans to back-translate by hand.
- Exception: identifiers that already contain hyphens in their upstream concept name (e.g., English phrases like `case-insensitive` in docstrings) stay hyphenated inside string literals; only structural identifiers get the underscore rule.

**Anti-pattern:**

```nomx
# Bad — hyphen in identifier (won't lex):
exposes default-theme as text.

# Good — underscore form:
exposes default_theme as text.
```

---

## I6. Docstrings translate to `intended to …`

**Resolves:** doc 16 row #7.

Source languages carry per-function prose in dedicated slots — Python docstrings, Rust `///`, Java javadoc, Go `// Package …` preambles. Nom collects all of that into the `intended to …` clause on the entity:

```nomx
the function get_python_source is
  intended to return the Python source string for a callable or string argument,
  or nothing when unavailable.

  requires input is a Python object.
  ensures result is text or nothing; never raises.

  favor correctness.
```

**Rules:**

- The `intended to …` phrase is a single sentence. Split longer docstrings into multiple sentences separated by `.`; each becomes part of the same intent slot.
- Discard markup syntax from the source (Markdown backticks, ReST directives, `:param:` fields). Contracts move to `requires` / `ensures`; examples move to separate nomtu tests.
- Keep the prose English-vocabulary per `ecd0609` — translate non-English docstrings, don't transliterate.
- The goal is **what the entity does**, not **how the upstream library implemented it**. Drop implementation details.

**Anti-pattern:**

```nomx
# Bad — verbatim-copied Python docstring with code fences + :param:
  intended to "Get Python source (or not), preventing exceptions.

  :param x: Any Python object
  :returns: Source string or None
  ".

# Good — prose rewrite:
  intended to return the Python source string for a callable argument,
  or nothing when it cannot be obtained.
```

---

## I7. Redundant v1 body when fully delegated

**Resolves:** doc 16 row #12.

When a function's whole job is to call one other function (thin wrapper, re-export, rename), the v1 body becomes redundant noise. In that case prefer the v2 form — the `uses` reference says everything needed:

```nomx
# v1 (redundant):
define base64_decode
  that takes input_text, returns bytes or a decode_error.
the bytes are the base64 decoding of input_text.

# v2 (preferred for thin wrappers):
the function base64_decode is
  intended to decode base64-encoded text into raw bytes.
  uses the @Function matching "base64 decode primitive" with at-least 0.9 confidence.
  requires input is valid base64.
  ensures output matches the encoded payload.
  favor correctness.
```

**Rules:**

- When the body is one `uses` call (no branching, no local values), drop the v1 form and keep only v2.
- When the body has >= 2 `uses` calls, the order matters — keep v1 (with imperative verbs) OR extend v2 with `then`-chained `uses` clauses.
- Never write both forms for the same function. Pick one.

---

## I8. Pipelines / command substitution → named intermediate values

**Resolves:** doc 16 rows #30 and #31 (partial).

Shell's `javac \`find java -name '*.java'\``-style command substitution compresses a pipeline into a single line. Nom unrolls it:

```nomx
# Bad — opaque nesting (authoring-hostile, even if the grammar accepts it):
compile every output of "find java -name '*.java'" with javac.

# Good — named intermediate value:
the java_sources are every .java file under the java folder.
compile java_sources with javac,
  targeting java 8,
  with boot class path android_jar.
```

**Rules:**

- Each intermediate expression gets a name via `the X is …`. Never nest more than two function calls in one sentence.
- The name should describe WHAT the value is (`the java_sources are …`), not HOW it was obtained (`the output_of_find is …`).
- A chain of three-plus pipes becomes a block: each line `the step<N> is …` ending with the final result.
- Preserve the source language's idiom in a comment only if the translation is non-obvious; otherwise the named values carry the intent.

---

## I9. Atomic / concurrency primitives

**Resolves:** doc 16 row #10.

When translating code that touches atomic state (compare-and-swap, fetch-add, memory ordering, mutex acquire/release), Nom surfaces the intent as a **single phrase** — the surrounding syntax doesn't carry the atomic-operation boilerplate:

```nomx
# "atomically become": compare-and-swap style mutation.
the lock flag atomically becomes true.
the counter atomically increases by 1.
the pending atomically decreases to zero.

# "atomically read": snapshot-read of a shared value.
the current_depth is atomically read from the depth_counter.

# Mutex / guard scopes.
while holding the cache mutex,
  the entry is perhaps found at key.
```

**Rules:**

- The adverb `atomically` prefixes the operation. Verbs: `becomes`, `increases by`, `decreases by`, `decreases to`, `read from`, `swap with`.
- Never expose memory-order choices at authoring level (`Relaxed` / `Acquire` / `Release`). The compiler picks the strongest consistency by default; the `hazard cpu_contention` effect can request a relaxed profile when needed.
- Guard scopes use `while holding the X mutex, … end` or the single-line variant `while holding X, …`. Never nest two `holding` scopes in one function — that's the deadlock smell.

**Anti-pattern:**

```nomx
# Bad — exposes atomic primitive by name:
the flag is compare_and_swap(true, false, relaxed_ordering).

# Good — intent-surface phrase:
the lock flag atomically becomes true.
```

---

## I10. Destructuring parameters

**Resolves:** doc 16 row #13.

TypeScript's `function foo({state, dispatch}: EditorView)`, Python's `def foo((a, b))`, Rust's `fn foo(Point { x, y }: Point)` — these compress "take this record and pull two named fields out of it" into one parameter. Nom's authoring-level form names the record and references its fields:

```nomx
# v1 — "takes" with an "and holds" phrase:
define indent_more
  that takes an editor_view that holds state and dispatch,
  returns a boolean.

# v2 — the function body references the named fields directly:
the function indent_more is
  intended to insert one indent unit at the current selection.

  requires editor_view's state is not read-only.
  uses the @Function matching "dispatch editor change" with at-least 0.85 confidence.
```

**Rules:**

- Parameter carries its record name (`editor_view`), not the destructured fields. This keeps the signature one line.
- Body references the fields by possessive (`the editor_view's state`, `editor_view's dispatch`). No need to re-bind local names.
- For three-or-more referenced fields, pre-bind them with `let` equivalents at the body top: `the state is editor_view's state. the dispatch is editor_view's dispatch.`
- Never use tuple-style destructuring (`takes a pair (x, y)`); always name the whole value.

---

## I11. List / text accessor primitives

**Resolves:** doc 16 row #26.

Translations keep bumping into a small cluster of list- and text-index accessors. Pin these as canonical phrases the parser should learn (each ships as an authoring-corpus primitive first, then a wedge-grammar rule later):

| Intent | Nom phrase | Source analogs |
|---|---|---|
| First element | `the first of L` | `L[0]`, `L.first()`, `L.at(0)` |
| Nth element | `the Nth of L` | `L[N]`, `L.at(N)` |
| Last element | `the last of L` | `L[-1]`, `L.last()` |
| Length | `how many in L` | `len(L)`, `L.length`, `L.size()` |
| Exists | `whether any of L is X` | `X in L`, `L.contains(X)` |
| Slice | `L from N to M` | `L[N:M]`, `L.slice(N,M)` |
| Find first | `the first of L where <predicate>` | `next(filter(...))`, `L.iter().find(...)` |
| Text prefix | `X's prefix up to "sep"` | `X.split("sep")[0]` |
| Text suffix | `X's part after the last "sep"` | `basename`, `rsplit_once("/").1` |

**Rules:**

- Every accessor phrase reads as English. No bracket syntax at the authoring surface.
- Phrases are idioms, not operators — the parser tokenizes them via the existing `Word` stream; eventually doc 13's W-wedges may hoist the most common forms into reserved tokens.
- `the Nth of L` uses 1-based ordinals (`the first`, `the second`, …) OR an explicit numeric literal (`the 7th of L`). Zero-indexed access is a translation-time concern, not authoring-time.

---

## I12. `uses` vs imperative verbs for side-effecting code

**Resolves:** doc 16 row #27.

Doc 14 surfaced ambiguity about whether side-effects should be modeled as `uses the @Function matching "…"` (intent layer) or as imperative verb clauses in the body (`print "…"`, `write X to Y`). Rule:

- **v2 (intent layer):** Always list the side-effect via a `uses` ref. The v2 form is declarative — the function's *intent* is "do X"; the `uses` is "by invoking a primitive that does X".
- **v1 (body layer):** Write imperative verbs directly (`print`, `write`, `delete`, `dispatch`). Matching the source-language's flow keeps translations straightforward.
- **Both forms in one translation:** acceptable when v1 gives the flow and v2 summarizes the intent. A hybrid entity can carry a v1 body section and still `uses` primitives for intent discoverability.

**Rule of thumb:** a side-effecting function with a single-sentence intent uses v2 only. Two or more distinct effects → write v1; add the most important effect as a v2 `uses` for discoverability.

**Effects (`benefit` / `hazard` + synonyms `boon` / `bane`):** attach on entity or composition decls inside `.nomtu` files, NOT on concept bodies inside `.nom` files. See ct12 / ct13 smoke tests in nom-concept for the canonical shape:

```nomx
the function cipher_rc4_set_key is given a key, returns nothing.
  requires key length is positive.
  hazard weak_cipher, deprecated.

the function write_cache is given a key and value, returns nothing.
  benefit cache_warmup.
```

For concept-layer positive/negative-effect statements, use prose inside `intended to …` instead (the effect-valence system is entity-local).

---

## I13. Config-as-data vs. config-as-code split

**Resolves:** doc 16 row #33.

A `.toml` / `.yaml` / `.json` config is **pure data** — it has no behavior. When translating such a file into Nom, use the `data` kind with `exposes` fields, NOT the `function` kind:

```nomx
# Pure data — declare the schema:
the data book_config is
  intended to hold the mdBook build configuration.

  exposes book_authors as text list.
  exposes book_language as text.
  exposes book_src as path.
  exposes html_cname as text.
  exposes html_default_theme as text.

  favor correctness.
  favor documentation.

# Pure data — provide a named value:
the book_config for the helix_manual is
  book_authors is ["Blaž Hrastnik"],
  book_language is "en",
  book_src is "src",
  html_cname is "docs.helix-editor.com",
  html_default_theme is "colibri".
```

**Rules:**

- **Schema declaration** (`the data X is … exposes … favor …`) goes in one concept file; **named instances** (`the X for Y is …`) go wherever they're used.
- Never mix schema + instance in the same sentence. `exposes` declares; `is` supplies.
- Emitters (Nom → TOML, Nom → YAML) round-trip by pairing the schema with an instance. Format-specific conventions (hyphen keys per I5, dotted-section names per W17) are reversed at emit time, not at authoring time.
- Config-as-code — things that look like config but run logic (a `build.rs`, a Makefile target, a GitHub Actions YAML with `run:` blocks) — stay in the `function` kind with an `intent to build X using Y` frame. The boundary is whether execution happens.

---

## Closure status (doc 16 rollup)

After this doc (I1-I13 landed):

- Row #20 — ✅ I1 (perhaps/nothing)
- Row #24 — ✅ I2 (exit codes)
- Row #25 — ✅ I3 (text-sprintf)
- Row #34 — ✅ I4 (UTF-8 string literals) + ct11 smoke test landed
- Row #35 — ✅ I5 (hyphen→underscore)
- Row #7  — ✅ I6 (docstrings → `intended to`)
- Row #12 — ✅ I7 (redundant v1 body rule)
- Row #30 + #31 — ✅ I8 (named intermediate values / pipeline unroll)
- Row #10 — ✅ I9 (atomic / concurrency primitives)
- Row #13 — ✅ I10 (destructuring parameters)
- Row #26 — ✅ I11 (list / text accessor primitives)
- Row #27 — ✅ I12 (uses vs imperative for side-effects)
- Row #33 — ✅ I13 (config-as-data vs config-as-code split)

**Authoring-guide backlog:** 0 rows remain. All 13 authoring-guide destinations closed by this doc.

Future cycles focus on the 12 wedge-queued rows (W4-A3/A4/A5/A6, W5-W18), the 4 remaining smoke-test rows, and the 2 open design questions (#4 Path/file subkinds, #15 closures).
