# Natural-Language Syntax (≥95%) — Grammar Redesign Proposal

> **Last verified against codebase: 2026-04-13, HEAD `afc6228`.**
> `.nomx v1` (this doc's grammar) and `.nomx v2 (keyed)` (doc 07's grammar) **coexist**.
> Doc 07 v2 is fully shipped per commits `97c836f` + `c405d2a` + `853e70b` + `c9d1835`.
> The `.nomx v1` parser lives at `nom-compiler/crates/nom-parser/src/nomx.rs`.
> The doc-08 layered parser at `nom-compiler/crates/nom-concept/src/lib.rs` is a **separate parser**
> for the `.nom`/`.nomtu` tier-1/tier-2 architecture — not a successor to `.nomx v1`.

Status: **Draft, needs human authoring**. Filed 2026-04-13 in response to
user directive: "make the Nom language similar to Natural language 95%…
it still too similar to syntax of C or python."

## 1. The tension

Two non-negotiable requirements pull in opposite directions:

1. **Prose readability (≥95%).** A non-programmer reading a `.nom`
   file should parse it the same way they parse a well-written spec.
2. **Zero execution mistakes.** No null, no data race, no silent
   overflow, no aliasing bugs, no partial function panics. The set of
   bugs the language structurally prevents must be **strictly larger**
   than what Rust/Haskell/Swift guarantee today.

Most languages that chase (1) sacrifice (2) (AppleScript, HyperTalk,
COBOL). Most that chase (2) don't chase (1) (Rust, Haskell). Nom has to
do both.

## 2. Current surface is too C-like

```nom
fn greet(name: text) -> text {
  return "hello " + name
}
```

Keywords `fn`, `return`, braces, arrow return type, colon-typed
parameters, concat-with-plus — every single one is a programmer
convention. A non-programmer must learn five conventions before the
single idea ("greet someone") surfaces.

## 3. Target surface

```nom
define greet that takes a name and returns a greeting:
  the greeting is "hello " followed by the name
```

Or, for one-liners:

```nom
to greet someone by name, respond with "hello" joined to their name.
```

Both parse into the same AST. The second is a **sentence form**; the
first is a **definition form**.

## 4. Design principles

### 4.1 Sentence-structured declarations

- `define <name> that <verb phrase>` replaces `fn name() -> Type`.
- `to <verb> <noun phrase>, respond with <expression>.` is the
  sentence form (implicit single-clause body, no `return`).
- `record <name> holds <field phrases>` replaces `struct`.
- `choice <name> is one of <options>` replaces `enum`.

### 4.2 Prepositions are operators

No `::`, `|>`, `->` where prose works:

- `the square of n` → `square(n)`
- `a greeting for the user` → `greeting(user)`
- `the name from the profile` → `profile.name`
- `apply filter then render` → `render(filter(...))`

### 4.3 Last sentence is the result

No `return` keyword. Block flow ends with the expression that
produces the value:

```nom
define tax on amount:
  the rate is 0.08
  the amount times the rate
```

### 4.4 Natural control flow

```nom
when the user is logged in, show the dashboard.
otherwise, show the landing page.
```

Parses into `if expr then block else block`. `unless` is syntactic
sugar for `when not`.

### 4.5 Types are phrases

- `a number` → Integer (arbitrary precision)
- `a 32-bit signed number` → i32
- `a piece of text` → Text
- `a list of names` → List<Text>
- `a maybe-number` → Option<Integer>
- `either a number or an error` → Result<Integer, Error>

The parser canonicalizes phrase → type; multiple phrases can map
to the same type (learnability without ambiguity — the canonicalizer
rejects ambiguous phrasing).

### 4.6 Concepts as proper nouns

`The User Profile` is a concept (capitalized, multi-word).
`user_profile` is a nomtu word. `use The User Profile` binds the
concept; `use user_profile@<hash>` binds the specific nomtu.

### 4.7 Contracts inline

