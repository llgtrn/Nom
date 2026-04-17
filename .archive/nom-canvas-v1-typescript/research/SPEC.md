# Part 17: The Final Language Design

**Everything decided. Everything in one place. Ready to build.**

---

## 1. What Nom Is

```
Nom is a programming language where you write sentences.
Each sentence describes what you want.
The compiler reads each word from an online dictionary.
The compiler checks grammar (contracts).
The compiler produces a native binary.
No braces. No tabs. No semicolons. No middle language.
Just: write a sentence, get an application.
```

## 2. The Files

```
.nom        source file       sentences you write (English, writing-style)
.nomtu      dictionary entry  a word (text: name + describe + contract + scores)
.nomiz      compiled IR       composition graph ready for code generation
nomdict     online dictionary nom.dev, indexes millions of .nomtu entries
nom         CLI tool          compiler + builder + debugger + dictionary manager
nom.toml    project manifest  configuration
nom.lock    lockfile          pinned .nomtu hashes for reproducible builds
binary      final output      native executable via LLVM
```

## 3. The Syntax

### Rules

```
1. Classifiers start declarations:
   system, flow, store, graph, agent, test, nom, gate, pool, view

2. Everything after a classifier belongs to that declaration
   until a blank line or the next classifier

3. No braces { } (except inside flow for branch forks)
   No semicolons
   No required tabs or indentation

4. English keywords by default
   Foreign words welcome when English is imprecise
   Locale packs translate keywords (Vietnamese, Chinese, Arabic, ...)

5. Operators are universal:
   ->   flow (then)
   ::   specialize (variant)
   +    combine
   ><=  comparison
   #    comment
```

### Keywords

```
CLASSIFIERS:     system flow store graph agent test nom gate pool view
DECLARATIONS:    need require effects where only describe
FLOW:            -> branch iftrue iffalse
TEMPORAL:        done active planned
EFFECTS:         good bad
VALUES:          true false none
TYPES:           number real text bool list map bytes
LOGIC:           and or not
CONTRACT:        contract in out effects pre post
IMPLEMENTATION:  implement
```

### Example Programs

**Hello World (2 lines):**
```nom
flow hello
flow "hello world"->print
```

**Web API (6 lines):**
```nom
system userapi
need http::server where port=8080
need database::postgres
flow /users/{id}->query->response
iftrue response200
iffalse response404
```

**Auth Service (6 lines):**
```nom
system auth
need hash::argon2 where security>0.9
need store::redis where reliability>0.8
need limiter::rate where performance>0.9
flow request->limiter->hash->store->response
require latency<50ms
```

**Chatbot (5 lines):**
```nom
system chatbot
need nlp::understand where accuracy>0.85
need memory::vector where recall>0.8
need llm::generate where quality>0.9
flow message->nlp->memory->llm->reply
```

**Test (5 lines):**
```nom
test authsecurity
given auth with mockstore inmemory
when request with token expired
then response.status = 401
and effects include bad(authfailure)
```

**Custom word (nom definition):**
```nom
nom customscorer
describe "calculate relevance score for legal documents"

contract
in document(text) query(text)
out score(real range 0.0 to 1.0)
effects [cpu]
pre document.length > 0

implement rust {
    fn score(doc: &str, query: &str) -> f32 {
        let tokens: Vec<&str> = doc.split_whitespace().collect();
        let matches = tokens.iter().filter(|t| query.contains(*t)).count();
        matches as f32 / tokens.len() as f32
    }
}
```

**Foreign borrowing:**
```nom
system kitchen
need mise-en-place where preparation=complete
need wok::carbonsteel where seasoning>0.8
need umami::dashi where depth>0.9
flow ingredients->mise-en-place->wok->umami->plating
```

## 4. The .nomtu Format

A .nomtu is a text file. No binary. A dictionary entry.

```
word: hash
describe: convert data into irreversible fixed-length string
          to protect passwords and verify integrity
kind: crypto

contract
in: bytes
out: hashbytes
effects: cpu
pre: in.length > 0
post: irreversible and unique

scores
security: 0.96
performance: 0.72
quality: 0.91
reliability: 0.95

source: rustcrypto
version: 3.2
license: MIT
tests: 847

variants
argon2: security=0.96 performance=0.72
bcrypt: security=0.89 performance=0.81
sha256: security=0.91 performance=0.95
```

