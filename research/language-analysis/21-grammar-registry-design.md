# 21 — Grammar Registry Design (AI-Retrievable Basic Syntax)

**Filed 2026-04-14 very-very-late.** Status: **Draft design; requires implementation wedge.**

> **The gap, stated by the user on 2026-04-14:** the current "basic syntax" of `.nomx v1` / `.nomx v2` (keywords, clause shapes, kind-specific rules) lives in two places: (a) Rust parser code in `nom-compiler/crates/{nom-parser,nom-concept}/src/` and (b) prose in markdown files (docs 05, 06, 07, 13, 16, 17). Neither is queryable by an AI client at authoring time. An LLM writing Nom today must fuzzy-search .md prose or guess clause shapes — this does not scale to the 100M-entry vocabulary thesis, and it does not scale to 450+ authoring-guide rules.

This doc specifies the fix: a **machine-readable grammar registry** (`grammar.sqlite` + CLI surface) so any AI client can deterministically query *what the parser accepts* and *which clause shape applies to a given paradigm*, without reading .md prose.

---

## 1. Scope — what counts as "basic syntax"

The registry carries **three knowledge layers** that AI clients need before they can write any `.nom` / `.nomtu` source. It does NOT carry concept-dictionary entries or word-dictionary entries — those live in `dict.sqlite` per doc 04 §4.4.6.

| Layer | Example question AI asks | Source of truth |
|---|---|---|
| **L1 keywords** | "Is `matching` a reserved word? What role does it play?" | `nom-concept/src/strict.rs` lexer |
| **L2 clause shapes** | "Given `kind = property`, what clauses are required and in what order?" | `nom-concept/src/stages.rs` S3+S4 passes |
| **L3 paradigm mappings** | "I'm writing an Erlang-OTP-like supervisor — what Nom shape do I use?" | doc 16 triage table (450+ rows) |

All three layers today have canonical forms in the repo; none is exposed as a queryable artifact. The registry materializes them.

---

## 2. Schema (`grammar.sqlite`)