```nom
define absolute-value of n:
  when given a negative n, this returns the positive of n.
  otherwise, return n.
  ensure the result is never negative.
```

The `when given …` + `ensure …` clauses parse into pre/post predicates.
Familiar to anyone who's written specs.

## 5. Zero-mistake guarantees

### 5.1 No null / undefined
Every binding has a value at its scope; optional values are typed
`a maybe-<X>` and force pattern matching before use.

### 5.2 No data races
Parallel-by-default with isolated actors. There's no shared-mutable-
state syntax — writes that cross actor boundaries go through
messages, which are values (immutable once sent).

### 5.3 No panics in user code
Every partial operation returns `either … or …` (Result). The parser
refuses to pass a `maybe-<X>` where an `<X>` is required, forcing
handling. Divide-by-zero, array-out-of-range, etc., are value-level
errors, not panics.

### 5.4 No integer overflow bugs
Integers are arbitrary-precision unless the type phrase says
otherwise (`a 32-bit signed number`). Sized-int overflow produces a
runtime Result::Err, not silent wrap.

### 5.5 No aliasing bugs
No references in surface syntax. Every binding is a value; the
compiler inserts borrows invisibly, proven safe via a linear-types
pass on the parsed AST.

### 5.6 No memory leaks
Structured scopes; values go out of scope as the sentence / paragraph
ends. No `Box::leak`, no `Rc` cycles in surface syntax.

## 6. Two-track migration

**Do NOT** drop-in replace the C-like surface. Ship both:

- Current surface stays on `.nom`.
- Natural-language surface ships as `.nomx` (or `.nom` with a
  `-- natural` pragma).
- Parser tries `.nomx` first when it sees the pragma; falls back to
  C-like.
- Lexer shared; grammar disjoint.
- Eventual migration: self-host scaffolds `planner.nom` / `codegen.nom`
  migrate to `.nomx` once the grammar stabilizes.

## 7. Milestones

| # | Deliverable | Proves | Status |
|---|-------------|--------|--------|
| 1 | `define … that …` + sentence form → same AST | Read-parity on trivial fns | ✅ SHIPPED (`nom-parser/src/nomx.rs`, commits `ca3fc97`–`b49430f`) |
| 2 | `when … otherwise …` branches | Read-parity on conditionals | ✅ SHIPPED (parser covers `when`/`unless`/`otherwise`) |
| 3 | `a number` / `a piece of text` canonicalization | Read-parity on types | ❌ NOT YET — tokens captured in `cond_tokens`/`rhs_tokens` but not canonicalized to types |
| 4 | `a maybe-<X>` + pattern-match gate | Zero-null guarantee | ❌ NOT YET — requires type system / expression tree |
| 5 | Parallel-by-default actors + message types | Zero-race guarantee | ❌ NOT YET — actor form not in parser |
| 6 | Inline contracts (`when given …, ensure …`) | Spec-level parity | ✅ SHIPPED (`require`/`ensure`/`throughout` in parser; `contracts.nomx` sample green) |
| 7 | Migrate planner.nom → planner.nomx | Self-host parity | ⏳ PLANNED — Phase 5+ (planner itself not yet ported) |

Each milestone includes a readability test: a randomly-selected
non-programmer must be able to describe what the code does after
one read, within 30 seconds, with <5% misinterpretation.

## 8. Open questions (needs user authoring)

- Vietnamese natural-language keyword aliases (lexer already has
  80+ aliases — extend to grammar level, so Vietnamese prose parses
  into the same AST)
- Ambiguity rules: when does "the name of the user" mean `user.name`
  vs. `name_of_user(...)`? Proposal: context-free in the grammar;
  resolved by type inference.
- Punctuation strictness: can the author drop the final `.`? How
  strict is sentence boundary detection? Proposal: periods required
  at sentence-form boundaries, optional inside blocks.
- Is `define` / `to` / `record` / `choice` the right keyword set, or
  should they be canonicalized from a wider phrase space (`let`,
  `make`, `name`, …)?

