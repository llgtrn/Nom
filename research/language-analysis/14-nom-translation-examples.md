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

## 11. `build-dex.sh` — Bash build pipeline

**Source:** [accesskit/platforms/android/build-dex.sh](../../../APP/Accelworld/upstreams/accesskit/platforms/android/build-dex.sh)

```bash
#!/usr/bin/env bash
set -e -u -o pipefail
cd `dirname $0`
ANDROID_JAR=$ANDROID_HOME/platforms/android-30/android.jar
find java -name '*.class' -delete
javac --source 8 --target 8 --boot-class-path $ANDROID_JAR -Xlint:deprecation `find java -name '*.java'`
$ANDROID_HOME/build-tools/33.0.2/d8 --classpath $ANDROID_JAR --output . `find java -name '*.class'`
```

### `.nomx v1` translation

```nomx
define build_dex
  that takes nothing, returns nothing.
  require halt on any error, unset variable, or pipeline failure.
  change directory to this script's folder.
  the android_jar is path android_home / "platforms/android-30/android.jar".
  delete every class file under java.
  compile every java file under java,
    targeting java 8,
    with boot class path android_jar,
    warning on deprecated use.
  package every class file under java into a dex,
    with class path android_jar, into this folder.
```

### `.nomx v2` translation

```nomx
the function build_dex is
  intended to compile Android Java sources to classes then pack them into a dex file.

  uses the @Function matching "run javac" with at-least 0.9 confidence.
  uses the @Function matching "run d8 dex packer" with at-least 0.9 confidence.
  uses the @Function matching "strict shell" with at-least 0.8 confidence.

  requires ANDROID_HOME is set.
  ensures a fresh dex is produced; intermediate .class files are rewritten.

  favor correctness.
  favor performance.
```

### Gaps surfaced

1. **Shebang / interpreter directive** — `#!/usr/bin/env bash` has no Nom analog. Nom's compilation model makes this largely irrelevant (Nom emits native binaries), but scripts translated INTO Nom may need a `run with shell` metadata clause. Candidate: **W15 interpreter-metadata clause.**
2. **Strict-mode flags (`set -e -u -o pipefail`)** — pins authoring-corpus entry `strict shell` to mean "halt on any error / unset var / pipeline fail". Good example of a composed invariant.
3. **Environment-variable interpolation (`$ANDROID_HOME/...`)** — Nom has no first-class env vocabulary. Candidate: **W16 env-var access**.
4. **Globbing / file-tree queries (`find java -name '*.class'`)** — needs a Nom primitive like `every class file under java`. Authoring-corpus seed.
5. **Process pipelines and command substitution** — `` `dirname $0` `` and `javac ... \`find ...\`` are syntactic shell gymnastics. Nom should force these into named intermediate values (`the java_sources are every .java under java`). Authoring-guide rule.

---

## 12. `book.toml` — Helix mdBook config

**Source:** [helix/book/book.toml](../../../APP/Accelworld/upstreams/helix/book/book.toml)

```toml
[book]
authors = ["Blaž Hrastnik"]
language = "en"
src = "src"

[output.html]
cname = "docs.helix-editor.com"
default-theme = "colibri"
preferred-dark-theme = "colibri"
git-repository-url = "https://github.com/helix-editor/helix"
edit-url-template = "https://github.com/helix-editor/helix/edit/master/book/{path}"
additional-css = ["custom.css"]

[output.html.search]
use-boolean-and = true
```

### `.nomx v1` translation

```nomx
record book_config that holds
  a book section with authors, language, src,
  an output section with html sub-section,
  the html sub-section holds cname, default_theme, preferred_dark_theme,
    git_repository_url, edit_url_template, additional_css, and a search sub-section,
  the search sub-section holds use_boolean_and.

the book_config for helix is
  book: authors = ["Blaž Hrastnik"], language = "en", src = "src",
  output.html: cname = "docs.helix-editor.com",
    default_theme = "colibri",
    preferred_dark_theme = "colibri",
    git_repository_url = "https://github.com/helix-editor/helix",
    edit_url_template = "https://github.com/helix-editor/helix/edit/master/book/{path}",
    additional_css = ["custom.css"],
  output.html.search: use_boolean_and = true.
```

### `.nomx v2` translation

```nomx
the data book_config is
  intended to hold the mdBook build configuration for the helix manual.

  exposes book_authors as text list.
  exposes book_language as text.
  exposes book_src as path.
  exposes html_cname as text.
  exposes html_default_theme as text.
  exposes html_preferred_dark_theme as text.
  exposes html_git_repository_url as text.
  exposes html_edit_url_template as text.
  exposes html_additional_css as text list.
  exposes search_use_boolean_and as boolean.

  favor correctness.
  favor documentation.
```

### Gaps surfaced

