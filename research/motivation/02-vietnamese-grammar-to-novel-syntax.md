# Part 2: Vietnamese Grammar → Nom Syntax Design

**How the structure of the Vietnamese language informs Nom's programming syntax.**

**Note (updated 2026-04-11):** Nom uses English as the primary keyword language.
Vietnamese grammar inspires the STRUCTURE (classifiers, topic-comment, no braces,
writing-style) but the WORDS are fully English. No VN tokens exist in the lexer.

**Note (updated 2026-04-13):** Vietnamese keyword vocabulary (cai/ham/etc.) that
previously shipped in commits `4b04b1d`/`c601f31`/`5b59f82` has been fully removed
([this-commit]). Grammar style inspires structure; vocabulary stays English; no VN
tokens in the lexer. The `agent_demo_vn` example has been deleted.

---

> **Status banner — Last verified against codebase: 2026-04-13, HEAD afc6228.**
>
> Per-section tags:
> - ✅ SHIPPED — backed by code at the cited commit/file
> - ⏳ PLANNED — on roadmap; no shipped code yet
> - ❌ ASPIRATIONAL — beyond current roadmap; no concrete plan
> - ⚠️ PARKED — work done but frozen; not being extended

---

## The Core Thesis

Vietnamese is an analytic (isolating) language. Words never change form. Meaning comes
from composition and word order, not from inflection or mutation. This is exactly what
Novel does with Noms — immutable atoms composed through structure, not through
transformation.

The parallel is not metaphorical. It is structural. Vietnamese grammar IS a
compositional-semantic system. Novel IS a compositional-semantic system. We are building
the same kind of thing — one for human meaning, one for software meaning.

---

## 1. Analytic Grammar → Immutable Tokens

**Section status: ✅ SHIPPED (implicit) — content-addressed `words_v2` schema enforces
immutability by construction. Each hash in `words_v2` is the identity of the entry;
changing the body produces a different hash, so tokens cannot mutate in place.
(commit `aaa914d`, `nom-dict/src/lib.rs`)**

### Vietnamese Property
Vietnamese is analytic/isolating. "ăn" (eat) is always "ăn" — no conjugation (ate,
eating, eats), no declension, no inflection. Grammatical relationships are expressed
entirely through word order and lightweight particles.

### Novel Mapping
**Every token in Novel means one thing, always, everywhere.**

```novel
# argon2_hash ALWAYS means "hash a password with argon2"
# It never becomes argon2_hashing, argon2_hashed, or changes signature by context
# This is the Vietnamese principle: what you see is what it means

# NO operator overloading (same symbol, different meaning = ambiguity)
# NO implicit conversions (silent meaning change = invisible mutation)
# NO context-dependent resolution of syntax (same code, different parse = confusion)
```

**Design rule:** Zero syntactic sugar that changes a token's meaning based on context.
A keyword means one thing. A Nom name means one thing. Period.

---

## 2. SVO Word Order → Flow Syntax

**Section status: ⏳ PLANNED (Phase 5/6) — the `->` operator parses correctly and the
concept-graph closure walker resolves flow chains (`c5cdce6`). The full SVO-as-planner
semantics (where modifier-follows-head is enforced by the planner, not just the parser)
await Phase 5.**

### Vietnamese Property
Subject-Verb-Object: "Tôi ăn cơm" (I eat rice).
Modifiers FOLLOW the head: "áo đỏ" (shirt red), "người đẹp" (person beautiful).

### Novel Mapping
The flow operator already reflects SVO:

```novel
# Subject → Verb → Object
request -> authenticate -> response

# Modifiers follow the head (Vietnamese order):
password_hasher where security > 0.9     # head first, qualifier after
session_store  where reliability > 0.8   # NOT: secure password_hasher (English order)
rate_limiter   where performance > 0.9   # NOT: reliable session_store

# This is "áo đỏ" (shirt red), not "red shirt"
# Declare WHAT, then constrain it
```

**Design rule:** Constraints, annotations, and qualifiers always follow the thing they
describe. Never precede it.

---

## 3. Classifiers (Loại Từ) → Nom Kind Classifiers

