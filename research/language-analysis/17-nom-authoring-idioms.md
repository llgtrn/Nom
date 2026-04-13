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

**Smoke test candidate (doc 16 row #34):** add a `ct11_utf8_string_literal_roundtrip` test in nom-concept that parses a concept containing `Blaž Hrastnik` and asserts the exact bytes survive through the AST.

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

## Closure status (doc 16 rollup)

After this doc:

- Row #20 — ✅ I1 above
- Row #24 — ✅ I2 above
- Row #25 — ✅ I3 above
- Row #34 — ✅ I4 above (smoke-test referenced; not yet landed)
- Row #35 — ✅ I5 above

Remaining authoring-guide rows in doc 16 (8 of 13): #7 (docstring→`intended to`), #10 (atomic primitives), #12 (redundant v1 body), #13 (destructuring params), #26 (list/text accessors), #27 (`uses` vs imperative), #30 (globbing primitives), #31 (pipelines→intermediate values), #33 (config-as-data split). These land in future cycles as I6-I13.
