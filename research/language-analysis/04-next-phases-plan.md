# Nom — Next Phases Plan (v2)

**Date:** 2026-04-12 (end of Phase 3)
**Status after Phase 3:** LLVM backend compiles real Nom programs end-to-end (strings, tuples, enums with payload, list[T], builtins). `examples/run_lexer.nom` tokenizes input natively on Windows. 39 commits on `origin/main`, 255 tests, 0 regressions. Self-hosting lexer compiles 17/18 functions.

**Correction from v1:** The original draft misread "minimalism" as trimming the Rust compiler's Cargo.lock. The user's actual intent is a **language-design** philosophy: Nom apps need no external dependency architecture. Apps are closures over a content-addressed dictionary, not dep trees over package managers. v1's Phase 4 (trim deps, feature-gate grammars, embed prelude) is demoted to a half-day cleanup task at the end of this doc — not a roadmap phase.

**Governing philosophy:** *"Enough, no less no more."* Apps today drag in unbounded dep trees — npm, pip, cargo, maven — of which 80% is unused. Nom eliminates the entire category: **the dictionary is the dependency system.** Every function, type, and constant is a content-addressed `.nomtu` entry. An app is a hash closure. No lockfile, no manifest, no `node_modules`.

This doc plans six phases. Each has a **verification hook** against the research docs (`02-fifty-language-analysis.md`, `03-self-hosting-roadmap.md`) and an explicit **size/scope budget**.

---

## Language-model framing — Nom is a two-tier, AI-assisted language

Before the phases, state the governing claim for everything that follows:

**Nom's syntax is two tiers.**

- **Tier 1 — fixed syntax.** Three operators (`->`, `::`, `+`). Sentence-layer keywords (~10). Typed side-table schema (fixed). This tier is tiny on purpose; it's learnable in an afternoon.
- **Tier 2 — dictionary as vocabulary.** Every `.nomtu` entry's `id` (hash) and `word` is a valid source token. The vocabulary is open-ended; target corpus size is 100M+ entries. This tier is the language's expressive power.

**No human can hold 100M entries in working memory.** Therefore Nom is, by construction, an **AI-assisted programming language**. The AI is not an acceleration tool grafted onto an editor — it is the vocabulary bridge between human intent and the dictionary. Without it, composing against a 100M-entry corpus is practically impossible.

**The nomtu as the unit of the language.** The `.nomtu` entry is Nom's atomic unit of meaning — a content-addressed hash + word + contract + body + its edges into the rest of the graph. An app is a composition of nomtu; writing Nom is selecting and chaining nomtu. The dictionary is the reservoir of nomtu a programmer draws from.

**Structural parallel — this kind of scale has worked before.** A natural-language analog with a compact grammar and vast vocabulary already exists: Chinese writing has ~50,000 Kanji; a literate adult works from ~2,000–3,000 as an active vocabulary; composition is mediated by an IME that bridges pronunciation → glyph. Nom uses the same structural pattern — compact grammar, unbounded vocabulary, machine-assisted intent-to-unit lookup — but the unit is the nomtu, not a Kanji. The mechanics carry over; the substance does not.

| Generic pattern | Chinese writing (existing example) | Nom |
|-----------------|------------------------------------|-----|
| Atomic unit | Kanji character | `.nomtu` entry |
| Intent bridge | Pinyin IME | AI intent resolver |
| Working set | ~2,000 familiar characters | User's ~1,000 most-used nomtu |
| Ambiguity resolver | Dictionary lookup | Dictionary is the syntax |
| Composition | Sentences | Apps |
| Fixed / unbounded | Grammar / vocabulary | Grammar / dictionary |

**Authoring model:**

1. Human expresses intent (free-form natural language, partial code, typed signature, concrete example, or some mix).
2. AI queries the intent resolver (§5.4) — returns ranked candidate hashes.
3. Human picks / refines / hash-pins.
4. Source code stored by the `nom store add` path is always hash-pinned (§ Phase 4). The stored artifact is deterministic.

**What this changes across phases:**

- **Phase 5** (ingestion) builds the vocabulary. Every entry it produces is a new syntax token in the language.
- **Phase 8** (architectural ADOPT) needs an embedding-retrieval substrate — BM25 alone is insufficient at 100M scale for "find me something like X." Dense retrieval for intent resolution lives here.
- **Phase 9** (LSP) is promoted from opt-in to **core**. The LSP is the AI-mediated authoring surface, not just code intelligence. Without it, Tier 2 is unusable.
- **New ongoing workstream — Authoring Protocol.** A structured schema for how the AI communicates intent queries and candidate results. Defined as an addition to the LSP in Phase 9.

**Governing invariant (new):**

- **The dictionary is part of the syntax.** Every `.nomtu` hash+word is a valid source token. The fixed-syntax set never grows beyond the three operators and sentence-layer keywords. New expressivity comes from new dictionary entries, not from new keywords.
- **Authoring is AI-mediated discovery at scale.** The language assumes an AI authoring layer exists. A plain text editor can emit valid Nom, but the expected workflow is AI-surfaced vocabulary. The language's learnability is independent of its vocabulary size because the user is never expected to memorize the vocabulary.
- **Determinism survives AI mediation.** Source stored in the dict is hash-pinned. Re-materializing a closure without AI produces byte-identical output to the AI-mediated build. The AI is a *search* surface, not a *generation* surface.

---

## Strategic overview

Sequential, because Phase 4 is foundational and Phase 5 consumes its output.

| # | Phase | Essence | Horizon |
|---|-------|---------|---------|
| **4** | **Dictionary-Is-The-Dependency-System (DIDS)** | Formalize the content-addressed `.nomtu` store as the replacement for every package manager. Hash closures, not dep trees. | 2–3 weeks |
| **5** | **Recursive symbol ingestion with hash rewriting** | Make every ingested source file self-contained in nom-dict by resolving, translating, and hash-rewriting every external reference. No dangling imports. | 4–6 weeks |
| **6** | **Phase 6 prerequisites** | Tuple destructure in let, list concat, keyword-named enum variants. Small backend gaps that block the parser port. | 1 week |
| **7** | **Parser in Nom** (roadmap Phase 2) | Port `nom-parser` (~1600 LOC Rust) to `stdlib/self_host/parser.nom`. Add `Box<T>` for recursive AST. | 10–14 weeks |
| **8** | **Architectural ADOPT** | Supervision trees, aspect markers, Datalog queries, unification inference, persistent collections. | 6–12 months (overlaps) |
| **9** | **LSP + `.nomtu` WASM plugins** | `nom lsp --stdio`, versioned WASM extension API (Zed pattern). | 7 weeks, gated |
| **10** | **Bootstrap** | Nom-implemented compiler compiles itself. | 6–8 weeks |

---

## Phase 4 — Dictionary-Is-The-Dependency-System (DIDS)

**Goal:** Make the content-addressed `.nomtu` store the **only** dependency mechanism for Nom apps. After this phase, SYNTAX.md contains no word like "package", "manifest", or "version range". Apps are hash closures.

### 4.1 Core claim to formalize

An app = a root `.nomtu` hash + the transitive closure of hashes it references.
- The closure **is** the build artifact definition.
- No lockfile, no manifest, no semver range.
- Reproducibility is structural: the closure is fully determined by the root hash.
- Versioning is content-addressing: "new version" means "new hash". Old hash never changes.

This realizes ADOPT-2 (content-addressed function search, Unison) and operationalizes AVOID-5 (no breaking changes to dict entries — they are immutable by construction).

### 4.2 `.nomtu` store protocol

Local store layout:
```
~/.nom/store/
  <hash>/body.nom          # source
  <hash>/contract.json     # pre/post/effects/refs
  <hash>/meta.json         # translation_score, origin, timestamps
```

Rules:
- **Write-once.** A hash, once present, never changes.
- **Remote mirrors** optional. Listed in a per-machine `~/.nom/mirrors.toml` (not per-project, because mirrors are a machine concern, not a project one).
- **GC via root-set.** `nom store gc` removes hashes unreachable from any declared root in `~/.nom/roots.txt`.
- **No fetch-on-miss network access.** If a hash is missing, `nom` fails loud and tells the user which mirror(s) to sync.

### 4.3 Reference syntax in source code

Today Nom code uses bare names (`use foo`). This works inside one repo. For cross-dict composition, introduce two forms:

- **Bare** — `use foo` or `foo()` — resolved at compile time by the nom-dict name-and-context index. Equivalent to today's behavior, but the resolver is explicit.
- **Hash-pinned** — `use #a3f2ef01@foo` — no resolution needed; the hash is the answer. This form always wins.

In compiled artifacts (the translated body stored in the dict), **only the hash-pinned form survives.** Bare names exist only in user-facing source.

Optional sugar: a project can declare `alias react = #a3f2ef01` in a local nomtu; the compiler replaces all bare `react` references with the hash during translation. Aliases are project-scoped, not global.

### 4.4 nom-dict schema adjustments

**Context (2026-04-12):** the previous `data/nomdict.db` (2 GB SQLite) was deleted. Its schema was a **single 53-column table** keyed on `(word, variant, language)` composite — see `nom-dict/src/lib.rs:51-132` and `nom-types/src/lib.rs:383-448` (the pre-deletion NomtuEntry shape). Two problems:

1. **Identity was wrong.** A composite `(word, variant, language)` key means two symbols with the same name in the same language collide even when their ASTs differ. Content-addressed storage needs the hash to be the identity, not a name.
2. **Mixed structural categories.** Scores, contracts, findings, graph metadata, translations, agent metadata, precompiled-artifact paths — all crammed into one wide row with JSON-in-TEXT columns for the multi-valued parts (`audit_findings`, `labels`, `effects`, `depends_on`, etc.). Querying "all Critical security findings across the corpus" required deserializing JSON per row.

**v2 is a normalized + EAV hybrid. Not a single wide table; not pure EAV.** Draft got this wrong (I claimed "~15 columns total" in conversation — that was too minimalist). Actual answer: ~8 tables, ~80 columns across them, each table typed for its query pattern.

### 4.4.1 Design principles (these come before the schema)

**Identity is exactly one column.** `id = hash(canonicalized_ast_body + contract)` on the `entries` table. Nothing else determines identity or equality. If two ingested symbols produce the same canonical AST and contract, they *are* the same entry. If they differ in any semantically-meaningful byte, different hashes.

**Canonicalization is part of the hash contract.** Hash runs over the normalized AST, not source text. Whitespace-only and comment-only differences must NOT change the hash. Semantically-meaningful differences (literal, operator, type annotation) MUST. The canonicalizer is one well-defined function; its output is what gets hashed.

**Structured data stays structured.** If a query pattern is predicable at schema time (score thresholds, severity filters, signature-shape lookups, graph traversals), the data gets its own typed table with typed columns and indexes. This rules out stuffing 8 named score dimensions into a JSON blob.

**Unbounded metadata goes EAV.** Labels, aliases, origin_ecosystem, commit_sha, license_spdx, wasm_target — anything where new facets will appear over time without predicate queries — lives in `entry_meta(id, key, value)`. New facets add rows, not columns.

**One-to-many relations get their own tables.** Security findings (many per entry), graph edges (many per entry), translations (one per target language) are never crammed onto the parent row.

### 4.4.2 The schema (8 tables)

```sql
-- Core entries. One row per content-addressed entry.
CREATE TABLE entries (
    id                   TEXT PRIMARY KEY,        -- hash(canonical_ast + contract)
    word                 TEXT NOT NULL,            -- human-readable name (not identity)
    variant              TEXT,                     -- optional variant tag
    kind                 TEXT NOT NULL,            -- Function|Method|Schema|ApiEndpoint|Ffi|ExternalOpaque|...
    language             TEXT NOT NULL,            -- source language of body
    describe             TEXT,                     -- short human description
    concept              TEXT,                     -- concept hint (auth|crypto|network|...)
    body                 TEXT,                     -- original source (null after Phase 11 nomization)
    body_nom             TEXT,                     -- Nom translation (canonical form)
    input_type           TEXT,                     -- contract: input type expr
    output_type          TEXT,                     -- contract: output type expr
    pre                  TEXT,                     -- contract: precondition predicate
    post                 TEXT,                     -- contract: postcondition predicate
    status               TEXT NOT NULL,            -- Complete|Partial|Opaque
    translation_score    REAL,                     -- 0.0–1.0
    is_canonical         BOOLEAN DEFAULT 1,
    deprecated_by        TEXT,                     -- hash of replacing entry, if any
    created_at           TEXT DEFAULT (datetime('now')),
    updated_at           TEXT
);
CREATE INDEX idx_entries_word ON entries(word);
CREATE INDEX idx_entries_word_variant ON entries(word, variant);
CREATE INDEX idx_entries_kind ON entries(kind);
CREATE INDEX idx_entries_language ON entries(language);
CREATE INDEX idx_entries_concept ON entries(concept);
CREATE INDEX idx_entries_status ON entries(status);

-- Quality scores. 8 named dimensions + overall. One row per entry.
-- Split from `entries` because ingestion often runs before scoring, and we
-- want scoring to not rewrite the big body columns.
CREATE TABLE entry_scores (
    id                   TEXT PRIMARY KEY REFERENCES entries(id) ON DELETE CASCADE,
    security             REAL,
    reliability          REAL,
    performance          REAL,
    readability          REAL,
    testability          REAL,
    portability          REAL,
    composability        REAL,
    maturity             REAL,
    overall_score        REAL
);
CREATE INDEX idx_scores_overall ON entry_scores(overall_score);
CREATE INDEX idx_scores_security ON entry_scores(security);

-- Unbounded metadata (EAV). New facets add rows, not columns.
-- Examples: labels, version_alias, wasm_target, ... (origin/provenance keys
-- were descoped 2026-04-12 — no source_repo/path/line/commit/author/license
-- are recorded; once code is translated to Nom, its origin is intentionally
-- forgotten).
CREATE TABLE entry_meta (
    id                   TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    key                  TEXT NOT NULL,
    value                TEXT NOT NULL,
    PRIMARY KEY (id, key, value)                    -- allow multi-valued keys
);
CREATE INDEX idx_meta_key_value ON entry_meta(key, value);

-- Function signatures. Structured so we can query "find all functions returning String".
-- One row per entry with a signature; entries without signatures (schemas, constants) have none.
CREATE TABLE entry_signatures (
    id                   TEXT PRIMARY KEY REFERENCES entries(id) ON DELETE CASCADE,
    visibility           TEXT,                     -- pub|pub(crate)|private
    is_async             BOOLEAN DEFAULT 0,
    is_method            BOOLEAN DEFAULT 0,
    return_type          TEXT,
    params_json          TEXT                      -- [{"name":"x","type":"i32"},...]
);
CREATE INDEX idx_sigs_return ON entry_signatures(return_type);

-- Security findings. Many per entry.
CREATE TABLE entry_security_findings (
    finding_id           INTEGER PRIMARY KEY AUTOINCREMENT,
    id                   TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    severity             TEXT NOT NULL,            -- Info|Low|Medium|High|Critical
    category             TEXT NOT NULL,            -- injection|secrets|crypto|auth|...
    rule_id              TEXT,
    message              TEXT,
    evidence             TEXT,
    line                 INTEGER,
    remediation          TEXT
);
CREATE INDEX idx_findings_entry ON entry_security_findings(id);
CREATE INDEX idx_findings_severity ON entry_security_findings(severity);
CREATE INDEX idx_findings_category ON entry_security_findings(category);

-- Structural closure refs. "This hash depends on those hashes."
-- Walked by nom build for closure materialization.
CREATE TABLE entry_refs (
    from_id              TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    to_id                TEXT NOT NULL REFERENCES entries(id),
    PRIMARY KEY (from_id, to_id)
);
CREATE INDEX idx_refs_to ON entry_refs(to_id);

-- Semantic graph edges. Distinct from entry_refs: these are typed relationships
-- (calls, imports, implements, similar_to) used by planner and analysis.
CREATE TABLE entry_graph_edges (
    edge_id              INTEGER PRIMARY KEY AUTOINCREMENT,
    from_id              TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    to_id                TEXT NOT NULL REFERENCES entries(id),
    edge_type            TEXT NOT NULL,            -- calls|imports|implements|depends_on|similar_to
    confidence           REAL DEFAULT 1.0
);
CREATE INDEX idx_edges_from ON entry_graph_edges(from_id);
CREATE INDEX idx_edges_to ON entry_graph_edges(to_id);
CREATE INDEX idx_edges_type ON entry_graph_edges(edge_type);

-- Translations. Multiple target languages per entry (rust, typescript, nom itself, c...).
CREATE TABLE entry_translations (
    translation_id       INTEGER PRIMARY KEY AUTOINCREMENT,
    id                   TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    target_language      TEXT NOT NULL,
    body                 TEXT NOT NULL,
    confidence           REAL,
    translator_version   TEXT,
    created_at           TEXT DEFAULT (datetime('now')),
    UNIQUE(id, target_language, translator_version)
);
CREATE INDEX idx_trans_entry ON entry_translations(id);
CREATE INDEX idx_trans_target ON entry_translations(target_language);
```

### 4.4.3 What moved where, from the deleted v1 schema