**Section status: ✅ SHIPPED — closed kind set in `nom-concept/src/lib.rs`. The
`function`/`module`/`concept` classifiers are enforced as mandatory in the `.nom`
and `.nomtu` parsers (commits `05ee1b6`, `d9425ba`). The `the function` / `the @Function`
typed-slot forms are also shipped (`c9d1835`). Every `.nom` declaration requires a
kind classifier; unclassified declarations produce a parse error.**

### Vietnamese Property
Vietnamese REQUIRES classifiers between numbers and nouns. They categorize reality:
- **cái** — inanimate objects: cái bàn (table), cái nhà (house)
- **con** — things with motion/animacy: con mèo (cat), con dao (knife — it cuts),
  con đường (road — it leads somewhere)
- **người** — humans: người bạn (friend)
- **cây** — long thin objects: cây bút (pen)
- **chiếc** — single from a pair: chiếc giày (one shoe)
- **quyển** — books: quyển sách (book)

Structure: Number + Classifier + Noun ("một con mèo" = one [animal] cat)
You CANNOT say "một mèo" (one cat) — the classifier is mandatory.

### Novel Mapping
**Nom kind classifiers are mandatory.** Every declaration must be classified.

```novel
# Classifier    Name              # Vietnamese parallel
flow   auth_pipeline              # "con" — things that move/flow
agent  Guardian                   # "người" — autonomous entities
graph  knowledge_base             # network of connections
store  user_profiles              # "cái" — persistent objects
gate   rate_limiter               # control/filtering point
nom    custom_scorer              # raw atom definition (escape hatch)

# You CANNOT declare without a classifier:
# auth_pipeline { ... }           # ERROR: unclassified declaration
# flow auth_pipeline { ... }      # CORRECT: classified

# Just as Vietnamese requires: một CON mèo (one [classifier] cat)
# Novel requires:              flow auth_pipeline (classifier + name)
```

The `con` classifier is especially insightful: it marks things with inherent motion or
direction — rivers (con sông), roads (con đường), knives (con dao). This maps to
`flow` — computational things that move data from A to B.

**Design rule:** Every declaration requires a kind classifier. No orphan declarations.

### Extended Classifier System

| Vietnamese Classifier | Concept | Novel Classifier | What It Classifies |
|----------------------|---------|-----------------|-------------------|
| con (motion/animacy) | Things that move | `flow` | Data pipelines, streams, chains |
| cái (inanimate) | Static objects | `store` | Databases, caches, state containers |
| người (human) | Autonomous beings | `agent` | Autonomous processes with capabilities |
| cây (long/thin) | Extended structures | `graph` | Networks, trees, DAGs |
| chiếc (singular) | One from a set | `gate` | Filters, limiters, validators |
| quyển (book) | Knowledge containers | `nom` | Atom definitions, raw implementations |
| bộ (set/collection) | Complete sets | `system` | Composed systems of multiple parts |
| con (as counter) | Countable instances | `pool` | Worker pools, connection pools |

---

## 4. Six Tones → Six Nom Modifiers

**Section status: ❌ ASPIRATIONAL — no modifier syntax (`!`, `~`, `?`, `^`, `.`) has
been shipped. The closed kind set covers classifiers (§3) but the tone-to-modifier
mapping is not implemented in any parser or AST node.**

### Vietnamese Property
Six tones change the meaning of the same syllable:
- **ma** (ngang/flat) = ghost — the base, unmarked form
- **mà** (huyền/falling) = but/that — connective, lighter
- **má** (sắc/rising) = mother — elevated, important
- **mả** (hỏi/questioning) = tomb — uncertain, questioning
- **mã** (ngã/breaking) = horse/code — transformed, different nature
- **mạ** (nặng/heavy) = rice seedling — grounded, persistent

### Novel Mapping
**Six universal modifiers that apply to any Nom, like tones on any syllable:**

```novel
# Base form (ngang) — default behavior
hash                        # generic hash, engine picks best

# Strict form (sắc/rising) — elevated requirements
hash! strict                # cryptographic, constant-time, audited

# Light form (huyền/falling) — relaxed requirements
hash~ light                 # fast non-crypto hash, for checksums

# Optional form (hỏi/questioning) — may not resolve
hash? optional              # try to find, graceful fallback if missing

# Streaming form (ngã/breaking) — transformed execution model
hash^ streaming             # processes chunks incrementally

# Persistent form (nặng/heavy) — durable, stored
hash. persistent            # result cached/memoized across invocations
```

