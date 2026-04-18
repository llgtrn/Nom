# Nom — Hybrid Composition Design

**Date:** 2026-04-18 | **Status:** Approved for implementation
**Spec covers:** IntentResolver · HybridResolver · AiGlueOrchestrator · ProviderRouter-BackendRegistry bridge · Multi-kind pipeline · UI surfaces · Grammar promotion lifecycle

---

## 0. Motivation

Today nom-compose has three completely isolated systems:
- `BackendRegistry` — routes by `BackendKind` enum to 16 concrete backends
- `ProviderRouter` — routes by `FallbackLevel` to registered `MediaVendor` impls
- `CredentialStore` — stores API keys but never injects them anywhere

No system knows about the others. There is no unified request envelope, no intent classification, no AI fallback when neither DB nor vendor covers the request.

This spec defines the **Hybrid Composition System**: a three-tier resolver (DB-driven → provider-driven → AI-leading) with an intent classifier at the front and a grammar promotion pipeline at the back. The system is fully Rust, one binary, zero subprocesses.

---

## 1. The `ComposeContext` Envelope

Every request through the entire pipeline carries one struct:

```rust
pub struct ComposeContext {
    pub kind: BackendKind,
    pub input: String,
    pub entity_word: Option<String>,      // nomtu word looked up in grammar.kinds
    pub vendor_hint: Option<String>,      // preferred vendor name
    pub credentials: Option<String>,      // key into CredentialStore
    pub constraints: ComposeConstraints,
    pub request_id: Uuid,
}

pub struct ComposeConstraints {
    pub streaming: bool,
    pub max_cost_microcents: u64,         // 0 = no limit
}

pub struct ComposeResult {
    pub output: String,
    pub tier_used: ComposeTier,           // Db | Provider | AiGlue
    pub vendor_used: Option<String>,
    pub cost_incurred: Option<u64>,       // microcents
    pub duration_ms: u64,
    pub glue_promoted: bool,              // true if AI glue was written to grammar.kinds
}

pub enum ComposeTier { Db, Provider, AiGlue }
```

`BackendRegistry::dispatch` and `ProviderRouter::route` both accept `&ComposeContext` instead of bare strings. All 16 backends receive the context; credentials are injected at dispatch time from `CredentialStore`.

---

## 2. IntentResolver — Kind Detection

Sits before everything else. Maps raw user input to `Vec<(BackendKind, f32)>` — ranked candidates with confidence.

### 2.1 Three detection steps in order

**Step 1 — Lexical scan against `grammar.kinds`**

Direct word match wins immediately. Reads `SELECT word, kind FROM grammar.kinds` and checks every token in the input. Confidence = 1.0 on exact match.

```
"make a promo video for nano banana" → "video" matches → BackendKind::Video (1.0)
"build a web app for tasks"         → "web app" matches → BackendKind::WebScreen (0.91)
"compose nano banana thing"         → NO match → proceed to Step 2
```

**Step 2 — BM25 + cosine over kind descriptions**

`nom_search::BM25Index` built over `grammar.kinds.description` + `grammar.kinds.word`. Returns ranked candidates:

```
"promo clip"     → Video (0.87), Image (0.71), Presentation (0.44)
"data dashboard" → DataQuery (0.89), WebScreen (0.63)
```

**Step 3 — `classify_with_react()` for ambiguous inputs**

When top-2 candidates within 0.15 of each other, fire `nom_intent::classify_with_react()`. The ReAct chain reasons through the ambiguity and returns a disambiguated ranking.

```
"something visual for my app store listing"
  Image (0.76) vs Video (0.74)  → delta 0.02 → triggers ReAct
  ReAct: "app store listing → static screenshot → Image wins"
  → [(Image, 0.88), (Video, 0.61)]
```

Low-confidence threshold: if top score below 0.6, present disambiguation card to user (see §6.2).

### 2.2 Multi-kind detection

Inputs spanning multiple kinds return multiple candidates above threshold (0.65):

```
"web app with animated intro and background music"
  → [(WebScreen, 0.91), (Video, 0.85), (Audio, 0.89)]
  → parallel composition pipeline (see §5)
```

### 2.3 Training signal