```sql
-- L1: keyword vocabulary (everything the lexer recognizes as a reserved token)
CREATE TABLE keywords (
  token           TEXT PRIMARY KEY,        -- exact surface form, case-sensitive
  role            TEXT NOT NULL,           -- determiner | kind | clause_opener | quantifier | ref_slot | connective | ...
  kind_scope      TEXT,                    -- NULL = any kind; else JSON array of kinds where valid
  source_ref      TEXT NOT NULL,           -- e.g. "nom-concept/src/strict.rs:L42"
  shipped_commit  TEXT NOT NULL,           -- commit hash where this token first became reserved
  notes           TEXT
);
-- Example rows:
--   ("the",        "determiner",    NULL,                              "strict.rs:L40",  "a04b91e", NULL)
--   ("matching",   "ref_slot",      '["function","data","concept","module","composition","route"]', "strict.rs:L88", "97c836f", "paired with @Kind")
--   ("at-least",   "quantifier",    NULL,                              "strict.rs:L102", "853e70b", "part of 'with at-least N confidence'")
--   ("ensures",    "clause_opener", '["function","property","concept","scenario"]', "strict.rs:L60", "a04b91e", NULL)

-- L2: clause-shape registry (what clauses each kind accepts, in what order, required vs optional)
CREATE TABLE clause_shapes (
  kind            TEXT NOT NULL,           -- function | data | concept | module | property | scenario | media | screen | event
  clause_name     TEXT NOT NULL,           -- intended | requires | ensures | hazard | uses | exposes | generator | favor | composes | ...
  is_required     INTEGER NOT NULL,        -- 0 = optional; 1 = required; 2 = required-at-least-one-of (see `one_of_group`)
  one_of_group    TEXT,                    -- NULL or group-name for mutual-required alternatives
  position        INTEGER NOT NULL,        -- canonical authoring order (for linter + LSP)
  grammar_shape   TEXT NOT NULL,           -- EBNF-ish shape template; see §3
  min_occurrences INTEGER NOT NULL DEFAULT 0,
  max_occurrences INTEGER,                 -- NULL = unbounded
  source_ref      TEXT NOT NULL,
  notes           TEXT,
  PRIMARY KEY (kind, clause_name, position)
);
-- Example rows:
--   ("function",  "intended",  1, NULL, 1, "'intended to' <prose-sentence>",           1, 1,    "stages.rs:L120", NULL)
--   ("function",  "uses",      0, NULL, 2, "'uses the' '@' Kind 'matching' <quoted-prose> 'with at-least' <number-0-to-1> 'confidence' '.'", 0, NULL, "stages.rs:L140", NULL)
--   ("function",  "requires",  0, NULL, 3, "'requires' <prose-precondition> '.'",       0, NULL, "stages.rs:L160", NULL)
--   ("function",  "ensures",   1, NULL, 4, "'ensures' <prose-postcondition> '.'",       1, NULL, "stages.rs:L180", "≥1 required")
--   ("function",  "hazard",    0, NULL, 5, "'hazard' <prose-hazard-note> '.'",          0, NULL, "stages.rs:L200", NULL)
--   ("function",  "favor",     0, NULL, 6, "'favor' <QualityName> '.'",                 0, NULL, "stages.rs:L220", "drawn from QualityName registry")
--   ("property",  "generator", 1, NULL, 2, "'generator' <prose-domain-descriptor> '.'", 1, 1,    "stages.rs:L240", NULL)
--   ("scenario",  "given",     1, NULL, 2, "'given' <prose-precondition> '.'",          1, NULL, "stages.rs:L260", "W47")
--   ("scenario",  "when",      1, NULL, 3, "'when' <prose-action> '.'",                 1, NULL, "stages.rs:L262", "W47")
--   ("scenario",  "then",      1, NULL, 4, "'then' <prose-postcondition> '.'",          1, NULL, "stages.rs:L264", "W47")

-- L3: paradigm → Nom-shape mappings (migrated from doc 16's 450+ rows)
CREATE TABLE authoring_rules (
  row_id          INTEGER PRIMARY KEY,     -- 1-indexed to match doc 16
  source_paradigm TEXT NOT NULL,           -- "Erlang OTP supervisor" | "Coq proof tactic" | "Pony reference capability" | ...
  gap_summary     TEXT NOT NULL,           -- one-sentence description of the source-language construct
  nom_shape       TEXT NOT NULL,           -- the Nom translation pattern (prose, ≤200 words)
  reuses_rows     TEXT,                    -- JSON array of row_ids this rule piggybacks on
  destination     TEXT NOT NULL,           -- 'authoring-guide' | 'wedge:Wxx' | 'deferred'
  status          TEXT NOT NULL,           -- 'queued' | 'closed' | 'blocked' | 'smoke-test-todo' | 'design-deferred'
  closed_in       TEXT,                    -- "doc 14 #85" when status='closed'
  source_doc_ref  TEXT NOT NULL            -- "doc 16 row 419"
);

-- L4: QualityName registry (supports 'favor' clause; ties to M7a required-axes)
CREATE TABLE quality_names (
  name            TEXT PRIMARY KEY,        -- forward_compatibility | numerical_stability | gas_efficiency | ...
  axis            TEXT NOT NULL,           -- registered measurement axis
  metric_function TEXT NOT NULL,           -- nomtu-hash of the metric function in DB2
  cardinality     TEXT NOT NULL,           -- 'any' | 'exactly_one_per_app' | 'exactly_one_per_concept' | ...
  required_at     TEXT,                    -- 'app' | 'concept' | 'function' | NULL
  source_ref      TEXT NOT NULL,
  notes           TEXT
);
-- Seeded with the 10 fixed QualityNames documented in session_2026_04_14_terminal_snapshot.md.

-- L5: kind registry (the closed 9-noun set)
CREATE TABLE kinds (
  name            TEXT PRIMARY KEY,        -- function | module | concept | screen | data | event | media | property | scenario
  description     TEXT NOT NULL,
  allowed_clauses TEXT NOT NULL,           -- JSON array pointing at clause_shapes.clause_name rows
  allowed_refs    TEXT NOT NULL,           -- JSON array of which @Kind typed-slots may appear in 'uses' clauses
  shipped_commit  TEXT NOT NULL,           -- W41 shipped property, W46 shipped scenario, etc.
  notes           TEXT
);
```

---

## 3. Grammar-shape template mini-language

Clause shapes use an EBNF-ish template with **three kinds of placeholders**:

| Syntax | Meaning | Example |
|---|---|---|
| `'literal'` | exact keyword match (case-sensitive) | `'ensures'` |
| `<placeholder>` | free-form prose slot with a named shape | `<prose-precondition>` |
| `'@' Kind` | typed-slot reference (the @ is literal; `Kind` resolves to `kinds.name`) | `'uses the' '@' Kind 'matching' ...` |

Placeholder shapes are themselves rows in a **`placeholder_shapes`** table (not enumerated here; content-addressed on future pass) with grammar hints like "single English sentence ≤200 chars", "floating-point in [0.0, 1.0]", "QualityName from registry", etc.

**Rationale for prose-placeholders not full BNF**: the ambient design (doc 05 §3) is that ≥95% of `.nomx v2` bodies are English prose constrained only by keyword framing. AI clients should get keyword-level precision + prose-slot descriptions, not a micro-grammar for every prose sentence — the prose is where paradigm-plurality lives.

---

## 4. CLI surface (`nom grammar ...`)

```bash
# L1: list all keywords reserved in the given kind scope
nom grammar keywords                             # all keywords
nom grammar keywords --kind property             # only keywords valid inside property decls
nom grammar keywords --json                      # machine-readable

# L2: clause shapes for one kind
nom grammar shape function                       # prints required + optional clauses in order
nom grammar shape property --json                # machine-readable
nom grammar shape scenario --placeholders        # includes the placeholder-shape column

# L3: paradigm lookup
nom grammar rule "Erlang OTP supervisor"         # fuzzy-matches source_paradigm column
nom grammar rule 419                             # direct row_id lookup
nom grammar rule --paradigm "effect handler" --json

# L4: QualityName
nom grammar quality                              # all registered
nom grammar quality forward_compatibility        # one entry with metric + cardinality

# L5: kinds
nom grammar kinds                                # the closed 9
nom grammar kinds --with-allowed-clauses
```

**Invariant — registry is generated, not hand-edited.**
A build-stage script (`nom grammar regen`) walks `nom-concept`, `nom-parser`, and `nom-dict` source and doc 16 to write the tables. Hand-edited rows fail CI. This mirrors the `proc-macro` generation pattern used by `nom-types` for EntryKind + EdgeType.

---

## 5. AI-retrieval flows

### 5.1 LLM authors a new `.nomtu` — needs the function-decl shape

```
LLM: nom grammar shape function --json
→ [
  {"clause":"intended","required":true,"position":1,"shape":"'intended to' <prose-sentence>"},
  {"clause":"uses","required":false,"position":2,"shape":"'uses the' '@' Kind 'matching' ..."},
  {"clause":"requires","required":false,"position":3,"shape":"'requires' <prose-precondition> '.'"},
  ...
]
LLM now emits a valid function decl without guessing.
```

### 5.2 LLM translates Erlang supervisor — needs paradigm mapping

```
LLM: nom grammar rule --paradigm "Erlang OTP supervisor tree" --json
→ [
  {"row_id":419,"gap_summary":"behavioral-module declaration","nom_shape":"structural requirements as 'uses @Data matching ...' + 'uses @Function matching ...' typed slots on concept","status":"closed","closed_in":"doc 14 #85"},
  {"row_id":420,"gap_summary":"boot-sequence coupling via exported callbacks","nom_shape":"'ensures at concept start, X' clauses describing first-activation behavior","status":"closed","closed_in":"doc 14 #85"},
  ...
]
LLM chains the rules into a supervisor concept without re-deriving the paradigm mapping.
```

### 5.3 MCP adapter exposes registry to remote agents

```
tool: nom_grammar_lookup({layer: "clause_shapes", kind: "scenario"})
→ same JSON as above, routed via the MCP adapter shipped in ebe530e.
```

No external API key required — the registry is local SQLite, read by any ReAct adapter.

---

## 6. Migration path

| Phase | Deliverable | Blocker | Effort |
|---|---|---|---|
| **P1** | Schema-only: `grammar.sqlite` empty tables shipped by `nom grammar init` | none | 1 cycle |
| **P2** | L1 populate: extract keyword list from `strict.rs` lexer into L1 rows | none; Rust `proc-macro` walk | 1 cycle |
| **P3** | L2 populate: emit clause-shape rows from `stages.rs` S3+S4 passes | needs `#[derive(ClauseShape)]` macro on the pipeline's step enums | 2 cycles |
| **P4** | L3 populate: parse doc 16's markdown table into L3 rows | markdown-table parser (simple) | 1 cycle |
| **P5** | L4 + L5 populate: extract QualityName + kinds from existing nom-types registry | none | 1 cycle |
| **P6** | CLI subcommand `nom grammar ...` wired to the registry | none | 1 cycle |
| **P7** | MCP + LSP + NomCli adapters expose registry to agents | none (adapters exist) | 1 cycle |
| **P8** | CI check: `nom grammar regen --check` fails on drift between code/docs and registry | needs P2–P5 complete | 1 cycle |