| Tone | Mark | Vietnamese Meaning | Novel Modifier | Semantics |
|------|------|-------------------|---------------|-----------|
| ngang (flat) | unmarked | base/default | (none) | Default behavior |
| sắc (rising) | `!` | elevated | `strict` | Enhanced requirements |
| huyền (falling) | `~` | lighter | `light` | Relaxed requirements |
| hỏi (questioning) | `?` | uncertain | `optional` | May fail gracefully |
| ngã (breaking) | `^` | transformed | `streaming` | Different execution model |
| nặng (heavy) | `.` | grounded | `persistent` | Durable/cached |

**Design rule:** Any Nom can take any modifier. The modifier doesn't change the Nom's
identity — it adjusts how the engine resolves and executes it. Like Vietnamese tones:
same syllable, different meaning.

---

## 5. Compound Words (Từ Ghép) → Two Composition Modes

**Section status: ⏳ PLANNED — the `::` (subordinate/specialization) and `+` (coordinate)
operators parse correctly. Full compound resolution through the dictionary (Phase 5+)
is PLANNED. The `words_v2` kind-based lookup (`c405d2a`) is the first step.**

### Vietnamese Property
Two types of compounds:

**Subordinate (từ ghép chính phụ)** — head + modifier (meaning narrows):
- xe (vehicle) + lửa (fire) = xe lửa (train)
- máy (machine) + bay (fly) = máy bay (airplane)
- tàu (vessel) + ngầm (submerge) = tàu ngầm (submarine)

**Coordinate (từ ghép đẳng lập)** — equal parts (meaning broadens):
- cha mẹ (father + mother) = parents
- ăn uống (eat + drink) = dining
- bàn ghế (table + chair) = furniture

### Novel Mapping

```novel
# SUBORDINATE COMPOSITION (từ ghép chính phụ)
# Head :: Modifier → narrower meaning
# Like xe :: lửa (vehicle :: fire = train)

auth_flow :: jwt            # auth narrowed to JWT-based
cache :: write_through      # cache narrowed to write-through
hash :: argon2              # hash narrowed to argon2

# The head carries the contract category
# The modifier selects the specific variant
# Meaning is NARROWER than head alone

# COORDINATE COMPOSITION (từ ghép đẳng lập)
# Equal peers → broader capability
# Like cha + mẹ = parents (broader than either alone)

system auth = compose {
    password_hasher + session_store + rate_limiter
    # Three equal peers combine into something more general
    # "auth" is BROADER than any individual Nom
}
```

**Design rule:** Novel has two composition operators:
- `::` for subordinate (specialization/narrowing)
- `+` for coordinate (combination/broadening)

---

## 6. Serial Verb Construction → Flow Chaining

**Section status: ⏳ PLANNED — the `->` operator chains parse correctly and the closure
walker traverses them (`c5cdce6`). The semantic claim that a chain is "one composite
operation" (not sequential independent steps) requires the Phase 5+ codegen path.**

### Vietnamese Property
Verbs chain without conjunctions to express one complex event:
- "Tôi chạy ra ngoài" (I run exit outside) = I ran outside
- "Cô ấy ngồi đọc sách" (She sit read book) = She sat reading a book

No linking words. Sequence implies relationship. The chain is ONE event, not separate actions.

### Novel Mapping

```novel
# Vietnamese: chạy ra ngoài (run exit outside) — one action
# Novel: read -> parse -> validate -> store — one flow

flow process = request -> authenticate -> authorize -> execute -> respond

# Key insight from Vietnamese SVC:
# These are NOT five separate operations
# This is ONE composite operation
# "authenticate -> authorize" is like "chạy ra" (run exit)
# — two facets of a single intent (secure access)
```

**Design rule:** The `->` operator creates a single composite operation, not a sequence
of independent steps. Data flows through; the whole chain is one unit.

---

## 7. Topic-Comment Structure → Declaration Syntax

