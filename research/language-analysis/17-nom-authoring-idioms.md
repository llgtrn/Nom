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

## Closure status (doc 16 rollup)

After this doc (I1-I8 landed):

- Row #20 — ✅ I1 (perhaps/nothing)
- Row #24 — ✅ I2 (exit codes)
- Row #25 — ✅ I3 (text-sprintf)
- Row #34 — ✅ I4 (UTF-8 string literals) + ct11 smoke test landed
- Row #35 — ✅ I5 (hyphen→underscore)
- Row #7  — ✅ I6 (docstrings → `intended to`)
- Row #12 — ✅ I7 (redundant v1 body rule)
- Row #30 + #31 (partial) — ✅ I8 (named intermediate values / globbing primitives unrolled)

**Remaining authoring-guide rows in doc 16 (5 of 13 still open):**
#10 (atomic primitives), #13 (destructuring params), #26 (list/text accessors), #27 (`uses` vs imperative), #33 (config-as-data split). These land in future cycles as I9-I13.