The `describe` field enables semantic search: when the compiler
doesn't find a word by name, it searches descriptions.

Implementation code is stored SEPARATELY in nom.dev registry,
not inside the .nomtu file. The .nomtu is the dictionary entry.
The registry holds the actual runnable code (compiled WASM).

## 5. The Pipeline

```
.nom source
    |
    nom compiler (written in Rust initially)
    |
    ├── tokenize (English keywords + identifiers)
    ├── parse (classifier-based, no braces)
    ├── resolve (look up .nomtu in nomdict)
    ├── select (pick best variant by score + constraints)
    ├── verify (check contracts across all edges)
    ├── plan (memory strategy, concurrency, optimization)
    |
    .nomiz (compiled composition graph)
    |
    ├── RELEASE: LLVM IR via inkwell → native binary
    ├── DEBUG: Cranelift → native binary (10x faster compile)
    ├── WEB: LLVM → WASM
    |
    binary (runs natively, assembly-smooth)
```

No Rust source generated. No Python. No C. Direct to LLVM IR.

## 6. The Dictionary

```
nom.dev (online):     all .nomtu ever published (millions)
~/.nom/nomtu/ (local): cached .nomtu files (downloaded on demand)
./nomtu/ (project):    custom .nomtu (escape hatch for new words)
nom.lock:              pinned .nomtu hashes (reproducible builds)
```

### How new words are created

```
1. Search dictionary → word found → use it
2. Search dictionary → word NOT found → 
   a. AI proposes new .nomtu by composing existing ones
   b. OR developer writes custom nom definition
   c. New .nomtu starts at score 0 (unverified)
   d. Testing raises score
   e. Review raises score
   f. Published to nom.dev
   g. Dictionary grew by one word
```

## 7. What Makes Nom Different

```
vs npm (3.2M packages):
    npm: trust-by-default, $60B supply chain losses
    nom: contracts + scores + provenance on every .nomtu

vs Copilot (20M users):
    Copilot: generates code (29% security issues)
    nom: composes from verified dictionary (0% fabrication in boundary)

vs every language:
    them: braces, semicolons, tabs, years to learn
    nom: writing-style, describe what you want, binary out

vs Unison (content-addressed):
    Unison: broke files/git/editors (~6.5K stars)
    nom: normal text files, works with git, online dictionary
```

## 8. The Build Sequence

```
Phase 0 (months 1-3):   parser + interpreter + nom run hello.nom works
Phase 1 (months 4-8):   50-200 .nomtu + nom.dev prototype + killer app demo
Phase 2 (months 9-18):  LLVM backend + .nomiz + native binary
Phase 3 (months 18-36): dictionary scaling + community + foundation
Phase 4 (year 3+):      stability promise (1.0) + self-hosting
```

## 9. The Design Principles (From Vietnamese Grammar)

Vietnamese grammar inspires the STRUCTURE, not the keywords:

```
Vietnamese principle          Nom implementation
──────────────────           ──────────────────
Analytic (words never change) .nomtu are immutable (content-addressed)
Classifiers mark categories   system/flow/store/test mark declarations
Topic-comment structure       classifier starts, lines describe
No inflection                 no operator overloading, no implicit conversion
Serial verbs (verb chaining)  flow chaining with ->
No articles/plural markers    no let/var/const, no semicolons
Flexible where disambiguated  order-free inside declarations
Compounds join words          :: joins concept to variant
Foreign borrowing natural     any language's words welcome
```

## 10. One Page Summary

```
A .nomtu is a word.
A .nom is a sentence.
nom.dev is the dictionary.
A binary is a story.

You write sentences using words from the dictionary.
The compiler checks grammar (contracts).
The compiler produces the story (binary).

No braces. No tabs. No middle language.
English by default. Any language when needed.
Verified. Scored. Transparent. Assembly-smooth.

Write a sentence. Get an application.
```