| v1 (single 53-col table) | v2 location |
|---|---|
| `word`, `variant`, `kind`, `language`, `describe`, `concept`, `body`, `input_type`, `output_type`, `pre`, `post` | `entries` |
| `hash`, `body_hash` | replaced by `entries.id` (the hash) |
| `signature` (JSON-in-TEXT) | `entry_signatures` typed columns |
| `labels` (JSON array) | `entry_meta` rows |
| 8 scores + `overall_score` | `entry_scores` |
| `audit_passed`, `audit_max_severity` | `entries` columns OR denormalized views over findings |
| `audit_findings` (JSON array) | `entry_security_findings` rows |
| `rust_body`, `translate_confidence` | `entry_translations` row with `target_language='rust'` |
| `community_id`, `callers_count`, `callees_count`, `is_entry_point` | denormalized view over `entry_graph_edges`, recomputable — don't store |
| `bc_path`, `bc_hash`, `bc_size` | `entry_meta` rows (precompiled-artifact paths aren't identity) |
| `capabilities`, `supervision`, `schedule` | `entry_meta` rows until agent metadata stabilizes into its own typed table |
| `version`, `tests`, `is_canonical`, `deprecated_by`, `created_at`, `updated_at` | `entries` columns (canonical/deprecated/timestamps) or `entry_meta` (version string) |
| `depends_on` (JSON array) | **split**: structural hash-refs → `entry_refs`; semantic edges → `entry_graph_edges` |

### 4.4.4 Why not pure EAV (correcting earlier draft)

An earlier draft of this section proposed `entries + entry_meta + entry_refs` only (3 tables, ~15 columns). That was wrong because:

- **Score thresholds are hot.** `WHERE security > 0.9 AND reliability > 0.8` on EAV requires two self-joins per predicate. On the current corpus scale (tens of millions of atoms eventually), that's unusable.
- **Security findings need category+severity filtering.** Pure EAV would store `(id, 'finding.0.severity', 'Critical'), (id, 'finding.0.category', 'injection'), ...` — filtering "all Critical injections" across the corpus would require an anti-pattern self-join.
- **Graph walks need typed edges.** Transitive `calls` closure requires an indexed `(from_id, edge_type)` column — not available in EAV.
- **Signatures need shape queries.** "Find all functions taking `NomString -> Result<T, E>`" requires structured return_type/params columns.

EAV is right for the **unbounded long tail** of metadata. Normalized tables are right for the **bounded, query-hot** structured subsets. v2 combines both.

### 4.4.5 No migration from v1

The composite-key v1 identity doesn't map to v2's hash identity. There's no safe automatic rewrite. Rebuild by re-running Phase 5 ingestion. User deletion of the old DB on 2026-04-12 reflects this decision.

### 4.4.6 Body-as-compiled-artifact — the dict is a binary cache, not a Nom-source store (architectural shift, 2026-04-12)

**The dict holds compiled artifacts. `.nom` is Nom's only source form. No Nom AST or Nom text lives in the dict.**

This supersedes earlier framings in §5.11 and §5.16 that spoke of `body_nom` as a declarative Nom description of UX or media. Those framings are replaced — their structural-analysis content survives as metadata/edges, but the body itself is bytes.

**Canonical body representations by kind:**

| Kind family | Body content | Why |
|---|---|---|
| Executable code (`Function`, `Method`, `Module`, …) | **LLVM bitcode** (`.bc`) | Every source language that feeds Nom already has a compiler that emits LLVM IR (rustc, clang, clang++, tsc via llvm-wasm, …). Store the output, not the input. |
| Still image (`MediaUnit` where visual) | **AVIF** bytes | AV1-based still image; best modern ratio; royalty-free. |
| Video (`MediaUnit` where motion) | **AV1** bytes (muxed in Matroska/MKV or ISOBMFF/MP4) | Best modern video codec; royalty-free. |
| Audio, lossy (`MediaUnit` where speech/general audio) | **AAC** bytes | Universal hardware decode; ubiquitous. |
| Audio, lossless (`MediaUnit` where archival) | **FLAC** bytes | Mature, royalty-free, lossless. |
| 3D mesh, font, vector, document, …  | Each its own canonical format (glTF for 3D, WOFF2 for fonts, PDF for documents, …) — one per media family | Same principle: pick one canonical; transcode to others on render. |

**Invariants (added to the language-model invariant list):**
- **Invariant 15 — Body is bytes.** `body` in the dict is a compiled artifact. Never Nom source. Never an AST.
- **Invariant 16 — `.nom` is the only Nom source.** Nom grammar only exists in user-authored `.nom` files on disk. The dict cannot be read as Nom code; it can only be consumed by the compiler/renderer as bytes.
- **Invariant 17 — One canonical format per modality.** Image → AVIF. Video → AV1. Audio lossy → AAC. Audio lossless → FLAC. Code → `.bc`. Alternative encodings (PNG, WebP, MP3, WAV, native `.exe`, wasm) are `Specializes` variants produced on render/build, not stored as primary bodies.

**What this changes vs. the earlier plan:**

1. **§5.2 equivalence gate redefinition.** Instead of "translate source → Nom → retranslate → compare," the gate becomes:
   - For code: compile source → `.bc`; re-run the same compilation from the stored artifact's ingestion-metadata → byte-compare (stripped of non-deterministic metadata per §10.3.1 pins).
   - For lossless media (FLAC, AVIF-lossless, PNG-of-origin): decode → encode back to canonical → byte-compare.
   - For lossy media (AV1, AAC, JPEG-of-origin, Opus): decode → perceptual-hash compare within declared tolerance.
2. **§5.10 canonicalizer redefinition.** Operates on `.bc` (LLVM normalization passes + debug-info strip + symbol sort) or on canonical-format bytes (re-mux, re-encode at fixed settings). Not on Nom AST — there is no Nom AST in the dict to canonicalize.
3. **§5.11 body-representation framing is superseded.** The declarative `Screen` / `UserFlow` / `UxPattern` / `DesignRule` / `Skill` kinds still exist, but their `body` is the compiled `.bc` of the UI component (e.g., a compiled Dioxus `rsx!` function) — NOT a Nom declarative widget tree. The declarative decomposition survives as **metadata + edges** (e.g., `Screen` → `InteractsWith` → `Button` is an edge between two hashes whose bodies are `.bc`; the relationship is recorded, the body isn't re-expressed in Nom). See §5.11.7 below.
4. **§5.16 body-representation framing is superseded.** `PixelGrid` / `AudioBuffer` / `VideoStream` / `VectorPath` etc. DO NOT exist as declarative Nom bodies. They exist as **analysis kinds** that record "this AVIF decodes to a 3000×2000 sRGB PixelGrid" as edges + metadata on a body whose actual content is the AVIF bytes. Decoding pulls the bytes through a codec `.bc` at runtime. See §5.16.14 below.
5. **§5.16.11's Tier 2 (pure-Nom codec rewrite) is removed.** Codecs are `.bc` forever. Ingesting rav1e/dav1d/fdk-aac/libFLAC/libavif produces codec `.bc` in the dict; there is no eventual rewrite to Nom.
6. **§5.17.4's Partial/Complete semantics tightens.** A `.bc` either compiled cleanly and passes its equivalence gate (`Complete`) or it didn't (`Partial` = compiler errored on ingestion / decoder failed / lossy round-trip exceeded tolerance). The "translator is immature" reason for Partial disappears because translators don't exist — only compilers do, and they're mature.
7. **§10.3.1 fixpoint proof unchanged in shape** but its inputs shift: the compiler's source is `.nom` files; compiled to `.bc` via the compiler itself. `s2 == s3` means the compiler's OUTPUT `.bc` hashes are byte-identical across stages — exactly the property the pins (§10.3.1 prerequisites) were designed to ensure.

**What this preserves:**
- Content addressing by hash. Still hash-by-bytes; bytes are just a different thing.
- Closure walking by `Calls`/`Imports`/`DependsOn`/`Specializes` edges. The graph structure is unchanged.
- `Specializes` per platform (x86_64-`.bc` vs wasm32-`.bc`; AVIF vs WebP vs PNG specializations).
- The two-tier language model (§language-model framing): fixed 3-operator syntax in `.nom` files + dictionary of entries. The dictionary entries are still addressable by word + hash; the body just isn't Nom source.
- §5.17 mass corpus ingestion mechanics (stream-and-discard, skip-list, checkpointing). Only the *output* of ingestion changes (`.bc` + media bytes instead of translated Nom source).

**What this loses (honestly named):**
- **Cross-language semantic dedup at the body level.** Rust's `fn sha256` compiles to different `.bc` than C's `sha256()` even if they implement the same algorithm. They won't hash-collide. Semantic equivalence is recorded via `ContractMatches` edges (still supported) after explicit §5.2 gate runs — not via automatic hash collision. This is a capability reduction vs. the earlier "canonical-Nom-AST hashes the same" claim.
- **AI-mediated body inspection.** The AI authoring layer (§5.19) can read a body's signature, contract, metadata, edges — but not its Nom source, because there isn't any. For inspecting behavior it must either decompile `.bc` (llvm-dis, llvm-cxxfilt) or read ingestion-source-metadata pointers (if any survived ingestion — they may have been descoped 2026-04-12; see the provenance-removal note).
- **Declarative composition of UX/media as Nom code.** You can't write a `.nom` function that composes two `Screen` nomtu into a larger screen by operating on their AST — the ASTs don't exist. You *can* reference them by hash and let the compiled UI runtime compose them at runtime (normal Dioxus component composition, just with hash-referenced imports). Authoring composition happens in `.nom` files, not across dict bodies.
- **§5.18 "aesthetic as programming".** The claim that "media primitives (§5.16) are programmable surfaces composed via the 3 operators" becomes: `.nom` code composes function CALLS into media-producing runtimes (e.g., call into a codec's `.bc` to encode a procedurally-generated frame). The surface is programmable via `.nom`; the media bytes in the dict are not programmable themselves.

**Verification against the shift:**
- **Dict compactness preserved.** A compiled `.bc` is often smaller than the source it came from (LLVM bitcode is tight). Average entry size estimate (§5.17.5) may shrink modestly.
- **Hash stability across compiler upgrades is now a harder problem.** When LLVM 18 → LLVM 19, the `.bc` format changes subtly; stored bodies may not be consumable. Mitigation: pin LLVM in `rust-toolchain`-equivalent pins (same discipline as §10.3.1), and ship a `nom store recompile` sweep on LLVM upgrade — akin to the canonicalizer-upgrade sweep in §5.10.1 but over the compiler, not the canonicalizer.
- **Building against the dict remains LLVM-native.** `nom build <.nom_file>` parses Nom, resolves `use` clauses to hashes, loads `.bc` from dict, links, emits target. Exactly one compile step (the user's own `.nom`); everything else is cached bitcode.

**No migration.** As with §4.4.5, the dict is wipe-and-rebuild under this shift. Ingestion pipelines are rewritten; §5.11 and §5.16 subsections are superseded on body-representation but retained for their kind/edge/metadata models (see callouts at §5.11 and §5.16 headers).

**Current implementation state (audit 2026-04-13) — invariant 15 is tagged, not yet migrated.** The dict schema has a `body_bytes` column ([nom-dict/src/lib.rs:47](../../nom-compiler/crates/nom-dict/src/lib.rs#L47)) and compile paths tag successful precompiles with `body_kind = "bc"` ([nom-cli/src/main.rs:3067-3074](../../nom-compiler/crates/nom-cli/src/main.rs#L3067-L3074)), but the `.bc` bytes still live on disk at `artifact_path`; nothing currently writes the file contents into `body_bytes`. Invariant 15 ("body is bytes, never Nom source, never an AST") therefore holds symbolically (the tag says it) but not physically (the bytes are off-row). The bitcode-into-body migration is the next concrete step to make §4.4.6 observably true: on successful precompile, read the `.bc` bytes, write them into `body_bytes`, and switch the build-by-hash load path from disk-via-`artifact_path` to in-row-via-`body_bytes`. A post-migration CI invariant check `SELECT COUNT(*) FROM nomtu WHERE body_kind = 'bc' AND (body_bytes IS NULL OR LENGTH(body_bytes) = 0) == 0` is the forcing function.

### 4.5 Verification against the schema principles

Mandatory property tests that must pass before v2 ships:

- **Whitespace invariance.** Ingest `fn f(x: int) -> int { x + 1 }` and `fn f(x:int)->int{x+1}`. Same `id`.
- **Smallest-difference sensitivity.** Ingest `fn f(x: int) -> int { x + 1 }` and `fn f(x: int) -> int { x + 2 }`. Different `id`s.
- **Dedup on ingest.** Ingest the same symbol twice from two different upstreams. One `entries` row. No provenance stored (origin is intentionally forgotten post-translation, 2026-04-12).
- **EAV correctness.** Adding a brand-new facet like `('quantum_safe', 'true')` requires zero `ALTER TABLE`.
- **Typed-table correctness.** Query `WHERE security > 0.9 AND EXISTS (SELECT 1 FROM entry_security_findings f WHERE f.id = entries.id AND f.severity = 'Critical') = 0` must run under 100 ms on a 1-million-entry corpus with the declared indexes.
- **Closure walk correctness.** For any `id`, recursive walk via `entry_refs` terminates, returns every reachable hash, and respects cycle termination via memoization.

The full identity property: for any two canonical ASTs `A`, `B`, `hash(A) == hash(B) ⟺ A == B` structurally.

### 4.5 CLI surface

Five subcommands, all additive (no existing command changes):

- `nom store add <path>` — ingest one file into the store, compute its hash, write entries.
- `nom store get <hash>` — print body + contract.
- `nom store closure <hash>` — list the transitive hash DAG as newline-separated hashes. Piped into `nom build` or `nom pack`.
- `nom store verify <hash>` — check every transitive ref is present and Complete.
- `nom store gc` — collect unreachable entries.

Plus a refactor of the existing `nom build`:
- `nom build <hash>` — materialize closure, compile, emit artifact.
- `nom build <source.nom>` — current behavior (compile-in-place) is just sugar for `nom store add <source.nom> && nom build <hash>`.

### 4.6 Deliverables

- Store protocol spec in `docs/store-protocol.md`.
- `nom store` subcommand wired into nom-cli.
- nom-dict schema v2 migration + backfill.
- Reference resolver in nom-parser: bare names → hashes via nom-dict; hash-pinned passthrough.
- Closure walker in a new tiny crate `nom-closure` (or as a module in nom-dict).
- **Test:** a 3-entry toy app — `main` calls `greet` calls `format`. Each lives in the store with its own hash. `nom build <main-hash>` resolves closure and emits a working executable. Closure size = 3.

### 4.7 Verification against research docs

- **ADOPT-2** fully realized (content-addressed search by contract shape).
- **AVOID-5** operationalized (dict entries immutable — hash change = new entry).
- **AVOID-10** (dep version resolution nightmares) structurally impossible — there are no versions.
- No new Rust crate deps introduced.

### 4.8 Size/scope budget

- ~600 LOC across nom-cli (store subcommand), nom-dict (schema v2 + migration), nom-parser (resolver).
- No new Rust external deps. No new tree-sitter grammars.
- Tests: +10 (store CRUD + closure walk + resolver + migration).

---

## Phase 5 — Recursive symbol ingestion, body-only translation, multi-edge graph (revised)

### 5.0 Three refinements from 2026-04-12

1. **The dict stores only `body_nom`.** No original-language source is retained. Translation is the ingestion output. If we can't translate a symbol to Nom equivalent-strong enough to pass contract-tests, the entry is stored with `status: Partial` and `body_nom` may be null — never the original language's source. Archival of original source, if desired, is a **separate concern** outside the dict.
2. **Dependencies = typed multi-graph edges.** A `body_nom` is a minimal self-contained unit. Every external reference lives in `entry_refs` (structural hash closure) plus `entry_graph_edges` (typed semantic relationships). Multiple edge types can exist between the same pair of hashes. The dependency system IS the multi-graph.
3. **Nomtu at corpus scale.** Target: 100M+ entries. An app is a composition of nomtu; writing Nom is selecting and chaining them. At this size intent resolution is an IR problem, not a symbol-table problem. (The structural pattern — compact grammar + vast unit inventory — is the same one that lets humans write Chinese at ~50,000-Kanji scale using ~2,000 active; Nom's unit is the nomtu, not a Kanji.)

*Supersession note (2026-04-12, §4.4.6):* refinement #1 above said `body_nom` is the canonical body. Post-§4.4.6 shift, the canonical body is **compiled bytes** (LLVM `.bc` for code, AVIF/AV1/AAC/FLAC bytes for media). `body_nom` remains a per-row column for back-compat but is no longer the primary body representation. See §4.4.6 for the invariants and §5.11 / §5.16 callouts for impact on UX + media body framings.

#### 5.0a Scaffolding status (2026-04-12 late, this-session)

Phase-5 introduces six new workspace crates. All six are **scaffolded**; `nom-media` additionally has the full **§5.16.13 codec roadmap landed** (10 of 10 entries). Status:

| Crate | § | Scaffolded | Functional work |
|---|---|---|---|
| `nom-media` | §5.16 | ✅ | ✅ All 10 §5.16.13 codecs landed (PNG/FLAC/JPEG real re-encode; Opus/AVIF/AV1/AAC identity-mapped; WebM/MP4 muxers; HEVC decode-only). 56 tests. Pending: real encoders for the 4 identity-mapped codecs, `nom media render` CLI, §5.16.11 FFI tier if/when needed. |
| `nom-ux` | §5.11 | ✅ | platform extractors (react/vue/etc.), Dioxus specialization edges |
| `nom-corpus` | §5.17 | ✅ | per-ecosystem drivers, checkpointing, bandwidth throttling |
| `nom-bench` | §5.13 | ✅ | runner + typed side-table storage |
| `nom-app` | §5.12 | ✅ | manifest parser + builder per target |
| `nom-flow` | §5.14 | ✅ | LLVM call-site instrumentation + DOT/Mermaid render |

Shared type surface carrying these crates (in `nom-types`, also scaffolded this session):
- `EntryKind`: 29 variants — 12 original + 17 added for UX / app-composition / bench / flow / media.
- `EdgeType`: 28 variants — 5 original + 23 added for UX + cross-platform + flow + media graph + lifecycle.
- `body_kind`: 9 constants per §4.4.6 invariant 17 canonical formats.
- All three surfaces have `const fn` accessors for zero-cost downstream consumption.

**Does not include:** commits, real codec code, real extractors, real bench runner, real corpus ingestion — those are per-iteration follow-ups once the scaffolding PR lands on `main`.

### 5.1 Body-only storage — what changes

**Before (v1 of this doc):** entries stored both `body` (original) and `body_nom` (translation), with a `translation_score` tracking progressive improvement; Phase 11 would later retire `body`.

**Now:** entries store ONLY `body_nom`. Original source never enters the dict. The translator is the ingestion boundary; if it can't produce an equivalence-checked Nom body, the entry is incomplete.

Schema adjustment to entries table:
- Remove `body TEXT` column entirely.
- Keep `body_nom TEXT` — required for `status: Complete` entries; nullable for `Partial` and `Opaque` (opaque = pure contract boundary, native FFI).
- `translation_score` stays, but its semantic tightens: 1.0 means "equivalence test passed"; any score < 1.0 means the entry is `Partial` and not eligible for Complete closures.

**Consequence:** massive structural dedup. A React component written in JavaScript, TypeScript, Flow, and CoffeeScript — if they translate to identical Nom AST + contract — collapse to one hash. Cross-ecosystem dedup becomes inherent, not a Phase-12 specialization afterthought.

**Consequence:** Phase 11 (nomization) as a distinct phase **is removed**. Its goal — fully-native dictionary over time — is now the default invariant of Phase 5. Nothing to retire later.

### 5.2 Equivalence gating — the translation is the contract

A symbol is translated iff we can prove its Nom body behaves the same as the original under the inferred contract. The gate:

1. Translate symbol → candidate `body_nom`.
2. Generate property tests from the inferred contract (ADOPT-1 — this is where property testing earns its keep). Inputs synthesized against `pre`; outputs checked against `post` + effect profile.
3. Run both candidate and original on the same inputs in a sandbox; compare outputs bitwise + effect traces.
4. If all pass → `status: Complete`, `translation_score: 1.0`. Store `body_nom`.
5. If some fail → `status: Partial`, store `body_nom` with a `translation_score < 1.0` and a `partial_reasons` metadata entry explaining what diverged.
6. If translation itself fails (parser error, unrepresentable construct) → `status: Partial`, `body_nom: null`. Entry exists as a placeholder the graph can refer to but never execute from.

The original source is **still needed at translation time** (you can't translate what you can't read), but it's a transient input — read from the ecosystem cache, translated, verified, discarded. Nothing enters the dict except the Nom body + the contract + the graph edges.

### 5.3 Multi-edge relationship model

A single `(from_id, to_id)` pair can have multiple edge types. The `entry_graph_edges` table already supports this (primary key is `edge_id`, not `(from, to)`). Edge-type catalog grows as semantic needs surface:

| Edge type | Semantics | Used when |
|-----------|-----------|-----------|
| `Calls` | Runtime invocation. `to_id` is executed by `from_id`. | Function calls, method dispatch. |
| `Imports` | Lexical import. `from_id`'s translation references `to_id`'s name in scope. | `use` statements; `import` in source. |
| `Implements` | `from_id` satisfies the trait/interface defined by `to_id`. | impl-for-trait relationships. |
| `DependsOn` | Build-time requirement — `to_id` must exist for `from_id` to be buildable, even if never called. | Compile-time macros, type definitions referenced in signatures. |
| `SimilarTo` | Semantic neighbour (for search, for auto-suggestion). Not a build dependency. | Near-duplicate detection, alternative-implementation surfacing. |
| `ContractMatches` | `from_id`'s contract is compatible with `to_id`'s input contract. Derived, not authored. | Auto-composition; the compiler can substitute one for the other. |
| `SupersededBy` | `from_id` is deprecated; `to_id` is the canonical replacement. | Versioning without version numbers. |
| `Specializes` | `from_id` is a monomorphized instance of `to_id` for specific types. | Phase 12 outputs — cached globally. |

Task B's resolver currently emits `entry_refs` (structural closure) only. Phase 5 extends ingestion to emit the typed edges above. A `.nomtu` entry's full dependency picture is the multi-graph restricted to its outgoing edges.

**Consequence:** a closure walk on `entry_refs` gives you the build-materialization set; a closure walk on `entry_graph_edges` filtered by edge type gives you whatever question you're asking (runtime calls? compile-time deps? semantic neighbours?). The dict answers all of these from one store.

### 5.4.0 Resolver architecture — similarity + relevance, no normalization across candidates

Before the ranking-function details in §5.4, the underlying architecture deserves a statement. The naïve design — "score every candidate against every signal, softmax-normalize, pick top-K" — does not survive 100M candidates. Softmax forces cross-candidate competition: top-1 wins because others are pushed down, not because it's genuinely a good match. At corpus scale this is both computationally infeasible and semantically wrong (a "None of these are relevant" answer is unrepresentable — softmax redistributes a fixed unit mass).

Adopted architecture (adapted from the "Screening Is Enough" line of work on attention without cross-key normalization):

**Two layers, distinct units.**

1. **Similarity layer.** Bounded, per-signal, query-vs-candidate dot product in [-1, 1]. Computed independently per signal (word match, signature compatibility, contract compatibility, concept match, label overlap, embedding cosine, caller proximity). Each signal is a normalized bounded number — no softmax, no cross-candidate mixing.

2. **Relevance layer (screening).** Per-candidate independent threshold applied to each signal. Output in [0, 1]. A candidate with relevance 0.01 on the embedding signal is genuinely irrelevant, not merely "less relevant than others." Trim-and-square or similar per-key transform: `relevance_ij = max(1 − r_i × (1 − similarity_ij), 0)²` where `r_i` is a learned cutoff per signal type.

**Consequences:**

- **Absolute relevance is representable.** Resolver can legitimately return `NotFound` when every candidate's composite relevance is below threshold, instead of being forced to pick a top-K from a pool of junk. This matches §5.4's "unique / ambiguous / not found" output classes cleanly.
- **Scaling is linear in most signals.** Exact-word, kind, language, origin-ecosystem matches are O(indexed lookup). Embedding cosine and BM25 scans go through screening tiles with learned windows — short-window tiles scale O(w), full-context tiles scale O(N) only when genuinely needed. Typical query touches O(w_word + w_signature + w_concept) ≈ a few thousand candidates, not 100M.
- **Independent key contribution.** Two candidates with distinct shapes both reaching threshold both get non-zero relevance. The resolver doesn't need to pick one under a competitive forcing function; it can return both, or return neither. This is what ambiguity detection structurally requires.
- **Stability under authoring iteration.** In the AI-compiler loop (§5.19), small intent changes (add a constraint hint, narrow a type) produce proportional changes in per-candidate relevance rather than all-or-nothing reshuffling. Author feedback is actionable.

Calibration: signal thresholds `r_i` and the composite-score function are learned from the §5.19 AuthoringTrace corpus — each authoring session's accept/reject pattern is supervised signal. The resolver gets better from real use.

This architecture replaces the older "ranking function" framing in §5.4 as a numbered list of signals. The signals are the same; the architecture of how they combine is the screening-per-candidate pattern, not softmax-across-candidates.

---

### 5.4 Intent decoding at 100M scale

Writing `use greet` against a dict with thousands of `greet`s must produce exactly one deterministic hash. The resolver is a ranking function:

**Resolution signal set** (scored in this order, each layer narrows):

1. **Exact word match.** `word = "greet"` AND `variant = <if specified>`. First cut.
2. **Language compatibility.** If the caller's `body_nom` dialect matches the candidate's origin_language. Most entries will be Nom-native and co-rank equally here; cross-language candidates only matter when translation originated elsewhere.
3. **Signature compatibility.** Input/output types must unify with the call site's expected shape. From Task B we have signature data in `entry_signatures` — query by `return_type` and param arity at least.
4. **Contract compatibility.** `pre`/`post` must not contradict the call site's known state. This is structural matching against caller-declared preconditions.
5. **Concept proximity.** `concept` label (auth, crypto, http, render, …) must match or be a superset of the caller's concept context.
6. **Label overlap.** Union of `entry_meta` `label` rows, weighted by frequency.
7. **Semantic similarity.** For ties: embedding-based rank from nom-search (BM25 today; dense retrieval in Phase 8).
8. **Caller context.** Closer-origin entries rank higher — same package, same author, same commit — using transitive `SimilarTo` edges as the proximity metric.

Output: one of
- **Unique resolution.** Top-1 wins by a configurable margin (default: top score must exceed runner-up by ≥ 20%). Return the hash.
- **Ambiguous.** Return structured error with ranked candidate list; CLI asks user to hash-pin.
- **None.** Return `NotFound` with the nearest-neighbor suggestions from `SimilarTo`.

**Intent hints in source.** Beyond `use <name>`, source can narrow intent:
- `use <name> : <return_type>` — pre-filter by signature.
- `use <name> :: <concept>` — pre-filter by concept.
- `use <name> # <metadata_query>` — arbitrary EAV filter (`# ecosystem=npm`, `# wasm_safe=true`).
- `use #<hash>@<name>` — bypass resolver, absolute pin.

Intent hints never change the hash of the caller (they're resolution-time directives). But they can be the difference between deterministic resolution and ambiguous failure in a dict with 50k candidates.

### 5.5 Nomtu-at-scale data properties

At 100M entries:

- **Storage.** Assume average 200 bytes of `body_nom` per entry + ~80 bytes of metadata + ~20 bytes of edges per entry. Total: ~30 GB on disk for the v2 schema. SQLite handles this fine with page size 4KB + mmap + WAL. If ever we outgrow SQLite, the schema transliterates cleanly to Postgres/RocksDB — the EAV + typed tables + content-addressed key model is portable.
- **Reads.** Intent resolution should be < 5ms at p95 on a cold cache. Required: dense indexes on `(word)`, `(return_type)`, `(concept)`, `(key, value)`, plus a BM25 inverted index on `describe` + `label` union. Covered by Tasks A/B + nom-search (already exists).
- **Writes.** Ingestion is bulk + idempotent (hashes dedup). Target: 10,000 entries/sec on a single-threaded `upsert_entry` path, more with transactions batched by hundreds.
- **Memory.** Resident set when serving a single build should be bounded by the closure size (typically 1k-10k hashes), not by the corpus. Streaming queries, no whole-dict loads.

**The model made concrete.** Nom's *grammar* (three operators + sentence layer) stays fixed; the *vocabulary* — the nomtu corpus — scales to 100M+ entries without grammar changes. Each user writes from a personal working vocabulary of ~1,000 most-used nomtu, resolved against the global corpus by intent, not by memorization. IDE autocomplete becomes a dictionary lookup in the literal sense — same structural pattern that makes Chinese writing tractable at its scale, but the unit is the nomtu.

### 5.6 Recursive symbol ingestion protocol (revised)

Same 5-step protocol as before, tightened by the three refinements:

1. **Parse + scan imports** from an ecosystem source file (tree-sitter; no source stored).
2. **Resolve from local ecosystem cache** — unchanged from prior spec.
3. **Recurse with memoization** — unchanged; content-hash dedup terminates on repeated work.
4. **Translate + verify equivalence → emit Nom body.** Translation is the boundary. Gate per §5.2. Store only the result.
5. **Emit multi-edge graph** — derive `Calls`, `Imports`, `DependsOn`, etc., from the translated AST. Populate `entry_refs` (structural) and `entry_graph_edges` (typed).

The ecosystem source file is read once, translated once, and never persisted. What ends up in the dict is purely Nom.

### 5.7 Deliverables (revised)

- Remove `body TEXT` column from `entries` table. Migration is a DROP COLUMN on SQLite 3.35+ or a rebuild (trivially — the dict is still small).
- `nom-extract/src/translate/` — per-language translators now mandatory (Rust → Nom, TypeScript → Nom, Python → Nom first pass). Each gets an equivalence-verification harness.
- `nom-extract/src/resolvers/` — unchanged (read ecosystem caches).
- `nom-extract/src/graph.rs` (new) — derives typed edges from a translated AST. Edge catalog in §5.3.
- `nom-resolve/src/intent.rs` (new, or extension of existing nom-resolver v2) — the ranking function per §5.4.
- New `use` syntax variants: `use foo : type`, `use foo :: concept`, `use foo # key=value` — parser extension in nom-parser; AST gains `UseStmt.constraints`.
- Intent test suite: seeded dict with 100 candidate `greet`s varying by signature/concept/labels, assert resolver picks the right one per test case.
- Scale benchmark: generate 1M synthetic entries, measure resolution latency p50/p95/p99. Commit the benchmark script and numbers to `benches/`.

### 5.8 Maps to research

- **ADOPT-1** (QuickCheck from contracts) — Phase 5 is now its first real customer; §5.2 equivalence gates use it.
- **ADOPT-2** (content-addressed search by contract shape) — §5.4 is the full realization.
- **ADOPT-10** (structural interface satisfaction) — §5.3 `ContractMatches` edges make this queryable.
- **Vietnamese classifiers** — §5.4 intent hints (`: type`, `:: concept`, `# key=value`) are Nom's classifier system generalized.

### 5.9 Size budget (revised)

- Translators: ~1500 LOC per mature ecosystem (Rust, TypeScript, Python). Ship incrementally, one at a time.
- Equivalence harness: ~800 LOC + sandboxed-execution substrate (reuse nom-runtime).
- Intent resolver: ~1200 LOC including the ranking function, signature/contract matchers, and the fallback SimilarTo walker.
- Scale benchmarks: ~500 LOC.
- Lifecycle ops (§5.10): ~1000 LOC across merge/evolve/gc commands and their tests.
- Zero new external Rust deps; may add an embedding library if dense retrieval lands in Phase 8 (not Phase 5).

### 5.11 UX as a first-class unit in the dictionary

> **Superseded on body-representation by §4.4.6 (2026-04-12).** This section's references to declarative Nom `body_nom` describing screens/widgets are no longer accurate: UX entries' bodies are `.bc` compiled from Dioxus/React/etc. The kinds, edges, and metadata models (`Screen`, `UserFlow`, `UxPattern`, `Styles`, `InteractsWith`, `TransitionsTo`) remain as **analysis metadata** attached to the `.bc` body — they record relationships between compiled UI components, not declarative widget trees.

The `.nomtu` corpus is not just function bodies. A complete program has user-facing behavior: screens, flows, interactions, accessibility affordances, visual style. These are as much "the app" as the code is. Phase 5 extends to extract UX logic alongside code during ingestion and seed the dictionary with a curated UX knowledge base.

#### 5.11.1 New kinds and edges for UX

Extend `EntryKind`:
- **`UxPattern`** — a design pattern, interaction rule, accessibility requirement, motion/timing rule, or layout heuristic. Body (when present) may be illustrative code or a declarative rule in Nom.
- **`DesignRule`** — a context → recommendation mapping. Example: `product_type=SaaS → recommend(Glassmorphism, FlatDesign); animation_timing=subtle→200–250ms`. Body is a declarative rule.
- **`Screen`** — a complete user-visible surface, composed of components + interactions + styles. Think React page, Vue view, SwiftUI screen.
- **`UserFlow`** — a sequence of screens + interactions achieving a user goal. E.g., "sign up and verify email", "checkout", "recover password."

Extend edge types (added to `entry_graph_edges`):
- **`Styles(from, to)`** — `from` (a Screen/component) is styled by `to` (a UxPattern or DesignRule).
- **`Constrains(from, to)`** — `from` (an accessibility rule) constrains `to` (a component).
- **`Recommends(from, to)`** — `from` (a DesignRule) recommends `to` (a UxPattern) in matching contexts.
- **`InteractsWith(from, to)`** — `from` (a Screen) has an interaction with `to` (another Screen or a backend Function).
- **`TransitionsTo(from, to)`** — `from` (a Screen) can navigate to `to` (a Screen) in a UserFlow.

A UX-aware closure walk on `entry_graph_edges` filtered by `Styles | Constrains | Recommends` returns the visual/interaction context of any Screen. A developer asking "what styles apply here?" gets a structured answer from the graph.

#### 5.11.2 Seed the dictionary with design-knowledge imports

External UX knowledge corpora ship structured data we can ingest as nomtu. The general command is:

```
nom ux seed <path-to-corpus>
```

Nothing in the resulting dictionary carries the external resource's brand name. Seeded entries get **native Nom descriptive names** following the project's naming conventions (lowercase, underscore-separated, concept-first). The external source's identity is preserved only in metadata (`entry_meta` rows tagged `origin_corpus=<source-id>`) for provenance, not in the entry's `word` field.

**Two corpus shapes to support, two importer patterns.**

**Shape A — static knowledge corpus (CSV/JSON).** Example at `C:\Users\trngh\Documents\GitHub\L_theorie_homepage.github.io\.agent\skills\ui-ux-pro-max`: structured CSVs with 50+ style categories, 29 product-context → pattern reasoning rules, 96 color palettes, 56 type pairings, 98 UX heuristics, 25 chart types, and 13 implementation-stack guides. The importer reads rows and emits one nomtu each:

- Style row → `word: glassmorphism`, `kind: UxPattern`, `labels: ["style", "surface"]`.
- Reasoning row → `word: saas_visual_recommendation`, `kind: DesignRule`, with `Recommends` edges into the matching pattern nomtu.
- Color palette → `word: cool_neutral_palette`, `kind: UxPattern`, `labels: ["color", "palette"]`.
- Accessibility heuristic → `word: focus_ring_contrast_aaa`, `kind: UxPattern`, `labels: ["accessibility", "contrast"]`.
- Motion heuristic → `word: subtle_transition_timing`, `kind: UxPattern`, `labels: ["motion", "timing"]`.
- Stack guide → `word: react_glass_impl`, `kind: UxPattern`, `origin_ecosystem: react` metadata.

**Shape B — runtime library corpus (npm / crates.io / PyPI package).** The importer runs the runtime-library ingestion path (per §5.6), extracts the package's public API, and emits one nomtu per API concept with a descriptive native name.

**Concrete reference:** Motion for React monorepo at `C:\Users\trngh\Documents\motion-main` (packages `framer-motion`, `motion`, `motion-dom`, `motion-utils`). Public surface: 50+ wrapped HTML/SVG elements, 5 orchestration components, ~30 props, 5 transition types, 25+ hooks, ~12 helper functions, 10+ easing functions, 6 contexts, plus value/event types. The importer walks `packages/*/src/index.ts` exports and maps each to a native Nom name.

The mapping below is indicative, not exhaustive — the shape of the rule, not the full table — grouped by category. Every entry gets `origin_ecosystem: npm` + `origin_package: motion` (historical `framer-motion`) in `entry_meta`, never in `word`.

**Components (React-only):**

| Source | → | Nom nomtu |
|---|---|---|
| `motion.*` proxy (50+ wrapped HTML/SVG elements) | → | `animated_element` (polymorphic; per-tag variants as `Specializes` children: `animated_div`, `animated_button`, `animated_path`, …) |
| `m` shorthand | → | alias; resolves to the same hash as `animated_element` |
| `AnimatePresence` | → | `exit_animation_scope` |
| `LayoutGroup` | → | `shared_layout_group` |
| `LazyMotion` | → | `lazy_loaded_animation_features` |
| `MotionConfig` | → | `animation_config_scope` |
| `Reorder.Group` / `Reorder.Item` | → | `reorderable_list` / `reorderable_item` |

**Props (React-only, de-duplicated from `packages/framer-motion/src/motion/types.ts`):**

| Source | → | Nom nomtu |
|---|---|---|
| `animate` | → | `animate_to_state` |
| `initial` | → | `initial_visual_state` |
| `exit` | → | `exit_state` |
| `transition` | → | `animation_transition_config` |
| `variants` | → | `named_animation_state_set` |
| `custom` | → | `dynamic_variant_input` |
| `whileHover` / `onHoverStart` / `onHoverEnd` | → | `hover_interaction_animation` + hover lifecycle callbacks as edges |
| `whileTap` / `onTap` / `onTapStart` / `onTapCancel` | → | `tap_interaction_animation` + lifecycle |
| `whileFocus` | → | `focus_interaction_animation` |
| `whileInView` / `viewport` | → | `scroll_in_view_animation` / `viewport_trigger_config` |
| `whileDrag` / `drag` | → | `drag_gesture_animation` / `drag_axis` |
| `dragConstraints` / `dragElastic` / `dragMomentum` / `dragTransition` | → | `drag_bounds` / `drag_elasticity` / `drag_momentum` / `drag_release_transition` |
| `onPan` / `onPanStart` / `onPanEnd` | → | `pan_gesture` lifecycle |
| `layout` / `layoutId` / `layoutDependency` / `layoutRoot` / `layoutScroll` | → | `layout_animation` / `shared_layout_transition` / `layout_dependency_key` / `layout_root_scope` / `layout_scroll_container` |
| `style` / `transformTemplate` | → | `motion_style` / `transform_composition_template` |
| `onUpdate` / `onAnimationStart` / `onAnimationComplete` | → | animation lifecycle edges on the parent `animate_to_state` |

**Transitions (vanilla, shared across React and DOM):**

| Source | → | Nom nomtu |
|---|---|---|
| `type: 'spring'` + `stiffness` / `damping` / `mass` / `bounce` / `visualDuration` | → | `spring_physics_transition` with tunables as named parameters |
| `type: 'tween'` + `duration` / `delay` / `ease` | → | `tween_transition` |
| `type: 'inertia'` + `power` / `timeConstant` / `bounceStiffness` / `modifyTarget` | → | `inertia_transition` |
| `type: 'keyframes'` + `times` | → | `keyframe_transition` |
| `type: 'decay'` | → | `decay_transition` |
| Common: `repeat` / `repeatType` / `repeatDelay` / `onUpdate` / `onPlay` / `onComplete` / `onRepeat` / `onStop` | → | transition-orchestration nomtu: `transition_repeat_config`, etc. |

**Variant orchestration (React-only):**

| Source | → | Nom nomtu |
|---|---|---|
| `staggerChildren` | → | `child_stagger_orchestration` |
| `staggerDirection` | → | `stagger_direction` |
| `delayChildren` | → | `child_delay_orchestration` |
| `when: 'beforeChildren' \| 'afterChildren'` | → | `child_ordering_mode` |

**Hooks (React-only, from `packages/framer-motion/src/index.ts`):**

| Source | → | Nom nomtu |
|---|---|---|
| `useMotionValue` | → | `reactive_motion_value` |
| `useMotionTemplate` | → | `css_template_from_motion_values` |
| `useMotionValueEvent` | → | `subscribe_motion_value_event` |
| `useTransform` | → | `derived_motion_value` |
| `useSpring` | → | `spring_motion_value` |
| `useVelocity` | → | `motion_value_velocity` |
| `useTime` | → | `elapsed_time_motion_value` |
| `useAnimate` | → | `imperative_animation_controller` |
| `useAnimateMini` | → | `imperative_animation_controller_minimal` |
| `useAnimationControls` / `useAnimation` | → | `legacy_animation_controls` (marked `SupersededBy: imperative_animation_controller`) |
| `useScroll` | → | `scroll_progress_tracker` |
| `useInView` | → | `viewport_presence_detector` |
| `useDragControls` | → | `imperative_drag_controller` |
| `useIsPresent` / `usePresence` / `usePresenceData` | → | `is_presence_mounted` / `presence_lifecycle_handle` / `presence_context_payload` |
| `useReducedMotion` / `useReducedMotionConfig` | → | `user_prefers_reduced_motion` / `reduced_motion_config` |
| `useAnimationFrame` | → | `animation_frame_callback` |
| `useCycle` | → | `cycle_through_states` |
| `useWillChange` | → | `css_will_change_manager` |
| `useInstantTransition` | → | `temporarily_disable_transitions` |

**Helpers (vanilla JS, from `motion-dom` and `motion-utils`):**

| Source | → | Nom nomtu |
|---|---|---|
| `animate(targets, anims, opts)` | → | `animate_elements` (imperative) |
| `scroll(callback)` | → | `observe_scroll` |
| `inView(element, callback)` | → | `observe_in_view` |
| `stagger(duration, opts)` | → | `stagger_delay_generator` |
| `mix(from, to, progress)` | → | `interpolate_between` |
| `transform(value, inputRange, outputRange)` | → | `map_value_range` |
| `clamp(min, max, value)` / `wrap(min, max, value)` | → | `clamp_to_range` / `wrap_around_range` |
| `distance(a, b)` / `pipe(...fns)` | → | `euclidean_distance` / `compose_functions` |

**Easing functions (from `motion-utils`):**

| Source | → | Nom nomtu |
|---|---|---|
| `easeIn` / `easeOut` / `easeInOut` | → | `ease_in_curve` / `ease_out_curve` / `ease_in_out_curve` |
| `circIn` / `circOut` / `circInOut` | → | `circular_ease_in` / `circular_ease_out` / `circular_ease_in_out` |
| `backIn` / `backOut` / `backInOut` | → | `back_ease_in` / `back_ease_out` / `back_ease_in_out` |
| `anticipate` | → | `anticipate_ease` |
| `cubicBezier(x1, y1, x2, y2)` | → | `cubic_bezier_curve` |
| `steps(count, direction)` | → | `stepped_easing` |
| `mirrorEasing(e)` / `reverseEasing(e)` | → | `mirror_easing` / `reverse_easing` |

**Value / control types:**

| Source | → | Nom nomtu |
|---|---|---|
| `MotionValue<T>` | → | `reactive_value` |
| `AnimationPlaybackControls` (+ `.then()` variant) | → | `animation_playback_handle` |
| `DragControls` | → | `drag_controller` |
| `LegacyAnimationControls` | → | `legacy_animation_controls` (marked deprecated via `SupersededBy`) |
| `Variant` / `Variants` | → | `single_animation_state` / `named_animation_state_set` |
| `TargetAndTransition` / `ResolvedValues` / `MotionStyle` | → | `target_with_transition` / `resolved_values` / `motion_style` |
| `HoverEvent` / `TapInfo` / `PanInfo` / `InViewEntry` | → | `hover_event` / `tap_info` / `pan_info` / `in_view_entry` |

**Contexts:**

| Source | → | Nom nomtu |
|---|---|---|
| `MotionConfigContext` | → | `animation_config_context` |
| `PresenceContext` | → | `presence_context` |
| `LayoutGroupContext` / `SwitchLayoutGroupContext` | → | `layout_group_context` / `exclusive_layout_group_context` |

---

**Shape B — cross-platform runtime library (one source → many OS targets).** Concrete reference: Dioxus monorepo at `C:\Users\trngh\Documents\APP\dioxus-main`. This is a qualitatively different Shape-B case from the animation library above — Dioxus compiles **one Rust component** to **web (wasm) / desktop (Windows/macOS/Linux) / mobile (iOS/Android) / native (Vello rasterizer, no webview) / SSR / LiveView (server-driven) / Fullstack (SSR + islands + server functions)**. The importer has to preserve that cross-target property in the emitted graph.

**Platform specialization as a first-class graph pattern.** One source concept (e.g., `Button`, the JS-interop function, `launch`) produces:

1. A **generic nomtu** capturing the concept, `kind: UxPattern` or `Function`, with an inferred contract that's platform-agnostic.
2. **Per-platform specialized nomtu**, each with its own content hash and its own `body_nom` implementing the concept for a specific renderer backend.
3. **`Specializes` edges** from each platform variant to the generic.
4. `origin_platform` metadata on each specialized variant: `web` | `desktop` | `mobile` | `native` | `ssr` | `liveview` | `fullstack` | `all`.

Resolving `button` in a context with a declared build target picks the specialized variant automatically (the §5.4 ranking function filters by `origin_platform == build_target || origin_platform == all`). One source, many deterministic builds.

**Mapping from Dioxus surfaces:**

**Core reactivity (all platforms):**

| Source | → | Nom nomtu | `origin_platform` |
|---|---|---|---|
| `rsx!` macro | → | `declarative_view_syntax` | all |
| `Component<P>` | → | `stateful_component` | all |
| `Element` (= `Result<VNode, RenderError>`) | → | `view_tree_result` | all |
| `VirtualDom` | → | `virtual_dom_engine` | all |
| `Signal<T>` | → | `reactive_signal` | all |
| `Memo<T>` | → | `memoized_signal` | all |
| `Resource<T>` | → | `async_resource` | all |
| `use_signal` / `use_memo` / `use_resource` / `use_effect` / `use_future` / `use_coroutine` / `use_callback` / `use_on_destroy` | → | `create_signal` / `create_memo` / `create_resource` / `run_effect` / `spawn_future` / `run_coroutine` / `memoize_callback` / `on_component_unmount` | all |
| `use_context` / `provide_context` / `consume_context` / `try_consume_context` | → | `read_context` / `provide_scoped_context` / `require_context` / `try_read_context` | all |
| `spawn(fut)` | → | `spawn_task` | all |
| `EventHandler<T>` / `Event<T>` | → | `event_callback` / `event_payload` | all |

**HTML + event primitives (all where the renderer supports DOM; otherwise platform-restricted):**

| Source | → | Nom nomtu | `origin_platform` |
|---|---|---|---|
| HTML/SVG tags (`div`, `button`, `input`, `a`, `svg`, `path`, …) | → | generic `html_element_<tag>` per tag | web/desktop/mobile/liveview (render via DOM); also ssr, native (Blitz) |
| `MouseEvent` / `KeyboardEvent` / `FormEvent` / `FocusEvent` | → | `mouse_event` / `keyboard_event` / `form_event` / `focus_event` | all |
| `TouchEvent` | → | `touch_event` | mobile + web |
| `WheelEvent` / `PointerEvent` / `ScrollEvent` | → | `wheel_event` / `pointer_event` / `scroll_event` | web + desktop |
| `DragEvent` / `ClipboardEvent` | → | `drag_event` / `clipboard_event` | web + desktop |

**Router (all platforms via abstract history):**

| Source | → | Nom nomtu | `origin_platform` |
|---|---|---|---|
| `Router` / `Outlet` / `Link` | → | `router_root` / `route_outlet` / `navigation_link` | all (Link requires html feature) |
| `use_navigator` / `use_router` / `use_route` / `navigate` | → | `navigator_handle` / `router_context` / `current_route` / `navigate_imperative` | all |
| `Routable` derive | → | `derivable_route_enum` | all |
| `GoBackButton` / `GoForwardButton` | → | `history_back_button` / `history_forward_button` | web |
| `MemoryHistory` / `WebHistory` / `HashHistory` | → | `in_memory_history` / `browser_history` / `hash_fragment_history` | all / web / web |

**Renderer launch points (each becomes its own specialized nomtu under a generic `ui_runtime_launch`):**

| Source | → | Nom nomtu | `origin_platform` |
|---|---|---|---|
| Generic concept | → | `ui_runtime_launch` | all |
| `dioxus_web::launch(app)` | → | specialized `ui_runtime_launch`, `Specializes(generic)` | web |
| `dioxus_desktop::launch(app)` + `DesktopContext` / menus / tray / file dialogs | → | specialized `ui_runtime_launch` + `desktop_window_context` / `native_menu` / `native_tray` / `native_file_dialog` | desktop |
| `dioxus_mobile::launch(app)` + platform entry glue | → | specialized `ui_runtime_launch` | mobile |
| `dioxus_native::launch(app)` (Blitz + Vello + Winit + AccessKit + muda) | → | specialized `ui_runtime_launch` + `accessibility_tree` + `gpu_rasterizer` | native |
| `dioxus_ssr::{render, render_element, pre_render}` | → | specialized `ui_runtime_launch` (sync string output) | ssr |
| `dioxus_liveview::launch` + `WebsocketTx` / `WebsocketRx` traits | → | `liveview_runtime` + `websocket_tx_adapter` / `websocket_rx_adapter` | liveview |

**Fullstack (isomorphic — server and client, paired):**

| Source | → | Nom nomtu | `origin_platform` |
|---|---|---|---|
| `#[server]` on an async fn | → | **pair** of entries: `server_function_endpoint` (server-side, `effect: io + server`) + `server_function_client_stub` (client-side, `effect: io + rpc`), linked by `ContractMatches` edge | server + client |
| `use_server_future` / `use_action` / `use_server_cached` | → | `server_future_loader` / `server_action_hook` / `server_cached_resource` | fullstack |
| Encoding options (Json / Cbor / MessagePack / Form / Text / SSE) | → | `rpc_encoding_json` / `rpc_encoding_cbor` / `rpc_encoding_msgpack` / `rpc_encoding_form` / `rpc_encoding_text` / `rpc_encoding_sse` | fullstack |
| Tower middleware layers | → | `rpc_middleware_layer` | fullstack (server side) |
| Streaming primitives (`ServerFuture`, `ServerCached`) | → | `streaming_server_future` / `server_cached_value` | fullstack |

**Assets and head metadata:**

| Source | → | Nom nomtu | `origin_platform` |
|---|---|---|---|
| `asset!("/path")` macro | → | `bundled_asset_reference` | all |
| `Asset` type | → | `bundled_asset` | all |
| `read_asset_bytes(&asset)` | → | `read_asset_bytes` | all |
| `asset_path(&asset)` | → | specialized `read_asset_bytes` resolved-to-path variant | desktop (web/mobile error-returning) |
| `document()` provider | → | `document_provider` | all (platform specializations below) |
| JS-interop function (the `eval` primitive accepting a script string) | → | `execute_javascript` | web + desktop + liveview (native/ssr return a `Partial` no-op specialization) |
| `<Title>` / `<Meta>` / `<Stylesheet>` components | → | `page_title_tag` / `meta_tag` / `external_stylesheet_link` | web + ssr (desktop/native specialize to window title only) |

**CLI subcommands (seed as `kind: Skill` entries, not UxPattern):**

| Source | → | Nom nomtu (kind: Skill) |
|---|---|---|
| `dx new` | → | `scaffold_new_project` |
| `dx init` | → | `initialize_in_existing_project` |
| `dx serve` | → | `dev_server_with_hot_reload` |
| `dx build` | → | `build_for_target_platform` |
| `dx bundle` | → | `package_for_distribution` |
| `dx run` | → | `build_and_run_locally` |
| `dx check` | → | `type_check_without_build` |
| `dx autoformat` | → | `format_component_source` |
| `dx translate` | → | `convert_html_to_view_syntax` |
| `dx doctor` | → | `diagnose_environment` |

Each `Skill` entry's `body_nom` is a declarative rule ("when the user asks to deploy, recommend `package_for_distribution` with the target-platform argument derived from context") rather than executable code. These Skills layer on top of the §9.8.8 intent-routing mechanism.

**Why this matters for Nom.** A Nom developer writes a `ui_runtime_launch` reference in their `.nom` source. At compile time, the target platform is known (e.g., `--target desktop-windows`). The resolver automatically picks the `desktop` specialization; the closure materializes the desktop-specific `body_nom`; the build produces a Windows executable. Swap `--target web` and the same `.nom` file produces a wasm bundle. One source, many OS deployments — achieved through hash-identified specializations, not code-level conditional compilation.

This is Nom's cross-platform story, and it's a natural consequence of the Phase-12 `Specializes` machinery combined with Phase-5 content-addressed storage. We didn't need a new mechanism; Dioxus just happens to be a corpus that exercises it fully.

**The common importer pattern.** Both shapes follow the same template in `nom-ux/src/seed/` (Shape A) and `nom-extract/src/resolvers/npm.rs` + `nom-ux/src/seed/react_motion_lib.rs` (Shape B):

1. Read the corpus from a local path (static CSV) or local ecosystem cache (resolved npm package source at `node_modules/motion` or similar).
2. Per concept, run the §5.2 equivalence gate: translate the Nom equivalent, synthesize property tests from the inferred contract, verify, store as Complete or Partial.
3. Canonicalize the name via `nom-ux/src/naming.rs` — concept-first, underscore-separated, lowercase. **Never carry the package or library brand into the `word` field.**
4. Emit edges: `Recommends` between related concepts, `Styles` when a pattern is applicable to a component kind, `Specializes` when a concept is a stack-specific refinement of a more general one.

One importer module per distinct corpus shape; adding new corpora later (another design-knowledge CSV, another animation library like `react-spring` or `lottie-react`) follows the same template. The dictionary's seeded UX vocabulary grows without any brand names ever landing in `word`.

These seed entries join Phase 9.4's starter vocabulary automatically — authors writing UI compose against them from day one.

#### 5.11.3 Extract UX logic from ingested repos

Phase 5 §5.6's ingestion protocol extends with a UX-extraction pass. While the code translators produce `body_nom` from source, a parallel UX-extractor reads the same files looking for UI patterns:

| Source | Extracts to |
|--------|-------------|
| JSX/TSX/Vue/Svelte component | `Screen` or component-level `UxPattern`; InteractsWith edges from event handlers. |
| CSS / styled-components / Tailwind classes | `UxPattern` entries (color, spacing, layout primitives); `Styles` edges to components. |
| `aria-*`, `role`, semantic HTML | `UxPattern` entries of accessibility kind; `Constrains` edges. |
| xstate / Redux slice / Zustand store | `UserFlow` entries; `TransitionsTo` edges. |
| React Router / Next.js app router / Vue Router | `UserFlow` entries; `TransitionsTo` edges from route → route. |
| Tailwind config / CSS variables | `DesignRule` entries (palette, type scale). |
| Figma JSON export (if present in repo) | `Screen` + `UxPattern` entries. |
| Storybook stories | `Screen` entries tagged `kind_subtype: story`; property matrices become variant entries. |

Per-ecosystem extractors live in a new workspace crate `nom-ux`, at `nom-compiler/crates/nom-ux/src/extractors/`: `react.rs`, `vue.rs`, `svelte.rs`, `flutter.rs`, `swiftui.rs`, `jetpackcompose.rs`. Each mirrors the corresponding code translator in structure. The crate depends on `nom-ast`, `nom-types`, `nom-dict`; it is *not* a subdirectory of `nom-extract` — UX extraction is a first-class concern with its own crate boundary.

Nothing extracted from a repo retains its original codebase's project name. Components become `Screen` entries with descriptive `word` values derived from their semantic role (e.g., a component named `HomePageHero.tsx` might become `word: landing_page_hero`, not `HomePageHero` or `home_page_hero_tsx`). The original path stays in `entry_meta` as provenance.

The output of a repo ingestion is now a **code-plus-UX graph**: code atoms linked by `Calls`/`Imports`/`Implements`; UX atoms linked by `Styles`/`Constrains`/`Recommends`/`TransitionsTo`; and cross-kind edges (e.g., a Screen `InteractsWith` a backend Function). Composition proceeds over the unified multi-graph.

#### 5.11.4 What supervision-develop contributes (honest scoping)

`C:\Users\trngh\Documents\supervision-develop` is a computer-vision library (Roboflow's `supervision`), not a UX framework. It does NOT provide UX content to seed the dictionary.

What it contributes is **architectural patterns** the Nom extractor layer can borrow:
- **Model-agnostic connectors** (YOLO / Transformers / MMDetection / Inference API → one unified `sv.Detections` type) — the same pattern Nom uses for per-ecosystem translators → one unified Entry type.
- **Format translation** (COCO ↔ YOLO ↔ Pascal VOC) — analogous to translation between ecosystem source languages and Nom canonical form.
- **Compositional annotators** — pattern for cross-cutting analysis passes (e.g., accessibility checker, performance checker) composing over the same base entry stream.

No direct seed data. Phase 5 deliverables cite supervision-develop as an implementation reference, not an ingestion source.

#### 5.11.5 Deliverables for §5.11

- `EntryKind::UxPattern`, `DesignRule`, `Screen`, `UserFlow`, `Skill` added to nom-types.
- Edge types `Styles`, `Constrains`, `Recommends`, `InteractsWith`, `TransitionsTo` added to `EdgeType`.
- **New workspace crate `nom-ux`** at `nom-compiler/crates/nom-ux/` with:
  - `src/extractors/{react,vue,svelte,flutter,swiftui,jetpackcompose}.rs` — 6 per-ecosystem UX extractors.
  - `src/seed/` — corpus importers (one module per known corpus shape). Each importer produces nomtu with descriptive native Nom names; external corpus identifiers appear only in `entry_meta` as provenance.
  - `src/naming.rs` — canonicalization helpers that transform source identifiers into native Nom descriptive names (concept-first, underscore-separated, lowercase).
- `nom ux seed <path>` CLI subcommand — detects the corpus shape, dispatches to the right importer, reports entries created.
- `nom ux extract <repo>` CLI subcommand — runs UX extraction over a repo, populates `Screen` / `UserFlow` / `UxPattern` / `Styles` / `InteractsWith` / `TransitionsTo` entries and edges.
- Integration tests:
  - Seed a UX corpus from a sample CSV fixture; verify entries with descriptive Nom names (never the corpus's brand name) and correct edges.
  - Extract UX from a small React app fixture; verify screens and flows land; verify a query like "screens styled by the glassmorphism pattern" traverses the graph correctly.
- ~1500 LOC extractors + ~500 LOC seed importers + ~200 LOC naming canonicalizer + ~200 LOC kind/edge additions.

#### 5.11.6 Platform-specialization compile path — `nom app build --target <platform>`

The same UX closure ships to three runtime targets through the `Specializes` edge mechanism:

| `--target` | Specialization root | Emitted artifacts |
|---|---|---|
| `web` | `ui_runtime_launch_web → dioxus_web::launch` (via WASM) | `index.html` + `app.wasm` + CSS bundle + asset dir |
| `desktop` | `ui_runtime_launch_desktop → dioxus_desktop::launch` (webview-embedded) | native `.exe` (Windows) / `.app` (macOS) / ELF (Linux) |
| `mobile` | `ui_runtime_launch_mobile → dioxus_mobile::launch` (Android: JNI + AAR; iOS: Objective-C bridge + XCFramework) | `.apk` (Android) / `.ipa` (iOS) |

**Resolution algorithm.** `nom app build <screen_hash> --target <platform>`:
1. Walk from `<screen_hash>` through `Calls`/`Imports`/`UsesPattern` edges to collect the app closure.
2. Replace every `ui_runtime_launch` node in the closure with its `--target`-specific `Specializes` variant.
3. Emit the per-target artifact via the platform's build driver (trunk/wasm-bindgen for web, `cargo build --release` for desktop, `cargo-mobile` for mobile).

**No source-code branching.** The application author never writes `#[cfg(feature = "web")]` equivalents. The `Specializes` edges encapsulate all per-target differences: which launch function to call, which event-loop runtime to use, how to embed assets.

**Asset bundling.** `MediaUnit` leaves in the closure (icons, images, fonts, audio cues — see §5.16) are extracted at build time and embedded in the target artifact:
- Web: asset dir served statically alongside `index.html`, referenced from WASM via `fetch`.
- Desktop: bundled into the executable via `rust-embed`-equivalent specialization.
- Mobile: bundled into the app's asset catalog per platform convention.

**Platform capability differences** (e.g., `window.open` on web vs. no equivalent on mobile) are surfaced by the verifier: if the closure uses a capability nomtu without a `RunsOn` edge to the requested platform, compilation fails with `NOM-U02` ("unavailable capability `<cap>` on target `<platform>` — specialize or gate").

**Build parity measured per commit.** §5.13 benchmark runs include a parity check: the same closure built for web + desktop + mobile must produce semantically equivalent output (same rendered screens under a headless renderer, same flow-step traces under `nom flow record`). Parity failures are `NOM-U03` diagnostics.

### 5.12 App-composition kinds — high-level structural units

A `.nomtu` corpus of functions and components can compose mid-sized programs, but full applications have structural units above the function level: data sources, queries bound to those sources, actions triggered by UI events, multi-page navigation, app-level variables. Low-code platforms have had this right for years — **an app is a manifest of declarative units**, not just a tree of function calls.

Concrete reference: the low-code internal-tools platform at `C:\Users\trngh\Documents\APP\Accelworld\services\ToolJet-develop`. Its manifest model — Components + DataSources + Queries + Events + Actions + Pages + Variables + Modules — is the right structural decomposition for Nom to expose as first-class nomtu kinds.

#### 5.12.1 New kinds

Extend `EntryKind`:

- **`AppManifest`** — the root nomtu of a whole app. Its closure walk yields every Page, Component, DataSource, Query, Action reachable from the entry point. The app's hash is its identity.
- **`DataSource`** — a typed connection to an external system (REST endpoint set, database, queue, SaaS service, etc.). Body describes the connection contract and the set of operations it exposes.
- **`Query`** — a parameterized operation against a DataSource. Body is the query template; contract declares input parameters and expected result shape.
- **`AppAction`** — a handler invoked in response to an event, composing queries + variable writes + navigation + side effects.
- **`AppVariable`** — a named mutable cell at page or app scope, with an initial value and a type contract.
- **`Page`** — a navigable surface composed of screens (from §5.11) with an app-scoped URL pattern.

Extend `EdgeType`:

- **`BindsTo(Component, Query)`** — a component property is bound to a query's result.
- **`Triggers(Event, AppAction)`** — a UI event fires an action.
- **`Reads(AppAction, AppVariable)`** / **`Writes(AppAction, AppVariable)`** — action state dependencies.
- **`NavigatesTo(AppAction, Page)`** — action triggers navigation.
- **`RunsOn(Query, DataSource)`** — query is executed against a data source.

#### 5.12.2 CLI and ingestion

- `nom app new <name> --template=<starter>` — scaffold a new AppManifest with opinionated defaults.
- `nom app import <path>` — ingest an existing app manifest (ToolJet JSON export, Retool export, Appsmith export) into the dict as an AppManifest + its dependency closure. Names are normalized per §5.11.2 (no brand leak; `origin_platform=tooljet` in `entry_meta`).
- `nom app build <hash> --target=<platform>` — materialize the closure + `Specializes` per platform (per §5.11.2 cross-platform mapping) and emit a buildable artifact.

#### 5.12.3 Multi-app sharing is automatic

Because everything is content-addressed, two apps that both use `rest_http_get` or `postgres_select_query` or `authenticated_user_variable` share those hashes. A corpus of 1,000 internal tools converges to ~3,000 unique nomtu (most of them reusable primitives) — the small delta per app is just the specific composition: which components, which queries, wired to which actions.

**Multi-app closure walk.** Given a set of AppManifest hashes, the union of their closures is the build set. `nom app build --all <hash1> <hash2> ...` produces N artifacts that share their intersecting closure. Cross-app specialization (§5.15) then picks per-target variants once for the shared subset.

---

### 5.13 Compiler-integrated benchmarking — measure, don't guess

Claim: optimization decisions in the content-addressed graph (which specialization to emit, which inline path to pick, whether to monomorphize or stay generic) should be **data-driven**, not heuristic. Every nomtu carries benchmark data. The compiler reads that data at build time and picks the best-performing variant for the declared context.

Concrete reference: the C++ micro-benchmarking framework at `C:\Users\trngh\Documents\APP\benchmark-main` (Google Benchmark). Its API — parameterized ranges, custom counters, statistical significance testing via U-test in `tools/compare.py` — is the right surface for Nom to adopt, but in a native Nom form.

#### 5.13.1 New kinds and a typed side-table

Extend `EntryKind`:

- **`Benchmark`** — a declarative benchmark definition. Contract: what nomtu it measures, what ranges of input it sweeps, what counters it reports. Body is the iteration scaffold.
- **`BenchmarkRun`** — a single measurement run: timestamp, platform, hash of the measured nomtu, iteration count, real_time, cpu_time, statistical moments (mean, median, stddev, cv), custom counters, hash of the compiler version that produced the build.

`BenchmarkRun` is query-hot (picking "the fastest specialization on iOS for this workload" is the whole point), so it gets its own typed table, not EAV metadata:

```sql
CREATE TABLE entry_benchmarks (
    run_id               INTEGER PRIMARY KEY AUTOINCREMENT,
    id                   TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    platform             TEXT NOT NULL,
    compiler_hash        TEXT NOT NULL,
    workload_key         TEXT NOT NULL,
    iterations           INTEGER,
    real_time_ns         REAL,
    cpu_time_ns          REAL,
    mean_ns              REAL,
    median_ns            REAL,
    stddev_ns            REAL,
    cv                   REAL,
    bytes_per_sec        REAL,
    custom_counters_json TEXT,
    measured_at          TEXT DEFAULT (datetime('now'))
);
CREATE INDEX idx_bench_entry_platform ON entry_benchmarks(id, platform);
CREATE INDEX idx_bench_workload ON entry_benchmarks(workload_key);
CREATE INDEX idx_bench_mean ON entry_benchmarks(mean_ns);
```

#### 5.13.2 CLI

- `nom bench <hash>` — compile the nomtu under every enabled specialization for the current platform, run each, record `BenchmarkRun` rows.
- `nom bench compare <hashA> <hashB>` — statistical comparison; report `(B - A) / |A|` per counter with a U-test p-value; equivalent to Google Benchmark's `tools/compare.py`.
- `nom bench regress <hash> --baseline <compiler_hash>` — compare current run against a baseline-compiler recorded run; flag regressions above a threshold.
- `nom bench curate` — garbage-collect old BenchmarkRuns older than N days OR where the compiler_hash has been superseded + the run is not the last-known-good for any specialization.

#### 5.13.3 Data-driven specialization (extension of Phase 12)

Phase 12 monomorphization is a static analysis. With benchmark data, specialization becomes a **cost-minimization query**:

- Multiple specialized variants of the same generic coexist (e.g., three SIMD-vectorized variants of `sum_reducer`, differing in inner-loop structure).
- `nom build --specialize <hash>` queries `entry_benchmarks`, filters by target platform and workload key, picks the variant with the best objective (lowest mean time / smallest binary / best memory-per-op, per a per-build cost function).
- If no benchmark data exists for a variant, it's excluded from the pool (conservative — never pick an unmeasured variant blindly). Gap triggers a warning suggesting `nom bench <hash>`.
- Across a multi-app multi-platform build (§5.15), the selector solves the joint problem: pick one specialization per (nomtu, platform) that minimizes total cost across the app set.

#### 5.13.4 Provenance

Benchmark runs are reproducible: the tuple `(compiler_hash, platform, workload_key, iterations)` is what makes a run comparable. Two runs with the same tuple should agree within CV bounds, or the measurement is noise. The U-test significance testing from Google Benchmark's `compare.py` is ported into `nom bench compare` — no hand-wavy "looks faster", always a p-value.

---

### 5.14 Flow concretization — every nomtu has a flow artifact

Claim: every closure's execution is observable as a concrete flow artifact, not just a static call graph. The AST already gives you structural flow; execution adds timing, data values at edges, fan-out/fan-in counts, retries, early-exits. A developer inspecting any nomtu should be able to answer "what actually happens when this runs?" without re-instrumenting their code.

Concrete reference: the LangGraph-based agent harness at `C:\Users\trngh\Documents\APP\deer-flow-main`. Its pattern — state-machine agent + middleware stack intercepting every step + mergeable ThreadState artifacts + checkpointed resume — is the right model for Nom, but generalized from agents to any closure execution.

#### 5.14.1 New kinds and edges

Extend `EntryKind`:

- **`FlowArtifact`** — a recorded execution trace of a closure invocation. Content-addressed like any nomtu: its id is the hash of the full trace (inputs + step sequence + outputs), so identical executions deduplicate.
- **`FlowStep`** — one atomic step in a FlowArtifact. Not stored as a separate entry usually (steps are embedded in the artifact), but used as an addressable unit when a user references a specific step in a query.
- **`FlowMiddleware`** — a declarative interceptor that runs before/after every step of a target closure and emits side-records (memory snapshots, audit entries, cost counters).

Extend `EdgeType`:

- **`HasFlowArtifact(Closure, FlowArtifact)`** — a recorded run of this closure.
- **`StaticFlow(entry)` intrinsic** — derivable from the AST alone; not a stored edge, computed on demand by a graph walk.
- **`FlowsTo(FlowStep, FlowStep)`** — data-flow edge within a FlowArtifact.

#### 5.14.2 Typed side-table for flow step data

Like benchmarks, flow step detail is query-hot (time-series queries over runs, filters by step kind, bottleneck analysis) and doesn't fit EAV. Minimal schema:

```sql
CREATE TABLE flow_steps (
    step_id              INTEGER PRIMARY KEY AUTOINCREMENT,
    artifact_id          TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    step_index           INTEGER NOT NULL,
    entry_id             TEXT NOT NULL,
    started_at_ns        INTEGER,
    ended_at_ns          INTEGER,
    input_hash           TEXT,
    output_hash          TEXT,
    effect_kind          TEXT,
    side_effect_ref      TEXT
);
CREATE INDEX idx_steps_artifact ON flow_steps(artifact_id, step_index);
CREATE INDEX idx_steps_entry ON flow_steps(entry_id);
CREATE INDEX idx_steps_slow ON flow_steps(ended_at_ns, started_at_ns);
```

#### 5.14.3 CLI

- `nom flow record <hash> --inputs <json>` — invoke the closure with the given inputs, record a FlowArtifact (and its FlowStep rows), print the artifact id.
- `nom flow show <artifact_id>` — render the trace as a DAG (default), a step-table, or a timeline. Formats: `--format=dot|json|mermaid|table`.
- `nom flow diff <a> <b>` — compare two FlowArtifacts, highlight where step sequences or data-flow edges diverged.
- `nom flow middleware <hash> attach <middleware_nomtu>` — register a FlowMiddleware on a closure at build time.

#### 5.14.4 The middleware pattern (borrowed from DeerFlow, generalized)

FlowMiddleware is a nomtu describing how to intercept a step. Examples that ship as seed entries:

| Seed nomtu (`kind: FlowMiddleware`) | Role |
|---|---|
| `measure_wall_time` | Records `started_at_ns` / `ended_at_ns` on every step. Default on for debug builds. |
| `capture_input_output_hashes` | Records content hash of step inputs and outputs so equal runs dedup. |
| `audit_side_effects` | Records `effect_kind` + `side_effect_ref` for io/ffi steps. |
| `loop_detection` | Raises an error if the same step is entered N times with identical inputs. |
| `memory_checkpoint` | Snapshots persistent state at step boundaries, enabling resumable runs. |
| `cost_counter` | Accumulates a per-step cost metric (CPU cycles, allocations, network bytes) for later benchmark correlation. |

Middlewares are regular nomtu — they get versioned, content-addressed, dedup'd. Two closures using the same middleware stack compose naturally; the stack is itself a hash closure.

#### 5.14.5 Relationship to §9.8 comprehensiveness

A flow artifact is the **dynamic** counterpart to §9.8.3's graph-layer static check. Graph-layer says "no orphan edges in the source"; flow layer says "no orphan edges in a real run" — if execution reached a step that should have been unreachable (per the author's declared flow), the flow artifact flags it. Combined, static + dynamic = full observability.

---

### 5.15 Multi-app, multi-platform optimization — the joint selection problem

The individual mechanisms are now in place: cross-platform specialization (§5.11.2 Shape B cross-platform), data-driven variant selection (§5.13), app-level composition (§5.12), and flow-driven observability (§5.14). The remaining piece is the **joint optimization** across multiple apps × multiple platforms.

**Problem shape.** Given:

- A set of AppManifest hashes `{A₁, A₂, ..., Aₘ}` the user wants to build.
- A set of target platforms `{P₁, P₂, ..., Pₙ}` (web, desktop, mobile, native, ssr, …).
- For each nomtu in `⋃ closure(Aᵢ)`, zero or more specialized variants per platform, each with benchmark data.
- A cost function (user-configurable): lowest binary size, lowest p95 latency, lowest battery drain, best composite, etc.

Find: a selection function `f: (nomtu, platform) → specialization_hash` that minimizes total cost across all (app, platform) pairs, subject to the constraint that specializations used in more than one app share (the specialization hash is the same, so materialization dedups).

#### 5.15.1 Solver mechanics

This is structurally a minimum-cost assignment on a bipartite graph:

- Left vertices: `(nomtu, platform)` pairs in the union closure.
- Right vertices: candidate specialization hashes with known benchmark data.
- Edges: valid assignments with cost = benchmark-derived scalar.
- Constraints: each `(nomtu, platform)` pair must be assigned exactly one candidate.

For typical corpus sizes (thousands to tens of thousands of nomtu, 2–8 platforms, 2–5 specialization candidates each) the problem is solvable in milliseconds by greedy per-pair selection, since the candidates for different pairs are largely independent. If cross-app sharing is maximized (a specialization wins if it's cheapest summed over all app-platform pairs that could use it), a simple LP relaxation suffices.

#### 5.15.2 CLI

- `nom app build --all <h1> <h2> ... --targets web,desktop,mobile --objective=size+p95` — kicks off the joint-selection build. Produces N × M artifacts. Reports: total artifact bytes, shared bytes across artifacts (the dedup win), p95 per artifact, which specialization was selected for each `(nomtu, platform)` pair.
- `nom app explain-selection <hash> --app <app_hash> --platform <p>` — for any specific nomtu in a specific app's build for a specific platform, show why the solver picked the specialization it did (cost comparison across candidates, benchmark run ids consulted).

#### 5.15.3 Benchmark gaps as first-class errors

If a `(nomtu, platform)` pair has no candidate with benchmark data, the solver can:

- **Fail** (default under `--strict`): refuse to build; the user must run `nom bench <hash> --platforms <p>` to produce data.
- **Fall back to the generic** (default under `--best-effort`): use the platform-agnostic variant, flag in the build report, suggest running the benchmark.

This turns "missing benchmark" into a surfaced build-time concern rather than an invisible cost.

#### 5.15.4 Cross-app dedup is measurable

After a multi-app build, `nom app build-report <session_id>` prints:

- Total bytes per artifact.
- Bytes shared across artifacts (the deduplication dividend — specializations reused).
- Specialization-swap candidates (nomtu where a different specialization would have helped another app more than it helps this one — a hint for re-running with a tweaked cost function).

At corpus scale (hundreds of apps, thousands of specialization candidates), the dedup dividend is what makes Nom's cross-platform story practical rather than a binary-size disaster. Every app that shares vocabulary with another is cheaper to add.

---

### 5.16 Media as nomtu — the dictionary replaces file extensions

> **Superseded on body-representation by §4.4.6 (2026-04-12).** Under the new architectural rule, media entries' bodies are **the canonical-format bytes** (AVIF for images, AV1 for video, AAC for lossy audio, FLAC for lossless audio). The declarative-decomposition kinds below (`PixelGrid`, `AudioBuffer`, `VideoStream`, `VectorPath`, `GlyphOutline`, `MeshGeometry`, `Color`, `Palette`) survive as **analysis metadata** — edges + side-table rows recording "this AVIF decodes to a 3000×2000 sRGB grid" — but they are NOT the body. Only one decoded form is materialized at render time through a codec `.bc`. The "every file is a composition" framing below describes the analysis graph, not the storage form.

**Claim:** every file format is a composition of media primitives. A PNG is pixels + a compression codec + a chunk container + metadata. A WAV is samples + a format descriptor + a container. An MP4 is frames + audio tracks + a box-structured container. An OBJ is vertices + faces + material references. A TTF is glyph outlines + metrics + an SFNT container.

If every primitive is a nomtu and every container is a composition, then **the dictionary can hold any media natively**. The ".png" extension disappears as an identity concept; it survives only as provenance metadata (`origin_encoding: png` on a specialized variant). A "file" becomes a hash closure whose materialization emits the bytes a user's downstream tool expects.

This is the full generalization of the principle that's driven the whole design: the dict is the dependency system for code (§4), for UX (§5.11), for apps (§5.12), for the compiler itself (§10), and now for **any content anyone might send or store anywhere**.

#### 5.16.1 New kinds

Extend `EntryKind`:

- **`MediaUnit`** — the generic atomic unit of media content. Its `body_nom` is a declarative description of what the media is; its `origin_encoding` metadata records which concrete byte format was ingested (if any). Has specializations per encoding.
- **`PixelGrid`** — a 2D array of color samples with a declared color space, bit depth, and (width, height).
- **`AudioBuffer`** — a sample stream with sample rate, channel count, bit depth, encoding.
- **`VideoStream`** — a sequence of `PixelGrid` frames with a framerate and, usually, a timing + inter-frame reference graph (keyframes via `DependsOn`, delta frames via `Derives`).
- **`VectorPath`** — a sequence of drawing operations (move, line, curve, close) in a declared coordinate space. SVG and PDF paths reduce to this.
- **`GlyphOutline`** — a vector path representing a single character glyph, plus metrics (advance width, bearings).
- **`MeshGeometry`** — vertices + faces + UV mapping for a 3D model.
- **`Color`** — a value in a declared color space (sRGB, OKLCH, CIELAB, DCI-P3).
- **`Palette`** — an ordered set of `Color` entries.
- **`Codec`** — an encoding/decoding pair. `png_codec`, `jpeg_codec`, `opus_codec`, `h264_codec`, `brotli_compression`. Body describes the algorithm declaratively (or via `Ffi` to a native binding).
- **`Container`** — a file-layout specification. `png_chunk_container`, `riff_wav_container`, `isobmff_mp4_container`, `sfnt_font_container`, `obj_text_container`, `gltf_json_container`.
- **`MediaMetadata`** — typed side metadata that modern containers carry: EXIF, XMP, ID3, ICC profile, color management tags.
- **`RenderPipeline`** — a composition that materializes a `MediaUnit` from its graph closure into a concrete byte stream under a chosen container + codec.

#### 5.16.2 New edges

- **`Encodes(MediaUnit, Codec)`** — this media unit is encoded via this codec. A single media can have multiple Encodes edges to different codecs (PNG + WebP + AVIF specializations of the same `PixelGrid`).
- **`ContainedIn(MediaUnit, Container)`** — this media is wrapped in this container layout.
- **`UsesColor(MediaUnit, Color)`** / **`UsesPalette(MediaUnit, Palette)`** — color-graph edges for indexing, deduplication, palette-swapping transforms.
- **`Derives(MediaUnit_out, MediaUnit_in)`** — this media was produced by a transformation from an input. Transformation kind stored in edge metadata (crop, resize, filter, colorspace-convert, reencode).
- **`EmbeddedGlyph(Font, GlyphOutline)`** — a font's glyph set.
- **`Frame(VideoStream, PixelGrid, timestamp_ns)`** — a video's frames indexed by timestamp.
- **`RendersOn(MediaUnit, Platform)`** — platform-specialized rendering; same image may have distinct specializations for web (compressed PNG/WebP), print (CMYK TIFF), mobile (HEIC), terminal (ASCII art).

#### 5.16.3 Shape C — binary media corpus

Extend the ingestion protocol (§5.6). Shape A was static knowledge (CSV), Shape B was runtime library (source package). Shape C is **binary media** — PNG/JPEG/MP4/WAV/TTF/OBJ files on disk or in a repo.

Ingestion per file:

1. **Read + hash the raw bytes.** The content hash is a stable identifier independent of filename or extension.
2. **Detect the container format** (magic bytes, MIME heuristic, content sniff). Pick a specific decoder from `nom-media/src/decoders/`.
3. **Decode the container into primitives.** A PNG yields: one `PixelGrid` (the decoded image data), one `png_codec` reference (Encodes edge), one `png_chunk_container` reference (ContainedIn edge), one or more `MediaMetadata` entries (ICC profile, EXIF, color type). Store each as a separate nomtu; link via edges.
4. **Equivalence-gate per §5.2.** Re-encode the decoded primitives back through the same codec → byte-compare to the original. If matches, the decoding is lossless and the entry is `Complete`. If the codec is lossy (JPEG), compare under the codec's declared tolerance (PSNR threshold) and mark `Partial` with `partial_reasons: lossy_codec_roundtrip`.
5. **Emit the root `MediaUnit`** with edges pointing at the decoded primitives. Root body describes the media declaratively (e.g., "photographic PixelGrid at 3000×2000, sRGB, with attached ICC profile and EXIF metadata"). Root `origin_encoding: png` in `entry_meta` is provenance only.

Per-encoding decoders live in `nom-media/src/decoders/`: `png.rs`, `jpeg.rs`, `webp.rs`, `avif.rs`, `gif.rs`, `tiff.rs`, `svg.rs`, `pdf.rs`, `wav.rs`, `flac.rs`, `mp3.rs`, `opus.rs`, `mp4.rs`, `webm.rs`, `mkv.rs`, `obj.rs`, `gltf.rs`, `fbx.rs`, `ttf.rs`, `otf.rs`, `woff2.rs`. Each 200–600 LOC, sharing common container-traversal utilities.

#### 5.16.4 Rendering — the inverse operation

Given a root `MediaUnit` hash and a target encoding, the compiler materializes bytes:

```
nom media render <hash> --target png --out image.png
nom media render <hash> --target jpeg --quality 85 --out image.jpg
nom media render <hash> --target webp --lossless --out image.webp
```

Materialization walks the closure, picks the specialization with `origin_encoding == target` if present, otherwise synthesizes one by re-encoding via the target codec. The result is written to disk or streamed. The hash of the output bytes becomes a new `Specializes` variant of the same semantic `MediaUnit`, cached for reuse.

Cross-format conversion is automatic: `nom media render <png_hash> --target webp` works because the root `MediaUnit` is encoding-agnostic; the WebP specialization is produced on demand.

#### 5.16.5 Multi-encoding specialization is the same pattern as multi-platform

The §5.11.2-cross-platform pattern applies verbatim: one generic `MediaUnit` → N encoding-specialized variants (each with its own hash, its own encoded bytes as `body_nom`) → linked by `Specializes` edges → distinguished by `origin_encoding` metadata.

A single 8K photograph could have 30+ specializations in the dict: PNG (lossless), JPEG at quality 50/70/85/95, WebP (lossy and lossless), AVIF, HEIC, BMP, TIFF (uncompressed, LZW, JPEG-in-TIFF), plus per-size variants (thumbnail 256×, preview 1024×, full 8K). The resolver picks by context: web serving asks for WebP at preview size; print asks for lossless TIFF at full size; terminal ASCII-art viewer asks for the generic pixel grid.

This absorbs all of image-serving infrastructure (srcset, CDN variants, responsive images) into the dict. Nothing external is needed; the specializations are already there.

#### 5.16.6 Multi-media compositions

A document is no longer "a PDF" or "a DOCX". It's a composition:

- A `Screen` (§5.11) containing text layout + embedded `MediaUnit`s.
- `VectorPath` nomtu for the layout's structural graphics.
- `Font` + `GlyphOutline` nomtu for the text rendering.
- Embedded images as `MediaUnit` with their own edges.
- Metadata (title, author, creation date) as `entry_meta` rows on the root.

"Render this document as PDF" becomes `nom media render <doc_hash> --target pdf`. "Render as HTML" becomes `--target html`. "Render as TXT" extracts text-bearing nomtu. The document is one hash; the renditions are specializations.

This collapses the "office document" problem: DOCX/ODT/RTF/PDF/HTML/Markdown/EPUB stop being distinct identities and become encoding specializations of the same semantic composition.

#### 5.16.7 `nom-media` workspace crate

New peer crate at `nom-compiler/crates/nom-media/`:

```
nom-media/
├── src/
│   ├── lib.rs
│   ├── decoders/          # Shape-C binary decoders, one file per encoding
│   ├── encoders/          # inverse: nomtu → bytes
│   ├── codecs/            # shared DCT, wavelet, zlib, etc.
│   ├── containers/        # chunk/box parsing utilities
│   ├── colorspace/        # sRGB, OKLCH, ICC profile handling
│   ├── metadata/          # EXIF, XMP, ID3, ICC extraction
│   └── render.rs          # nom media render dispatch
```

Depends on `nom-ast`, `nom-types`, `nom-dict`. Does NOT depend on `nom-ux` or `nom-extract` — media is a first-class concern with its own crate boundary.

CLI:

- `nom media import <file>` — ingest a single file as a MediaUnit (Shape C).
- `nom media import-dir <path> --recurse` — bulk ingest a media directory.
- `nom media render <hash> --target <encoding> [--quality N] --out <path>` — materialize to a file.
- `nom media transcode <hash_in> --target <encoding> --out <hash_or_path>` — sugar for render + re-ingest.
- `nom media diff <hash_a> <hash_b>` — perceptual diff (pixel-wise or PSNR for lossy) between two media.
- `nom media similar <hash>` — find `SimilarTo` neighbors via embeddings or color-histogram index.

#### 5.16.8 Relation to the rest of Nom

- **Compiler is an app** (§10): the compiler's `.archive/` may hold old Rust source, old test fixtures, old doc renders — all ingested as media nomtu, queryable, deduplicated.
- **UX is media**: screens, icons, component mockups, design system exports — all `MediaUnit`. A design system's "brand asset library" is a dict query: `kind:MediaUnit AND meta:origin_brand=<brand>`.
- **Apps include media**: §5.12 `AppManifest` closures naturally include embedded images, fonts, sound effects, video as `MediaUnit` leaves. A game's asset bundle is its closure minus code.
- **Flow artifacts are media**: §5.14 `FlowArtifact` renders can themselves be media — a `MediaUnit` rendering of a recorded flow graph (DOT, Mermaid, Graphviz PNG, interactive HTML). Recursive: flow artifacts describing renders of flow artifacts.
- **Benchmarks are media**: §5.13 `BenchmarkRun` visualizations (charts, heatmaps, time-series PNGs) are media nomtu too.

Everything is in the dict. Everything dedupes by content. Everything specializes by platform or encoding or context.

#### 5.16.9 Governing invariant (added)

**Media is ordinary Nom.** Files with extensions are a user-facing view, not an identity. Every byte that has ever been a file can be a hash in the dict, with its structural decomposition as graph edges and its encodings as `Specializes` variants. "Replace extensions" is not a feature goal — it's a natural consequence of the content-addressed, typed-graph, specialization-driven design applied to media inputs.

#### 5.16.10 Size budget

- `nom-media` core + ~20 common decoders (PNG, JPEG, WebP, AVIF, GIF, TIFF, SVG, PDF skim, WAV, FLAC, MP3, Opus, MP4, WebM, MKV, OBJ, glTF, TTF, OTF, WOFF2): ~15,000 LOC.
- Encoders (inverse of decoders): ~8,000 LOC.
- Color management + metadata: ~2,500 LOC.
- Render dispatch + CLI: ~1,500 LOC.

Total: ~27,000 LOC, shipped incrementally — one decoder+encoder pair per PR. The first PR lands PNG decode (most common lossless image format); the pipeline proves out; subsequent PRs add formats in usage order.

Existing Rust crates provide mature implementations for most codecs (`image`, `rexiv2`, `lopdf`, `symphonia`, `font-kit`, `hound`, `gltf`). Phase 5 ingests them as Shape-B runtime library corpus; `nom-media` composes their Nom-translated primitives. The work is substantial but not speculative — every encoding already has a library to learn from.

#### 5.16.11 Codec runtime strategy — FFI-first, pure-Nom later

Modern codec libraries (AV1 encoder, AAC encoder, FLAC encoder, AVIF still/animation) are hundreds of thousands of LOC of hand-tuned C/asm. Reimplementing them in Nom is a multi-year effort per codec. Pragmatic design: **two-tier codec residency**.

**Tier 1 — FFI-binding nomtu (shipped first).** A `kind: Codec` nomtu whose `body_nom` is a thin Nom wrapper over an FFI call into a mature native library. Metadata carries:
- `ffi_target`: Rust crate name + extern fn signature (e.g., `rav1e::Encoder::encode_frame`) OR C-ABI symbol + header declaration.
- `linker_requires`: list of `StaticLibrary` / `DynamicLibrary` nomtu (ingested via §5.17 mass corpus) that must be linked into the final artifact.
- `codec_parameters`: the type-safe Nom record of tunables (bitrate, quality, color primaries, chroma subsampling, …) with defaults.

FFI-nomtu examples:
- `codec_av1_encoder_rav1e` — wraps `rav1e` (pure-Rust AV1 encoder).
- `codec_av1_decoder_dav1d` — wraps `dav1d` (fast C AV1 decoder).
- `codec_aac_encoder_fdk` — wraps `fdk-aac` (patent-covered — user opt-in only).
- `codec_aac_encoder_faac` — alternative, patent-free path.
- `codec_flac_encoder_libflac` — wraps `libFLAC`.
- `codec_avif_still_encoder_libavif` — wraps `libavif` (built on `libaom` or `rav1e`).
- `codec_avif_animation_encoder_libavif` — same library, image-sequence mode.

**Tier 2 (pure-Nom codec rewrite) is REMOVED per §4.4.6.** Codec bodies are `.bc` forever — the result of compiling the upstream library (rav1e, dav1d, fdk-aac, libFLAC, libavif) via its native toolchain. Platforms without native library access (wasm32, restricted embedded) compile the same library to a wasm32-`.bc` specialization via `Specializes(codec_wasm → codec_native)`. No "pure-Nom algorithm rewrite" step. This simplifies the roadmap and matches the §4.4.6 invariant that the dict holds compiled artifacts, not Nom source.

**Neither tier duplicates library internals in `body_nom`.** FFI wrappers are declarative: the codec's semantic contract (input `PixelGrid` + parameters → output bitstream) is the public face; the implementation (FFI call or pure-Nom algorithm) is a specialization detail.

#### 5.16.12 Build dispatch — `--target <codec>` resolves a codec+container closure

`nom media render <hash> --target av1 --out video.mp4` resolves in five steps:

1. **Codec resolution.** Find the best `kind: Codec` nomtu for the request: filter `word ~= "av1_encoder"`, rank by `is_canonical`, current platform in `RunsOn` edges, `status: Complete`, and whether FFI-required libs are present locally. Pick the head. Emit `NOM-M01` if none matches.
2. **Container resolution.** Determine the output container from `--out` file extension OR explicit `--container <word>`. `.mp4` → `container_isobmff_mp4`; `.avif` → `container_avif_heif` (still) or `container_heif_sequence` (animated); `.webm` → `container_matroska_webm`.
3. **Closure walk.** From the source media hash, collect the transitive `Encodes` + `ContainedIn` + `Requires` closure. The resulting set is the compile set.
4. **Link phase.** If any codec in the closure carries `linker_requires`, the output artifact is linked against those static/dynamic libraries. On Windows: `link.exe` with `.lib` paths; on Linux/macOS: `clang -l<name>`.
5. **Emit.** Run the encode pipeline: pixels/samples → codec → container bytes → file or streamed output.

**Composition.** MP4 with both an AV1 video track and an AAC audio track resolves *two* codec nomtu, *one* container nomtu, and *one* muxer nomtu (`muxer_isobmff`, generic over codec tracks). The muxer is its own nomtu because track ordering, timestamp mapping, and metadata boxes are independent concerns.

**Format-preservation invariant.** `nom media render <hash_x> --target png; nom media import result.png; nom media render <imported_hash> --target png` must produce byte-identical output (PNG is lossless). For lossy codecs (JPEG, AV1 lossy, AAC, MP3), the invariant is PSNR-above-threshold + metadata preserved, checked by the §5.2 equivalence gate.

#### 5.16.13 First-codec roadmap — incremental landings

Ship the codec subsystem one format at a time. Each PR adds one codec nomtu + its FFI binding + an equivalence-gate test (decode → re-encode → byte-compare or PSNR-compare).

| Order | Codec | Primary library | Pair | Why at this position |
|---|---|---|---|---|
| 1 | PNG | `image` (zlib + DEFLATE) | encoder+decoder | Lossless; smallest; byte-identical round-trip is the easiest gate to pass |
| 2 | FLAC | `hound` or `claxon` | encoder+decoder | Lossless audio; same byte-identical gate discipline |
| 3 | JPEG | `image` + `mozjpeg` | encoder+decoder | Most-common lossy image; validates PSNR gate |
| 4 | Opus | `opus` / `symphonia-codec-opus` | encoder+decoder | Modern lossy audio; validates ODG-scored audio gate |
| 5 | AVIF (still) | `libavif` via FFI | encoder+decoder | AV1-based still image; Shape-C ingestion target |
| 6 | AV1 (video) | `rav1e` encoder, `dav1d` decoder | two-library pair | Modern video; differential decoder validates encoder output |
| 7 | AAC | `fdk-aac` (opt-in patent) + `faac` fallback | encoder+decoder | Legacy ubiquity; dual-path required |
| 8 | WebM/MKV mux | `matroska` crate | muxer+demuxer | Container for AV1+Opus |
| 9 | MP4 mux | `mp4` crate | muxer+demuxer | Container for AV1+AAC (+ legacy H.264/AAC read) |
| 10 | HEVC (decode only) | `ffmpeg` via FFI | decoder only | Legacy iPhone-video read support; encoder out of scope |

After order 10 the full AV1+Opus+WebM and AV1+AAC+MP4 pipelines are buildable. Order 11+ picks up long-tail formats (HEIC, WebP, TIFF, GIF, …) at a slower cadence.

**Each codec is §5.2 equivalence-gated** and cannot land as `Complete` until its round-trip test passes. Partial codecs (known bugs in the FFI binding, format subsets only) land as `status: Partial` with explicit `partial_reasons` listing the failed sub-tests — they're searchable and referenceable but fail the build-mode resolver.

**Budget adjustment.** §5.16.10's ~27 kLOC estimate covered the decoders+encoders. Adding the 10-codec FFI-wrapper landings (~200 LOC each = ~2 kLOC) + equivalence-gate test harness (~1 kLOC) + muxer nomtu (~800 LOC each × 3 = ~2.4 kLOC) raises the total to ~32 kLOC. Shipped incrementally over the PR roadmap.

---

### 5.17 Mass corpus ingestion — PyPI + top GitHub

The dictionary gets rich only when real code from real ecosystems has been translated and stored. Phase 5 ingestion (§5.6) was per-symbol on demand. §5.17 is the **systematic enrichment pass**: ingest entire package ecosystems and the most-used open-source repos as bulk jobs, with disciplined disk management so the host machine doesn't drown.

#### 5.17.1 Two mass corpora

**PyPI — the Python Package Index.** Hundreds of thousands of packages; top 500 cover ~80% of the real-world Python usage (numpy, pandas, requests, django, flask, tensorflow, pytorch, sklearn, scipy, matplotlib, pillow, beautifulsoup4, lxml, cryptography, sqlalchemy, pytest, …). Each package's installed form (`site-packages/<pkg>/`) goes through the Phase-5 Shape-B ingestion path.

**Top GitHub — by stars and per-ecosystem popularity.** Not one global top-500; instead **top 500 per major ecosystem**: JavaScript/TypeScript, Python, Rust, Go, Java/Kotlin, C/C++, Swift, Ruby, PHP. Ecosystem-curated lists avoid the Linux-kernel-dominates-everything effect. Total: ~4,500 repos after dedup, weighted by real usage.

#### 5.17.2 Disk-management protocol

The machine running ingestion must not balloon its filesystem. Discipline:

**Stream-and-discard.** For each repo or package:
1. Shallow-clone (git: `--depth=1`) or pip-download into a workspace directory.
2. Run the Phase-5 ingestion pipeline. Produces dict entries (bytes of body_nom + contract + refs + metadata) — typically ~10% of source size due to translation compactness and dedup.
3. **Delete the source tree** as soon as ingestion completes. The dict retains; the raw clone does not.
4. Move to the next repo.

Peak disk usage is bounded by `max(per-repo-source-size) + current-dict-size`, not by the total corpus size.

**Skip rules.** Some repos are genuinely too large or not worth ingesting as-is:
- `> 2GB` shallow-clone size: skip by default; ingest only a subset on explicit request (`--include-path`).
- Vendored monorepos with no exports of interest (build tooling, test fixtures): skip via a maintained skip-list in `nom-extract/src/corpus/skip_list.txt`.
- Binary-dominant repos (datasets, model weights): Phase-5 code ingestion skips; Phase-5 Shape-C media ingestion (§5.16) may pick them up separately.
- Known duplicate mirrors of other repos: skip via URL-normalization + prior-ingestion check.

**Checkpointing + transaction boundaries.** Ingestion can be interrupted and resumed. State: `(repo_url or package_name, stage_in_pipeline, last_completed_symbol_hash)`. Resume from last checkpoint, never redo committed work.

The commit boundary is the **per-symbol SQLite transaction**: one transaction wraps the entry row insert + all its `entry_refs` + all its `entry_graph_edges` + all its `entry_meta` + any `entry_scores` / `entry_security_findings` / `entry_signatures` produced by the equivalence gate. Either all land atomically, or none do. The checkpoint file advances `last_completed_symbol_hash` only after transaction commit; a crash mid-transaction is seen as "symbol N not yet completed" on resume, and the in-flight rows are not present (SQLite rolls them back).

Per-repo transactions (coarser) are available via `--batch-repo` for small repos where many symbols compose a tight subgraph; there, the whole repo commits or nothing does. Default is per-symbol for resumability granularity.

In-flight state that lives *outside* the SQLite transaction — sandbox processes running property tests, partial workspace downloads — is cleaned up by `nom corpus workspace-gc` on resume. No orphaned sandbox state survives a crash.

**Bandwidth throttling.** Default: max 20MB/s per source to avoid crushing the local network. Configurable per-run.

**Workspace GC.** After an ingestion run, `nom corpus workspace-gc` removes any partial downloads or aborted decode attempts. Workspace returns to empty.

**Dict dedup dividend measurable.** After ingesting the top-500 of each ecosystem, report `nom corpus report` prints: source bytes processed, dict bytes added, dedup ratio per ecosystem, per-status entry count (Complete vs Partial vs Opaque). Claim under measurement: 100×+ dedup at corpus scale — most real-world code shares the same underlying vocabulary.

#### 5.17.3 New CLI and module

- `nom corpus ingest pypi --top <N>` — fetch top-N PyPI packages, ingest, clean up. Default N=500.
- `nom corpus ingest github --lang <ecosystem> --top <N>` — top-N GitHub repos for a given ecosystem tag.
- `nom corpus ingest repo <url>` — single-repo pass, for targeted enrichment.
- `nom corpus status` — ongoing ingestion progress, checkpoint state.
- `nom corpus pause` / `nom corpus resume` — interrupt and continue cleanly.
- `nom corpus report` — post-run summary (dedup, per-ecosystem stats).
- `nom corpus workspace-gc` — clean up stray workspace dirs.

Implementation in `nom-extract/src/corpus/`: driver + `pypi.rs` + `github.rs` + `skip_list.rs` + `checkpoint.rs`.

#### 5.17.4 Partial status is the honest initial condition (not the steady state)

Initial mass ingestion will land with a high Partial ratio — dynamic Python typing, heterogeneous GitHub code, large packages all defeat first-pass equivalence. Realistic trajectory: **first-pass ~80% Partial, steady state >90% Complete** via §5.10 canonicalization upgrades over months. The corpus is a living target.

**Partial entries are NOT build-eligible.** They are graph-referenceable (edges can point at them), searchable (they rank in `nom search`), and surface-visible in the LSP (`NOM-G05` diagnostic on use), but the §5.4 intent resolver operating in **build mode** filters to `status: Complete` entries only. A draft `.nom` source whose closure walk hits a Partial entry fails `nom check --comprehensive` with a `NOM-G05` diagnostic telling the author: "reference to Partial entry `<hash>` — either pin a different specialization (Complete), hash-pin with explicit `:: partial` acknowledgment, or wait for the translator upgrade."

This preserves §5.2's equivalence gate as the real guard on buildable closures: **a Complete closure contains only Complete entries; a build is a closure where the Partial escape hatch was never taken.**

**Partial → Complete transitions always produce a new hash.** When §5.10 canonicalization lifts a Partial entry, the updated `body_nom` + contract hashes to a new id, and the old id gains a `SupersededBy(old_partial → new_complete)` edge. Callers still resolving the word pick up the new Complete entry automatically. Old closures remain reproducible (the old hash is still in the dict; §5.10 eliminate rules preserve `SupersededBy` anchors).

#### 5.17.5 Storage budget (trajectory, not steady state)

Numbers reconciled with §5.5: average Complete entry ~300 bytes (body_nom + contract + key metadata; edges and secondary metadata in side tables); Partial entries ~600 bytes (carry `partial_reasons` and more provenance).

- **First-pass target**: ~20M Complete + ~80M Partial ≈ 54 GB, Partial-heavy because translators are immature.
- **Steady state (12-24 months of §5.10 lifecycle passes)**: ~90M Complete + ~10M Partial ≈ 33 GB. The canonicalization + translator-upgrade loop compresses the working corpus by ~40% as Partial entries resolve.
- **Peak working-set source**: ≤ 2 GB (one large repo at a time, per §5.17.2 stream-and-discard).
- **Flow artifacts, benchmark runs, media**: stored in typed side-tables, budgeted separately per §5.13, §5.14, §5.16.

A developer's laptop running the ingestion sees ≤ 10 GB temporary + a growing dict; a dedicated corpus host at first-pass sits at ~60 GB, settles to ~35 GB. Not cheap, but tractable — and unlike a traditional package-manager cache, this is the *working vocabulary* of the entire ecosystem, deduplicated, reusable across all future projects without re-download.

#### 5.17.6 Supply-chain safety (mandatory operational discipline)

Mass-ingesting ~5,000 real repositories and ~500 PyPI packages crosses a real-world hazard the design must name explicitly. This is an operational discipline, not a language invariant — but it's non-optional.

**License hygiene, origin tracking, and attribution are descoped** (2026-04-12). Once code is parsed and translated into Nom, its origin — repository URL, file path, commit, author, upstream license — is intentionally forgotten. The translated `body_nom` stands on its own identity (its hash); no derivative-work chain is tracked. Users accept that this places license-compliance responsibility outside the compiler.

**Retroactive-malware closure recheck cost.** When a package is flagged malicious after ingestion, `nom check --audit` walks the reverse-edge graph to find every closure that transitively reached the quarantined hash. At 100M-entry corpus with typical fan-out, this is minutes per audit event, not seconds — bound the cost, schedule it asynchronously, and let the user opt into foreground mode (`--block`) for high-security contexts.

**Supply-chain safety.** PyPI has a documented history of typosquatting, malicious post-install scripts, and malware uploads. GitHub's long tail is uneven. Mass-ingesting arbitrary packages is itself an attack surface.

- Sandbox every equivalence-gate execution (§5.2 step 3) in a process isolated from the host: no network, restricted filesystem, no environment variables beyond what the test needs, CPU + memory + wall-clock budget.
- Refuse to run arbitrary install-time code during ingestion. `pip download --no-deps --no-build-isolation` then *read the source*; do not execute `setup.py`. `npm install --ignore-scripts`. `cargo fetch` never runs build.rs.
- Reject ingestion of any package whose source contains a known-malicious pattern (YARA-style rules maintained in `nom-extract/src/corpus/safety_rules.yar`): base64 payloads over 1 KB, network calls in install scripts, filesystem writes outside the package tree.
- Quarantine + alert mode: suspicious packages land in a `quarantined_*` kind (not a regular nomtu), with source preserved for human review; never promoted without explicit approval.

**Malware retroactively discovered.** If a package already ingested is later disclosed as malicious:
- Emit a `SupersededBy(malicious_entry → empty_stub)` edge to route resolvers away.
- Flag any Complete closure that transitively included the malicious entry. CI surfaces these via `nom check --audit`.
- The original entry is not deleted (immutability — invariant #5) but is marked `quarantined: true` in metadata. Builds routing through it fail loud.

Without this section the §5.17 subsystem is production-hostile. With it, the subsystem is adequate for the scale the plan targets.

---

### 5.18 Aesthetic as programming — generative media via composition

Media primitives (§5.16) are not just storage. They are also **programmable surfaces**. A `.nom` program can compose `PixelGrid`, `VectorPath`, `AudioBuffer`, `VideoStream`, `MeshGeometry` nomtu via functions — same grammar as any other Nom program — to produce generative aesthetic output.

**Three operators, all three apply.** `->` composes a sequence of transforms; `::` specializes (render at 4K vs. 1080p); `+` combines (layer-stacking for images, mix for audio, concat for video). No new syntax — aesthetic programming is ordinary Nom applied to media primitives.

#### 5.18.1 Examples

- **Generative image.** Compose a `PixelGrid` from procedural noise nomtu + a palette nomtu + a radial gradient nomtu + a post-processing chain. Output materialized as PNG via `nom media render`.
- **Live music piece.** Compose an `AudioBuffer` from oscillator nomtu + envelope nomtu + reverb nomtu + a MIDI-like sequence nomtu. Materialized as WAV, FLAC, Opus via `nom media render`.
- **Animation.** Compose a `VideoStream` from per-frame `PixelGrid` functions over time, with easing curves from §5.11's motion nomtu. Materialized as MP4/WebM/GIF.
- **Procedural 3D.** Compose a `MeshGeometry` from primitive shape nomtu + boolean operations + subdivision. Materialized as OBJ/glTF.
- **Typography.** Generate a variable font instance by interpolating between `GlyphOutline` nomtu with weight/width axes; materialize as TTF/OTF/WOFF2.

#### 5.18.2 Seed the aesthetic vocabulary

New seed entries in `nom-media` covering:

| Category | Example nomtu |
|---|---|
| Procedural patterns | `perlin_noise`, `worley_noise`, `fbm_fractal`, `simplex_noise`, `voronoi_cells` |
| Color synthesis | `gradient_linear`, `gradient_radial`, `gradient_conic`, `palette_from_image`, `oklch_interpolate` |
| Image composition | `layer_stack`, `alpha_composite`, `blend_mode_multiply`, `blend_mode_screen`, `mask_by_alpha` |
| Filters | `gaussian_blur`, `sharpen`, `edge_detect`, `denoise`, `tone_map_aces`, `chromatic_aberration` |
| Geometric transforms | `rotate`, `scale`, `skew`, `perspective_warp`, `kaleidoscope`, `displacement_map` |
| Audio synthesis | `sine_oscillator`, `saw_oscillator`, `noise_source`, `adsr_envelope`, `fm_synthesis`, `granular_sampler` |
| Audio effects | `convolution_reverb`, `bitcrush`, `spectral_gate`, `pitch_shift_psola`, `sidechain_compression` |
| Animation | `ease_cubic_bezier_curve` (from §5.11.2 easing), `keyframe_interpolate`, `spring_animate` |
| 3D | `subdivision_catmull_clark`, `boolean_union`, `boolean_difference`, `sdf_sphere`, `sdf_box`, `marching_cubes` |
| Typography | `variable_font_axis`, `kerning_pair`, `ligature_substitute`, `hinting_instruction` |

All reach `Complete` status via §5.2 equivalence gates. Two implementation paths for each primitive, chosen per entry at seed time:

- **Native Nom** — pure-math primitives (procedural noise, color interpolation, convolution kernels, path arithmetic, FFT, boolean geometry). Bodies are Nom code; equivalence testing is straightforward.
- **Declared FFI tier** — for computationally-dense codecs and pixel/audio engines where a mature Rust crate exists (`image`, `lyon`, `rodio`, `hound`, `kurbo`, `tiny-skia`, `symphonia`), the primitive is an `EntryKind::Ffi` entry whose `ffi_boundary` descriptor points at a stable C ABI surface exposed by the crate. These entries have `body_nom: None` and `origin_ecosystem: crates.io` provenance. The linker resolves them at build time.

**Phase 10 retirement compatibility.** The Rust *compiler* retires; the "media-codec FFI tier" (a specifically scoped, CI-gated exception to invariant #13) remains as linked native libraries the runtime depends on. This exception is narrow: it covers codec/pixel/audio/geometry math kernels only — **never** compiler logic, dict logic, or authoring logic. A CI gate enforces the scope: any new `Ffi` entry with `origin_ecosystem: crates.io` must carry a `media_codec_allowed: true` metadata flag signed off by a maintainer; gate refuses to build if any other crate-origin FFI creeps in. See §10.8 for the CI gate specification.

**Nom-native eventual replacement path.** Each media-codec FFI entry has a `planned_nom_replacement: <hash_or_null>` metadata pointer. The long-term goal is replacing every FFI entry with a Nom-native equivalent once the Nom compiler + runtime can match Rust performance on the relevant math. Until then, the FFI entries are honest `Opaque`/`Ffi` boundaries with declared scope, not hidden compromises.

#### 5.18.3 Aesthetic skills (seed entries)

New `kind: Skill` nomtu specifically for aesthetic authoring:

| Skill nomtu | Purpose |
|---|---|
| `compose_brutalist_webpage` | heuristics + recommended UxPatterns + code scaffold |
| `compose_glassmorphism_dashboard` | glass surfaces, backdrop blur recipes, legibility rules |
| `compose_data_visualization` | chart-type selection from data shape + accessibility + color |
| `compose_generative_art_piece` | procedural pattern + color + post-processing recipes |
| `compose_lofi_audio_loop` | sample + filter + stuttering-delay + vinyl-texture pipeline |
| `compose_technical_diagram` | isometric/orthographic grid + layered notation + line weights |
| `compose_animated_transition` | easing + stagger + entry/exit pairing |

Each skill's `body_nom` is a declarative rule set + example composition chain; the AI Authoring Protocol (§9.2) invokes them when user intent matches.

#### 5.18.4 Voice synthesis — concrete corpus example

§5.18.1's audio-synthesis vocabulary (oscillators, envelopes, reverb, granular) covers the low-level generative layer — useful for procedural sound design but not for human-speech TTS at production quality. Speech is a distinct problem: it needs trained models, speaker-identity control, multilingual coverage, and diffusion/flow-matching acoustic decoders. Nom ingests full speech-synthesis pipelines as Shape-B runtime-library corpora, same pattern as framer-motion or Dioxus.

**Reference corpus at `C:\Users\trngh\Documents\VoxCPM-main`.** A tokenizer-free end-to-end TTS system supporting 30 languages, 16–48 kHz output, with three synthesis modes (voice design from text description, controllable cloning from reference audio, ultimate cloning via audio continuation). Architecture: MiniCPM-4 text semantic LM + residual acoustic LM + AudioVAE + local diffusion transformer (continuous flow matching) + stop predictor + optional ZipEnhancer denoiser + optional LoRA adapters.

**Native Nom mapping (indicative; brand lives in `origin_ecosystem: pypi` + `origin_package: voxcpm` only):**

| Source surface | → | Nom nomtu | Kind |
|---|---|---|---|
| `VoxCPM` top-level class | → | `voice_synthesis_engine` | `Function` |
| `generate(text, ...) → waveform` | → | `synthesize_speech` | `Function`, `effect: pure + gpu_compute` |
| `generate_streaming(...) → chunks` | → | `synthesize_speech_streaming` | `Function` |
| `build_prompt_cache(prompt_wav, prompt_text)` | → | `voice_context_cache_builder` | `Function` |
| `reference_wav_path` parameter | → | `voice_reference_embedding` | `UxPattern` (input spec) |
| `prompt_wav_path + prompt_text` | → | `audio_continuation_prompt` | `UxPattern` |
| Text-prefix voice descriptor (e.g., `"(young female, warm tone) …"`) | → | `natural_language_voice_descriptor` | `UxPattern` |
| `cfg_value` | → | `classifier_free_guidance_scale` | `UxPattern` |
| `inference_timesteps` | → | `diffusion_step_budget` | `UxPattern` |
| `min_len` / `max_len` | → | `audio_token_length_bounds` | `UxPattern` |
| `denoise: bool` + ZipEnhancer | → | `acoustic_denoiser` | `Function` |
| `normalize: bool` | → | `text_normalization_pass` | `Function` |
| `retry_badcase: bool` | → | `regeneration_on_quality_fail` | `UxPattern` |
| Text Semantic LM (MiniCPM-4 backbone) | → | `text_to_semantic_encoder` | `Function` |
| Residual Acoustic LM | → | `residual_acoustic_refiner` | `Function` |
| Local Encoder (`feat_encoder`) | → | `audio_feature_projector_in` | `Function` |
| Local DiT (`feat_decoder`, UnifiedCFM) | → | `flow_matching_acoustic_decoder` | `Function` |
| AudioVAE encode/decode | → | `audio_latent_vae_encoder` / `audio_latent_vae_decoder` | `Function` |
| FSQ (scalar quantization) | → | `finite_scalar_quantizer` | `Function` |
| Stop predictor | → | `audio_boundary_predictor` | `Function` |
| LoRA adapter (rank, alpha, target modules) | → | `voice_personality_lora_adapter` | `UxPattern` |
| `LoRAConfig{enable_lm, enable_dit, enable_proj, r, alpha, dropout, target_modules_*}` | → | `lora_training_config` | `UxPattern` |

**Per-language variants via `Specializes`.** One generic `synthesize_speech` nomtu + 30 language-specialized siblings distinguished by `origin_language: en|zh|ja|ko|…` metadata, each `Specializes(generic)`. Calling `synthesize_speech` with a detected language auto-routes via the §5.4.0 resolver to the right specialization.

**Per-quality / per-latency specializations.** Same pattern applied to `(cfg_value, inference_timesteps)` tuples: a real-time mode (low steps, low cfg) and a studio-quality mode (high steps, high cfg) coexist as Specializes children. The §5.13 benchmark-driven selector picks the right one based on declared build objective (low latency vs. top quality).

**Dependencies through Shape-B ingestion.** `torch`, `torchaudio`, `transformers`, `librosa`, `soundfile`, `funasr`, `modelscope` are all ingested via the standard Phase-5 Shape-B path (§5.6) for Python corpora. Their translated primitives back the `voice_synthesis_engine` composition. The FFI tier (§10.8 category `signal_math` or a new category `ml_inference` if the enumeration expands) handles the underlying tensor math; the voice-specific surface is Nom-native composition over those primitives.

**Aesthetic skills for voice (seeded alongside §5.18.3).** Add:

| Skill nomtu | Purpose |
|---|---|
| `compose_podcast_voice` | narration-friendly timbre + prosody recipes |
| `compose_audiobook_voice` | long-form reading, chapter-boundary pacing |
| `compose_assistant_voice` | conversational latency-aware design (low timesteps, medium cfg) |
| `compose_character_voice` | stylized character voices for games / animation |
| `clone_voice_ethically` | legal + consent checklist + technical pipeline for voice cloning |

**Music is a separate corpus, not covered by VoxCPM.** VoxCPM is speech-only by design — its encoder, training corpus, and decoder are tuned for intelligibility and expressiveness of human voice, not instrumental or ambient audio. A music-generation corpus (candidates: MusicGen from Meta, Stable Audio from Stability AI, AudioCraft's MelodyFlow, Meta's Voicebox for sound effects) would ingest under the same Shape-B pattern with a `music_generation_engine` root nomtu and its own Specializes children per genre / duration / modality. Explicitly NOT in this section — don't conflate.

#### 5.18.5 Governing insight

The user's framing captures it: **aesthetic IS programming.** Visual, auditory, kinetic design isn't adjacent to code — it's composition of the same kind, with the same `->` / `::` / `+` operators, over a different vocabulary (media primitives instead of compute primitives). The dict holds both. The compiler materializes both. The AI authoring layer surfaces candidates for both from the same ranking function.

---

### 5.19 The AI invokes the compiler — closing the intent loop

Phase 9 established the AI authoring layer as a *search* surface: intent → ranked dict candidates → human picks → hash-pinned source. §5.19 extends this: the AI also **invokes the compiler as a tool** during authoring, using its deterministic output as feedback.

#### 5.19.1 The loop

When a user says "build me an app that does X":

1. **AI composes a draft.** Queries the dict via Authoring Protocol (§9.2), produces a candidate `.nom` source. Does not write it to disk yet.
2. **AI calls `nom check --comprehensive`** on the draft. The three-layer check (§9.8) reports sentence/paragraph/graph diagnostics.
3. **Fix loop.** Any diagnostics become refined intent queries. Missing references → orphan resolution. Contract mismatches → adjust compositions. Repeats until comprehensiveness score ≥ threshold.
4. **AI calls `nom build`** to produce artifacts.
5. **AI calls `nom bench`** to measure (when benchmark data is relevant — performance-sensitive builds only).
6. **AI calls `nom flow record`** with sample inputs to verify execution.
7. **AI reports back with evidence.** User sees: the draft `.nom` file, the artifacts, the benchmark numbers, the flow artifact DAG. Not "I generated this, trust me" — "I generated this, here's the compiler's own verdict."

The compiler is a **deterministic oracle the AI consults**. This is what makes AI-assisted Nom production more than a fancy autocomplete: every generated artifact has machine-verified properties, not LLM-hopeful output.

#### 5.19.2 Authoring Protocol extensions

§9.2's schema gains new request modes:

- **`verify`** — the AI submits a draft source; the protocol returns comprehensiveness diagnostics.
- **`build`** — the AI submits a draft source + build targets; the protocol returns artifact hashes + any errors.
- **`measure`** — submit a hash + benchmark workloads; returns benchmark run records.
- **`trace`** — submit a hash + inputs; returns flow artifact.
- **`explain`** — submit a hash + a diagnostic; returns a human-language explanation backed by flow data + benchmark data + closure context.

All five modes are deterministic in the compiler half; the AI's natural-language translation layer is the only non-deterministic part. Re-running the same AI with the same seed + same dict state produces the same draft → same artifacts.

#### 5.19.3 Self-documenting skills for Nom

Seed skills specifically about **using Nom itself**, layered above the domain-specific skills from ECC (§9.8.8) and aesthetic skills (§5.18.3):

| Skill nomtu | Purpose |
|---|---|
| `author_nom_app` | how to write a Nom application from intent |
| `compose_from_dict` | how to find and chain nomtu for a goal |
| `declare_contract` | how to write pre/post/effects |
| `debug_nom_closure` | how to inspect a failing closure |
| `optimize_nom_build` | how to use `nom bench` + specialization selection |
| `troubleshoot_parity` | how to debug Rust-vs-Nom output divergence during Phase 10 |
| `extend_nom_compiler` | how to add a new compiler nomtu (§10) |
| `ingest_new_ecosystem` | how to write a Shape-B importer for a new language |
| `create_ux_pattern` | how to author a new UxPattern (§5.11) |
| `build_cross_platform_app` | how to use `Specializes` + `origin_platform` (§5.11.2 Shape B cross-platform) |
| `create_media_encoder` | how to add a new encoder to `nom-media` (§5.16) |
| `run_mass_ingestion` | how to use `nom corpus ingest` with disk safety (§5.17) |
| `author_generative_art` | how to compose aesthetic programs (§5.18) |
| `use_ai_loop` | how to drive the AI-compiler loop (§5.19) |

Each skill's `body_nom` is a concise declarative rule-set plus example compositions, hash-pinned to current-best dict entries. **These skills are Nom's self-documentation, stored as nomtu, queryable by AI and humans alike.** A user new to Nom types "how do I X?" → LSP queries skills matching X → gets a ranked list with example compositions. A compiler contributor types "how do I extend codegen?" → gets `extend_nom_compiler`.

This is the ECC homunculus pattern (§9.8.8) applied recursively: Nom's own methodology lives in the dict, evolves with the dict, and gets surfaced to new users (or new AIs) through the same intent-routing mechanism everything else uses.

#### 5.19.4 Cost and non-determinism boundary

**Cost model.** An authoring session is not free. A conservative per-iteration cost:

| Step | Cost (order of magnitude) |
|------|---------------------------|
| Intent query via Authoring Protocol | ~100 ms resolver + ~1–3 s AI natural-language translation |
| `nom check --comprehensive` on a draft | ~200 ms to 5 s depending on closure size |
| `nom build` to artifacts | ~1 s to 30 s depending on closure size + specialization selection |
| `nom bench` (when perf-sensitive) | ~5 s to 60 s depending on workload sweep |
| `nom flow record` | ~100 ms to 5 s depending on input size |

Typical session: 3–5 iterations × ~10–40 s per iteration = 30 s to ~3 minutes of compute per completed `.nom` source of non-trivial size. The compiler-half costs grow O(closure size); keep closures small via composition, not inlining.

Per-session budget under `nom build --ai-loop`: default cap 10 minutes wall-clock, configurable via `--ai-budget-ms=<N>`. On budget exhaustion the loop surfaces the current best draft with diagnostics instead of silently continuing.

**Non-determinism boundary.** Invariant #12 (determinism survives AI mediation) requires care here. Two different AIs — or the same AI with different seeds — will not produce identical candidate-selection trajectories for the same intent. To keep the claim honest:

- **Deterministic half (enforced):** `verify`, `build`, `measure`, `trace` Authoring Protocol modes. Same input → same output. Invariant.
- **Non-deterministic half (acknowledged):** `explain` mode produces human-language prose via an LLM; different calls yield different phrasing. `explain` output is NOT a build input; it's user-facing diagnostic text. Downstream tools must not parse `explain` output as data.
- **Draft composition is non-deterministic.** Two authoring sessions targeting the same intent may land on different hash-pinned sources, both valid. Record the authoring trajectory explicitly so this divergence is auditable instead of silent.

**Draft-composition query uses SEARCH mode, not build mode.** §5.19.1 step 1 (AI composes draft) issues Authoring Protocol queries against the search-mode resolver — which returns Complete, Partial, AND Opaque candidates. This is deliberate: Partial entries are useful stepping stones for the AI to propose, because the §5.19.1 step 2 `nom check --comprehensive` will reject closures that transitively reach unacknowledged Partial entries and the AI can iterate. AuthoringTrace records Partial candidates that were proposed-and-rejected as valuable supervisory signal for §5.4.0 threshold calibration.

Invariant #17 is refined accordingly: `nom check --comprehensive` and `nom build` use build mode (Complete only). Draft composition in `nom lsp` and the AI authoring loop use search mode. The enforcement boundary is the check step, not the suggestion step.

**`AuthoringTrace` — a new kind for capturing trajectories.** Extend `EntryKind`:

```
AuthoringTrace {
    session_id: String,
    intent_query: String,             // the user's original ask, verbatim
    candidate_pools: Vec<CandidatePool>,  // per iteration: what the resolver returned + what the AI picked
    diagnostic_trail: Vec<Diagnostic>,    // every `nom check` / `nom build` result
    final_source_hash: String,        // the hash the session landed on
    compiler_hash: String,            // which compiler version ran
    ai_identifier: String,            // "anthropic:claude-opus-4-6" or similar
    ai_seed: Option<String>,          // if the AI exposes a deterministic seed
    started_at: String,
    completed_at: String,
    total_compute_ms: i64,
}
```

Stored like any other nomtu. Two sessions with the same intent but different final hashes have two distinct `AuthoringTrace` entries, and `nom flow diff <trace_a> <trace_b>` surfaces exactly where the trajectories diverged (iteration 2, candidate selection changed, …). The AI is still the source of variation, but the variation is now on the record. Teams can audit: "did switching from AI X to AI Y change what code lands in the dict?"

**AuthoringTrace retention policy.** Candidate pools per iteration accumulate fast — a 3-5 iteration session with ~8 candidates each at ~20KB per trace means ~200GB/year at a large user base. Without retention this would dwarf the dict body itself. Policy:

- **Default: 90-day rolling full-fidelity retention.** Traces within 90 days keep full `candidate_pools` for resolver calibration.
- **Lossy compression after 90 days.** Traces older than 90 days compress to: `session_id` + `intent_query` + `final_source_hash` + `compiler_hash` + `ai_identifier` + `total_compute_ms`. `candidate_pools` and `diagnostic_trail` are dropped. The compressed trace stays as a historical record but doesn't count against the full-fidelity storage budget.
- **Permanent full-fidelity exception.** Traces attached to a `Complete` entry that landed in the dict are kept full-fidelity indefinitely as provenance — they show how the canonical entry was authored, which is useful for teaching `use_ai_loop` and `author_nom_app` skills.
- **Enforcement:** `nom store gc --authoring-traces` runs the compression pass, scheduled daily by convention. Budget: AuthoringTrace storage is tracked separately in `nom corpus report` under `trace_bytes` — NOT part of the 33GB steady-state dict budget.
- **Privacy note.** `intent_query` contains user natural-language asks, which may be sensitive. Mass supervisory training on the trace corpus requires explicit user opt-in (`nom config set trace_share_for_training true`). Default: traces stay local, used only for the user's own resolver calibration.

**What "AI is replaceable" actually means given this.** Two AIs plugged into the same Authoring Protocol will produce different trajectories. The language + compiler + dict + Authoring Protocol schema are AI-independent. The output *source* is AI-dependent — but its reproducibility is guaranteed by hash-pinning: once a source is stored, rebuilding it without any AI produces byte-identical artifacts. The AI is replaceable *at the authoring boundary*, not retroactively through a project's code.

#### 5.19.5 Skill entries are exempt from build-closure walk; resolution vs walking

Self-documenting skills (§5.19.3 + earlier skill-kind entries from §9.8.8, §5.18.3) are documentation artifacts, not code. They compose via `Recommends` edges that are semantic suggestions, not build dependencies. Without an exemption rule, the skill graph's circular references (`use_ai_loop` references `author_nom_app` references `compose_from_dict` references `use_ai_loop`) would make `nom check --comprehensive` on any composition that transitively touches a Skill never terminate.

**Distinguish RESOLUTION from CLOSURE WALKING.** These are two different graph operations:

- **Resolution** (§5.4): maps a bare name or partial reference to a specific canonical hash. Resolution **does** follow `SupersededBy` chains to the head — that's how `use foo` finds the current canonical entry, and how `SupersededBy(malicious → empty_stub)` routes resolvers away from quarantined entries (§5.17.6). Resolution **does not** follow `Recommends` or `SimilarTo` (those are for search/browse, not for picking a hash).
- **Closure walking** (`nom build`): given a set of resolved hashes, transitively expands via build-relevant edges to collect the full build set. Closure walking **does not** follow `SupersededBy`, `Recommends`, or `SimilarTo` — they are not transitive dependency relationships for build purposes. Closure walking follows only `Calls`, `Imports`, `Implements`, `DependsOn`, `Specializes`, `ContractMatches`.

**So the full flow for every bare reference in source is**: resolve (short-circuits SupersededBy to current head) → emit the head hash into the pre-walk seed set → closure-walk from seed set (ignoring SupersededBy because you're already at the head). This reconciles §5.10.2 (resolver walks short-circuit to head), §5.17.6 (malware quarantine via SupersededBy routing), and §5.19.5's closure-walk exemption without contradiction.

Rules:

- **Skills are never build-closure-walked.** The outgoing edges carried on `kind: Skill` entries exist only for documentation traversal (`nom store history`, `nom ux seed`, intent routing, `nom flow show`) — the `nom build` closure walker skips Skill nodes entirely and does not expand their edges.
- **`Recommends`, `SimilarTo`, `SupersededBy` edges are never build-closure-walked.** Independent of entry kind. These edges exist for other graph operations (search, supersession routing during resolution, lifecycle queries), not for build materialization.
- **Skills may carry Recommends edges to nonexistent hashes** (stale links from supersessions). This is NOT a `NOM-G03` orphan error. Skills are exempt from the graph-layer orphan check in §9.8.3 because their edges are suggestions, not dependencies. CI surfaces stale skill recommendations via `nom store verify-skill-graph --report-stale`, but compilation is never blocked.
- **Skill circularity is tolerated and useful.** A user asking "how do I write an app?" gets a skill chain that loops naturally — that's the documentation structure, not a compile error.

CI check: `nom store verify-skill-graph` ensures no skill nomtu has a non-`Recommends`/`SimilarTo` outgoing edge (build-relevant edge types like `Calls`/`Imports`/`Implements`/`DependsOn` are forbidden on Skills — if a skill ever "depends on" code, the design is wrong, the skill should *describe* the code, not *import* it).

The CI check itself is run by a standard CI pipeline (GitHub Actions, pre-commit hook, or equivalent external orchestrator) — CI pipelines are not nomtu in the current scope of §10.8, though their configuration files can be ingested as Shape-A (static knowledge) entries for auditability.

#### 5.19.6 What makes this different from "AI writes code"

- **The AI doesn't invent primitives.** It composes existing nomtu. The dict is authoritative.
- **Every draft is machine-verified.** Comprehensiveness, build, trace — deterministic checks, not self-grading.
- **Artifacts are hash-pinned and reproducible.** Same draft source → same hash → same artifact on every host.
- **The AI is replaceable.** A better AI tomorrow plugs into the same Authoring Protocol; the dict + compiler stay the same. The compiler outlasts the AI.
- **Users can audit.** The flow artifact shows what actually ran; the bench shows cost; the closure walk shows what was reached. Trust is backed by evidence, not opinion.

This is what "AI-assisted programming language" means when taken seriously.

---

### 5.10 Dictionary lifecycle operations — merge, eliminate, evolve

The corpus is a living graph. Content addressing dedups trivially, but at 100M-entry scale the dict also needs **organic consolidation**: near-duplicates collapse, dead entries get pruned, improved translations supersede older ones without losing history. Three operation classes, each rooted in the multi-edge schema from §5.3.

#### 5.10.1 Merge — collapsing equivalence classes

Three distinct merge mechanisms with different guarantees.

**Hash-identity merge (automatic, free).** Same canonical AST + same contract → same id. Already delivered by Task A (Phase 4). No action needed. Cross-ecosystem dedup happens inherently: React in JS/TS/Flow, translated to the identical Nom AST, lands on one hash.

**Canonicalization-upgrade merge.** The canonicalizer evolves — new Nom constructs get normalized forms, sugar gets desugared, operator associativity gets fixed. When the canonicalizer changes, existing entries may re-canonicalize to a different hash.

**Resolution of the append-only/evolution tension** (adversarial-review follow-up, 2026-04-12): The canonicalizer comment in `nom-types/src/canonical.rs` declares an append-only discipline — *"Adding new variants appends — never reorder, never recycle; doing so would invalidate every id ever computed."* That discipline holds **within a single canonicalizer version**. Cross-version changes (desugaring, operator associativity fixes) are NOT append-only at the tag-stream level; they deliberately rehash existing bodies. We reconcile this by **version-scoping**:

- Hash pins in source carry an optional canonicalizer-version prefix: `use #<canon_v>:<hash>@<word>` (default `canon_v` elides to the current version).
- When `nom store recanonicalize` lifts entries to a new canonicalizer version, every changed entry gains a mandatory `SupersededBy(old_id → new_id)` edge — this is NOT opt-in; the sweep fails loud if the edge cannot be inserted (e.g. body couldn't be re-parsed).
- Source files with un-prefixed hash pins resolve via `SupersededBy` to the current head. Source files with explicit version-prefixed pins freeze at that version and require `nom source migrate <file>` to move forward.
- The proof-of-bootstrap tuple (§10.3.1) records the canonicalizer version in use.

Detection:
1. Periodic sweep: `nom store recanonicalize [--sample N]` recomputes `id` for a random sample (or all) of entries using the current canonicalizer.
2. If the recomputed hash differs from the stored hash, and the recomputed hash already exists as another entry → the two entries are the same symbol under the new canonicalizer.
3. Emit `SupersededBy(old → new)`. Keep both entries (dict is immutable), but resolver ranks the new one higher and the old entry's `is_canonical` flag flips to false.

**Semantic-equivalence merge.** Two entries with **different** canonical ASTs that pass the same property tests under a unified contract. These are different implementations of the same semantic contract — not duplicates. Detection:
1. Cluster candidates by signature shape and concept (the heuristic pre-filter).
2. Within a cluster, run cross-property tests: generate inputs from one entry's contract, run both bodies, compare outputs + effects.
3. If all pass over a sufficient sample → emit **`ContractMatches`** edge between the pair. Both entries remain first-class. The resolver and Phase-12 specializer can substitute one for the other based on context.

`ContractMatches` is **not** `SupersededBy`. The former says "interchangeable"; the latter says "the new one is authoritative." Merge infrastructure distinguishes these cleanly.

#### 5.10.2 Eliminate — prune noise, keep the graph sharp

Garbage collection today (`nom store gc`) removes entries unreachable from any root in `~/.nom/roots.txt`. Phase 5 extends the rule set:

**Unreachable after cooldown.** An entry with zero incoming refs AND not present in any root is a GC candidate. A `last_referenced_at` metadata row (auto-updated on any closure walk that touches the entry) provides the cooldown timer. Default: 30 days since last reference before GC considers it.

**Stale Partial.** Entries with `status: Partial` and `updated_at` older than `partial_stale_ttl` (configurable, default 7 days) get their `body_nom` demoted to null and are labeled for re-ingestion. The entry id stays (graph references don't break); the body is reclaimable disk. `nom store reingest <id>` picks it up from the ecosystem cache and retries translation with the current translator.

**Collapsed supersession chains.** A chain `A → B → C → D` via `SupersededBy` edges gets flattened to `A → D` directly. Intermediate entries keep their historical edges, but resolver walks always short-circuit to the head.

**Duplicate Ffi / ExternalOpaque.** Two entries with identical `ffi_boundary` descriptors but different ids (often from independent ingestion of the same native library across repos) merge. Canonical rule: lower id string wins; the other gets `SupersededBy(winner)`.

**What gc never removes:** entries with any incoming `SupersededBy` or `Specializes` edge. These are historical anchors; deleting them would break version lineage.

#### 5.10.3 Evolve — progression without version numbers

The dict holds the version history of every symbol as a subgraph. No semver, no version strings in identifiers. Only immutable hashes and `SupersededBy` + `Specializes` edges.

**Translator improvement.** A new nom-extract translator version produces a better `body_nom` for an existing symbol. Because canonical AST may differ, a new hash is produced. The new entry inherits the old entry's name + concept + labels (via merge of `entry_meta` on hash-bridge), and `SupersededBy(old → new)` is emitted. Resolving the word now returns the new hash.

**Contract refinement.** Static analysis discovers a stricter `pre` or `post` that the original symbol in fact satisfies. The refined contract produces a new id (contract participates in the hash). `SupersededBy(old → refined)`. Type-check call sites benefit from the stricter contract automatically.

**Specialization (Phase 12 output).** A generic `foo<T>` invoked only with `T = i64` anywhere in a closure produces a specialized `foo_i64` with its own hash. Edges: `Specializes(foo_i64 → foo)`. The specialization is globally cached (it's content-addressed too), so every closure that ends up wanting `foo<i64>` reuses it.

**Deprecation.** A curator (or `nom store recommend-deprecation` heuristic) marks an entry `deprecated_by: <new_hash>`. Resolver stops returning it except when explicitly hash-pinned. `nom store gc` preserves it by default (history anchor); `--purge-deprecated` opts into removing entries whose deprecation is older than configurable TTL AND have no incoming refs beyond `SupersededBy`.

**The version-history query.** For any hash `H`:

```
nom store history H
→ walk `SupersededBy` backwards to all predecessors (older versions)
→ walk `SupersededBy` forwards to find current canonical
→ walk `Specializes` both directions for the variants graph
→ render as a DAG
```

No version string was needed. The graph *is* the version history.

#### 5.10.4 Operational commands

| Command | Purpose |
|---------|---------|
| `nom store recanonicalize [--sample N] [--dry-run]` | Re-hash entries under the current canonicalizer; emit SupersededBy where a collision to an existing entry is found. |
| `nom store find-equivalents [--concept <c>] [--min-confidence <f>]` | Cluster + cross-test candidates; emit ContractMatches edges. |
| `nom store compress-supersession` | Flatten SupersededBy chains to direct edges. |
| `nom store reingest <id>` | Retry translation for a Partial entry against the ecosystem cache. |
| `nom store history <id>` | Render the version-lineage DAG for an entry. |
| `nom store recommend-deprecation [--heuristic <h>]` | Suggest entries to mark deprecated based on usage + successor presence. |
| `nom store gc [--purge-deprecated]` | Extended GC with the rule set in §5.10.2. |

Each command is a read-mostly analysis pass (recanonicalize, find-equivalents, history) OR a write-small graph surgery (compress-supersession, recommend-deprecation). Nothing modifies stored `body_nom` — immutability preserved.

#### 5.10.5 Why these operations, not others

The design constraint: the dict must evolve, but **no stored body can ever change**. Immutability is the bedrock of content addressing (AVOID-5). Evolution happens only through:

- New hashes (new entries).
- New edges (graph mutations).
- Metadata merges (EAV rows add/update; never delete unless GC-driven).
- GC (entries go away but their hashes' historical presence is preserved via supersession edges pointing at them).

Operations that would violate immutability — e.g., "patch this `body_nom` to fix a typo" — are **not** supported. The path is always: new entry, new hash, `SupersededBy` edge from the old. The cost of this discipline is the dict grows; the benefit is every closure that ever resolved is reproducible.

---

## Phase 5 — Recursive symbol ingestion with hash rewriting (DEPRECATED, superseded by §5 above)

(Retained below for diff-trail purposes; the active spec is §5.0–5.9.)

**Goal:** Every source file ingested into nom-dict becomes **self-contained**. All `import react`, `use serde`, `#include <stdio.h>`, `from numpy import ...` are replaced with hash references to `.nomtu` entries in the dict. The stored body has zero ecosystem-package names. Feeding the closure to `nom build` produces a working artifact without touching npm/pip/cargo ever again.

### 5.1 The protocol (5 steps)

**1. Parse + scan imports.** Tree-sitter the source. Extract every import/require/use/include and every call to a symbol defined outside the current file. Produce a list of `(symbol, originating_package, resolution_hint)`.

**2. Resolve from local ecosystem cache.** Per-language resolver walks the user's local ecosystem cache (never the network):
- **JS/TS** → walk up from the source file through `node_modules/`, honor `package.json#exports`, follow `.d.ts` for typings, handle scoped packages.
- **Python** → `site-packages` for the active interpreter, `__init__.py` re-exports, PEP 517 wheels.
- **Rust** → `~/.cargo/registry/src/` (hash-versioned crate sources), `target/doc/` for signatures.
- **Go** → `$GOPATH/pkg/mod/`.
- **Java/Kotlin** → jar extraction or decompilation from `~/.gradle/caches/modules-2/`, Maven's `~/.m2/repository/`.
- **C/C++** → header search paths for `#include`, library paths for symbol linkage.

If the resolver can't find the symbol locally, the entry gets `status: Partial` with a structured note telling the user exactly which ecosystem fetch command will resolve it (`npm install`, `pip install`, `cargo fetch`). Nom never auto-fetches.

**3. Recurse, memoized.** Before descending into a resolved source, compute its hash and check the store. If present, stop. This gives structural dedup across the whole corpus and terminates cycles.

**4. Translate each symbol to a `.nomtu` entry.** Per function / type / class / constant:
- `body` — original source snippet as a string (for traceability).
- `body_nom` — best-effort translation into Nom syntax. Translation quality varies by language; expose a `translation_score: 0.0–1.0`. Languages close to Nom (F#, OCaml, Rust without lifetimes) translate well; languages far from Nom (Perl, dynamic Python, C++ templates) translate partially.
- `contract` — inferred from the signature + static analysis of the body (effects: pure/io/ffi/dynamic_dispatch; pre/post where provable).
- `refs` — the hashes of every external symbol this entry references.
- `hash = hash(body + contract)`.

**5. Rewrite references.** Walk `body_nom`'s AST; every identifier that referenced an external symbol becomes `#<hash>@<name>`. The stored Nom body contains no bare ecosystem identifiers. A re-build of the closure depends only on the dict.

### 5.2 Hard edges

**Native libraries (libc, OpenSSL, syscalls, CUDA, GPU shaders).** The dict stores a `.nomtu` entry with `kind: Ffi`, `body_nom: None`, and a structured `ffi_boundary` descriptor: calling convention, shared-object name, symbol name, ABI type signature. The linker resolves the actual binary at build time (it's a user/runtime concern, not a dict concern). Contract verification still works statically against the FFI signature.

**Closed-source or unparseable deps.** `kind: ExternalOpaque`, `body: None`, `body_nom: None`. The contract is inferred from usage sites across the corpus (if the same opaque symbol is called many ways, the contract is the union of observed call shapes). Compile-time contract checking works; materialization at build time requires the user to provide a binary implementation.

**Macros and metaprogramming.** When expansion is static (C `#define`, Rust declarative macros with fully-expanded output), ingest the post-expansion AST. When expansion depends on runtime (Rust proc-macros, Python decorators with side effects), store both the call site and the last-observed expansion, mark `status: Partial`.

**Dynamic languages.** Python monkey-patching, JS prototype pollution, Ruby's `method_missing` — impossible to fully capture statically. Mark `effect: dynamic_dispatch`. Contract-check at use sites rather than definition sites. Accept that coverage is best-effort, not complete.

### 5.3 Content-addressed dedup is the scale win

Claim to benchmark: ingesting 10 popular Node apps (all using React, Lodash, Axios, etc.) produces a dict where ≥70% of hashes are shared across multiple apps. Across a large corpus (thousands of apps), dedup ratio approaches the ratio of unique symbol definitions to total imports — empirically ~100–1000×.

This matters because it's what makes the whole approach feasible on disk. A raw `node_modules/` forest for 10,000 Node apps would be terabytes. A content-addressed nom-dict of the same corpus is on the order of gigabytes.

### 5.4 Deliverables

- `nom-extract/src/ingest.rs` — recursive memoized walker.
- `nom-extract/src/resolvers/` — per-ecosystem resolver modules (node.rs, python.rs, cargo.rs, gomod.rs, gradle.rs, cinclude.rs). ~200–400 LOC each.
- `nom-extract/src/rewrite.rs` — AST rewriter that swaps external idents for hash refs.
- `nom-extract/src/translate/` — per-language → Nom translation modules. Ship quality is incremental; Rust → Nom first (highest fidelity), then TypeScript → Nom, then Python → Nom.
- New `AtomKind` variants: `Ffi`, `ExternalOpaque`.
- New `Relationship` edges: `ReferencesHash` (replaces the package-manifest-level `DeclaresExternalDep` concept — there are no packages).
- `nom ingest <repo-path>` subcommand. Prints: total symbols ingested, dedup rate, partial-status count, opaque count.
- Tests: fixture repos for each ecosystem (mini Node app, mini Flask app, mini Cargo workspace, mini Go module, mini Maven project). Each has a `expected-closure.txt` describing the closure.

### 5.5 Verification

- **Self-contained check:** for every `status: Complete` entry in the dict, running a hash-closure walk produces a DAG where every leaf is either `Ffi` or `ExternalOpaque`. Zero `#bare_name` references remain in any translated body.
- **Dedup check:** ingest the same repo twice; the second run adds zero new hashes.
- **Build check:** `nom build <hash>` on a small ingested-from-real-ecosystem entry produces a working artifact (FFI deps satisfied by normal linker).

### 5.6 Maps to

- **ADOPT-2** — content-addressed search becomes the primary composition mechanism.
- **ADOPT-10** — structural interface satisfaction: an `ExternalOpaque` can be swapped for any entry with a compatible contract, no `implements` declaration needed.
- **AVOID-8** — single binary: `nom` is the only tool needed for the whole ingestion-to-build loop. `npm`/`pip`/`cargo` are only ever used to populate the local cache, never invoked by `nom`.

### 5.7 Size/scope budget

- ~3000–4000 LOC total across resolvers + ingest walker + rewriter + per-language translators + CLI.
- Each resolver is independent and can ship in its own PR.
- Zero new tree-sitter grammars; reuse what's already linked.
- Two new `AtomKind` variants, one new `Relationship`.
- Tests: +20 fixture-based (~2 per ecosystem × 10 ecosystems).

---

## Phase 6 — Parser-in-Nom prerequisites (1 week)

Unchanged from v1. Three small backend gaps:

1. **Tuple destructuring in `let`** — change `LetStmt.name: Identifier` to `LetStmt.pattern: Pattern`. Parser extension. ~1–2 days.
2. **List concat operator** (`list + list`, `list + [elem]`) — runtime `nom_list_concat` + LLVM binop lowering. ~1–2 days.
3. **Enum variants named with statement keywords** (`enum Token { If, Else, For, ... }`) — contextual-keyword mode in variant lists and struct field names. ~1–2 days.

Verification: targeted tests per fix; workspace stays ≥255.

---

## Phase 7 — Parser in Nom (10–14 weeks)

Unchanged from v1. Port `nom-parser` to `stdlib/self_host/parser.nom`. Add `Box<T>` for recursive AST types (nom_alloc + ptr). Recursive-descent mirroring the Rust parser. Existing fixtures pass. Maps to roadmap Phase 2.

---

## Phase 8 — Architectural ADOPT (6–12 months, overlaps Phase 7)

Unchanged from v1.

- **8.1 ADOPT-4** — Supervision tree for flow faults (`onfail: restart_from|abort|escalate`).
- **8.2 ADOPT-5** — Aspect-qualified flows (`flow::once|stream|scheduled`).
- **8.3 ADOPT-6** — Datalog-style dictionary queries over the attribute graph. The Phase 5 hash-refs make these queries trivial — "find every entry that transitively references hash X" is a graph walk.
- **8.4 ADOPT-7** — Bidirectional contract inference via unification.
- **8.5 ADOPT-8** — Persistent immutable data structures (HAMT) for `effect: pure` entries.

Each ships as a separate PR.

---

## Phase 9 — LSP + Authoring Protocol + `.nomtu` WASM plugins (12–16 weeks, CORE not gated)

**Reframed from v1 ("gated, optional"):** the LSP is now the AI-mediated authoring surface that makes the Tier-2 vocabulary usable at 100M-entry scale. Without it, the language is effectively write-only. Phase 9 is core infrastructure, not opt-in polish.

**Dependency ordering (adversarial review item 6, 2026-04-13):** Phase 9 LSP should **precede or overlap §5.17** mass ingestion — `nom check --audit` during ingestion (which surfaces diagnostic-grade issues on translated bodies) depends on the same diagnostic infrastructure the LSP consumes. Shipping §5.17 first means ingestion runs without any authoring-loop feedback; shipping Phase 9 first lets the dict grow under the LSP's quality gate from day one. Similarly, §5.10 lifecycle (merge/eliminate/evolve) should defer to after Phase 7 — the canonicalizer is owned by the Rust parser until Phase 7 ships the Nom parser, and lifecycle ops must invalidate + recompute source hashes through the canonicalizer, which is more brittle when two implementations exist in parallel.

### 9.1 LSP server (the substrate)

`nom lsp --stdio` subcommand; new `nom-lsp` crate. Standard LSP methods (diagnostics, hover, definition, references, completion, rename) backed by the same nom-parser / nom-verifier / nom-dict stack.

Completion is the critical path. When the user types `use `, `.`, `->`, or any expression-position token, the LSP issues a structured query to the Authoring Protocol (§9.2) and returns ranked candidate entries. No static keyword list can substitute — candidates come from the dict.

### 9.2 Authoring Protocol — how the AI and the language cooperate

A structured request/response schema (JSON) between an AI assistant and the Nom LSP. The AI is any LLM or embedded model; the protocol abstracts over them.

**Intent query shape:**

```json
{
  "kind": "intent",
  "context": {
    "position": { "file": "...", "line": 10, "col": 20 },
    "scope_types": ["integer", "List[NomString]"],
    "concept": "auth",
    "imports_in_scope": ["#a3f2ef...@validate", "..."],
    "partial_source": "let token = \n",
    "natural_language": "generate a JWT from the user id"
  },
  "limits": { "max_candidates": 8, "min_confidence": 0.15 }
}
```

**Candidate response:**

```json
{
  "candidates": [
    {
      "id": "#b4c1...",
      "word": "sign_jwt",
      "signature": "(user_id: integer, secret: NomString) -> NomString",
      "describe": "HMAC-SHA256 JWT signer",
      "concept": "auth",
      "confidence": 0.83,
      "rank_signals": { "signature": 1.0, "concept": 0.9, "bm25": 0.7, "embedding": 0.85 },
      "similar_to": ["#e8f3...", "#0a2b..."],
      "contract_summary": { "pre": "user_id > 0", "post": "result.length > 0", "effects": ["pure"] }
    },
    ...
  ],
  "deterministic": true
}
```

Queries flow **only** from LSP → AI (the AI is a consumer of the dict through the LSP, not a first-class dict writer). The LSP uses the §5.4 ranking function as the source of truth. AI adds natural-language understanding on top of it — e.g., translating `"JWT signer"` into a concept filter and a BM25 query. AI suggestions never bypass the resolver's ranking — they refine the query.

**Modes the protocol supports:**

- **Completion** — ranked candidates at a cursor position.
- **Explain** — for a given hash, pretty-print its contract, callers, SimilarTo neighbors.
- **Find-by-example** — user provides input/output pairs; resolver runs candidate entries against them and returns matches.
- **Compose** — given start and end types, return a chain of entries whose contracts compose.
- **Disambiguate** — when resolver returns Ambiguous, the AI picks among candidates using natural-language context.

### 9.3 `.nomtu` WASM plugin host (gated)

Unchanged from v1 (gated `--features wasm-plugins`, default off). WASM plugins are a separate axis from the LSP: they provide a way to embed transformation/codegen logic in nomtu entries themselves. Versioned API envelopes per Zed pattern. This stays opt-in because most authors never write WASM plugins.

### 9.4 Fallback / no-AI authoring

The language remains usable without an AI. The LSP's completion still works — it shows dict entries ranked by the §5.4 function alone, without natural-language context. A **starter vocabulary** ships as a default: ~1,000 well-known nomtu covering common concepts (arithmetic, collections, string handling, I/O, control flow). The role is equivalent to a core vocabulary in any language with a vast underlying corpus — authors compose manually against the starter set; the 99.99% beyond is searchable but not auto-surfaced.

### 9.5 Determinism + reproducibility invariants

- **Source stored is hash-pinned.** AI-mediated composition resolves to specific hashes; those hashes are what the store records. Re-building the closure months later, without AI, produces byte-identical output.
- **The AI is a search surface.** It does not generate `body_nom`. It surfaces existing dict entries. New entries enter the dict only through Phase 5 ingestion, Phase 12 specialization, or explicit `nom store add` of human-authored .nom files.
- **Protocol queries are stateless.** An intent query is pure; same input → same ranked output. The AI's natural-language layer is a deterministic transform over the query; the resolver's rank is deterministic over the dict state.

### 9.6 Deliverables

- `nom-lsp` crate — standard LSP methods over nom-parser/verifier/dict/resolve.
- Authoring Protocol schema published as `docs/authoring-protocol.md`.
- A reference AI adapter: `nom-ai-adapter` (optional, ships as a separate crate) wrapping the Anthropic API + the Authoring Protocol so an `nom lsp --with-ai` mode works out of the box for users with API keys.
- `.nomtu` WASM plugin host behind `--features wasm-plugins`.
- IDE integration recipes (VS Code extension, Neovim, Emacs) in `editor-integrations/`.

### 9.7 Size budget

- nom-lsp core: ~1500 LOC.
- Authoring Protocol schema + implementation: ~800 LOC.
- nom-ai-adapter (optional): ~600 LOC.
- WASM plugin host: ~1000 LOC (gated).
- Editor integration recipes: ~500 LOC total across three editors.
- Comprehensiveness checker (§9.8): ~900 LOC.
- Total in the core binary (non-gated): ~3200 LOC + wasmtime dep (~30 MB, gated).

### 9.8 Authoring-time comprehensiveness — sentence, paragraph, graph

**Invariant:** a `.nom` file must be **graph-complete before compile time**. Every reference resolves, every contract unifies, no orphans. Comprehensiveness is a property of the source at authoring time — not an outcome of compilation. The compile pipeline enforces it; the authoring environment is what helps the human reach it.

Three layers of check, each with its own diagnostics and its own retrieval surface:

#### 9.8.1 Sentence layer — expression / single flow

Scope: one expression, one flow edge, one pattern match arm. Question: *does this make logical sense on its own?*

- Type checks at every sub-expression.
- Contract `pre` satisfied at the call site; `post` propagated to the next step.
- Every identifier resolves to a local binding OR a ranked dict candidate.
- Literals valid under their declared types.

LSP diagnostic codes: `NOM-S01` (unresolved ident), `NOM-S02` (type mismatch), `NOM-S03` (pre violation at call site), `NOM-S04` (invalid literal).

#### 9.8.2 Paragraph layer — function / composition chain

Scope: one function body, one flow graph, one block. Question: *do the sentences compose — grammar and logic together?*

- Output type of step N unifies with input type of step N+1.
- Effect sets propagate cleanly (pure ⊆ io ⊆ ffi; lower can't contain higher).
- Match exhaustiveness across enum variants.
- No dead branches (reachability analysis).
- No divergent-type joins in if/match expressions without a declared unified type.
- Return paths all yield the declared return type.

LSP diagnostic codes: `NOM-P01` (type gap between steps), `NOM-P02` (effect escalation unhandled), `NOM-P03` (non-exhaustive match), `NOM-P04` (unreachable code), `NOM-P05` (divergent join type).

#### 9.8.3 Graph layer — file + transitive closure

Scope: the `.nom` file plus every nomtu its references reach. Question: *does this plug into the rest of the corpus without orphans?*

- Every `use <name>` resolves to exactly one nomtu hash. Ambiguity is an error, not a warning; the author must hash-pin or narrow with intent hints.
- Every transitive reference is `Complete` OR explicitly acknowledged as `Partial` / `Opaque` at the use site.
- Every required contract (what the caller expects) has a matching provider (what the callee advertises). Unification failure is a graph error.
- **No orphan edges.** An orphan is a reference whose target doesn't exist in the dict. Detected at `nom store add` time and at compile time.

LSP diagnostic codes: `NOM-G01` (unresolved reference — multiple candidates), `NOM-G02` (unresolved reference — no candidates), `NOM-G03` (orphan edge — target hash not in dict), `NOM-G04` (contract mismatch across graph boundary), `NOM-G05` (transitive Partial entry not acknowledged).

#### 9.8.4 Retrieval modes (what the Authoring Protocol actually produces at authoring time)

When short or incomplete source exists, the LSP calls the Authoring Protocol with signals extracted from context:

| Signal source | Becomes |
|---------------|---------|
| Partial expression with a type-hole | Target contract for gap-fill |
| Variable names, comments | Concept + label filters for the resolver |
| Caller's existing closure | Proximity context via `SimilarTo` edges |
| Natural-language comment on a line | Embedding query for semantic match |
| Declared return type | Post-condition constraint |
| Surrounding code's effect profile | Effect constraint (must stay pure, etc.) |

Four retrieval modes:

1. **Gap-fill.** Cursor between two expressions; system knows the required bridge contract (left's `post` → right's `pre`); resolver returns nomtu whose contract matches the gap. Ranked by usage frequency in similar chains.

2. **Chain extension.** Partial chain ends at type `T`; surface nomtu accepting `T` as input. Ranked by `Calls` edges mined from existing closures — "in corpus, after producing `T`, authors commonly invoke these next."

3. **Intent completion.** Natural-language comment lowered by the AI to a structured intent query; resolver returns ranked candidates with contract summaries. User picks; LSP inserts hash-pinned `use`.

4. **Orphan resolution.** Unresolved `use foo` with K candidates; resolver returns top-K ranked list; LSP shows inline picker.

#### 9.8.5 Comprehensiveness score

For any `.nom` file the LSP can emit a comprehensiveness score in [0.0, 1.0]:

```
score = (sentences_clean / sentences_total) * 0.2
      + (paragraphs_clean / paragraphs_total) * 0.3
      + (refs_resolved / refs_total) * 0.3
      + (closure_complete_ratio) * 0.2
```

Displayed in the editor gutter; configurable threshold for `nom check --strict`. Files below threshold fail `nom build` unless overridden.

#### 9.8.6 `nom check --comprehensive <file.nom>` subcommand

Run the full three-layer analysis outside the IDE. Reports:
- Sentence-layer diagnostics (line + code + message).
- Paragraph-layer diagnostics.
- Graph-layer diagnostics.
- Comprehensiveness score.
- Suggested next action: "resolve this orphan", "pick among these candidates", "narrow this ambiguity".

Exit code 0 if score ≥ threshold (default 0.95), exit 2 otherwise. CI-friendly.

#### 9.8.7 `nom build` enforcement

Compile front-end runs graph-layer checks before any backend:
- Zero orphan edges — required. Compile refuses if any exist.
- All references resolved to specific hashes — required.
- Partial/Opaque transitive deps — allowed only if each is explicitly acknowledged in the source (e.g., `use foo :: opaque` or a file-level `allow: partial` declaration).

The author can opt into building an incomplete closure only with explicit acknowledgement. Default: strict comprehensiveness.

#### 9.8.8 Intent understanding via skill routing (borrowed from everything-claude-code)

`C:\Users\trngh\Documents\everything-claude-code-main` ships 47 specialized agents and 156 skills with intent-routing infrastructure: agents dispatch by matching user intent to specialization; skills layer knowledge (base → language → domain); the homunculus pattern persists session learnings as `.instinct` files that feed back as new skills; quality gates enforce verification checkpoints.

Adopting this pattern into the Authoring Protocol:

**Skill as a first-class unit.** Introduce `EntryKind::Skill` — a knowledge unit describing *how to approach* a class of problems. Unlike a `Function` (which computes), a `Skill` is *guidance* with associated patterns, anti-patterns, canonical compositions, and quality criteria. Skills are stored as `.nomtu` entries like any other, with `body_nom` in a structured prose form (declarative rules + example compositions) and edges to the functions/patterns they recommend.

Examples (seeded from ECC):
- `Skill::FrontendDesign` — heuristics for component structure, state management, responsive layouts. Edges: `Recommends` into UxPattern entries; `Calls` into component helper functions.
- `Skill::ApiDesign` — REST/GraphQL endpoint shapes, validation patterns, response envelopes. Edges: `Recommends` into validation patterns and response-type entries.
- `Skill::SecurityReview` — audit checklist, OWASP patterns, common pitfalls. Edges: `Constrains` into pattern entries that must be verified.
- `Skill::Debugging` — systematic-debugging workflow. Edges: `Recommends` composition chains for bisection, logging, reproduction.

**Skill dispatch in the Authoring Protocol.** §9.2's intent query gains a dispatch mode: given a natural-language intent, first rank Skills by match, then let the top Skill's edge set narrow the nomtu candidate search. A two-stage retrieval: intent → skill → skill-filtered entry candidates. This is ECC's agent-routing pattern generalized from "which agent handles this" to "which skill narrows this vocabulary search."

**Homunculus memory loop.** Authoring sessions accumulate patterns: "this author consistently composes X then Y", "this repo repeatedly uses pattern Z". A background extractor (analogous to ECC's homunculus) surfaces these as new `Skill` entries, attributed to the author or the repo. The dict grows its intent-understanding layer from real usage.

**Quality gates at authoring time.** ECC's `/quality-gate` + `eval-harness` patterns map directly to §9.8's three-layer comprehensiveness check. Gates are Skills whose `body_nom` is a declarative validation rule; invoking a gate runs its rule across the current file and reports diagnostics. Adding a new gate = adding a new Skill entry.

**Cross-platform dispatch.** ECC runs across Claude Code / Codex / Cursor / OpenCode / Gemini via shared skill definitions. Nom inherits this pattern through Phase 12 specialization — one skill, many backend-specific specializations (web/mobile/native), all dedup'd by content-addressing.

#### 9.8.9 Why this is authoring-time, not compile-time

The insight driving §9.8: **comprehensiveness is cheap to fix during authoring and expensive to fix during compile.** At authoring time:
- Context is rich (IDE, AI, running resolver).
- Iteration is fast (keystroke-level diagnostics).
- The author remembers what they meant.
- Retrieval is interactive — candidates surface and the author picks.

At compile time:
- The build may be automated (CI, batch).
- Iteration is slow (edit/compile cycles).
- Context is a diff, not a live editor.
- Retrieval is expensive.

Front-loading the work at authoring keeps compile-time deterministic and fast. The `.nom` file becomes a fully-resolved program before any backend runs.

---

## Phase 10 — Bootstrap + Retirement (the compiler is remade in Nom, Rust archives)

**Goal reframed:** Phase 10 is not just "run the Nom-implemented compiler on itself" — it's the **permanent retirement of the Rust implementation**. The nom-compiler ceases to be a Rust workspace and becomes a hash closure of `.nomtu` entries drawn from the same dictionary every user of Nom draws from. The entire Rust source tree moves to `.archive/`.

This is the practical consequence of the whole v2 design: if the dictionary is rich enough to host any app (via §5.5 scale properties and §5.11–5.12 corpus ingestion), it's rich enough to host the compiler itself.

### 10.1 Prerequisites — what must land first

- **Phase 5 ingestion covers the Rust compiler's primitives.** The current Rust compiler depends on: sha256 (hashing), rusqlite (storage), tree-sitter (parsing foreign grammars), inkwell + llvm-sys (LLVM backend), clap (CLI), walkdir (FS traversal), serde (serialization), anyhow/thiserror (errors), ariadne (diagnostic rendering). Each crate gets ingested via §5.6 Shape-B runtime library corpus path. Their Nom equivalents become the primitives the rebuilt compiler draws from.
- **Phase 7 parser-in-Nom is complete.** The parser — the single largest Rust component — is already Nom. Phase 10 rebuilds the surrounding stack (lexer, types, AST, planner, verifier, resolver, closure walker, LLVM codegen, CLI) in the same style.
- **§5.13 benchmarking has baseline runs of the Rust compiler.** Bit-for-bit output parity is the acceptance test; timing parity (within 2× initially, closing over time) is the operational goal.
- **§5.14 flow artifacts for the Rust compiler pipeline exist.** Recorded traces of Rust-compiler builds on the reference test corpus become the fixture the Nom compiler's outputs are diffed against.

### 10.2 The new compiler is an ordinary app

The rebuilt compiler is a single `AppManifest` (from §5.12). Its closure contains:

- `lexer_pipeline` — composed of `tokenize_source`, `handle_keyword_aliases`, `emit_token_stream` (all nomtu from the dict).
- `parser_pipeline` — from Phase 7, composed of `parse_declaration`, `parse_expression`, `recover_to_statement_boundary`, etc.
- `verifier_pipeline` — contract checks (pre/post/effects via ADOPT-1 property tests from §8.1).
- `planner_pipeline` — composition-graph construction.
- `resolver_pipeline` — the v2 reference resolver from Task B.
- `closure_walker_pipeline` — the transitive hash closure from Task B.
- `codegen_pipeline_llvm` — LLVM IR emission.
- `cli_entry_points` — `nom build`, `nom run`, `nom store add`, `nom ux seed`, `nom app build`, `nom bench`, `nom flow record`, etc., each an `AppAction` (§5.12) nomtu.

There is no special treatment. `nom app build <compiler_hash>` materializes the compiler exactly the way it materializes any other app. The compiler is not a privileged binary; it's a hash closure.

**Why this matters beyond aesthetics.** When the compiler is an ordinary app:

- §5.13 benchmarking applies to the compiler. The compiler's own specializations (e.g., LLVM codegen for different architectures) get picked by the data-driven selector.
- §5.14 flow artifacts record actual compiler runs. Post-mortem analysis of slow builds uses the same `nom flow show` tooling.
- §5.15 joint optimization includes the compiler in multi-platform builds. The compiler's wasm specialization makes in-browser builds feasible; the native specialization is the default CLI.
- §5.10 lifecycle ops work on the compiler. Canonicalization upgrades, equivalence merges, supersession chains — all apply.
- The compiler can compile itself from its own closure, with its own benchmark data driving its own specialization. The loop closes.

### 10.3 The bootstrap + retirement protocol

**The Rust compiler, at maturity, is used to build the Nom compiler — and the act of a Nom compiler successfully rebuilding itself to semantic parity plus a fixpoint is the proof that Nom is a complete language.** This is the classical self-hosting rite; we're not inventing it, we're honoring it. A language that can describe its own compiler is a language that can describe anything that's been described in any language.

The protocol has two parallel tracks, both of which are **real proof**, not one real plus one housekeeping:

- **Parity track** (§10.3.2) — semantic equivalence on arbitrary Nom programs. **This is the primary correctness proof.** If every reachable program compiles to the same behavior under the Nom-built compiler as it does under the Rust-built one, the Nom compiler is correct by the only definition that matters to users.
- **Fixpoint track** (§10.3.1) — byte-identical self-build across stages. This is the **aesthetic-plus-pinned-toolchain** proof: the language plus its canonicalizer plus its codegen, running on a pinned toolchain, close back on themselves to a bit-level fixed point. It's a strictly stronger statement than parity but only meaningful in conjunction with the `rust_toolchain_channel` + `llvm_major_version` + `canonicalizer_version` pin discipline in the proof tuple. Drop any of those pins and fixpoint becomes a different, weaker claim.

Both tracks must be green before retirement. If forced to choose one, parity is the floor; fixpoint is the ceiling.

#### 10.3.1 The fixpoint track — self-hosting proof via N-stage equality

Three stages, each a binary:

- **Stage 0 — `rust-nomc`.** The Rust-implemented nom-compiler at maturity. Built by `cargo build -p nom-cli`.
- **Stage 1 — `nomc-s1`.** The Nom-implemented nom-compiler, compiled from Nom source BY Stage 0.
  - `rust-nomc build compiler-manifest.nom → nomc-s1.exe`
  - Stage 1 is written in Nom but produced by a Rust compiler. Its correctness depends on Stage 0's correctness plus the Nom source's correctness.
- **Stage 2 — `nomc-s2`.** The Nom-implemented nom-compiler, compiled from the same Nom source BY Stage 1.
  - `nomc-s1 build compiler-manifest.nom → nomc-s2.exe`
  - Stage 2 is both written in Nom AND produced by a Nom compiler. Its correctness no longer depends on Rust.
- **Stage 3 — `nomc-s3`.** Stage 2 compiles itself again.
  - `nomc-s2 build compiler-manifest.nom → nomc-s3.exe`

**The fixpoint test:** `nomc-s2` must equal `nomc-s3` byte-for-byte, **modulo a fixed set of normalized metadata** (see prerequisites below).

**Prerequisites for the fixpoint test to be mechanically achievable** (adversarial-review follow-up, 2026-04-12):
- **LLVM pinned.** The fixpoint toolchain pins LLVM to a specific point release (currently 18.x via `inkwell`'s `llvm18-0` feature). Cross-LLVM-major-version fixpoint attempts are explicitly out of scope.
- **Rust toolchain pinned.** `rust-toolchain.toml` at `nom-compiler/` fixes `rustc`/`cargo` version so differential-compile tests run on the pinned toolchain, not the caller's default. Landed 2026-04-12 as `channel = "1.94.1"` with `components = ["rustc", "cargo", "rust-std", "clippy", "rustfmt"]`. Verified 2026-04-13 in-situ: `rustup show active-toolchain` inside `nom-compiler/` returns `1.94.1-x86_64-pc-windows-msvc (overridden by '…/nom-compiler/rust-toolchain.toml')` and `rustc --version` reports `rustc 1.94.1 (e408947bf 2026-03-25)` — pin is honored, not just written.
- **`SOURCE_DATE_EPOCH` set, `llvm.ident` stripped, debug-info paths remapped.** The LLVM IR and final object emitter must have deterministic metadata — no embedded build timestamps, no absolute paths, no toolchain version strings differing between Stage 2 and Stage 3.
- **PDB/COFF timestamps zeroed** (Windows) or equivalent stripping on other OSes.
- **DIBuilder wired through codegen.** Debug info (DWARF on Linux/macOS, PDB on Windows) is where most non-determinism hides — absolute source paths, compilation-unit ordering, embedded compiler-version strings. Until the codegen pipeline invokes LLVM's DIBuilder explicitly with path-remapping + sorted CU order, `-g` builds cannot be part of the fixpoint set. Either the fixpoint restricts to `-C debuginfo=0` (tractable near-term) or the DIBuilder wiring must land (harder, better long-term).
- **Panic unwinding vs abort decided.** The choice between unwind tables and `panic = "abort"` changes the emitted object layout. Must be pinned in the compiler's own manifest (so Stage 2 and Stage 3 match) and documented in the proof-of-bootstrap tuple's `compiler_manifest_hash`. Default is `abort` until a profiler actually needs unwinding.

If these prerequisites are not all met, the fixpoint track cannot produce byte-identical artifacts. This is the same discipline rustc applies to `-Z verify-llvm-ir` + reproducible-builds work; we do NOT claim stronger determinism than rustc achieves. The parity track (§10.3.2) is the mandatory backstop: semantic equivalence over a test corpus is a weaker but strictly achievable proof.

If they match, the Nom compiler is a fixpoint of its own compilation function. That's the proof: Nom's grammar + the compiler's implementation have closed back on themselves. Any further stage (s4, s5, …) produces the same binary; the compiler is stable under self-application.

If they differ, something in Stage 2's output was non-deterministic or Stage 2 had a bug that Stage 1 didn't trigger. Debug, fix in source, re-run from Stage 1. Never proceed with an unstable fixpoint.

Secondary check: `nomc-s1` vs `nomc-s2` should differ only in metadata that the two compilers produce differently (timestamps, optimizer heuristics the Rust compiler made vs. the Nom compiler made). **Semantic equivalence** of `s1` and `s2` is required — bit-for-bit equality is not, because the two compilers are genuinely different programs. Only `s2 == s3` is the fixpoint requirement.

Record all binary hashes + toolchain state as a signed tuple in the dict: `(s1_hash, s2_hash, s3_hash, fixpoint_at_date, compiler_manifest_hash, canonicalizer_version, rust_toolchain_channel, llvm_major_version)`. This tuple is the proof-of-bootstrap record, never deleted, permanently referenced in `.archive/migration-notes/` and in the compiler's own `entry_meta`. `canonicalizer_version` is required because the canonicalizer is part of the compilation function (see §5.10.1); changing it changes source-hash identity, so re-running the fixpoint on a different canonicalizer yields a logically different bootstrap. `rust_toolchain_channel` + `llvm_major_version` pin the Stage-0 frontends, both of which the fixpoint depends on per §10.3.1's prerequisite list.

#### 10.3.2 The parity track — regression guard on arbitrary programs

In parallel with the fixpoint work, both compilers run against the full reference test suite (a curated corpus of Nom programs — small programs, medium programs, the self-host lexer, the examples under `examples/`, any user-contributed benchmarks).

- For every test program `P`, compile under Stage 0 (`rust-nomc`) → `ir_rust(P)`.
- Compile `P` under Stage 2 (`nomc-s2`) → `ir_nom(P)`.
- Compare semantically (via `nom bench compare` structural diff; minor differences in allocator-choice heuristics are acceptable if benchmark parity holds within 5%).
- Target: ≥ 99% of programs produce semantically equivalent IR; 100% produce correct runtime output on the test-case inputs.

Parity track must remain green for 4+ weeks before Step 3 (default flip).

#### 10.3.3 Five-step cutover (replacing the older description)

Reversible through step 3.

**Step 1 — Mature `rust-nomc`.** The Rust compiler reaches a "maturity bar": it compiles the full Phase-7 parser-in-Nom, the Phase-5 ingestion pipeline, and the LLVM codegen, without known correctness bugs on the reference corpus. This is the precondition for bootstrap.

**Step 2 — Fixpoint attempt (§10.3.1).** Produce Stage 0, 1, 2, 3. Verify `s2 == s3` bit-for-bit. Record the fixpoint tuple. If fixpoint holds, the Nom compiler self-hosts and the language is proven. **This is the phase milestone — declared victory here.**

**Step 3 — Parity period (§10.3.2) + default flip.** Run the parity track for ≥ 4 weeks. When green, flip default: `nom build` resolves `nomc-s2` (or a later stage, re-built periodically to capture dict improvements). Rust compiler accessible only via `--compiler=rust-legacy`. Parity remains a CI check.

**Step 4 — Archive.** Move the Rust sources:

```
nom-compiler/
├── .archive/
│   └── rust-<version>/
│       ├── README.md              # retirement date, last commit sha, fixpoint tuple, compiler closure hash that replaced it
│       ├── Cargo.toml
│       ├── Cargo.lock
│       └── crates/                 # the full 20-crate workspace, frozen
├── manifest.nom                    # the AppManifest nomtu for the current Nom compiler (one line: hash-pinned ref)
└── README.md                       # points at the current compiler hash + .archive/ history
```

Git history preserves the old layout; the move is a rename operation. No code is lost. `.archive/rust-<version>/` is read-only by convention; no new changes land there.

A permanent edge lands in the dict: `SupersededBy(rust_compiler_crate_set → nom_compiler_app_hash)`. Anyone browsing the Rust crates in the dict is routed to the current authoritative Nom compiler.

**Step 5 — Grace period before deletion.** 3 months after archiving, the Rust source tree may be removed from the working tree entirely (git history still has it). By this point the Nom compiler has been the default long enough that the Rust one exists only for historical comparison; the archive directory is a curiosity, not a fallback.

#### 10.3.4 Why the fixpoint, specifically

Other bootstrap designs settle for "compiles itself once" and declare victory. That's insufficient — a one-shot self-compile can mask non-deterministic output or a compiler bug that the earlier-stage compiler didn't exercise. The two-stage fixpoint (`s2 == s3`) forces the compiler to BE its own reproducible input, not just produce a working binary once.

This matches what every serious self-hosting language does (GHC, rustc, OCaml, Zig, Chez Scheme). Nom inherits the discipline, not just the pattern.

**The day `s2 == s3` holds is the day Nom stops being a language-in-Rust and becomes a language.** That's the phase milestone. Everything else (parity, flip, archive) is housekeeping after the proof is in hand.

### 10.4 What becomes possible after retirement

- **Compiler evolution in the dict.** Contributions to the compiler become contributions to the dict. Fixing a codegen bug is `nom store add` of a new entry with `SupersededBy(old_codegen → fixed_codegen)`. No PRs to a Rust codebase.
- **User-level compiler forks for free.** A user can declare `compiler_alias: my_fork = #hash_of_my_variant` in a project-local nomtu, test their fork on their own workload, and roll back by changing one line. The upstream compiler is not disturbed.
- **The compiler composes with the AI authoring layer.** §9.2's Authoring Protocol can query the compiler's own closure: "what step in the LLVM codegen pipeline is slow for this input?" "which specialization is chosen and why?" — answers come from §5.14 flow artifacts + §5.13 benchmark data attached to the compiler's own nomtu.
- **Cross-platform compiler ships itself.** The compiler's wasm specialization means `nom build` can run in a browser, targeting any platform the backend supports. The native specialization remains the default CLI.

### 10.5 What gets added to `.archive/` beyond the Rust compiler

Any pre-Nom artifact of significant size gets archived, not deleted, on the same convention:

- `.archive/rust-<version>/` — the retired Rust compiler workspace.
- `.archive/docs-pre-nomization/` — documentation that references the Rust-specific build process, superseded by Nom-based docs.
- `.archive/benchmarks-rust-baseline/` — the §5.13 benchmark runs against the Rust compiler, kept as a reference baseline.
- `.archive/migration-notes/` — the cutover notes, parity-period reports, any manual interventions taken during Steps 1–4.

The `.archive/` convention makes the project history legible without burdening the working tree.

### 10.6 Failure modes and rollback

- **Parity never fully achieves.** If Step 2 stalls for more than 12 weeks without green parity, re-assess: either extend ingestion (§5.6) to cover the gap in Nom primitives, or identify a specific Rust-compiler feature that has no Nom equivalent yet and patch the Nom compiler. The rewrite does not proceed until parity is real.
- **Specialization data sparse.** The Nom compiler's own specializations may lack benchmark coverage initially. Fall back under §5.15's `--best-effort` mode, flagging gaps. `nom bench` sweeps close them over time.
- **A dict entry the compiler depends on gets deprecated.** `SupersededBy` chains route to current canonical. If a dict entry genuinely disappears (GC purged it), the compiler's closure is broken. Solution: the compiler's root AppManifest pins every critical dependency by hash, not by word, so lifecycle changes below the root don't affect it. Only an explicit compiler-root update pulls in the new versions.
- **Archive integrity.** `.archive/rust-<version>/` must remain byte-identical to what it was at archive time. Any edit lands as a new `SupersededBy` entry in the dict, not a mutation to the archive.

### 10.7 The governing invariant

**The compiler is an ordinary Nom app.** Not a privileged tool. Not a separate build system. Not a bootstrap problem with special solvers. It composes from the dict, specializes by benchmark data, emits flow artifacts, builds as an `AppManifest`. Its source is its hash. Its history is the SupersededBy subgraph rooted at `rust_compiler_crate_set`. The entire Rust era of Nom development lives in git history and `.archive/`, honored but not authoritative.

**Phase 10 is still not terminal.** Phase 12 (closure-level specialization) continues forever after Phase 10 — the compiler, like every app, is something to keep specializing. Phases 11 (nomization) has already been absorbed into Phase 5 per the earlier refactor.

### 10.8 The media-codec FFI tier — scoped exception with CI gate

Phase 10 retires the Rust *compiler*. It does NOT retire a narrow, declared class of linked native code: the media codecs and pixel/audio/geometry math kernels described in §5.18.2. This subsection makes the exception explicit, bounded, and CI-enforceable so it cannot silently expand.

**What the exception covers:**
- Image codecs (PNG, JPEG, WebP, AVIF, GIF, TIFF decoders and encoders).
- Audio codecs (FLAC, Opus, MP3, MP4 audio).
- Video codecs (H.264, H.265, VP9, AV1 via system libraries).
- Font rasterization (`font-kit`-class glyph rendering).
- Vector-graphics tessellation (`lyon`-class path → triangle mesh).
- 2D rasterization (`tiny-skia`-class software rendering).
- Core math kernels behind the above (FFT, DCT, wavelets, color transforms) where a mature native implementation outperforms what Nom-native code can match today.

**What the exception does NOT cover:**
- Any compiler logic (parser, verifier, planner, resolver, closure walker, codegen).
- Any dict logic (entry CRUD, closure walks, intent resolution, benchmarks, flow recording).
- Any authoring logic (LSP, Authoring Protocol, skill dispatch).
- Any I/O or system-integration logic beyond what the media codecs intrinsically require.

**CI gate.** A `.nom/ci/media_codec_allowlist.nom` file in the compiler's AppManifest closure enumerates the precise set of hashes permitted to carry `origin_ecosystem: crates.io` + `kind: Ffi`. Any new Ffi entry outside this allowlist fails the pre-commit hook and the release CI pipeline. Additions to the allowlist require a maintainer signature — in practice, a `nom store add` of an updated allowlist signed by a maintainer's keypair, with the old allowlist `SupersededBy → new allowlist`.

**Scope is structurally enforced via a closed category enumeration.** Each allowlist entry must declare `ffi_category ∈ {image_codec, audio_codec, video_codec, font_raster, vector_tessellate, raster_2d, color_math, signal_math}`. The enumeration itself is a hash-pinned nomtu (`kind: Skill` holding the declarative enumeration) in the compiler's AppManifest closure. Expanding the enumeration — adding a new category — is a **separately-signed governance act** distinct from adding an entry under an existing category. CI rejects any allowlist entry whose `ffi_category` is not in the currently-hash-pinned enumeration. This structurally prevents scope drift: a future maintainer cannot add `tokio_async_runtime` as "media-codec" because `network_runtime` is not a valid category, and adding it requires a separate governance step that's visible in the SupersededBy chain of the enumeration.

**Replacement trajectory.** Every allowlisted entry carries `planned_nom_replacement: <hash_or_null>` metadata. When Nom-native performance catches up for a given codec, a new Complete nomtu implements it, the replacement metadata points at it, and eventually the Ffi entry is marked `SupersededBy(ffi_entry → nom_native_entry)`. Once all callers migrate, the allowlist shrinks. The long-term goal is an empty allowlist. The honest short-term admission is a small, bounded, signed list.

This section preserves invariant #13 ("compiler is an ordinary Nom app") by declaring the media-codec tier as a **separate** exception from the compiler, not a back-door in the compiler itself. The compiler genuinely has no privileged native code; the media subsystem has a bounded, audited one.

---

## Phase 11 — Nomization (post-bootstrap, ongoing / multi-month)

**Goal:** Every `.nomtu` entry's `body_nom` becomes the canonical form. The original `body` (source in the ecosystem language) is retired — stored null or dropped — once the Nom translation is provably equivalent. The dict becomes fully native over time.

### 11.1 Why this has to come after the language is mature

Phase 5 stores both `body` (original) and `body_nom` (best-effort translation) with a `translation_score` because early translations are imperfect. The dict **cannot** drop the original until:
- The language is stable (Phases 4, 7 done).
- The contract system (Phase 8) is rich enough to express the original's semantic contract.
- A property-test harness (ADOPT-1) can verify behavioral equivalence.

Trying to nomize prematurely risks losing information. This is why it's Phase 11, not earlier.

### 11.2 The nomization loop

For each entry with `translation_score < 1.0`, run periodically (continuous background task or cron):

1. **Re-translate.** Invoke the current translator (improved since the entry was first ingested) on the stored `body`.
2. **Equivalence-test via contracts.** Generate N property tests from the contract (pre/post/effects). Run both bodies against the same inputs, compare outputs and effects.
3. **Score.** If all tests pass and the translator self-reports full coverage, `translation_score = 1.0`. Otherwise bump the score and store the improved `body_nom`.
4. **Retire original.** When `translation_score == 1.0` **and** at least one real-world closure built from `body_nom` alone passes its own tests, set `body = null`.

Per-ecosystem ordering (best semantic fit first): Rust → OCaml/F# → Go → TypeScript → Java/Kotlin → Python → C/C++ → PHP → Ruby → dynamic-heavy (Perl, late-bound Ruby, runtime-macro Python).

### 11.3 Disk impact (concrete)

`data/nomdict.db` is currently ~2 GB of ingested atoms. Most of that bytes-volume is `body` strings (original source). Full nomization collapses two bodies into one — expected 40–60% size reduction. On a 100× ingestion corpus, the difference is terabyte-scale.

### 11.4 Verification

- A corpus entry pool reaches `body: null`, `translation_score: 1.0` for ≥50% of its entries within the first milestone.
- Closures built from nomized-only entries pass all the original property tests.
- `data/nomdict.db` size shrinks monotonically with nomization progress.

### 11.5 Size budget

- Translator improvements across `nom-extract/src/translate/*`. Ongoing; no single bound.
- Property-test harness: ~400 LOC, shared with ADOPT-1 infra.
- Contract inference refinement: ~800 LOC incremental.
- Schema: add nullable `body`, freeze `translation_score` / `last_nomized_at` metadata fields.

### 11.6 Maps to

- **ADOPT-1** (property-based tests from contracts) — this phase is its ultimate consumer.
- **AVOID-9** (no claim without implementation) — `translation_score` is the honest marker of what works.

---

## Phase 12 — Closure-level specialization (value-based optimization)

**Goal:** Given a hash closure and concrete entry-point values, emit a specialized, minimal binary containing only the code actually executed. The unit is the whole closure, not a single compilation unit — this is whole-program optimization elevated to content-addressed scale.

This is *the* "actually value code" stage — we keep only what the program in practice runs.

### 12.1 Why after Phase 11

Phase 12 reads `body_nom` to reason about the program. If `body_nom` is partial or low-score, specialization can't see through foreign-language semantics. Once Phase 11 nomization is ≥90% complete on the closure being built, Phase 12 gets maximum leverage.

### 12.2 Transformations (bottom-up, standard but closure-scoped)

Applied during `nom build <hash>` after closure materialization:

1. **Reachability-based dead-code elimination.** Walk the closure from the entry point; drop hashes unreferenced from any reachable code path. The raw closure is complete; the built closure is minimal.

2. **Monomorphization.** A generic `.nomtu` only ever called with one concrete type across the closure is specialized. Specialized entries get new hashes and live alongside the generic one — the generic stays discoverable for other closures. This realizes Rust-style zero-cost generics at the dict level.

3. **Constant propagation across hash boundaries.** If `main` calls `foo(42)` and `foo`'s contract pattern is `pre: x == 42`, inline `foo` with `x=42` pre-bound and fold. Propagates through the closure transitively.

4. **Partial evaluation (Futamura-style).** If the entry point's arguments are fully static (a CLI tool with a baked-in config, a build-time generator), evaluate as much as possible at build. The result is a residual program of whatever *couldn't* be statically resolved — often near-nothing for config-heavy tools.

5. **Effect-aware inlining.** `effect: pure` entries inline freely; `effect: io` preserve boundaries for debuggability; `effect: ffi` never inline (boundary must remain for the linker).

6. **Cross-closure sharing.** If a specialized variant of an entry appears in many different closures (e.g. `HashMap<String,i64>` across 500 apps), cache it globally. The dict naturally handles this because specialized variants are also content-addressed.

### 12.3 Scale outcome

Binary-size reduction target vs. unspecialized closure: **70–95%**, comparable to GraalVM native-image or Rust `--release + LTO + strip`. A Node-app-class closure of ~10 MB of reachable `body_nom` specializes to a ~500 KB–1 MB binary.

This is where the "enough, no less no more" philosophy becomes quantitative. A typical Node app today ships 100–300 MB of `node_modules` of which <5% is executed. Nom specializes down to exactly what runs.

### 12.4 Deliverables

- `nom-spec` crate — the closure optimizer. Input: closure DAG + entry-point value-fingerprint. Output: specialized DAG.
- Integration with `nom build`: `nom build --specialize <hash>` (default on for release builds, off for debug).
- Value-fingerprint schema: how to describe "entry point with args X, Y, Z" as a hash input for cache keying of specialized closures.
- Dead-hash pruner for the specialized output.
- Tests: a reference closure (Phase 5 ingested Node app), measure before/after binary size and runtime. Target ≥70% reduction.

### 12.5 Verification

- Specialized binaries are observationally equivalent to unspecialized binaries on the test inputs used for specialization.
- Size reduction target met on the reference corpus.
- Re-specialization with the same fingerprint is deterministic (cached).

### 12.6 Size budget

- ~2500 LOC total for nom-spec: DCE walker (~300), monomorphization (~600), const-prop (~500), partial eval (~800), effect-aware inlining (~300).
- Schema: one new table for specialized-variant cache keyed on `(original_hash, value_fingerprint)`.
- Zero new Rust deps.

### 12.7 Maps to

- **C's zero-cost composition** (systems-lang takeaway #2) realized at closure scale.
- **Rust monomorphization** lifted to cross-package scope (not possible in Rust because packages aren't content-addressed).
- **Zig `comptime`** generalized — `comptime` within a file becomes "partial eval across the closure".
- **ADOPT-3** (explicit allocator strategy): specialization exposes exact allocation patterns, which inform arena/pool/stack/heap choices per call site.

---

## Appendix A — Rust-side cleanup (half-day task, not a roadmap phase)

v1's "Phase 4 minimalism" was dep-trimming of the Nom compiler's own Cargo.lock. That's valuable but doesn't belong in the forward roadmap — it's a one-time chore.

Actions:
- Remove unused workspace deps: `miette`, `reqwest`, `globset`, audit `tracing`.
- Accept transitive dups (`thiserror 1+2` via inkwell, `hashbrown 0.14+0.17` via hashlink).
- Feature-gate the 45 tree-sitter grammars with a `core-grammars` default and `all-grammars` opt-in.
- Embed `stdlib/prelude.nom` via rust-embed so cold start has no disk I/O.
- Close the lexer.nom:682 FatArrow parse error.

Do this any time. Not blocking on anything. ~1 day of work.

---

## Governing invariants

1. **One binary.** `nom` is the whole toolchain (AVOID-8).
2. **No feature without implementation.** SYNTAX.md never describes something that doesn't compile (AVOID-9).
3. **Three operators stay three.** `->`, `::`, `+` (AVOID-3).
4. **No `any` escape hatch.** Contracts required (AVOID-2, AVOID-4).
5. **Dict entries are immutable.** New version = new hash (AVOID-5). Phase 4 makes this structural.
6. **Structural, not nominal.** Interface satisfaction by shape (ADOPT-10).
7. **No package managers, no lockfiles, no version ranges.** The hash closure is the build spec. This is the new rule that replaces v1's "dep count only decreases."
8. **No network in compile/build.** All resolution is from local cache. `nom` never calls npm/pip/cargo.
9. **Self-contained or labeled.** Every `.nomtu` entry is either `Complete` (full hash closure), `Partial` (user action required to resolve), or `Opaque` (native/closed-source contract only). Never silently broken.
10. **The dictionary IS the syntax.** Tier 1 is fixed (three operators + sentence keywords). Tier 2 is the dictionary — every `.nomtu` hash+word is a valid source token. New expressivity comes from new dictionary entries, never from new keywords.
11. **Authoring is AI-mediated discovery at scale.** The language assumes an AI authoring layer exists (Phase 9 LSP + Authoring Protocol). Learnability is independent of vocabulary size because the user is never expected to memorize the vocabulary.
12. **Determinism survives AI mediation.** The AI is a search surface, not a generation surface. Source stored in the dict is hash-pinned. Re-materializing a closure without AI produces byte-identical output to the AI-mediated build.
13. **The compiler is an ordinary Nom app.** After Phase 10, the compiler is a hash closure of `.nomtu` entries drawn from the same dictionary every user of Nom draws from. It has no privileged position. Contributions are `nom store add` + `SupersededBy` edges. User forks are one-line compiler-alias declarations. The Rust implementation lives in `.archive/rust-<version>/`, honored but not authoritative.
14. **Media is ordinary Nom.** Files with extensions are a user-facing view, not an identity. Every byte that has ever been a file can be a hash in the dict, with its structural decomposition as graph edges and its encodings as `Specializes` variants. "Extensions" are provenance metadata (`origin_encoding: png|wav|mp4|…`), never identity. A file's "format" is one specialization among many of a single semantic `MediaUnit`.
15. **Aesthetic is programming.** Visual, auditory, kinetic design is composition of the same kind as compute, over media-primitive nomtu (§5.18). Three operators, one grammar, one dict, one compiler. "Design tooling" and "compiler" are not separate categories.
16. **The dict schema stores no raw source content.** No `entries` row contains a repo's raw bytes. `body_nom` holds post-translation Nom form; raw sources live transiently in ingestion workspace and are deleted per §5.17.2. Structurally enforced via two checks: (a) `nom store add` parses every `body_nom` under the Nom grammar on ingest and refuses entries that don't parse — raw Python/JS/Rust would fail Nom's parser; (b) §4.5 verification includes a mandatory `body_nom_parses_as_nom` property test on every insert. Non-Nom bytes cannot reach the `entries` table by any legitimate code path.
17. **Build-mode resolver filters to Complete entries only.** The §5.4 intent resolver has two modes: **search mode** returns all matching entries (Complete, Partial, Opaque) for browse/surface/AI-draft-composition/supersede; **build mode** returns only `Complete` entries. `nom check --comprehensive` and `nom build` use build mode and reject closures that transitively reach unacknowledged non-Complete entries. The AI authoring loop (§5.19) uses search mode in step 1 (draft composition) and the build-mode check step 2 enforces the boundary — Partial candidates can be *proposed* by the AI as stepping stones; they cannot *land in a build* without explicit `:: partial` acknowledgment. Enforced in the resolver API, not left to calling convention.

**Operational disciplines (not invariants, but non-optional):** (a) Mass corpus ingestion streams sources, ingests, discards — peak disk bounded by `max(per-repo-source-size) + current-dict-size`, never total corpus size. Skip-lists, checkpointing, `nom corpus workspace-gc`, license-metadata propagation, sandbox-gated equivalence testing, and malicious-pattern quarantine are non-optional parts of the `nom corpus ingest` workflow. See §5.17.2 and §5.17.6. (b) AI-assisted authoring is a verify → build → bench → flow loop (§5.19). The compiler is a deterministic oracle; artifacts are hash-pinned and reproducible. See §5.19 for the authoring workflow.

---

## Cumulative horizon (honest restatement 2026-04-12)

An earlier version of this table claimed "~70–90 weeks post-Phase 3" with Phase 5 at 4–6 weeks. That estimate was written before Phase 5 absorbed §5.11–§5.19 (UX, apps, bench, flow, multi-platform optimization, media, mass corpus, aesthetic programming, AI-compiler loop). The earlier number is therefore wrong by roughly an order of magnitude; keeping it would misplan the whole project. This section is the corrected estimate.

Phase 5 decomposes into five sub-phases that can run partly in parallel:

| Sub-phase | Scope | Horizon |
|---|---|---|
| 5.0 Core ingestion | §5.1–§5.10 (body-only translation, multi-edge graph, §5.4.0 resolver at scale, recursive symbol ingestion, lifecycle ops) | 10–14 weeks |
| 5a UX + apps + aesthetic | §5.11 (nom-ux crate), §5.12 (apps), §5.18 (aesthetic programming) | 8–12 weeks (parallel with 5.0) |
| 5b Benchmarking + flow + multi-platform | §5.13 (bench), §5.14 (flow), §5.15 (joint multi-app multi-platform) | 6–10 weeks (parallel) |
| 5c Media as nomtu | §5.16 (nom-media crate, ~20 decoder/encoder pairs) | 16–24 weeks (parallel long-tail, one format pair per PR) |
| 5d Mass corpus + AI-compiler loop | §5.17 (PyPI + top GitHub + license + safety), §5.19 (AuthoringTrace + Authoring Protocol extensions) | 8–12 weeks after Phase 9 |

Critical-path-aware Phase 5 total: **~24–38 weeks** (critical path = 5.0 → 5b → 5d, with 5a and 5c running in parallel; 5c extends past the critical path but doesn't block 5d on core features).

Full phase order with realistic horizons:

| # | Phase | Horizon |
|---|-------|---------|
| 4 | Dictionary-Is-The-Dependency-System | 2–3 weeks ✅ DONE |
| 5 | Ingestion + UX + apps + bench + flow + media + mass corpus + AI loop (5 sub-phases above) | 24–38 weeks (critical path) |
| 6 | Parser-in-Nom prerequisites | 1 week |
| 7 | Parser in Nom | 10–14 weeks |
| 8 | Architectural ADOPT items | 24–48 weeks (overlaps 7) |
| 9 | LSP + Authoring Protocol + WASM plugins | 12–16 weeks |
| 10 | Bootstrap + Retirement (two-track fixpoint protocol) | 8–12 weeks after 7+9 |
| 11 | (retired, absorbed into Phase 5) | — |
| 12 | Closure-level specialization | ongoing (perennial) |

Critical-path-aware total post-Phase-3 through Phase 10: **~120–160 weeks** (with Phase 8 partially overlapping Phase 7, and Phase 5c long-tail overlapping 7 + 8). Previous 70–90 week claim was for a narrower scope that did not include §5.11–§5.19; the expansion honestly doubles the horizon. Still less than the original self-hosting roadmap's 66–88 weeks for a much narrower scope — content-addressed reuse across sub-phases and eliminating the need for a separate package manager / lockfile / build system is what keeps the number tractable.

Phase 11 no longer exists as a separate phase (absorbed into Phase 5 per §5.2 equivalence-gated translation). Phase 12 remains open-ended.
