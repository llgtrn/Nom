# `.nomx` Keyword Set (proposed)

Status: **Draft, needs human authoring** on the open questions in
[05-natural-language-syntax.md §8](./05-natural-language-syntax.md).

Companion to proposal 05. Enumerates the English phrase tokens the
`.nomx` lexer needs to recognize, grouped by role. Vietnamese aliases
TBD per the open question on natural-language extension.

## 1. Declaration keywords (verb phrases)

| Phrase | Role | Maps to (existing) |
|--------|------|-------------------|
| `define ... that ...` | named pure fn / method | `fn` |
| `to <verb> <phrase>` | imperative one-liner fn | `fn` (one-expression body) |
| `record ... holds ...` | struct | `struct` |
| `choice ... is one of ...` | tagged union | `enum` |
| `concept ... groups ...` | concept membership block | `concept { … }` |
| `actor ... holds ...` | stateful parallel unit | `actor` (new to surface) |
| `rule ... requires ...` | module-level contract | `require` |

Open question: `a/an/the` articles before the name. Permitted
syntactic fluff, ignored by the AST? Or part of the AST for
round-trip? **Proposal**: ignored at lex time (articles are
Token::Article, collapsed to whitespace by the parser).

## 2. Type phrase vocabulary

Canonical type phrases the parser recognizes. Plural forms also
accepted (`a list of numbers` == `lists of numbers`).

| Phrase | Type | Notes |
|--------|------|-------|
| `a number` | Integer | arbitrary precision |
| `an integer` | Integer | synonym |
| `a 32-bit signed number` | i32 | size-qualified |
| `a 64-bit signed number` | i64 | " |
| `a 32-bit unsigned number` | u32 | " |
| `a fraction` | Rational | arbitrary precision rationals |
| `a decimal` | Decimal | fixed-point base-10 |
| `a piece of text` | Text | canonical UTF-8 |
| `a word` | Text | synonym (short form) |
| `a character` | Codepoint | single Unicode codepoint |
| `true or false` | Bool | |
| `a date` | Date | ISO 8601 |
| `a duration` | Duration | |
| `a list of <T>` | List<T> | |
| `a map from <K> to <V>` | Map<K,V> | |
| `a set of <T>` | Set<T> | |
| `a maybe-<T>` | Option<T> | forces pattern match |
| `either <T> or <E>` | Result<T,E> | forces pattern match |
| `nothing` | Unit | bottom value type |
| `anything` | Any | forbidden in user code except `unless`-clauses |

Open question: `a user` (struct) vs `a User` (concept) vs `a
user-id` (nomtu word). **Proposal**: capitalized multi-word
phrase → concept, lowercase single-word / hyphenated → nomtu,
lowercase multi-word phrase in type position → lookup in phrase
dict or parse error.

## 3. Control flow

| Phrase | Role | Maps to |
|--------|------|---------|
| `when <cond>, <clause>` | true-branch | `if cond { clause }` |
| `unless <cond>, <clause>` | negated | `if !cond { clause }` |
| `otherwise, <clause>` | else | `else { clause }` |
| `for each <x> in <coll>, <clause>` | iteration | `for x in coll { clause }` |
| `while <cond>, <clause>` | loop | `while cond { clause }` |
| `break out of <loop-name>` | break | `break` |
| `skip to the next` | continue | `continue` |

## 4. Contract phrases

| Phrase | Role |
|--------|------|
| `when given <pred>, ...` | precondition clause |
| `require <pred>.` | precondition statement |
| `ensure <pred>.` | postcondition |
| `throughout, <pred>.` | invariant (actor/loop body) |
| `this never panics` | effect annotation (panic-free proof) |
| `this has no side effects` | effect: Pure |
| `this may read ...` | effect: Reads(…) |
| `this may write ...` | effect: Writes(…) |

## 5. Operators as prepositions

| Phrase | Op | Notes |
|--------|----|----|
| `<a> plus <b>` | `+` | |
| `<a> minus <b>` | `-` | |
| `<a> times <b>` | `*` | |
| `<a> divided by <b>` | `/` | |
| `<a> to the power of <b>` | `^` | |
| `<a> and <b>` | `&&` | logical |
| `<a> or <b>` | `\|\|` | logical |
| `not <a>` | `!` | unary |
| `<a> is the same as <b>` | `==` | structural equality |
| `<a> is less than <b>` | `<` | |
| `<a> is at most <b>` | `<=` | |
| `<a> of <b>` | `call(b)` / `b.a` | context-resolved |
| `<a> from <b>` | `b.a` | field access |
| `<a> with <b>` | `a(b)` or struct update | context-resolved |
| `<a> then <b>` | sequencing | `a; b` (value of last) |
| `apply <f> to <x>` | `f(x)` | fallback when prepositions ambiguous |

Open question: resolution order for `a of b`. Proposal: type
inference picks field-access (if `b` has field named like `a`),
else function-call (if there's a nomtu named like `a`), else
parse error. Never both simultaneously — the grammar is
context-sensitive via the type system.

## 6. Value literals

| Phrase | Literal | Type |
|--------|---------|------|
| `zero` | 0 | Integer |
| `one` | 1 | Integer |
| `true` | true | Bool |
| `yes` | true | alias |
| `false` | false | Bool |
| `no` | false | alias |
| `"..."` | string | Text |
| `0`..`999999` | digits | Integer |
| `0.5` etc | floating | Decimal |
| `nothing` | () | Unit |

Open question: allow `twelve`, `twenty-three` etc as integer
literals? **Proposal**: no — too much parsing ambiguity vs.
identifier names. Stick with digits for >9.

## 7. Reserved concept prefixes

Capitalized multi-word phrases starting with `The` or `An`
bind to concepts:

- `The User Profile` → concept `user_profile`
- `An Active Session` → concept `active_session`

This gives proper-noun concepts visual distinction in prose.

## 8. Reserved article words (lexer-level whitespace)

`a`, `an`, `the`, `that`, `which`, `who`, `whose` — stripped
before AST generation. Keeps the prose natural without making
the AST carry filler.

Open question: what if a user names a nomtu word `the_ratio`? The
underscore breaks the article-detection boundary, so it still
parses as an identifier. Non-issue.

## 9. Vietnamese alias layer (open)

The current lexer has 80+ Vietnamese keyword aliases (see
`lexer.nom` line ~45). The proposal is to extend them to the
`.nomx` phrase set one-for-one:

- `define ... that ... ` ↔ `định nghĩa ... rằng ...`
- `when ... otherwise ...` ↔ `khi ... nếu không ...`
- `a number` ↔ `một số`
- `a piece of text` ↔ `một đoạn văn`

Needs native-speaker authoring. **Proposal**: ship English-only
in milestone 1; Vietnamese aliases in milestone 3 after the
grammar stabilizes.

---

This enumeration is the vocabulary the `.nomx` lexer and parser
need to know. Reviewer should confirm:

1. Is the phrase set right? (§1-6)
2. Is the capitalization rule (proper-noun concepts) acceptable?
3. Article-stripping at lex time: OK?
4. Defer Vietnamese aliases to milestone 3: OK?
5. Any missing vocabulary from real-world Nom use cases?
