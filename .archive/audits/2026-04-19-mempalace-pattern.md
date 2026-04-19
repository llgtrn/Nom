# MemPalace Pattern Extraction Report

**Date:** 2026-04-19
**Source:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\mempalace`
**Analyst:** Pattern-extraction subagent

---

## 1. Pattern Summary

MemPalace is a local-first, long-context memory system built around **verbatim storage + structured navigation + hybrid retrieval**. Its core insight is that raw text with good embeddings outperforms LLM-extracted summaries (96.6% R@5 on LongMemEval with zero API calls).

### Architecture Patterns

| Pattern | Implementation | File |
|---------|---------------|------|
| **4-Layer Memory Stack** | `Layer0` (identity, ~100t), `Layer1` (essential story, ~500-800t), `Layer2` (on-demand wing/room retrieval), `Layer3` (deep semantic search) | `layers.py` |
| **Verbatim Storage** | Source files chunked into 800-char "drawers" with 100-char overlap, stored as raw text in ChromaDB. No summarization at ingest. | `miner.py` |
| **Palace Taxonomy** | **Wings** (projects/people) ? **Rooms** (topics) ? **Closets** (summaries) ? **Drawers** (verbatim chunks). **Halls** connect rooms within a wing; **tunnels** connect rooms across wings. | `README.md`, `palace_graph.py` |
| **Hybrid Retrieval** | Semantic search (ChromaDB) + keyword overlap re-ranking. Fuses embedding distance with stopword-filtered keyword overlap score. | `longmemeval_bench.py` (`build_palace_and_retrieve_hybrid`) |
| **Temporal Boosting** | Parses relative time expressions ("a week ago", "last month") from queries, computes target date, applies up to 40% distance reduction for sessions within tolerance window. | `longmemeval_bench.py` (`parse_time_offset_days`, `apply_temporal`) |
| **Synthetic Preference Docs** | Regex extracts 16+ preference/concern patterns (e.g. "I’ve been having trouble with X", "I prefer X") at ingest, creates synthetic docs with same corpus_id to bridge vocabulary gaps. | `longmemeval_bench.py` (`extract_preferences`) |
| **Two-Pass Assistant Retrieval** | Detects assistant-reference questions ("you suggested..."). Pass 1: search user turns only ? top-3 sessions. Pass 2: re-index those sessions with user+assistant turns and re-query. | `longmemeval_bench.py` (`is_assistant_reference`) |
| **Hall Classification** | Sessions classified into `hall_preferences`, `hall_facts`, `hall_events`, `hall_assistant`, `hall_general`. Query classified into hall ? tight search first, fallback to full haystack with hall-boosted scoring (25% distance reduction). | `longmemeval_bench.py` (`classify_session_hall`, `classify_question_hall`) |
| **LLM Re-rank (optional)** | Top-10/20 sessions sent to Claude Haiku/Sonnet; model picks most relevant session and it is promoted to rank 1. Enables 100% R@5 with ~$0.001/query. | `longmemeval_bench.py` (`llm_rerank`) |
| **Knowledge Graph** | SQLite-backed temporal entity-relationship graph (`valid_from`/`valid_to`). Competes with Zep/Neo4j but runs locally. | `knowledge_graph.py` |

### 96.6% LongMemEval Techniques (Raw Mode)
The baseline 96.6% comes from **raw verbatim text** + ChromaDB default embeddings (`all-MiniLM-L6-v2`) + simple semantic search. No keyword fusion, no temporal boost, no LLM. The finding is that *not losing information* beats clever extraction.

### Progression to 100%
- `hybrid` (96.6% ? ~97%) — keyword overlap re-ranking
- `hybrid_v2` — temporal date boost + two-pass assistant retrieval + preference broadening
- `hybrid_v3` — synthetic preference docs at ingest + expanded re-rank pool
- `hybrid_v4` — memory/nostalgia patterns + person name boost + quoted phrase boost
- `palace` — hall classification + two-pass navigation (tight hall search ? full fallback)
- `diary` — LLM topic extraction per session (Haiku) as synthetic docs
- `+ rerank` — LLM re-ranks top-10/20 ? 100% R@5

---

## 2. Key Source Files

| File | Lines | Role |
|------|-------|------|
| `mempalace/layers.py` | 515 | 4-layer memory stack (`Layer0`, `Layer1`, `Layer2`, `Layer3`, `MemoryStack`). Always-loaded identity + essential story, on-demand room recall, deep semantic search. |
| `mempalace/miner.py` | 641 | Project ingestion. `chunk_text()` (800-char chunks, 100-char overlap, paragraph-aware splitting), `detect_room()` (folder/filename/keyword routing), `add_drawer()` (ChromaDB upsert with mtime tracking), `GitignoreMatcher`. |
| `mempalace/searcher.py` | 152 | Semantic search API. `search()` prints verbatim results; `search_memories()` returns structured dict with similarity scores. |
| `benchmarks/longmemeval_bench.py` | 3,405 | Complete benchmark suite. Defines `build_palace_and_retrieve_*` for raw, aaak, rooms, hybrid (v1-v4), palace, diary, full modes. Also `llm_rerank()`, `diary_ingest_session()`. |
| `mempalace/knowledge_graph.py` | 393 | SQLite temporal KG. `KnowledgeGraph.add_triple()`, `query_entity()`, `invalidate()`, `timeline()`. WAL mode, indexed on subject/object/predicate/valid dates. |
| `mempalace/dialect.py` | 1,075 | AAAK lossy abbreviation dialect (entity codes, emotion flags, zettel format). Experimental; scores 84.2% vs raw 96.6%. Not used for storage default. |
| `mempalace/palace_graph.py` | 227 | Graph traversal over palace metadata. `build_graph()`, `traverse()` (BFS), `find_tunnels()` (cross-wing rooms). No external graph DB. |
| `mempalace/config.py` | 209 | `MempalaceConfig` — env/file defaults, name sanitization, topic wings, hall keywords. |

---

## 3. Nom Mapping

**Target:** `nom-canvas/crates/nom-compose/src/memory.rs`

### Current State (`memory.rs`)
- `MemoryEntry` — timestamp, role, content, token_count (whitespace estimator).
- `VerbatimMemory` — simple `Vec<MemoryEntry>` with FIFO eviction on `max_tokens`.
- `retrieve_relevant()` — **keyword-only** retrieval: substring match ? word overlap ratio. No embeddings, no layering, no structured taxonomy.

### Gap Analysis

| MemPalace Capability | Nom `memory.rs` Status | Gap |
|---------------------|----------------------|-----|
| Semantic embedding search | Missing entirely | Needs vector store integration |
| 4-layer context stack (L0-L3) | Missing | Needs `MemoryStack` equivalent |
| Structured taxonomy (wing/room/hall) | Missing | Needs metadata schema + routing |
| Hybrid retrieval (embedding + keyword) | Only keyword | Needs fusion scoring |
| Temporal query parsing/boosting | Missing | Needs date extraction + scoring |
| Synthetic preference docs | Missing | Needs regex extraction at ingest |
| Two-pass assistant retrieval | Missing | Needs role-aware indexing |
| Hall classification | Missing | Needs heuristic or learned classifier |
| Knowledge graph (temporal triples) | Missing | Needs SQLite KG or equivalent |
| Verbatim chunking with overlap | Missing | Currently stores whole entries |

### Recommended Adoption Path

1. **Layered Memory Stack** — Implement `MemoryStack` trait with L0 (identity text), L1 (top-k important entries), L2 (filtered by topic/project), L3 (full semantic search). This directly maps to `layers.py`.
2. **Embedding Backend** — Replace keyword-only `retrieve_relevant` with a pluggable embedding store (e.g., `ort` + local ONNX model, or SQLite-vec). ChromaDB is Python-native; Nom needs a Rust equivalent.
3. **Hybrid Scoring** — Keep keyword overlap as a secondary signal. The formula used by MemPalace is: `fused_dist = embedding_dist * (1.0 - hybrid_weight * keyword_overlap)` where `hybrid_weight ˜ 0.30`.
4. **Temporal Awareness** — Add `parse_time_offset_days()` equivalent for Nom’s use case. Parse relative dates in queries, boost entries near the computed target date.
5. **Synthetic Docs at Ingest** — Add lightweight regex extraction for preferences/concerns (the 16 patterns in `PREF_PATTERNS`). Store synthetic docs alongside raw entries with shared IDs.
6. **Hall/Topic Classification** — Start with keyword heuristics (`classify_session_hall` / `classify_question_hall`). No LLM required for baseline.
7. **Verbatim Chunking** — Add `chunk_text()` equivalent (800-char chunks, 100-char overlap, paragraph boundary aware) to `MemoryEntry` ingestion pipeline.

---

## 4. Licensing/Complexity Notes

- **License:** MIT ( permissive, compatible with Nom’s license )
- **Language:** Python 3.x
- **Key Dependencies:** `chromadb`, `fastembed` (optional), `pyyaml`
- **Complexity Assessment:**
  - **Storage layer** (miner, palace, config): **Low-Medium** — straightforward ChromaDB ops + file walking.
  - **Retrieval layer** (searcher, layers): **Low** — thin wrappers around ChromaDB.
  - **Benchmark/optimization** (`longmemeval_bench.py` hybrid v1-v4, palace, diary): **High** — extensive heuristic tuning, 16+ regex patterns, temporal math, two-pass logic, LLM rerank integration. The 96.6% is simple; the last 3.4% is complex.
  - **Knowledge graph** (`knowledge_graph.py`): **Low** — standard SQLite schema with temporal validity.
  - **AAAK dialect** (`dialect.py`): **Medium** — large regex-driven abbreviation system, but experimental and currently regresses retrieval.

**Risk Note:** The palace architecture (wings/rooms/halls) is elegant but the retrieval gains come primarily from **hybrid scoring + temporal boosting + synthetic docs**, not from the spatial metaphor alone. The README acknowledges that metadata filtering is a standard ChromaDB feature, not a moat.

---

## 5. Adoption Effort Estimate

| Feature | Effort | Notes |
|---------|--------|-------|
| Verbatim chunking + FIFO eviction | **1–2 days** | `chunk_text()` is ~40 lines of Python; Rust port is straightforward. |
| 4-layer memory stack (`MemoryStack`) | **2–3 days** | Thin composition layer over existing storage. |
| Keyword-only hybrid scoring | **1–2 days** | Add stopword list + overlap ratio to existing `retrieve_relevant`. |
| Semantic embedding search (local ONNX) | **1–2 weeks** | Requires embedding model integration (e.g., `rust-bert`, `ort`, or `candle`). Most of the work is embedding backend selection, not search logic. |
| Temporal query parsing + boost | **2–3 days** | Port regex patterns and date math from `parse_time_offset_days`. |
| Synthetic preference docs at ingest | **2–3 days** | Port 16 regex patterns; store synthetic entries with shared IDs. |
| Hall/topic heuristic classification | **2–3 days** | Port keyword signal lists from `classify_session_hall` / `classify_question_hall`. |
| Two-pass assistant retrieval | **1–2 days** | Detect assistant-reference triggers, do filtered search then re-index top-k with full text. |
| SQLite temporal knowledge graph | **3–5 days** | Port `knowledge_graph.py` schema and queries to Rust (`rusqlite`). |
| LLM re-rank integration | **2–3 days** | HTTP client to Anthropic API, prompt formatting, rank promotion. Optional for baseline. |
| **Total (full feature parity)** | **~4–6 weeks** | Assumes one engineer, sequential work. Parallelizable to ~2–3 weeks. |
| **Total (96.6% baseline only)** | **~1 week** | Verbatim storage + local semantic search + simple retrieval. The 96.6% score comes from the simple thing. |

### Recommendation

Adopt the **verbatim storage + semantic search + hybrid keyword scoring** pattern first. That gets Nom to the 96.6% baseline with minimal complexity. The layered stack (L0-L3) and hall taxonomy are useful for context-window management but are secondary to the retrieval quality. The advanced features (temporal boost, synthetic docs, two-pass retrieval) should be added incrementally as needed.