1. **Nested sections (`[output.html.search]`)** — TOML's dot-path sections have no v1/v2 analog. Flat-flattening (as v2 does) is lossy. Candidate: **W17 nested-record-path syntax** or a convention "dot-path becomes underscore-joined identifier".
2. **Config-as-data vs. config-as-code** — a `.toml` is pure data; v1 attempts to wrap it inside a `record`+assignment block. Distinguishing "declare a schema" from "assign values" is under-specified in Nom. Candidate: **authoring-guide — config literals get a dedicated `the data <name> for <repo> is { ... }` form.**
3. **Non-ASCII author names (`Blaž Hrastnik`)** — string content is UTF-8, but the lexer has the ASCII-identifier restriction for tokens. Confirm string literals keep their UTF-8 verbatim. Smoke-test candidate.
4. **Keys with hyphens (`default-theme`, `edit-url-template`)** — Nom identifiers use underscores. Mapping rule: TOML hyphen-keys become Nom underscore-exposes. Authoring-guide rule.

---

## 13. `acompress_documents` — Python async bridge

**Source:** [langchain-master/libs/core/langchain_core/documents/compressor.py:55-74](../../../APP/Accelworld/upstreams/langchain-master/libs/core/langchain_core/documents/compressor.py#L55)

```python
async def acompress_documents(
    self,
    documents: Sequence[Document],
    query: str,
    callbacks: Callbacks | None = None,
) -> Sequence[Document]:
    """Async compress retrieved documents given the query context."""
    return await run_in_executor(
        None, self.compress_documents, documents, query, callbacks
    )
```

### `.nomx v1` translation

```nomx
define acompress_documents
  that takes documents, query, and callbacks,
  returns a sequence of documents,
  as an asynchronous operation.
  the callbacks is perhaps nothing.
the compressed is the result of running compress_documents
  on documents, query, and callbacks, in an executor.
acompress_documents returns the compressed.
```

### `.nomx v2` translation

```nomx
the function acompress_documents is
  intended to asynchronously compress a batch of retrieved documents
  using the synchronous compress_documents primitive in an executor.

  uses the @Function matching "run synchronous function in an executor" with at-least 0.9 confidence.
  uses the @Function matching "compress retrieved documents" with at-least 0.85 confidence.

  requires documents is non-empty.
  ensures result is the same length and order as documents.

  favor correctness.
  favor performance.
```

### Gaps surfaced

1. **Async marker** — `as an asynchronous operation` is a placeholder phrasing. No Nom primitive exists for asynchronous execution contexts. Candidate: **W19 async-marker clause** (`a function that is asynchronous, returns ...` or `the asynchronous function X is …`).
2. **`await` expression** — `the result of running X in an executor` is prose; real async code needs a first-class await form. Possibly subsumed by W19.
3. **Executor / runtime metadata** — Python's `run_in_executor(None, ...)` wraps a sync call as async. Nom could express this via a typed effect clause `hazard thread_pool_switch` once effect-to-runtime mapping lands (doc 11). Authoring-corpus seed.
4. **Default parameter values** (`callbacks: Callbacks | None = None`) — Nom v1 uses `perhaps nothing`; v2 has no explicit "default = X" syntax. Authoring-guide rule: declare defaults inside `intended to …` prose, not as a type-system feature (follows the data-over-grammar principle from §I13).
5. **`Sequence[Document]` generic type** — `a sequence of documents` works at v1; v2 implicitly handles this via `matching "compress retrieved documents"`. No new wedge needed — generic-type lowering is a doc 04 §borrow-model concern deferred with lifetime annotations (doc 16 row #11).

---

## 14. `flat_map` — pure functional pattern (sampled)

Representing functional map-and-flatten, which shows up in every modern codebase (`flatMap` in Kotlin / Rust's `Iterator::flat_map` / Python's itertools).

```rust
pub fn flat_map<T, U, I, F>(input: I, f: F) -> Vec<U>
where
    I: IntoIterator<Item = T>,
    F: FnMut(T) -> Vec<U>,
{
    input.into_iter().flat_map(f).collect()
}
```

### `.nomx v1` translation

```nomx
define flat_map
  that takes an input sequence and a function from item to a sub-sequence,
  returns a flat sequence.
for each item in the input sequence,
  append every element of the function applied to item
  to the result.
```

### `.nomx v2` translation

```nomx
the function flat_map is
  intended to apply a function that returns a sub-sequence to every item
  in the input sequence and concatenate all the sub-sequences in order.

  uses the @Function matching "apply function to each item" with at-least 0.9 confidence.
  uses the @Function matching "concatenate sequences" with at-least 0.9 confidence.

  requires the function is deterministic on each item.
  ensures the order of output elements preserves input order
    and sub-sequence order for each input item.

  favor correctness.
  favor performance.
```

### Gaps surfaced

1. **Higher-order parameter** — `a function from item to a sub-sequence` is an anonymous callback, resolved per [doc 19 §D2](19-deferred-design-decisions.md) (lift to a named entity rather than inline closure). Translation confirms the D2 rule handles real-world `flat_map` shapes.
2. **Generic parameters (`T`, `U`)** — Rust's type-variables vanish at the v1 prose layer; v2 uses prose descriptors (`item`, `sub-sequence`) that the resolver grounds via the matching clause. Deferred with lifetime annotations (doc 16 row #11, blocked).
3. **Iterator vs. concrete `Vec<U>`** — translation hides the iterator-vs-materialized distinction behind `sequence`. Authoring-guide rule: **sequences are lazily evaluated by default; force materialization explicitly** (`collect the sequence into a vector` idiom).

---

## 15. SQL view — declarative query

Representing a declarative SQL view (pattern sampled from typical `CREATE VIEW`):

```sql
CREATE VIEW active_users AS
SELECT u.id, u.name, u.email
FROM users u
WHERE u.deleted_at IS NULL
  AND u.last_login_at > NOW() - INTERVAL '30 days';
```

### `.nomx v1` translation

```nomx
record active_users that holds the id, name, and email
  of every user whose deleted_at is nothing
  and whose last_login_at is within the last 30 days.
```

### `.nomx v2` translation

```nomx
the data active_users is
  intended to project the users table down to id + name + email
  filtered to those who haven't been soft-deleted and have logged in recently.

  uses the @Function matching "project table columns" with at-least 0.85 confidence.
  uses the @Function matching "filter rows by predicate" with at-least 0.85 confidence.

  exposes id as integer.
  exposes name as text.
  exposes email as text.

  requires the users table is available.
  ensures every row has non-null deleted_at absent and last_login_at within 30 days.

  favor correctness.
```

### Gaps surfaced

1. **Relational operators** — projection (`SELECT columns`) and selection (`WHERE predicate`) are first-class SQL concepts. Nom describes them via `uses the @Function matching "..."` prose, which works but loses the algebraic shape. Candidate: **W20 relational-algebra keywords** (`project X from Y`, `where predicate holds`).
2. **Time predicates (`NOW() - INTERVAL '30 days'`)** — no Nom primitive for relative-time ranges. Authoring-corpus seed: `within the last N days` idiom.
3. **NULL-as-sentinel vs. `nothing`** — SQL's `IS NULL` maps to doc 17 §I1's `is nothing` predicate. Confirmed: the existing idiom handles this cleanly.
4. **View-as-data vs. view-as-query** — A SQL view is both a data shape AND a query. Nom's `data` kind is data-only per doc 17 §I13. Suggestion: translate as `data` + separate internal query function; doc 17 rule preserved.

---

## 16. CSS rule — stylesheet selector + properties

```css
.button--primary {
  background-color: #0366d6;
  color: white;
  padding: 8px 16px;
  border-radius: 4px;
  cursor: pointer;
}
```

### `.nomx v1` translation

```nomx
record button_primary_style that holds
  a background_color of "#0366d6",
  a color of "white",
  a padding of "8px 16px",
  a border_radius of "4px",
  a cursor of "pointer".
```

### `.nomx v2` translation

```nomx
the data button_primary_style is
  intended to carry the visual styling for primary-action buttons.

  exposes background_color as text.
  exposes color as text.
  exposes padding as text.
  exposes border_radius as text.
  exposes cursor as text.

  favor documentation.

the button_primary_style for the default_theme is
  background_color is "#0366d6",
  color is "white",
  padding is "8px 16px",
  border_radius is "4px",
  cursor is "pointer".
```

### Gaps surfaced

1. **Selector vs. payload split** — CSS conflates "which elements does this apply to" (selector) and "what styling" (payload). Nom's `data` kind captures the payload cleanly; the selector (`.button--primary`) needs a separate concept expressing **when this style applies**. Candidate: **W21 selector-predicate clause** on `data` instances (e.g., `applies when element has class primary-button`).
2. **Kebab-case identifiers (`background-color`)** — doc 17 §I5's underscore-mapping rule handles the translation mechanically (`background_color`). Confirmed.
3. **Typed dimensions (`8px`, `4px`)** — Nom stores these as `text` today; a future `@Dimension` kind would catch unit errors. Candidate: **W22 typed dimension literals.**
4. **Hex color literals (`#0366d6`)** — stored as `text`; a `@Color` kind or colour-literal grammar would validate at parse time. Candidate: **W23 color literal grammar.**

---

## 17. GraphQL type — schema with non-null + list modifiers

Representing a typical GraphQL schema fragment:

```graphql
type Post {
  id: ID!
  title: String!
  body: String!
  tags: [String!]!
  author: User
  publishedAt: DateTime
}
```

### `.nomx v1` translation

```nomx
record Post that holds
  an id that is an identifier,
  a title that is text,
  a body that is text,
  a tags that is a list of text,
  an author that is perhaps a User,
  a published_at that is perhaps a timestamp.
the id, title, body, and tags are required.
the author and published_at may be nothing.
```

### `.nomx v2` translation

```nomx
the data Post is
  intended to represent a published blog post with metadata.

  exposes id as identifier.
  exposes title as text.
  exposes body as text.
  exposes tags as text list.
  exposes author as perhaps User.
  exposes published_at as perhaps timestamp.

  favor correctness.
  favor documentation.
```

### Gaps surfaced

1. **Non-null-by-default inversion** — GraphQL's `!` marks required; Nom's `perhaps` marks optional. The inversion is conceptually cleaner (required is the default; optional is an explicit opt-in), but translation requires inverting every nullability-marker. Authoring-guide rule confirmed from doc 17 §I1.
2. **`ID!` scalar** — GraphQL has distinguishable `ID` vs `String` scalars; Nom today would map both to `text` or `identifier`. Candidate: reserve `identifier` as a distinct shape in the data-type vocabulary. Overlaps with **W22 typed literals.**
3. **`[String!]!` list-of-non-null` vs `[String]`** — two axes of nullability per collection (collection-itself + each element). Nom's `text list` is non-null-by-default at both axes; the permissive variants would need explicit phrasing. Candidate: **W24 nested nullability modifiers** (`list of perhaps text`, `perhaps list of text`, `perhaps list of perhaps text`).
4. **GraphQL `type Post implements Node { … }`** — interface implementation has no Nom analog yet. The `extends` keyword in concept grammar is closest but serves a different purpose (concept composition). Candidate: defer; might become relevant when interface-style subtyping enters the language.
5. **Custom scalar `DateTime`** — the authoring corpus needs canonical time/date primitives. Similar to translation #15's time predicates. Consolidate under the time-range idiom authoring-guide row.

---

## 18. Makefile rule — target + prereqs + recipe

```makefile
build: src/main.c src/util.c
	gcc -O2 -Wall -o build/app src/main.c src/util.c

clean:
	rm -rf build/
.PHONY: build clean
```

### `.nomx v1` translation

```nomx
define build_app_binary
  that takes nothing, returns nothing.
  require every source file under src/ exists.
  compile src/main.c and src/util.c
    with gcc using level 2 optimization and all warnings,
    producing build/app.

define clean_build_output
  that takes nothing, returns nothing.
  delete the build folder recursively.
```

### `.nomx v2` translation

```nomx
the function build_app_binary is
  intended to compile the C sources into a single app binary
  under the build folder with optimization level 2 and all warnings on.

  uses the @Function matching "run c compiler" with at-least 0.9 confidence.

  requires every source file under src/ exists.
  ensures build/app exists and is executable.

  favor correctness.
  favor performance.

the function clean_build_output is
  intended to remove all build artifacts.

  uses the @Function matching "delete directory recursively" with at-least 0.9 confidence.

  ensures build/ folder is absent after invocation.
  hazard data_loss.

  favor correctness.
```

### Gaps surfaced

1. **Prereq declarations (`build: src/main.c src/util.c`)** — Make's dependency graph has no Nom analog; the prose `require every source file under src/ exists` is weaker (it only asserts presence, not "rebuild when they change"). Candidate: **W25 build-time dependency graph** (requires integration with `nom build` incremental model).
2. **Shell commands in recipes (`gcc -O2 -Wall -o build/app …`)** — Nom translation punts to `uses the @Function matching "run c compiler"`. The authoring-corpus needs a canonical "shell exec" primitive with arg + stdout semantics. Authoring-corpus seed confirmed.
3. **`.PHONY` targets** — Make's metadata that a target is not a file. Nom has no analog; `intended to` prose disambiguates implicitly. Translation note, not a wedge.
4. **`hazard data_loss`** — this is the right valence for `rm -rf`. Confirmed the effect system already expresses destructive operations cleanly.

---

## 19. Dockerfile — base image + ordered directives

```dockerfile
FROM rust:1.82-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/myapp /usr/local/bin/
ENTRYPOINT ["/usr/local/bin/myapp"]
```

### `.nomx v1` translation

```nomx
define build_release_image
  that takes nothing, returns nothing.
  start with a base image of rust version 1.82 in slim variant.
  the working_directory is /app.
  copy Cargo.toml and Cargo.lock into the working_directory.
  copy the src folder into the working_directory.
  run cargo build release.

define pack_runtime_image
  that takes the release binary path, returns nothing.
  start with a base image of debian bookworm slim.
  copy the release binary into /usr/local/bin/.
  set the entrypoint to /usr/local/bin/myapp.
```

### `.nomx v2` translation

```nomx
the function build_release_image is
  intended to compile the Rust source tree into a release binary
  inside a containerized build stage.

  uses the @Function matching "select base image" with at-least 0.9 confidence.
  uses the @Function matching "copy path into container layer" with at-least 0.85 confidence.
  uses the @Function matching "run cargo build release" with at-least 0.9 confidence.

  requires Cargo.toml and the src folder exist at the host root.
  ensures /app/target/release/myapp exists after invocation.

  favor correctness.
  favor performance.

the function pack_runtime_image is
  intended to assemble a minimal runtime image carrying only
  the compiled app binary.

  uses the @Function matching "select base image" with at-least 0.9 confidence.
  uses the @Function matching "copy path from previous stage" with at-least 0.85 confidence.
  uses the @Function matching "set container entrypoint" with at-least 0.9 confidence.

  requires the builder stage produced /app/target/release/myapp.
  ensures the final image runs the app on container start.

  favor correctness.
```

### Gaps surfaced

1. **Multi-stage builds (`AS builder` then `FROM debian …`)** — Dockerfile's stage graph is linear but named; each stage has its own filesystem. Translation splits into two functions + a shared understanding that `pack_runtime_image` depends on `build_release_image`'s output. The dependency edge is implicit in the prose. Candidate: **W26 stage-chain declarations** or reuse `then` composition from `.nomtu` module syntax.
2. **`COPY --from=builder`** — cross-stage copy. Unique to Dockerfile; translation punts. Likely subsumed by W26.
3. **Image tags (`rust:1.82-slim`, `debian:bookworm-slim`)** — hyphenated identifiers that represent registry coordinates. Maps to a `@Data matching "container base image"` typed-slot ref with a canonical naming scheme.
4. **`ENTRYPOINT ["..."]` as a JSON-array directive** — Dockerfile mixes declarative + imperative syntax in one file. Nom's translation separates the concerns (two functions for two stages). Authoring-guide confirmed: container directives map to function-kind entities, not data-kind.

---

## 20. GitHub Actions workflow — YAML with logic

Representing a typical CI workflow (mixing declarative structure + shell-exec payloads):

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --all
        env:
          RUST_BACKTRACE: "1"
```

### `.nomx v1` translation

```nomx
record ci_workflow that holds
  a name "CI",
  a triggers section listing push to main and pull_request,
  a jobs section with one job named test.

the test job
  runs on ubuntu-latest,
  checks out the repository at version 4 of the checkout action,
  installs the stable rust toolchain via actions-rs at version 1,
  runs `cargo test --all` with RUST_BACKTRACE set to "1".
```

### `.nomx v2` translation

```nomx
the data ci_workflow is
  intended to run the test suite on every push to main and every pull request.

  exposes name as text.
  exposes triggers as event list.
  exposes jobs as job map.

  favor correctness.

the function run_ci_test_job is
  intended to run `cargo test --all` after checking out the repo
  and installing the stable rust toolchain.

  uses the @Function matching "checkout repository action" with at-least 0.9 confidence.
  uses the @Function matching "install rust toolchain action" with at-least 0.9 confidence.
  uses the @Function matching "run shell command" with at-least 0.9 confidence.

  requires ubuntu-latest runner is available.
  ensures all tests pass before the job succeeds.

  favor correctness.
  favor performance.
```

### Gaps surfaced

1. **YAML's data+logic mix** — the workflow file is mostly data (structure, triggers, job map) but `steps.run` and `with.toolchain` parameters embed logic. Nom's translation splits into a `data` schema (the config shell) plus `function` entities (the per-job actions). Confirms doc 17 §I13 split rule scales to CI configs.
2. **Third-party "actions" references (`actions/checkout@v4`)** — version-pinned external tool invocations. Maps to `@Function matching "checkout repository action" with at-least N confidence` plus corpus-registered action vocabulary. Overlaps with doc 16 #45 (identifier shape). Candidate: **W27 pinned-external-action ref grammar** if this recurs.
3. **Environment variable injection (`env: RUST_BACKTRACE: "1"`)** — already queued as **W16** (env-var access). Confirmed this is a real authoring surface.
4. **Event-triggered execution model** — CI workflow's `on:` section describes WHAT triggers execution, not what to run. Nom has no first-class event-trigger primitive today. Candidate: **W28 event-trigger declarations** (`runs when X happens`).
5. **Matrix / strategy clauses** — not in this minimal example but common in CI. Defer to authoring-corpus seed for future translations.

---

## 21. Java class — interface + builder pattern

Representing a typical immutable-record Java class with a builder:

```java
public final class User {
    private final String id;
    private final String email;
    private final Instant createdAt;

    private User(Builder b) {
        this.id = b.id;
        this.email = b.email;
        this.createdAt = b.createdAt;
    }

    public String getId() { return id; }
    public String getEmail() { return email; }
    public Instant getCreatedAt() { return createdAt; }

    public static class Builder {
        private String id;
        private String email;
        private Instant createdAt;

        public Builder id(String id) { this.id = id; return this; }
        public Builder email(String email) { this.email = email; return this; }
        public Builder createdAt(Instant createdAt) { this.createdAt = createdAt; return this; }
        public User build() { return new User(this); }
    }
}
```

### `.nomx v1` translation

```nomx
record User that holds
  an id that is text,
  an email that is text,
  a created_at that is a timestamp.
every field is set at construction and cannot change afterward.

define build_user
  that takes an id, an email, and a created_at, returns a User.
the result is a new User with id, email, and created_at as given.
```

### `.nomx v2` translation

```nomx
the data User is
  intended to represent an immutable user record with id, email, and creation time.

  exposes id as text.
  exposes email as text.
  exposes created_at as timestamp.

  favor correctness.
  favor documentation.

the function build_user is
  intended to construct a User from the three required fields.

  uses the @Function matching "assemble immutable record" with at-least 0.85 confidence.

  requires id is non-empty.
  requires email is a well-formed address.
  ensures every field of the returned User matches the input.

  favor correctness.
```

### Gaps surfaced

1. **Builder pattern dissolves** — Java's Builder + `build()` exists because the language lacks named arguments and default values. Nom's v2 form uses `uses the @Function matching "assemble immutable record"` and the v1 form calls the constructor directly with `takes an id, an email, and a created_at`. **The Builder pattern is a non-concept in Nom** — translations should drop it, not preserve it. Authoring-guide rule confirmed from doc 19 §D2's logic (lift implementation patterns to named entities).
2. **Access modifiers (`public final` / `private`)** — Nom has no visibility markers today; every entity is accessible by name. Candidate: **W29 visibility modifiers** (probably just `private` for scope hiding, since `public` is the default).
3. **Nested class `Builder`** — Java's inner-class scoping has no analog. Translated as a peer entity (`build_user`) that the caller invokes directly. Scoping-by-entity-name is the Nom convention.
4. **Getters (`getId()` etc.)** — boilerplate that Nom's `exposes` covers automatically. Data kind auto-projects readable fields; no method declarations needed. Confirms doc 17 §I13.
5. **`final` immutability** — Nom data kind's default is immutable; the `final` annotation is implicit. No wedge.

---

## 22. Kotlin sealed class + data class — algebraic data types

Representing Kotlin's common sum-of-products shape (HTTP result as a `Result<T>` variant):

```kotlin
sealed class HttpResult<out T> {
    data class Ok<T>(val value: T, val latencyMs: Long) : HttpResult<T>()
    data class Error(val code: Int, val message: String) : HttpResult<Nothing>()
    object Timeout : HttpResult<Nothing>()
}
```

### `.nomx v1` translation

```nomx
choice HttpResult that is one of
  a ok variant that holds a value and a latency_ms,
  an error variant that holds a code and a message,
  a timeout variant that holds nothing.
```

### `.nomx v2` translation

```nomx
the data HttpResult is
  intended to represent the three outcomes of an HTTP request:
  success with a value, a server-side error, or a client-side timeout.

  exposes kind as text one of ("ok", "error", "timeout").
  exposes value as perhaps any.
  exposes latency_ms as perhaps integer.
  exposes code as perhaps integer.
  exposes message as perhaps text.

  favor correctness.
  favor documentation.
```

### Gaps surfaced

1. **`choice` / sealed sum-type declaration** — the v1 form `choice HttpResult that is one of …` cleanly expresses the algebraic-data-type shape that Kotlin's `sealed class` + three `data class` subtypes encode. Grammar already lists `choice` in doc 05 §4.1's declaration-verb set but the parser doesn't yet accept it. Candidate: **W30 `choice` grammar** (promotes row from `.nomx v1` prose-level to first-class). Overlaps with **W11** (enum/variant declarations); merge into a single wedge.
2. **Variant-associated data** — `data class Ok<T>(val value: T, ...)` couples fields to the variant tag. Nom's v2 flattens to nullable fields + a `kind` discriminator; v1 keeps the variant+fields structure. Authoring-guide rule: prefer flat data shape in v2 unless the variants have disjoint field sets (in which case split into separate concepts).
3. **Type parameter `<T>` / `<out T>`** — generics, again. Blocked on borrow-model work (doc 16 row #11).
4. **`object Timeout`** — Kotlin's singleton object. Translates to a variant with no fields (`a timeout variant that holds nothing`). Zero wedge cost — composition of already-known primitives.
5. **Exhaustive pattern match** — Kotlin's `when(result) { is Ok -> … }` requires all variants be handled. Nom's `matches against every variant` grammar candidate for when `choice` actually parses. **Part of W30.**

---

## 23. Ruby method with block — implicit-callback pattern

```ruby
class Cache
  def with_retry(max_attempts = 3)
    attempts = 0
    loop do
      attempts += 1
      begin
        return yield
      rescue StandardError => e
        raise e if attempts >= max_attempts
        sleep(attempts * 0.5)
      end
    end
  end
end

result = cache.with_retry(5) { fetch_from_upstream(url) }
```

### `.nomx v1` translation

```nomx
define retry_with_backoff
  that takes a max_attempts and an attempt_action,
  returns whatever attempt_action returns.
  the attempts is 0.
  while true,
    the attempts is attempts plus 1.
    when attempt_action succeeds, return its result.
    when attempt_action raises an error,
      when attempts is at least max_attempts, propagate the error.
      otherwise, sleep for attempts times 0.5 seconds.
```

### `.nomx v2` translation

```nomx
the function retry_with_backoff is
  intended to invoke an action repeatedly with linear backoff
  until it succeeds or the attempt limit is reached.

  uses the @Function matching "invoke action with fallible result" with at-least 0.9 confidence.
  uses the @Function matching "sleep for seconds" with at-least 0.9 confidence.

  requires max_attempts is at least 1.
  ensures the returned value is the action's success value
    or propagates the last error after max_attempts failures.
  hazard rate_limited.

  favor correctness.
```

And the call site:

```nomx
define fetch_with_cache
  that takes a url, returns perhaps content.
the content is the result of retry_with_backoff
  with max_attempts 5 and attempt_action fetch_from_upstream.
```

### Gaps surfaced

1. **Implicit blocks (`yield`, `{ … }`)** — Ruby's block argument is anonymous and positional. Nom translation lifts the block to a named function per doc 19 §D2 (e.g., `fetch_from_upstream`) and passes it as an ordinary parameter. **D2 confirmed for Ruby blocks too.**
2. **`begin/rescue` → when-clause** — Nom's prose handles exception flow via `when X raises an error, …` / `propagate the error` / `when attempts is at least max_attempts, …`. No new grammar needed; closed under existing conditional prose.
3. **Default parameter (`max_attempts = 3`)** — already queued in doc 16 row #37 (authoring-guide rule: declare defaults in `intended to …` prose). Ruby's common convention confirms this needs an explicit authoring-guide entry.
4. **`sleep(attempts * 0.5)` arithmetic in args** — simple numeric expression; v1 prose `sleep for attempts times 0.5 seconds` captures it. Confirms doc 17 §I11's list/text accessor primitives generalize to arithmetic. No new wedge.
5. **Class scope (`class Cache`)** — Ruby classes are namespaces holding methods. Nom's flat entity space has no class analog; the method becomes a top-level `function` that takes the cache as a parameter. Authoring-guide confirmation: **lift methods into functions that take the receiver explicitly.**

---

## 24. Go goroutine + channel — concurrent fan-out

```go
func fetchAll(urls []string) []Result {
    results := make(chan Result, len(urls))
    var wg sync.WaitGroup
    for _, url := range urls {
        wg.Add(1)
        go func(u string) {
            defer wg.Done()
            results <- fetch(u)
        }(url)
    }
    wg.Wait()
    close(results)
    out := make([]Result, 0, len(urls))
    for r := range results {
        out = append(out, r)
    }
    return out
}
```

### `.nomx v1` translation

```nomx
define fetch_all
  that takes a list of urls, returns a list of results.
  require every url in the list is well-formed.
the results_channel is a channel of results with capacity len(urls).
the work_group tracks active workers.
for each url in urls,
  add one worker to work_group.
  start a goroutine that
    when the goroutine finishes, remove its worker from work_group.
    the result is fetch of the url.
    send result into results_channel.
wait for work_group to drain.
close results_channel.
the out is an empty list of results with capacity len(urls).
for each r received from results_channel,
  append r to out.
return out.
```

### `.nomx v2` translation

```nomx
the function fetch_all is
  intended to fetch every url concurrently and collect the results
  into a single list in no particular order.

  uses the @Function matching "fetch single url" with at-least 0.85 confidence.
  uses the @Function matching "spawn concurrent worker" with at-least 0.85 confidence.
  uses the @Function matching "wait for all workers to finish" with at-least 0.85 confidence.

  requires every url is well-formed.
  ensures the result list has exactly one entry per input url
    and contains no duplicates.
  hazard thread_contention.

  favor correctness.
  favor performance.
```

### Gaps surfaced

1. **Goroutine spawn (`go func(u) { … }(url)`)** — concurrent worker launch. Nom's v2 abstracts this behind `uses the @Function matching "spawn concurrent worker"`. The v1 form uses `start a goroutine that …` which is prose; no first-class `spawn` keyword exists yet. Candidate: **W31 concurrent-spawn clause** (`start a worker that …`).
2. **Channels as buffered queues** — Go's `make(chan Result, N)` creates a bounded queue. Nom translation describes it as `a channel of results with capacity N`. Candidate: **W32 channel-type grammar** with capacity annotation.
3. **`defer wg.Done()` → finalizer semantics** — Go's defer runs at function exit. Nom has no defer analog today. Translation inlines it as "when the goroutine finishes, …". Candidate: **W33 finalizer clause** or stick with prose.
4. **Range-over-channel (`for r := range results_channel`)** — consumer loop that stops when channel closes. Translates to `for each r received from results_channel`. Maps to existing v1 iteration prose; no new wedge.
5. **`sync.WaitGroup` / `Add` / `Wait`** — worker-tracking primitive. Nom's prose `work_group tracks active workers` / `wait for work_group to drain` captures it. Could be merged with doc 17 §I9 atomic primitives; confirm whether authoring-guide needs a `work_group` idiom addition.

---

## 25. Haskell — pure function with type class constraint

```haskell
groupBy :: Ord k => (a -> k) -> [a] -> Map k [a]
groupBy keyOf xs = foldr insert Map.empty xs
  where
    insert x m = Map.insertWith (++) (keyOf x) [x] m
```

### `.nomx v1` translation

```nomx
define group_by
  that takes a key_of function and a list of items,
  returns a map from keys to lists of items.
  require keys are orderable.
the result is built by folding insert into the empty map,
  where insert adds an item to the list under its key.
```

### `.nomx v2` translation

```nomx
the function group_by is
  intended to partition a list into groups keyed by a
  function applied to each item.

  uses the @Function matching "fold list into accumulator" with at-least 0.9 confidence.
  uses the @Function matching "insert into map with merge" with at-least 0.85 confidence.

  requires the key_of function is deterministic on each item.
  requires keys support ordering.
  ensures every input item appears exactly once in exactly one group.
  ensures within a group, items preserve their input order.

  favor correctness.
```

### Gaps surfaced

1. **Type-class constraint (`Ord k =>`)** — Haskell's typeclass bounds are an algebraic structure over types. Nom's `requires keys support ordering` captures the intent as a runtime contract, not a static guarantee. Candidate: **W34 typeclass-style constraints** (`requires <Type> supports <Operation>`). Overlaps with borrow-model row #11 (blocked); revisit when generics land.
2. **Fold / higher-order iteration** — `foldr insert Map.empty xs` is classic HOF. Nom's v2 abstracts as `uses the @Function matching "fold list into accumulator"`; the v1 form uses prose `the result is built by folding insert into the empty map, where insert …` — matches doc 17 §I8's named-intermediate rule. No new wedge needed.
3. **`where` clause for local helpers** — Haskell's `where` defines helpers scoped to the function. Nom lifts `insert` to a named peer entity or a `the <name> is <expr>` intermediate per doc 17 §I8. Confirmed: D2 closure-lift handles this.
4. **Map type from another module (`Map.empty`, `Map.insertWith`)** — module-prefixed identifiers. Nom translations punt these to `@Function matching "…"` typed-slot refs; the concrete `Map` module reference lives in the corpus. No new wedge; module-imports unblocked elsewhere in §5.10.

---

## 26. Lean 4 theorem — dependent types + proof

Representing a typical formal statement + proof:

```lean
theorem add_comm (a b : Nat) : a + b = b + a := by
  induction b with
  | zero => rfl
  | succ k ih => simp [Nat.add_succ, ih]
```

### `.nomx v1` translation

```nomx
theorem add_comm
  that asserts for every natural a and b, a plus b equals b plus a.
proof by induction on b:
  base case when b is zero, both sides reduce to a.
  inductive step when b is the successor of k with hypothesis ih:
    simplify using add_succ and ih.
```

### `.nomx v2` translation

```nomx
the proof add_comm is
  intended to assert that natural addition is commutative:
  for every naturals a and b, a plus b equals b plus a.

  uses the @Function matching "induct on natural argument" with at-least 0.9 confidence.
  uses the @Function matching "rewrite using hypothesis" with at-least 0.85 confidence.

  requires a and b are natural numbers.
  ensures the equality a + b = b + a is established.

  favor correctness.
```

### Gaps surfaced

1. **New kind `@Proof` / `theorem` declaration** — Nom's closed KINDS set (7 nouns) does NOT include `theorem`/`proof`/`lemma` today. Candidate: **W35 proof-kind** — possibly a subtype of `function` where the signature is `(args...) → Equality/Proposition`, captured via the `requires`/`ensures` contract pair, OR a new closed-kind entry. Deferred 11 §B argues math is a first-class citizen; this decision needs explicit direction (doc 15-level planning).
2. **Dependent types (`a b : Nat`)** — universe-of-types as values. Blocked alongside row #11 (lifetime annotations) + W34 typeclasses. Unblocks together when the borrow/type-system lane starts.
3. **Tactic language (`by induction b with | zero => rfl`)** — proof is a program in a different language layered onto the theorem. Nom's translation flattens the tactic script into prose (`proof by induction on b`). Candidate: **W36 proof-tactic DSL** — very long-tail, defer until Nom has any proof-checking infrastructure.
4. **Universe cumulativity / elaboration** — Lean's type-universe polymorphism. Deep-proof-system concern; punt until math-library integration becomes a real milestone.
5. **`rfl` / `simp` / named lemmas** — proof-search primitives. Each maps to a named entity in Nom's corpus (`refl`, `simplify`, `Nat.add_succ`). The `uses the @Function matching "..."` abstraction handles this cleanly once the proof-kind decision lands.

---

## Running gap list → migrated to doc 16

As of commit following `370f96d`, the 35-gap list has been promoted to its
own document: [16-nomx-syntax-gap-backlog.md](./16-nomx-syntax-gap-backlog.md).
It organizes gaps by destination (wedge, authoring-guide, deferred), tracks
status per row, and carries a wedge master-index. New gaps from this doc's
subsequent translations should be appended to doc 16 directly, with a single
back-reference added to each translation's "Gaps surfaced" section here.

## Next cycle plan

- **This cycle added translations #11-12**: `build-dex.sh` (Bash, accesskit) and `book.toml` (TOML, helix). 35 total gaps now; 3 new wedges queued (W15/W16/W17); 5 new authoring-guide items.
- Next cycle: promote gap list to a dedicated **doc 15b** now that 35 gaps crossed the 20+ threshold. (doc 15 stays focused on the 100-repo ingestion harness.)
- Alternative next-cycle targets: more translations (Lua, Makefile, JSON schema) or ship the first authoring-guide wedge (format-strings, exit-codes, list-accessors).
- Feed each translation into a nom-parser smoke test that asserts the canonical lexing + parse tree shape. First smoke-test candidate: example 3's `is_even` at both v1 and v2.