## 9. Side-by-side examples

Current Nom (C-like) on the left; proposed `.nomx` (natural) on the right.
Each pair produces the same AST after parsing.

### 9.1 A pure function with a contract

```nom                                      .nomx
fn absolute(n: integer) -> integer {        define absolute of a number n:
  require n != 0                              when given a negative n,
  if n < 0 {                                    the result is the positive of n.
    return 0 - n                              otherwise, the result is n.
  }                                           ensure the result is never negative.
  return n
  ensures result >= 0
}
```

### 9.2 A conditional render

```nom                                      .nomx
fn render(user: User) -> Page {             define render for a user:
  if user.logged_in {                         when the user is logged in,
    return dashboard(user)                      show the dashboard for the user.
  } else {                                    otherwise, show the landing page.
    return landing_page()
  }
}
```

### 9.3 A record + choice (struct + enum)

```nom                                      .nomx
struct User {                               record a user holds:
  name: text,                                 a piece of text called name.
  age: integer,                               a number called age.
  email: maybe<text>                          a maybe-text called email.
}

enum Status {                               choice a status is one of:
  Active,                                     active.
  Suspended(reason: text),                    suspended with a reason.
  Deleted                                     deleted.
}
```

### 9.4 A list operation

```nom                                      .nomx
fn sum(ns: list<integer>) -> integer {      define sum of a list of numbers ns:
  let mut total = 0                           the total starts at zero.
  for n in ns {                               for each n in ns, add n to the total.
    total = total + n                         the total.
  }
  return total
}
```

### 9.5 A concept + its nomtu

```nom                                      .nomx
concept authentication {                    The Authentication concept groups:
  use login_user@a1b2                         use login_user@a1b2.
  use logout_user@c3d4                        use logout_user@c3d4.
  use verify_token@e5f6                       use verify_token@e5f6.
}
```

### 9.6 An actor (parallel-by-default, per §5.2)

```nom                                      .nomx
actor counter {                             an actor called counter holds a number n,
  state: integer = 0                          starting at zero.
  on_message(add, v: integer) {               when it receives "add" with a value v,
    state = state + v                           n becomes n plus v.
  }                                           when it receives "current",
  on_message(current) -> integer {              respond with n.
    return state
  }
}
```

## 10. Implementation status (updated 2026-04-13, HEAD `afc6228`)

Prototype code has landed alongside this proposal. The `.nomx` track
coexists with the C-like grammar — no existing code paths touched.

**Note on parser count**: `cargo test -p nom-parser` (HEAD `afc6228`)
reports **81 passed** across both the `.nomx` parser module (`nomx.rs`)
and the broader `nom-parser` crate (lib-level tests). The `.nomx`
lexer sub-crate (`nom-lexer`) reports **29 passed**.

The "34/34" count cited in earlier doc-09 snapshot referred to
`.nomx`-specific parser tests only as of an earlier HEAD; the current
totals are higher due to subsequent error-path test additions (commits
`468bba6`–`e94a866`) and the `mixed_forms.nomx` fixture (commit
`b49430f`).

### Lexer (`nom_lexer::nomx`, ~330 LOC)

- `NomxToken` enum: 38 variants grouped by role (DeclarationVerb /
  ControlFlow / LinkingVerb / PrepositionalOperator / ContractVerb /
  Value / Punctuation / Eof). Contract verbs
  (require/ensure/throughout/given) per §4.4.
- `tokenize_nomx_with_spans(src) -> Vec<SpannedNomxToken>` —
  primary API. Each token carries a `NomxSpan { start, end }` byte
  range for parser diagnostics.
- `tokenize_nomx(src) -> Vec<NomxToken>` — span-free convenience.
- Articles (`a/an/the/which/who/whose`) stripped at lex time per §4.8.
- `NomxToken::role()` + `starts_declaration()` + `ends_sentence()`
  helpers for the parser and future LSP semantic tokens.