When user corrects a classification, the correct kind + input pair is recorded in `glue_cache` as a BM25 training example. The index re-weights for next time with no recompilation.

---

## 3. HybridResolver — Three Tiers

```
ComposeContext { kind, entity_word, ... }
        │
        ▼
 ┌──────────────────────────────────────────┐
 │            HybridResolver                │
 │                                          │
 │  Tier 1: DictReader::find_by_word()      │
 │    entity in grammar.kinds (Complete)?   │
 │    YES → BackendRegistry::dispatch()  ───┼──→ ComposeResult { tier: Db }
 │                                          │
 │  Tier 2: ProviderRouter::route(kind)     │
 │    vendor registered?                    │
 │    YES → UnifiedDispatcher::via_vendor() ┼──→ ComposeResult { tier: Provider }
 │                                          │
 │  Tier 3: AiGlueOrchestrator::synthesize()│
 │    graph RAG + deep think → .nomx        │
 │    sandbox execute → output          ────┼──→ ComposeResult { tier: AiGlue }
 └──────────────────────────────────────────┘
```

**Tier 1 (DB-driven):** `DictReader::find_by_word(entity_word)` returns `GrammarKindRow`. If `status == Complete` → `BackendRegistry::dispatch_with_context(ctx)`. If `status == Partial` → fall through to Tier 2.

**Tier 2 (Provider-driven):** `ProviderRouter::route_with_context(ctx)` returns a vendor. `UnifiedDispatcher` reads `ctx.credentials`, calls `credential_store.get(vendor.name())`, injects `Credential.value` into `vendor.compose(input, credential, ctx)`.

**Tier 3 (AI-leading):** Neither DB entity (Complete) nor registered vendor. `AiGlueOrchestrator::synthesize(ctx)` fires.

---

## 4. AiGlueOrchestrator

### 4.1 Synthesis flow

```
AiGlueOrchestrator::synthesize(ctx)
        │
        ├── GraphRagRetriever::retrieve(entity_word)
        │     → Vec<RetrievedNode> as grammar context
        │
        ├── DictReader::list_clause_shapes(kind)
        │     → valid verb phrases for this kind (grammar constraint)
        │
        ├── ReActLlmFn::call(prompt)
        │     prompt = "Given kind '{kind}', clause shapes {shapes},
        │                nearby entities {rag_context},
        │                generate a .nomx definition for '{entity_word}'."
        │     returns .nomx string
        │
        ├── nom_concept::stage1_tokenize(nomx_str)
        │     → validate generated .nomx parses cleanly
        │
        ├── sandbox::interpret(ast, env)  OR  dispatch_with_context(ctx)
        │     → String output
        │
        └── GlueBlueprint { code: nomx_str, confidence: f32, kind }
```

The LLM fills in clause *values* (purpose, query, steps), not structure. Because the sentence structure comes from `clause_shapes` rows, syntactically invalid `.nomx` is structurally impossible.

### 4.2 `ReActLlmFn` adapters (pluggable, no hardcoded API calls)

| Adapter | When |
|---|---|
| `StubAdapter` | Tests — returns canned `.nomx` |
| `NomCliAdapter` | Fully offline — nom-compiler as oracle |
| `McpAdapter` | Local LLM via MCP server (Ollama, etc.) |
| `RealLlmAdapter` | Anthropic/OpenAI — requires credential in `CredentialStore` |

`AiGlueOrchestrator` holds `Box<dyn ReActLlmFn>`, injected at startup. The only I/O leaving the process is the HTTP call from `RealLlmAdapter`.

### 4.3 Glue code language

The AI generates **`.nomx`** for entity and workflow definitions. For imperative transformation steps, the AI generates a **JavaScript AST** executed by `sandbox::interpret_expr()` (already depth-guarded and overflow-checked after AE8/AE14 fixes). Never Rust source, never Python, never subprocesses.

---

## 5. Grammar Promotion Lifecycle