**Total: ~8 small cycles**, ~2 days at the current /loop cadence.

---

## 7. Relation to existing crates + docs

- **Doc 05** (natural-language-syntax) — specifies `.nomx v1` grammar as prose; registry is the executable form.
- **Doc 06** (nomx keyword set) — the closed-keyword table. Registry's L1 must match doc 06 exactly; CI enforces.
- **Doc 07** (keyed-similarity-syntax) — `.nomx v2` typed-slot form. Registry's L2 shapes for `uses` clauses come from here.
- **Doc 08** (layered-concept-component-architecture) — kind set + 3-tier architecture. Registry's L5 matches.
- **Doc 13** (nomx-strictness-plan) — W4 strictness wedges. Registry's L1 must reject anything doc 13 §5 rejects.
- **Doc 16** (nomx-syntax-gap-backlog) — the 450+ authoring-guide rows. Registry's L3 is populated from here via P4.
- **Doc 17** (nom-authoring-idioms) — I1-I20 authoring guide. Registry's L3 cross-references these per row.
- **Doc 19** (deferred-design-decisions) — any row with `status = design-deferred` in L3 links here.
- **`nom-concept` crate** — source of truth for L1 + L2 via P2 + P3.
- **`nom-dict` crate** — does NOT overlap; dict stores nomtu entries, not grammar. Registry is a separate SQLite file.

---

## 8. What this is NOT

- **Not** a replacement for doc 14 translation corpus. Doc 14 is proof-of-coverage; registry is runtime-queryable distillation.
- **Not** a replacement for the compiler's own parser. The parser stays authoritative; registry is a projection for external clients.
- **Not** a dictionary of nomtu entries. Those stay in `dict.sqlite` DB2.
- **Not** a type system. The registry describes what the parser accepts at syntax level; type-checking is the job of `nom-verifier` + contracts.

---

## 9. Open questions

1. **Regen trigger** — should `grammar.sqlite` regenerate on every `cargo build`, or only when an explicit `nom grammar regen` is invoked? Decision deferred to P8; current lean = regen on `cargo build -p nom-cli` via `build.rs`.
2. **Version skew** — how does an older LLM's cached registry cope with a shipped-commit mismatch? Registry carries `shipped_commit` per row; clients can query `WHERE shipped_commit <= their_pinned_commit`. Each P-phase respects this invariant.
3. **Translation table** — do we need per-row locale packs for the gap_summary / nom_shape columns? Deferred until doc 14 expands beyond the English-first corpus.
4. **Fuzzy-match ergonomics** — `nom grammar rule --paradigm "actor"` should match across OTP supervisor, Pony, Elixir, Erlang. Ship a trigram index on L3.source_paradigm per P4.
5. **Per-wedge "before / after" pairs** — when wedge Wxx ships, L1/L2 rows gain a new `shipped_commit`; we may want a separate `grammar_diffs` table that shows "v2.0.0 added keyword 'at-most', clause 'generator', and kind 'property'". Deferred.

---

## 10. Verification criteria

The registry is "done" when:

1. An external LLM can author a valid `.nomtu` file using only (a) a prose intent, (b) `nom grammar shape <kind>` output, and (c) `nom grammar rule --paradigm "<X>"` output — **without reading any .md file in this repo**.
2. The parser and the registry stay in lockstep: a `cargo test -p nom-cli grammar_drift` test fails when any parser change lacks a corresponding registry-row update.
3. Doc 16's 450+ markdown rows all appear as L3 rows; closing a gap in doc 16 updates the row's `status` column; a reverse check (`grammar → doc 16`) confirms symmetry.
4. `nom grammar lookup` returns in ≤50 ms on the full registry (sub-5k rows across all layers).

---

**Next step:** user approval of this design, then P1 scaffold lands in the next cycle. All subsequent wedges (P2–P8) are small and fit the /loop cadence.