- Tests: **29 green** (HEAD `afc6228`; `cargo test -p nom-lexer`).

### Parser (`nom_parser::nomx`, ~600 LOC)

Four declaration forms (3 block + 1 sentence):

- `define <name> that takes <param> and returns <ret>:` + body (block)
- `record <name> holds:` + `<field> is <type_tokens>.` fields
- `choice <name> is one of:` + variant list
- `to <verb>, respond with <expr>.` sentence-form (lowered to Define)

Five body-statement forms:

- `<subject> is <rhs_tokens>.` binding
- `when <cond>, <then>. otherwise, <else>.` conditional
- `unless <cond>, <then>.` (sugar for `when not`; Not prepended to
  cond_tokens so downstream AST is uniform)
- `for each <var> in|of <coll>, <body>.` iteration
- `while <cond>, <body>.` loop
- `require <pred>.` / `ensure <pred>.` / `throughout, <pred>.`
  contract clauses (ContractKind discriminant)

AST types: `NomxDecl` (3 variants — to-oneliner lowers to Define) +
`NomxStatement` (5 variants: Binding, When, ForEach, While,
Contract) + `NomxRecordField` + `NomxChoiceVariant` + `ContractKind`,
all carrying spans.

Expression parsing within `rhs_tokens` / `cond_tokens` etc. captures
the raw token sequence. Typed expression tree lands with the type
system — this keeps the AST shape stable while grammar bells grow.

Tests: **81 passed** across the full `nom-parser` crate (HEAD `afc6228`;
`cargo test -p nom-parser`). Covers every decl form + every statement
form + error-path tests that pin both diagnostic message and span
accuracy (missing name, missing colon, missing `is`, missing `,`
after when/unless/while/for-each, missing `,` after `to <verb>`
phrase, record field missing `is`). Includes end-to-end parse of
six shipped samples (hello.nomx, todo_app.nomx, greet_sentence.nomx,
loops.nomx, contracts.nomx, mixed_forms.nomx).

### Samples

- `examples/hello.nomx`: one-line `define greet that takes a name
  and returns a greeting:` demo.
- `examples/todo_app.nomx`: full grammar exercise (record + choice
  + 3 defines with when/otherwise bodies).
- `examples/greet_sentence.nomx`: three `to <verb>, respond with`
  sentence-form decls (proposal 05 §3).
- `examples/loops.nomx`: `for each` + `while` + nested
  `when`/`unless` body statements (§4.4).
- `examples/contracts.nomx`: `require` / `ensure` / `throughout`
  contract phrases across three `define` bodies (§4.4).
- `examples/mixed_forms.nomx`: record + choice + block-define +
  two `to`-oneliners in one file, proving the outer-body
  terminator from commit 95f9bdc cleanly switches between block
  bodies and sentence-form decls.

### What's still missing

- Expression parsing inside rhs_tokens (requires type system —
  today the RHS captures raw tokens verbatim).
- Canonical type phrases per §4.5 (`a number`, `a piece of text`,
  `a maybe-<T>`) — tokens already exist in cond/rhs streams, but
  the parser doesn't canonicalize them to types yet.
- Actor form (`actor ... holds ...`) per §4.6.
- Vietnamese aliases (milestone 3 per §8).
- Type inference / checker (phase after expression tree lands).
- Lowering to LLVM bitcode via the existing planner+codegen.

## 11. Readability gate — the 30-second test

Before any milestone ships, five non-programmers read three code
samples. Each must answer, within 30 seconds, the question "what
does this do, and what does it return?" The pass bar is: ≥4/5 give
a correct answer with <5% material misinterpretation (wrong branch,
wrong direction of comparison, wrong value flow).

If the current surface fails and the `.nomx` form passes the same
test, the milestone ships. If both fail, the sample program is too
clever for the language — simplify the program, not the grammar.

---

This proposal is the minimum scope for the rethink. Landing it is a
multi-quarter effort; the **shape** must be right before the first
character is typed into the parser.