```
Stage 0 — Transient
  Lives in GlueCache (SharedState) only. NOT in grammar.kinds.
  usage_count: 1, status: transient
  UI badge: ⚡ (amber)

Stage 1 — Partial  [trigger: usage_count >= 3 AND confidence >= 0.7]
  DictWriter::insert_partial_entry(word, kind, nomx_code, confidence)
  grammar.kinds row inserted: status=Partial, source=ai_generated
  NomtuRef assigned immediately → first-class canvas object
  UI badge: ◑ (half-filled)

Stage 2 — Complete  [trigger: used 10+ times AND passes compiler validation]
  DictWriter::promote_to_complete(word)
  grammar.kinds row: status=Complete, source=ai_generated
  UI badge: ● (filled, indistinguishable from human-authored)
  Palette entry moves from "AI" group to regular kind group
```

**Promotion thresholds** stored in `glue_promotion_config` DB table — not Rust constants. Tunable without recompilation.

**Key invariant:** every entity gets a `NomtuRef` on first promotion to Partial. The `entity: NomtuRef` non-optional mandate is maintained from the moment an AI entity enters the DB.

**`GlueCache`** fields added to `SharedState` in `nom-compiler-bridge/src/shared.rs`:

```rust
pub glue_cache: RwLock<HashMap<String, GlueCacheEntry>>,
```

Background promotion task polls every 60 seconds, promotes eligible entries.

---

## 6. Provider Integration

Any external service becomes a Nom provider by implementing `MediaVendor` with credential injection:

```rust
impl MediaVendor for NanoBananaVendor {
    fn name(&self) -> &str { "nano-banana" }
    fn supports(&self, kind: BackendKind) -> bool {
        matches!(kind, BackendKind::Video | BackendKind::Image | BackendKind::Audio)
    }
    fn compose(&self, input: &str, credential: Option<&str>, ctx: &ComposeContext)
        -> Result<String, String>
    {
        // HTTP call to nano-banana API using credential
    }
}
```

`UnifiedDispatcher` bridges `ProviderRouter` and `BackendRegistry`:

```rust
pub struct UnifiedDispatcher {
    router: ProviderRouter,
    registry: BackendRegistry,
    credentials: CredentialStore,
}
```

Credential flow: `ctx.credentials` key → `CredentialStore::get(vendor.name())` → `Credential.value` passed to `vendor.compose()`. No credential ever touches Rust source or appears in logs (Debug redaction already in place).

---

## 7. Multi-Kind Parallel Pipeline

When `IntentResolver` returns multiple kinds above threshold (0.65):

```
[(WebScreen, 0.91), (Video, 0.85), (Audio, 0.89)]
        │
        ▼
ComposeOrchestrator::plan(kinds, input)
  → CompositionPlan {
      steps: [
        Step { kind: Audio,     depends_on: [],    priority: 0 },
        Step { kind: Video,     depends_on: [],    priority: 0 },
        Step { kind: WebScreen, depends_on: [0,1], priority: 1 },
      ]
    }
        │
        ▼
TaskQueue dispatches priority-0 steps in parallel
Wait → inject outputs as assets into priority-1 context
        │
        ▼
ComposeResult { output: web_app_with_video_and_audio }
```

`CompositionPlan` + `TaskQueue` already exist. `ComposeOrchestrator` is new — wraps them with intent-driven step generation.

---

## 8. UI Surfaces

### 8.1 Intent Preview card (right dock)

```
┌─────────────────────────────────────┐
│  Nom understands you want to make:  │
│                                     │
│  🎬 Video             92%  ████████ │
│  🖼  Image             71%  █████░░░ │
│  📊 Presentation      45%  ███░░░░░ │
│                                     │
│  [Compose Video]  [Change]  [All 3] │
└─────────────────────────────────────┘
```

Low-confidence (top score below 0.6): shows disambiguation picker — full `grammar.kinds` list as DB-driven dropdown.

### 8.2 Doc mode

`.nomx` sentence as AFFiNE prose block. `⚡` gutter icon for AI-generated status. LSP highlights `@kind`, confidence values, mime types. Hover tooltip shows the matching `clause_shape` row.

### 8.3 Graph mode

Node card with amber frosted-glass tint and `⚡` badge. Ports derived from `clause_shapes` identical to Complete entities. Confidence score in subtitle. Tint and badge removed on Complete promotion.

### 8.4 AI Review card (right dock)

