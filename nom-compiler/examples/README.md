# Examples

Runnable fixtures covering the full authoring lifecycle: prose draft →
.nom or .nomx → compile → run. Grouped by authoring stage.

## Prose drafts (`.md`)

Input to `nom author translate <input> --target app` — each tests the
prose-extraction heuristic at a different complexity level.

| File | Complexity | What it exercises |
|------|-----------|-------------------|
| `draft_sentence.md` | 1 intent line | floor case (≥1 proposal) |
| `draft_paragraph.md` | 8 sketch bullets | ordinary shape (intent + flow) |
| `draft_todo_app.md` | 7 bullets | the canonical walkthrough |
| `draft_essay.md` | 13+ bullets, 3 sections | multi-section tracking |

Shape assertions are locked by `nom-cli::src::author.rs` tests:
proposal counts scale monotonically sentence < paragraph < essay,
and draft_todo_app.md yields ≥7 proposals across intent + sketch.

## Natural-language Nom (`.nomx`)

Input to `nom author check <file>` — parsed via the experimental
`.nomx` grammar track from
[research/language-analysis/05-natural-language-syntax.md](../../research/language-analysis/05-natural-language-syntax.md).

| File | Grammar used | Decls |
|------|------------|-------|
| `hello.nomx` | `define X that takes Y and returns Z:` block + Binding body | 1 |
| `todo_app.nomx` | `record` + `choice` + 3 `define` with `when`/`otherwise` | 5 |
| `greet_sentence.nomx` | `to X, respond with Y.` sentence form | 3 |
| `loops.nomx` | `for each` + `while` + nested `when`/`unless` | 4 |
| `contracts.nomx` | `require` / `ensure` / `throughout` contract verbs | 3 |

Parse gates are locked by `nom-parser::src::nomx.rs` tests (23
assertions including end-to-end parse of every sample).

## Classic Nom (`.nom`)

Pre-dates the `.nomx` track; still the production surface.

| File | What it shows |
|------|---------------|
| `hello.nom` | classic `fn main() { }` entry |
| `hello_llvm.nom` | LLVM-target variant |
| `imperative.nom` / `natural.nom` / `natural_pure.nom` | different styles |
| `auth.nom` / `webapi.nom` | realistic multi-decl programs |
| `run_lexer.nom` | self-host lexer driver, compiles to `.bc` + `.ll` |
| `best_practice.nom` | the Phase-4 demo |
| `custom_word.nom` | first-class nomtu reference |
| `test_auth.nom` | test-case example |

Compile + run:

```sh
nom build path/to/file.nom
./path/to/file
```

## Full authoring loop

```sh
# 1. Draft a sentence or paragraph.
cp examples/draft_sentence.md drafts/my_app.md
$EDITOR drafts/my_app.md

# 2. Extract proposals + materialize into a dict.
nom author translate drafts/my_app.md --target app --write ./data

# 3. Author the Nom (classic .nom or natural .nomx).
$EDITOR drafts/my_app.nomx

# 4. Validate the parse.
nom author check drafts/my_app.nomx

# 5. (Future) Compile.
# nom build drafts/my_app.nomx
```
