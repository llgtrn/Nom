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

## I14. Default parameter values

**Resolves:** doc 16 row #37.

Languages that support defaults (Python `def f(x=3)`, Kotlin `fun f(x: Int = 3)`, Ruby `def f(x = 3)`) let the caller omit arguments. Nom handles defaults **declaratively inside `intended to …` prose**, NOT as a type-system feature. Don't extend the signature grammar; extend the intent description:

```nomx
the function fetch_with_retry is
  intended to fetch a url with exponential backoff,
  using max_attempts of 3 if none is given.

  uses the @Function matching "retry with backoff" with at-least 0.85 confidence.

  requires url is well-formed.
  ensures the response is either the fetched body or the last error.

  favor correctness.
```

**Rules:**

- Defaults live in the `intended to …` sentence via a phrase like `using X of <value> if none is given` or `defaulting X to <value>`.
- Callers that omit the argument implicitly get the default. The compiler doesn't enforce defaults statically; it's a documentation + authoring convention.
- Never add a `= 3` syntax to the parameter list. Keeps the signature grammar one shape.
- For runtime validation (refuse unknown / require explicit value), use a `requires X is given` contract clause instead of a default.

**Rationale:** defaults are authoring ergonomics, not type signatures. Putting them in prose keeps the grammar closed under the 7-kind set + contract clauses.

---

## I15. Iterator vs. materialized sequences — lazy by default

**Resolves:** doc 16 row #38.