```
┌──────────────────────────────────────┐
│  ⚡ AI Generated Entity               │
│  nano_banana_promo  ·  media         │
│  ████████████░░░░  75% confidence    │
│                                      │
│  "the media nano_banana_promo is     │
│   intended to compose a 30-second    │
│   promotional video..."              │
│                                      │
│  [Accept]  [Edit grammar]  [Skip]    │
└──────────────────────────────────────┘
```

**Accept** → bumps `usage_count` to `PROMOTE_AFTER` → writes `grammar.kinds` row as Partial immediately.
**Edit grammar** → opens `.nomx` sentence inline for user editing, re-validates against `clause_shapes`.
**Skip** → stays transient, AI re-generates on next request.

### 8.5 Node palette

AI-generated entities appear under their parent kind group with `⚡` icon and `used Nx` count. After Complete promotion, both disappear.

### 8.6 Status bar

`⚡ 1 AI entity pending review` — clickable, opens Review card.

---

## 9. Initial Seed Kinds in `grammar.kinds`

The kind registry is a DB table, not a Rust enum. Initial seeds:

| word | kind | description |
|---|---|---|
| video | media | compose visual media, film, clips |
| picture | media | generate images, photos, graphics |
| audio | media | compose sound, music, voice |
| presentation | screen | build slides, decks, presentations |
| web_app | screen | build web applications, browser UIs |
| mobile_app | screen | build iOS and Android apps |
| native_app | screen | build desktop applications |
| document | concept | generate PDF, DOCX, reports |
| data_extract | data | extract structured data from files |
| data_query | data | query, transform, analyze structured data |
| workflow | scenario | automate multi-step processes |
| ad_creative | media | compose advertising content |
| 3d_mesh | media | generate 3D geometry and meshes |
| storyboard | media | sequential visual narrative |

New kinds are added by user `.nomx` definition or by AI glue promotion — automatically available to `IntentResolver` with no code changes.

---

## 10. New Components — Implementation Order

| Order | Component | Location |
|---|---|---|
| 1 | `ComposeContext` + `ComposeResult` + `ComposeTier` | `nom-compose/src/context.rs` |
| 2 | `DictWriter` (insert_partial_entry + promote_to_complete) | `nom-compiler-bridge/src/dict_writer.rs` |
| 3 | `GlueCache` + `GlueCacheEntry` + `GlueCacheEntry` in `SharedState` | `nom-compiler-bridge/src/shared.rs` |
| 4 | `UnifiedDispatcher` | `nom-compose/src/unified_dispatcher.rs` |
| 5 | `IntentResolver` (lexical + BM25 + ReAct) | `nom-compose/src/intent_resolver.rs` |
| 6 | `AiGlueOrchestrator` + `GlueBlueprint` + `ReActLlmFn` adapters | `nom-compose/src/ai_glue.rs` |
| 7 | `HybridResolver` | `nom-compose/src/hybrid_resolver.rs` |
| 8 | `ComposeOrchestrator` (multi-kind pipeline) | `nom-compose/src/orchestrator.rs` |
| 9 | Intent Preview card + AI Review card | `nom-panels/src/right/` |

### Existing components — minimal changes

| Component | Change |
|---|---|
| `ProviderRouter` | Add `route_with_context(&ComposeContext)` |
| `BackendRegistry` | Add `dispatch_with_context(&ComposeContext)` |
| `MediaVendor` trait | Add `credential: Option<&str>` and `ctx: &ComposeContext` to `compose()` |
| `SharedState` | Add `glue_cache` field |
| `CredentialStore` | Unchanged |

---

## 11. Non-Negotiables

1. Zero Python, zero subprocesses, zero IPC — all inline Rust, one binary
2. Glue code language is `.nomx` (entity/workflow) or JS AST (imperative) — never Rust source
3. `ReActLlmFn` is a trait — no hardcoded API client; `NomCliAdapter` is the default offline adapter
4. Every promoted AI entity gets `NomtuRef` immediately — `entity: NomtuRef` non-optional invariant maintained
5. Promotion thresholds in `glue_promotion_config` DB table — not Rust constants
6. `grammar.kinds` is the kind registry — `BackendKind` enum is internal routing only; user-facing names always from DB
7. All 14 existing initial seed kinds are DB rows, not enum variants used for user-facing resolution