**Section status: ⏳ PLANNED — the current parser accepts the implicit topic-comment
form (classifier + name followed by indented properties). Explicit `{...}` topic-comment
block markers are not yet formalized; the `{` as topic separator shown below is design
intent, not shipped syntax.**

### Vietnamese Property
Vietnamese is topic-prominent:
"Sách này thì tôi đọc rồi" (Book this [topic-marker] I read already)
= "As for this book, I've already read it."

Topic comes first (what we're talking about), then comment (what we say about it).
The particle "thì" separates topic from comment.

### Novel Mapping

```novel
# Topic-comment structure:
# [Topic] { [Comment] }
# The { is the "thì" particle — it marks where topic ends and comment begins

auth_service {                    # TOPIC: what we're talking about
    need password_hasher          # COMMENT: what it needs
    need session_store            # COMMENT: more needs
    flow: request -> auth -> out  # COMMENT: how it works
    require security > 0.9       # COMMENT: constraints
}

# Vietnamese parallel:
# "Auth_service thì cần password_hasher, cần session_store, chạy như..."
```

**Design rule:** `{` is the topic marker. Everything before it is the topic (what you're
defining). Everything inside is the comment (properties, requirements, behavior).

---

## 8. Aspect Markers (đã/đang/sẽ) → Temporal State

**Section status: ⚠️ PARKED — Vietnamese keyword vocabulary was removed
([this-commit]). This section describes the design intent; aspect-marker state
tracking (`verified`/`active`/`deferred`) has low semantic value without a runtime.
Grammar style still influences how Nom structures flow: vocabulary stays English.**

### Vietnamese Property
Three pre-verbal particles mark temporal aspect WITHOUT changing the verb:
- **đã** (completed): "Tôi đã ăn" (I [completed] eat = I ate)
- **đang** (ongoing): "Tôi đang ăn" (I [ongoing] eat = I am eating)
- **sẽ** (future): "Tôi sẽ ăn" (I [future] eat = I will eat)

The verb "ăn" never changes. Only the particle changes.

### Novel Mapping

```novel
# đã (completed) → verified/resolved state
da auth_service        # this composition has been verified and is ready
                       # contracts satisfied, code generated

# đang (ongoing) → active/running state
dang auth_service      # this service is currently executing
                       # monitoring active, metrics streaming

# sẽ (future) → planned/deferred state
se auth_service        # this will be composed when triggered
                       # lazy resolution, deferred compilation

# Applied to flows:
flow pipeline = da(validate) -> dang(process) -> se(archive)
#               pre-computed    live processing   deferred
```

| Vietnamese | Marker | Novel State | Meaning |
|-----------|--------|-------------|---------|
| đã | `da` | verified/resolved | Composed, contract-checked, ready |
| đang | `dang` | active/running | Currently executing, monitored |
| sẽ | `se` | planned/deferred | Will compose when needed |

**Design rule:** State markers are prefixes. They don't change the Nom — they mark its
temporal state. Like Vietnamese: the verb never changes.

---

## 9. Được/Bị → Effect Valence

**Section status: ✅ SHIPPED — English-only (`benefit`/`hazard` keywords). Effect
valence keywords are shipped in `nom-concept/src/lib.rs` (commit `c9d1835`) and
surfaced in `nom build manifest` output (commit `eeb1e23`). The Vietnamese loanwords
`duoc`/`bi` were explicitly rejected per user clarification: vocabulary stays English.
The `agent_demo` fixtures at `nom-compiler/examples/agent_demo/` use `benefit`/`hazard`
in practice.**

### Vietnamese Property
Two passive markers encode ATTITUDE:
- **được** (positive): "Tôi được tăng lương" (I [benefited] raise salary = I got a raise)
- **bị** (negative): "Tôi bị mất ví" (I [suffered] lose wallet = wallet stolen)

Same grammatical operation (passive voice), different semantic valence.

### Novel Mapping

```novel
# được (positive effect) → beneficial system events  (English: "benefit")
benefit cache_hit              # good: response served from cache
benefit load_balanced          # good: load was distributed
benefit auto_scaled            # good: capacity increased

# bị (negative effect) → adverse system events  (English: "hazard")
hazard  timeout                # bad: upstream didn't respond
hazard  rate_limited           # bad: request was throttled
hazard  memory_pressure        # bad: resources strained

# In effect declarations:
flow request_handler
    benefit [cached, optimized]           # positive effects
    hazard  [timeout, rate_limited]       # negative effects
```

> Note: The syntax above shows design intent. The `duoc`/`bi` loanword aliases
> shown in earlier drafts of this document are NOT shipped; they were explicitly
> rejected. The English keywords `benefit`/`hazard` are canonical.

**Design rule:** Effects carry semantic valence. The engine knows whether an effect is
beneficial or adverse, driving monitoring, alerting, and system behavior automatically.
No existing programming language does this.

---

## 10. No Articles, No Plural → Minimal Syntax

**Section status: ✅ SHIPPED (implicit) — the `.nom`/`.nomtu` parsers require no
articles, no plural forms, and no semicolons. This is enforced by the parser grammar
in `nom-concept/src/lib.rs`.**

### Vietnamese Property
No articles (a, an, the). No required plural forms. "Sách" = book, books, the book,
a book. Context determines which. Explicit plurality only when needed ("những", "các").

### Novel Mapping

```novel
# Minimal syntax — no noise:
build web_app {
    pages: [home, about, dashboard]   # no "new", no "let", no type annotation
    auth: google_oauth                # no "const", no semicolons
    database: user_profiles           # context determines singular/collection
}

# Explicit plurality only when needed:
need [all] password_hasher where security > 0.9   # explicit: all variants
need password_hasher where security > 0.9          # default: best single
```

**Design rule:** Every syntactic token must earn its place. If it can be inferred, omit it.
No semicolons. No let/const/var. No new. No type annotations when inferrable.

---

## 11. Sino-Vietnamese Morphemes → Core Nom Kinds

**Section status: ✅ SHIPPED (implicit) — the closed kind set in `nom-concept/src/lib.rs`
(`function|module|concept|screen|data|event|media` etc.) serves as the finite root
vocabulary from which Nom declarations are built. The full `~200-500 root vocabulary`
registration in a corpus is PLANNED for Phase 5+.**

### Vietnamese Property
~3,000 Sino-Vietnamese base morphemes compose into unlimited technical vocabulary:
- điện (electric) + thoại (speech) = điện thoại (telephone)
- máy (machine) + tính (calculate) = máy tính (computer)
- khoa (study) + học (learn) = khoa học (science)

30-70% of Vietnamese vocabulary is Sino-Vietnamese compounds.

### Novel Mapping
The core Nom kinds (auth_flow, cache_strategy, kernel_module, etc.) are like
Sino-Vietnamese morphemes — a finite set of composable roots that generate unlimited
technical vocabulary:

```novel
# Core morphemes compose into specialized concepts:
auth + flow        = auth_flow       (like điện + thoại = điện_thoại)
cache + strategy   = cache_strategy  (like máy + tính = máy_tính)
kernel + module    = kernel_module   (like khoa + học = khoa_học)
```

**Design rule:** Define ~200-500 core Nom kinds (like Sino-Vietnamese morphemes).
All domain-specific concepts are compounds of these cores. The dictionary grows by
composition, not by inventing new roots.

---

## 12. Reduplication (Từ Láy) → Parameterized Variants

**Section status: ❌ ASPIRATIONAL — no reduplication or parameterized-variant syntax
has been designed or shipped. The `::` specialization operator (§5) is the closest
analog but does not implement the softening/strengthening semantic described here.**

### Vietnamese Property
~10% of Vietnamese vocabulary uses reduplication — systematic sound patterns that
create related meanings:
- đỏ → đo đỏ (red → reddish) — full reduplication softens
- xanh → xanh xanh (green → greenish)
- đẹp → đèm đẹp (beautiful → somewhat beautiful)

### Novel Mapping
Reduplication maps to **parameterized weakening/strengthening** of Noms:

```novel
# Full reduplication: soften/approximate
auth :: strict       → auth :: approximate    # strict auth → relaxed auth
validate :: full     → validate :: partial     # full validation → partial
encrypt :: strong    → encrypt :: light        # strong encryption → lightweight

# This is systematic — any Nom can be "reduplicated" to get a softer version
# Like đỏ → đo đỏ, where the transformation is regular and predictable
```

---

## 13. Grammatical Particles → Operators

**Section status: ✅ SHIPPED for most particles — `where`, `+`, `->`, `matching` all
parse correctly. The `{...}` topic-comment block (§7) is design intent. The `da`/`dang`/`se`
aspect markers (§8) are ⚠️ PARKED locale-pack aliases. The `benefit`/`hazard` valence
keywords (§9) are ✅ SHIPPED English-only.**

### Vietnamese Property
Particles connect clauses and manage information flow:

| Particle | Function | Example |
|----------|----------|---------|
| và | and (conjunction) | A và B |
| mà | but/that (adversative/relative) | A mà B |
| thì | as for (topic marker) | A thì B |
| để | in order to (purpose) | A để B |
| cho | for/to (benefactive) | A cho B |
| rồi | already (completion) | A rồi |

### Novel Mapping

| Vietnamese | Novel Operator | Syntax | Meaning | Status |
|-----------|---------------|--------|---------|--------|
| và (and) | `+` | A + B | Coordinate composition | ✅ SHIPPED |
| mà (but/that) | `where` | A where P | Constraint/guard | ✅ SHIPPED |
| thì (topic) | `{` | A { ... } | Topic-comment block | ⏳ PLANNED |
| để (purpose) | `->` | A -> B | Flow/pipeline | ✅ SHIPPED |
| cho (for) | `for` | A for B | Target/beneficiary | ⏳ PLANNED |
| rồi (done) | `da` | da A | Verified/complete | ⚠️ PARKED |
| đang (ongoing) | `dang` | dang A | Active/running | ⚠️ PARKED |
| sẽ (will) | `se` | se A | Planned/deferred | ⚠️ PARKED |

---

## Summary: The Vietnamese-Novel Grammar Correspondence

```
Vietnamese Grammar          Novel Syntax                          Status
─────────────────          ────────────                          ──────
Analytic (no inflection)   Immutable Noms (no context-dep.)      ✅ SHIPPED (words_v2, aaa914d)
SVO word order             entity -> action -> target             ✅ SHIPPED (parser)
Modifier follows head      constraint follows declaration (where) ✅ SHIPPED (parser)
Mandatory classifiers      Mandatory kind classifiers             ✅ SHIPPED (nom-concept/src/lib.rs)
6 tones on same syllable   6 modifiers on same Nom (!, ~, ?, ^.) ❌ ASPIRATIONAL
Subordinate compounds      Specialization (::)                    ✅ SHIPPED (parser + c405d2a)
Coordinate compounds       Combination (+)                        ✅ SHIPPED (parser)
Serial verb construction   Flow chaining (->)                     ✅ SHIPPED (parser + c5cdce6)
Topic-comment structure    Declaration blocks ({ })               ⏳ PLANNED
Aspect markers (đã/đang/sẽ) Temporal state (da/dang/se)          ⚠️ PARKED (4b04b1d)
Được/bị valence           Effect valence (benefit/hazard)        ✅ SHIPPED English-only (c9d1835)
No articles/plural         Minimal syntax (no noise)              ✅ SHIPPED (parser)
Sino-Vietnamese morphemes  Core Nom kinds (~200-500 roots)        ✅ PARTIAL (closed kind set)
Reduplication patterns     Parameterized variants                 ❌ ASPIRATIONAL
Grammatical particles      Operators (where, +, ->, for)          ✅ SHIPPED (most)
```

The deepest insight: Vietnamese grammar is already compositional-semantic. It composes
meaning from stable atoms using word order and particles — no inflection, no mutation,
no hidden state changes. Novel does the same with software. The language is not INSPIRED
by Vietnamese. It IS Vietnamese grammar applied to computation.

> **Vocabulary clarification (updated [this-commit]):** The Vietnamese-loanword
> vocabulary layer (cai/ham/duoc/bi etc.) has been fully removed from the lexer and
> examples. It is NOT part of the Nom language. Nom's keywords are English-only ASCII.
> Vietnamese contributes GRAMMAR STYLE (the structural mappings above), not vocabulary.
