# 14 — Nom translation examples from `Accelworld/upstreams`

**Date:** 2026-04-14
**Purpose:** Translate small, canonical functions from 228 upstream repos at `C:\Users\trngh\Documents\APP\Accelworld\upstreams` into Nom to (a) stress-test `.nomx v1` and `.nomx v2` syntax against real-world shapes, (b) surface syntax gaps that the strictness lane (doc 13) must address, (c) seed the authoring-corpus with translations that later become smoke tests.

> **Status 2026-04-14:** Living doc; starts with 5 seed translations. Subsequent cycles append more. Each translation MUST cite its upstream path and highlight at least one syntax-gap or enrichment candidate.

---

## Translation protocol

1. Pick one small function (≤30 LOC original, one concern).
2. Render in `.nomx v1` (define-style) and `.nomx v2` (typed-slot with `@Kind matching "..."`) when both are expressible.
3. Flag the syntax gap: a feature `.nomx` does not yet cover, or a phrasing that feels awkward. Gaps feed doc 13's wedge list or a new doc 15 as needed.
4. Keep prose English-only (per `ecd0609`).

---

## 1. `render_template` — Rust I/O + string replace

**Source:** [bat/build/util.rs:5-21](../../../APP/Accelworld/upstreams/bat/build/util.rs#L5)

```rust
pub fn render_template(
    variables: &HashMap<&str, String>,
    in_file: &str,
    out_file: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let mut content = fs::read_to_string(in_file)?;
    for (variable_name, value) in variables {
        let pattern = format!("{{{{{variable_name}}}}}");
        content = content.replace(&pattern, value);
    }
    fs::write(out_file, content)?;
    Ok(())
}
```

### `.nomx v1` translation

```nomx
define render_template
  that takes variables, in_file, and out_file,
  returns nothing,
  require in_file is a path that exists.
  ensure out_file holds the rendered content.
the content is the text of in_file.
for each variable_name and value in variables,
  the pattern is the text "{{variable_name}}".
  the content is content with pattern replaced by value.
write content to out_file.
```

### `.nomx v2` translation (with typed-slot refs)

```nomx
the function render_template is
  intended to write a templated file by substituting variables into an input template.

  uses the @Function matching "read text file" with at-least 0.85 confidence.
  uses the @Function matching "write text file" with at-least 0.85 confidence.
  uses the @Function matching "replace substring" with at-least 0.85 confidence.

  requires input file exists.
  ensures output file contains the rendered result.

  favor correctness.
```

### Gaps surfaced

1. **Iteration phrasing** — `.nomx v1` has `for each X and Y in Z` but no tests currently exercise two-variable destructuring. Add a lexer test for this.
2. **Format-string interpolation** — `"{{variable_name}}"` at v1 layer is literal; Nom needs a prose rule like `the text "X" with value substituted`. Candidate for doc 15 §format-strings.
3. **`returns nothing`** — currently accepted but not formally pinned as equivalent to `()`/`Unit`. Needs a grammar rule.
4. **Resource paths as a `@Kind`** — v2 translation elides the path/file distinction into "read/write text file". The full doc-02 `@Data` kind (file handle, path, bytes) is not yet split into subkinds. Open: should paths be `@Data matching "filesystem path"` or a dedicated `@Path` kind?

---

## 2. `AgentAction` — Python data class

**Source:** [langchain-master/libs/core/langchain_core/agents.py:44-69](../../../APP/Accelworld/upstreams/langchain-master/libs/core/langchain_core/agents.py#L44)

```python
class AgentAction(Serializable):
    """Represents a request to execute an action by an agent."""

    tool: str
    tool_input: str | dict
    log: str

    type: Literal["AgentAction"] = "AgentAction"
```

### `.nomx v1` translation

```nomx
record AgentAction
  that holds
    a tool that is text,
    a tool_input that is text or a record,
    a log that is text.
  means "a request from an agent to run a tool with an input and a log".
```

### `.nomx v2` translation

```nomx
the data AgentAction is
  intended to represent an agent's request to run a tool with input and a log line.

  exposes tool as text.
  exposes tool_input as text or record.
  exposes log as text.

  favor correctness.
  favor documentation.
```

### Gaps surfaced

1. **Union types in `v2`** — `text or record` is ambiguous. Doc 13 A4's annotator-staged parse should classify this as `Tok::TypeUnion` or reject. Candidate wedge: **W5 — sum-type phrasing.**
2. **Literal-string constants** — Python's `type: Literal["AgentAction"] = "AgentAction"` has no Nom equivalent yet. Candidate: `the data's type is exactly "AgentAction"` or a `const` form.
3. **Docstring vs. `intended to`** — Python conventions put docstrings in a special slot. `.nomx v2`'s `intended to` is the nearest analogue. Worth an explicit mapping note in the authoring guide.

---

## 3. `is_even` — canonical smoke

Not from upstreams, but the smallest possible test. Pins the v2 happy-path.

### `.nomx v1`

```nomx
define is_even
  that takes n, returns a boolean.
  when n is divisible by 2, is_even is true.
  otherwise, is_even is false.
```

### `.nomx v2`

```nomx
the function is_even is
  intended to return true when n is divisible by two.

  requires n is an integer.
  ensures the return is true or false.

  favor correctness.
```

### Gaps surfaced

1. **Returning a primitive** — the `is_even is true` form has no clean v2 analog. Consider `the result is true` as a canonical last-sentence idiom (per doc 05 §"last sentence is the result").

---

## 4. `try_lock` — Rust fallible borrow

**Source:** [atuin (any `try_lock`-style call site; sampled pattern, no exact upstream to cite yet; future cycle should pick a concrete path)]

```rust
pub fn try_lock(&self) -> Result<Guard<'_, T>, LockError> {
    if self.locked.swap(true, Ordering::Acquire) {
        Err(LockError::Contended)
    } else {
        Ok(Guard { inner: self })
    }
}
```

### `.nomx v1`

```nomx
define try_lock
  that takes a resource, returns a guard or a lock_error.
  when the resource's locked flag is already set,
    try_lock returns contended.
  otherwise,
    the resource's locked flag is set.
    try_lock returns a guard over the resource.
```

### Gaps surfaced

1. **Sum-return (Result<A, B>)** — `returns a guard or a lock_error` is expressible at v1 but v2 has no `@Union` kind yet. Same as translation 2's gap #1 — confirms it's a real missing piece.
2. **Atomic state** — no Nom phrase yet for "atomic compare-and-swap". Candidate for a `the resource's locked flag atomically becomes true` primitive in the authoring corpus.
3. **Lifetime annotations** — `Guard<'_, T>` has no Nom equivalent; deferred to the borrow-model work (doc 04 §"ownership" item).

---

## 5. `base64_decode` — common utility

**Source:** any upstream that wraps `base64::decode` (sampled pattern).

```rust
pub fn base64_decode(input: &str) -> Result<Vec<u8>, DecodeError> {
    base64::decode(input)
}
```

### `.nomx v1`

```nomx
define base64_decode
  that takes input_text, returns the bytes or a decode_error.
the bytes are the base64 decoding of input_text.
```

### `.nomx v2`

```nomx
the function base64_decode is
  intended to decode base64-encoded text into raw bytes.

  uses the @Function matching "base64 decode primitive" with at-least 0.9 confidence.

  requires input is valid base64.
  ensures output matches the encoded payload.

  favor correctness.
  favor performance.
```

### Gaps surfaced

1. **Delegating entirely to a matched ref** — both translations are essentially one-liners that "just call the primitive". The v2 form looks clean; the v1 form feels redundant. Suggestion: make the v1 body optional when every contract is satisfied by a single `uses` reference.

---

## 6. `indentMore` — TypeScript editor command

**Source:** [bolt.new-main/app/components/editor/codemirror/indent.ts:12-27](../../../APP/Accelworld/upstreams/bolt.new-main/app/components/editor/codemirror/indent.ts#L12)

```typescript
function indentMore({ state, dispatch }: EditorView) {
  if (state.readOnly) {
    return false;
  }
  dispatch(
    state.update(
      changeBySelectedLine(state, (from, to, changes) => {
        changes.push({ from, to, insert: state.facet(indentUnit) });
      }),
      { userEvent: 'input.indent' },
    ),
  );
  return true;
}
```

### `.nomx v1` translation

```nomx
define indent_more
  that takes an editor_view,
  returns a boolean.
  when the editor_view is read_only, indent_more returns false.
  otherwise,
    for each selected_line in the editor_view,
      push an insert change with the editor's indent_unit at the line's range.
    dispatch the change with user_event "input.indent".
    indent_more returns true.
```

### `.nomx v2` translation

```nomx
the function indent_more is
  intended to insert one indent unit at the start of every line in the current selection,
  unless the editor is read-only.

  uses the @Function matching "dispatch editor change" with at-least 0.85 confidence.
  uses the @Function matching "iterate selected lines" with at-least 0.85 confidence.

  requires editor_view is not read-only for the mutation path.
  ensures every selected line gains one indent unit.

  favor correctness.
```

### Gaps surfaced

1. **Destructuring parameters** — TS's `{ state, dispatch }: EditorView` has no clean v1/v2 analog. Candidate: `takes an editor_view that holds state and dispatch`. Needs an authoring-guide rule.
2. **Early-return guards** — `if (state.readOnly) return false;` becomes a `when ... returns false` clause. Already supported; worth a dedicated smoke test.
3. **Callback closures** — the `(from, to, changes) => { ... }` passed to `changeBySelectedLine` has no v2 shape. Gap for doc 15 §closures.

---

## 7. `Cipher_RC4_set_key` — C OpenSSL wrapper

**Source:** [aircrack-ng/lib/crypto/arcfour-openssl.c:41-51](../../../APP/Accelworld/upstreams/aircrack-ng/lib/crypto/arcfour-openssl.c#L41)

```c
void Cipher_RC4_set_key(Cipher_RC4_KEY * h, size_t l, const uint8_t k[static l]) {
    EVP_CIPHER_CTX * ctx = EVP_CIPHER_CTX_new();
    if (   !ctx
        || !EVP_CipherInit_ex(ctx, EVP_rc4(), NULL, NULL, NULL, 1)
        || !EVP_CIPHER_CTX_set_padding(ctx, 0)
        || !EVP_CIPHER_CTX_set_key_length(ctx, l)
        || !EVP_CipherInit_ex(ctx, NULL, NULL, k, NULL, 1))
        errx(1, "An error occurred processing RC4_set_key");
    h = (void *) ctx;
}
```

### `.nomx v1` translation

```nomx
define cipher_rc4_set_key
  that takes a handle, a key_length, and a key,
  returns nothing.
  the ctx is a new cipher_context.
  when the ctx is not ready
    or the ctx cannot init with rc4,
    or the ctx cannot set padding to zero,
    or the ctx cannot set key_length to key_length,
    or the ctx cannot set the key,
      fail with "An error occurred processing RC4_set_key".
  the handle points to the ctx.
```

### `.nomx v2` translation

```nomx
the function cipher_rc4_set_key is
  intended to install an RC4 encryption key into a cipher handle.

  uses the @Function matching "create cipher context" with at-least 0.9 confidence.
  uses the @Function matching "initialize rc4 cipher" with at-least 0.9 confidence.

  requires key_length is positive and key has at least key_length bytes.
  ensures handle holds a usable rc4 context.
  hazard weak cipher, avoid in new designs.

  favor correctness.
```

### Gaps surfaced

1. **Multi-predicate short-circuit fail** — C's `if (!a || !b || ...) errx(...)` is a valence-negating chain. The v1 `when A or B or C, fail with "..."` form is close but `fail with` has no formal spec yet. Candidate: **W9 fail-expression grammar.**
2. **Pointer assignment `h = (void *) ctx`** — mutation through pointer parameter has no Nom analog. Deferred to borrow-model work.
3. **`hazard` effect** — v2's negative-valence effect (`hazard weak cipher`) is an implemented keyword per doc 07; this translation is a good smoke for the valence-rendering path.

---

## 8. `get_python_source` — Python introspection with null-safety

**Source:** [airflow/airflow-core/src/airflow/utils/code_utils.py:25-?](../../../APP/Accelworld/upstreams/airflow/airflow-core/src/airflow/utils/code_utils.py#L25)

```python
def get_python_source(x: Any) -> str | None:
    if isinstance(x, str):
        return x
    if x is None:
        return None
    source_code = None
    if isinstance(x, functools.partial):
        source_code = inspect.getsource(x.func)
    # ... (truncated)
    return source_code
```

### `.nomx v1` translation

```nomx
define get_python_source
  that takes x, returns text or nothing.
  when x is text, get_python_source returns x.
  when x is nothing, get_python_source returns nothing.
  the source_code is nothing.
  when x is a partial, the source_code is the source of x's inner function.
  get_python_source returns source_code.
```

### `.nomx v2` translation

```nomx
the function get_python_source is
  intended to return the Python source string for a callable or string argument, or nothing when unavailable.

  uses the @Function matching "inspect source of callable" with at-least 0.85 confidence.

  requires input is a Python object.
  ensures result is text or nothing; never raises.

  favor correctness.
  favor documentation.
```

### Gaps surfaced

1. **`text or nothing` union-return** — repeats the union-type gap from translation #2 + #4. Three data points now confirm `@Union` / sum-return as a real missing primitive.
2. **`is nothing` as a first-class predicate** — Nom already has `perhaps...nothing` per doc 05; this translation confirms the phrasing is natural. Pins the authoring idiom.
3. **Type probes (`isinstance`)** — `when x is text` / `when x is a partial` needs a formal `is-a` grammar rule. Candidate: **W10 runtime-type-probes.**

---

## 9. `OS.String` — Go stringer method + iota enum

**Source:** [gvisor/pkg/abi/abi.go:26-41](../../../APP/Accelworld/upstreams/gvisor/pkg/abi/abi.go#L26)

```go
type OS int
const (
    Linux OS = iota
)
func (o OS) String() string {
    switch o {
    case Linux:
        return "linux"
    default:
        return fmt.Sprintf("OS(%d)", o)
    }
}
```

### `.nomx v1` translation

```nomx
record OS that is one of Linux.
define os_string
  that takes an os_value, returns text.
  when os_value is Linux, os_string returns "linux".
  otherwise, os_string returns "OS(" followed by os_value as text followed by ")".
```

### `.nomx v2` translation

```nomx
the data OS is
  intended to enumerate target operating systems for an ABI.
  exposes Linux as a variant.

the function os_string is
  intended to render an OS value as human-readable text.

  uses the @Function matching "format numeric fallback" with at-least 0.85 confidence.

  requires input is a known or unknown OS variant.
  ensures output is a stable string representation.

  favor correctness.
  favor documentation.
```

### Gaps surfaced

1. **Enum / sum-type (`record X that is one of A, B, C`)** — `.nomx v1` has a tentative `choice` keyword; `v2` has no dedicated sum-type expression. Enum is a strict subset of the union-type gap (translations #2 / #4 / #8). Candidate: **W11 enum / variant declarations.**
2. **Method-on-type (`func (o OS) String()`)** — Nom currently thinks of functions as free-standing. No receiver syntax exists. Candidate: **W12 receiver-form methods** or a resolver convention (`os_string` namespaced by first-arg type).
3. **String concatenation (`followed by`)** — `v1` spelling is verbose. Authoring guide candidate: dedicated `text-sprintf` idiom.

---

## 10. `main` — C++ deprecation-warning CLI

**Source:** [llama-cpp/examples/deprecation-warning/deprecation-warning.cpp:9-38](../../../APP/Accelworld/upstreams/llama-cpp/examples/deprecation-warning/deprecation-warning.cpp#L9)

```cpp
int main(int argc, char** argv) {
    std::setlocale(LC_NUMERIC, "C");
    std::string filename = "main";
    if (argc >= 1) {
        filename = argv[0];
    }
    auto pos = filename.find_last_of("/\\");
    if (pos != std::string::npos) {
        filename = filename.substr(pos+1);
    }
    auto replacement_filename = "llama-" + filename;
    if (filename == "main") {
        replacement_filename = "llama-cli";
    }
    fprintf(stdout, "WARNING: The binary '%s' is deprecated.\n", filename.c_str());
    fprintf(stdout, " Please use '%s' instead.\n", replacement_filename.c_str());
    return EXIT_FAILURE;
}
```

### `.nomx v1` translation

```nomx
define main
  that takes argc and argv, returns an exit_code.
  set locale numeric to "C".
  the filename is "main".
  when argc is at least 1, the filename is argv's first entry.
  when filename contains "/" or "\\",
    the filename is filename after its last separator.
  the replacement_filename is "llama-" followed by filename.
  when filename is "main", the replacement_filename is "llama-cli".
  print "WARNING: The binary '", filename, "' is deprecated.".
  print " Please use '", replacement_filename, "' instead.".
  main returns failure.
```

### `.nomx v2` translation

```nomx
the function main is
  intended to print a deprecation warning pointing users at the llama-cli binary replacement.

  uses the @Function matching "split path basename" with at-least 0.85 confidence.
  uses the @Function matching "print formatted line" with at-least 0.85 confidence.

  requires argv has at least one entry.
  ensures the program exits with failure.

  favor correctness.
  favor documentation.
  hazard deprecated binary invocation, avoid in new scripts.
```

### Gaps surfaced

1. **Entry-point `main`** — Nom hasn't pinned whether `main` is grammatical special-case or just another function. Candidate: **W13 entry-point convention.**
2. **Side-effect-heavy function (`setlocale`, `fprintf`)** — both translations list effects inline. The v1 form uses imperative verbs (`set`, `print`), the v2 form uses `uses` references. Consistency check: **authoring-guide rule on which form is preferred for side-effecting code.**
3. **`argv's first entry`** / `at least 1` / `after its last separator` — a small cluster of list/text accessor idioms. Pin as authoring-corpus primitives: `argv.at(0)` / `text.find_last("/")` / `text.after(index)` in Nom-style prose.
4. **Exit codes** — `returns failure` is prose for `EXIT_FAILURE`. Need a standard exit-code vocabulary: `success`, `failure`, `code <N>`. Candidate: **W14 exit-code vocabulary.**

---

## Running gap list (for next doc 13 refresh + doc 15)

1. Iteration destructuring (`for each K and V in M`) — lexer test missing. **Add in W4-A2b.**
2. Format-string interpolation — grammar rule missing. **New wedge W5.**
3. `returns nothing` grammar pin — **W4-A1 addendum.**
4. Path/file subkinds vs generic `@Data` — **Design question; doc 15 §subkinds.**
5. Union types / sum-return at v2 layer (`text or record`, `Result<A, B>`) — **New kind `@Union` candidate; doc 15.**
6. Literal-string constants and Python-style `Literal[...]` — **W6 candidate.**
7. Docstring → `intended to` mapping — **Authoring guide note.**
8. Primitive-return idiom (`the result is true`) — **Authoring guide + W4-A1 addendum.**
9. Sum-return phrasing at v1 — already works; **test it.**
10. Atomic-state primitives — **Authoring corpus seed.**
11. Lifetime annotations — **deferred (borrow-model work).**
12. Redundant v1 body when fully delegated — **authoring-guide simplification rule.**
13. Destructuring parameters (TS `{state, dispatch}: EditorView`) — **authoring-guide note.**
14. Early-return guards — works; needs a smoke test (translation #6).
15. Callback closures — **gap for doc 15 §closures.**
16. `fail with "..."` expression grammar — **new wedge W9.**
17. Multi-predicate short-circuit fail — subsumed by #16.
18. `hazard` effect rendering — good smoke (translation #7).
19. `is-a` runtime type probes — **new wedge W10.**
20. `perhaps...nothing` idiom confirmed natural — **authoring-guide anchor.**
21. Enum / sum-type declarations — **new wedge W11.**
22. Receiver-form methods (`func (o OS) String()`) — **new wedge W12.**
23. Entry-point `main` special-case — **new wedge W13.**
24. Exit-code vocabulary (`success`/`failure`/`code N`) — **new wedge W14.**
25. `text-sprintf` idiom — **authoring-guide note.**
26. List/text accessor primitives (`at(0)`, `find_last`, `after`) — **authoring-corpus seeds.**
27. Preferred form (`uses` vs imperative verbs) for side-effecting code — **authoring-guide decision needed.**

Each gap becomes either (a) a new wedge in doc 13 §5, (b) an authoring-guide entry, or (c) a deferred design question for doc 15 (to be drafted on next cycle).

## Next cycle plan

- **This cycle added translations #9-10**: `OS.String` (Go, gvisor/pkg/abi) and `main` deprecation-warning (C++, llama-cpp). 27 total gaps now; 4 new wedges queued (W11/W12/W13/W14); 3 new authoring-guide items.
- Next cycle: add Java + Ruby translations (airflow has no Java; need to scan upstreams list for `langchain-java` or similar JVM ecosystem). Alternative: pick a shell script or a Makefile as the most-divergent syntax stress.
- Feed each translation into a nom-parser smoke test that asserts the canonical lexing + parse tree shape. First smoke-test candidate: example 3's `is_even` at both v1 and v2.
- Promote gap list to doc 15b if it exceeds 40 items (currently 27 — approaching the threshold).