Every `sequence` (Nom's generic list-like value) is **lazy by default**. Operations like `map`, `filter`, `flat_map`, and `where` compose into a chain that produces elements on demand. Materialization into a concrete list is explicit:

```nomx
the interesting_items is every item of input_items where item is active.
# ^ lazy sequence — no allocation yet

the interesting_list is collect the interesting_items into a vector.
# ^ materialized vector

the count is how many in interesting_items.
# ^ materialization implicit (driven by the accessor)
```

**Rules:**

- Chained sequence transformations stay lazy until a **terminal accessor** forces evaluation: `collect the X into a vector`, `how many in X`, `the first of X`, `the Nth of X`, `whether any of X is Y`.
- `for each x in seq, …` iterates lazily; no full materialization.
- Side-effecting iteration (e.g., `for each …, print`) runs the chain once, no storage.
- When you explicitly want allocation, say so: `collect X into a vector` / `build X into a map`. Editors + linters catch unintentional re-traversals of a lazy sequence.

**Anti-pattern:**

```nomx
# Bad — unclear whether this materializes and how many times:
the items is input_items filtered by is_active mapped through to_summary.

# Good — explicit about lazy vs. materialized:
the summaries is every input_item where item is active, summarized by to_summary.
the summary_list is collect the summaries into a vector.
```

---

## I16. `identifier` as a distinct data shape

**Resolves:** doc 16 row #45.

GraphQL has `ID!`. TypeScript / Rust often use branded types (`type UserId = string & { __brand: 'user' }`). Nom treats **identifier** as a separate data shape in the authoring vocabulary — NOT a new kind in KINDS (that stays at 7), but a first-class shape label alongside `text` / `integer` / `timestamp` / `path`.

```nomx
the data User is
  intended to represent a registered user.

  exposes id as identifier.       # not `text` — a content-addressed ID
  exposes email as text.
  exposes created_at as timestamp.

  favor correctness.
```

**Rules:**

- `identifier` is a distinct type shape; `id as text` should flag in the strict validator.
- Identifiers are **opaque** — downstream code doesn't assume structure (no length checks, no pattern-match). Corpus-registered helper functions like `@Function matching "generate new identifier"` / `@Function matching "compare identifiers"` provide the operations.
- Different identifier families (UserId vs. SessionId) get different **feature-stack words**: `user_id_uuid_v7` vs. `session_id_short`. Resolver picks by kind + matching clause.
- Never render identifiers via `text.concat` / `text.startsWith` / etc. Those are text operations; applying them to an identifier is a strict-mode warning.

**Rationale:** identifier is the most common "looks like text but means something else" value in practice. Making it a distinct shape label catches confusions at the authoring layer without growing the closed kind set.

---

## I17. Time-range idiom: `within the last N days`

**Resolves:** doc 16 row #43.

Relative-time predicates (`created_at > NOW() - INTERVAL '30 days'`, `deleted_at > Time.now - 7.days.ago`, `updated_at > Instant.now() - Duration.ofDays(30)`) are one of the most universal patterns. Nom's canonical phrasing is **`within the last N <unit>`**:

```nomx
the active_users is every user
  where user's last_login_at is within the last 30 days
  and user's deleted_at is nothing.
```

**Rules:**

- Units: `days` / `hours` / `minutes` / `seconds` / `weeks` / `months` / `years`. Mixed units (`within the last 1 hour 30 minutes`) are not supported — compose as two clauses or round to the smaller unit.
- `within the last N <unit>` is evaluated relative to "now" at the point of prose evaluation; the time origin is implicit.
- Inverse: `older than N <unit>` (past horizon) or `in the next N <unit>` (future horizon). Symmetric idioms; pick whichever reads more natural.
- For non-relative times use explicit literals: `after 2026-04-14` or `between 2026-04-01 and 2026-04-30`.

---

## I18. Shell-exec primitive: `run <command> with <args>`

**Resolves:** doc 16 row #48.

Every translation that invokes a subprocess (`gcc`, `cargo`, `rm`, `docker build`) reaches for the shell. Nom's canonical phrasing names the binary + args + optional stdin/stdout/stderr redirection, without exposing the shell itself:

```nomx
the build_result is run "gcc" with the args ["-O2", "-Wall", "-o", "build/app", "src/main.c"]
  and capture stdout into the build_log.

run "rm" with the args ["-rf", "build/"].
# ^ no capture — fire-and-observe
```

**Rules:**

- `run "<binary>" with the args [<list>]` is the base form.
- `and capture stdout into X` / `and capture stderr into Y` / `and feed stdin from Z` are optional post-fix modifiers.
- **Don't translate shell pipelines directly** — unroll into named intermediate values per doc 17 §I8: `the sources are run "find" with args [...]. pass sources to run "gcc" with args [...]` beats `find ... | gcc ...`.
- Exit codes surface via `the result is run "<binary>" with args ...; the exit_code is result's exit_code` — integer `0` is success per doc 17 §I2.
- `hazard shell_injection` valence gets attached whenever any arg comes from untrusted input.

---

## I19. Method → receiver-as-parameter rule

**Resolves:** doc 16 row #53.

OOP methods (Ruby's `instance.method(arg)`, Python's `instance.method(arg)`, Java's `instance.method(arg)`) are functions that take the instance as an implicit first parameter. Nom has no class/method syntax; **translate every method into a top-level function whose first parameter is the receiver, named for what the receiver IS**:

```
# OOP source:
cache.with_retry(5) { fetch(url) }

# Nom translation:
retry_with_backoff(cache, max_attempts = 5, attempt_action = fetch_upstream)
```

**Rules:**

- The first parameter is **always** the receiver. Name it for the noun, not for `self` / `this`.
- Chained methods (`builder.id("abc").email("x@y").build()`) unroll into a sequence of transformations or collapse into a single `build_<X>` call per doc 19 §D2.
- Don't introduce `object.method()` dot-call syntax at the authoring layer — Nom is prefix-call.
- For static / class methods, the receiver slot becomes the class-as-data: `user_count(users)` where `users` is the registered collection.

---

## I20. `work_group` idiom for concurrent work tracking

**Resolves:** doc 16 row #57.

Go's `sync.WaitGroup`, Rust's `JoinSet`, and Java's `CompletableFuture.allOf` all solve the same problem: "run N workers concurrently, then wait for all of them to finish". Nom's canonical form is **`work_group`** as a first-class authoring noun:

```nomx
the results_channel is a channel of results with capacity len(urls).
the work_group tracks active workers.

for each url in urls,
  add one worker to work_group.
  start a worker that
    when the worker finishes, remove it from work_group.
    send fetch_upstream(url) into results_channel.

wait for work_group to drain.
close results_channel.
```

**Rules:**

- `work_group` is a noun — create with `the <name> work_group tracks active workers`, never with typed syntax like `WorkGroup::new()`.
- `add one worker to <wg>` / `remove worker from <wg>` are the verbs. Pair them via `when the worker finishes, remove it from <wg>` finalizer phrasing (doc 17 §I9's atomic-primitive style).
- `wait for <wg> to drain` blocks until the group's worker count reaches zero.
- **Don't mix work_group with direct thread counters** — one or the other per function. Mixing surfaces as a strict-mode warning once W33 finalizer-clause grammar lands.

---

## Closure status (doc 16 rollup)

After this doc (I1-I20 landed):

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

- Row #37 — ✅ I14 (default parameter values)
- Row #38 — ✅ I15 (iterator vs. materialized sequences)
- Row #45 — ✅ I16 (identifier as distinct data shape)
- Row #43 — ✅ I17 (time-range idiom `within the last N days`)
- Row #48 — ✅ I18 (shell-exec primitive `run X with args Y`)
- Row #53 — ✅ I19 (method → receiver-as-parameter rule)
- Row #57 — ✅ I20 (work_group concurrent work tracking)

**Authoring-guide backlog: 0 rows remain.** All authoring-guide destinations closed. Doc 17 is the canonical authoring chapter (I1-I20).

Future cycles focus on the 12 wedge-queued rows (W4-A3/A4/A5/A6, W5-W18), the 4 remaining smoke-test rows, and the 2 open design questions (#4 Path/file subkinds, #15 closures).
