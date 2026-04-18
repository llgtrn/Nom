# NomInspector — Universal Inspect / Clone / Replicate Engine

**Date:** 2026-04-19  **Status:** Draft

---

## Vision

NomInspector turns any external URL, identity, or media artifact into a content-addressed `.nomx` entry in the Nom dictionary.  
One command: detect target kind → fetch metadata → emit findings → write DB rows.  
Replicate anything the web knows about into the Nom graph.

---

## InspectTarget Kinds (7)

| Kind | Trigger | Primary provider |
|---|---|---|
| YouTube | `youtube.com` / `youtu.be` | yt-dlp |
| GitHub | `github.com` | git clone (bare) + tree walk |
| Website | any `http(s)://` not above | Playwright |
| Person | single word, no dot | Sherlock (OSINT) |
| Company | domain without http | Playwright + Sherlock |
| Video | local path `.mp4/.mkv/.webm` | yt-dlp / ffprobe |
| Image | local path `.png/.jpg/.webp` | native decoder |

---

## Pipeline

```
input
  └─ detect (InspectRequest::new)
       └─ inspect (provider adapter)
            └─ findings: Vec<Finding>
                 └─ nomx_entry: String   ("define <kind> that …")
                      └─ DB row: content_hash → entry  (ContentStore::dedup_insert)
```

Each stage is a pure function; the panel holds history only.

---

## Provider Adapters

| Provider | Kinds served | Output |
|---|---|---|
| **yt-dlp** | YouTube, Video | title, channel, duration, description, thumbnail hash |
| **git** (bare clone) | GitHub | repo name, stars, topics, top language, README excerpt |
| **Playwright** | Website, Company | title, meta description, visible text, screenshot hash |
| **Sherlock** | Person, Company | found-on-platform list, profile URLs, confidence scores |

All adapters produce `Vec<Finding>` (label + value + strength).  
Adapters are pluggable — implement the `InspectAdapter` trait, register by `InspectKind`.

---

## Sherlock Integration (Person / Company OSINT)

- Spawn `sherlock <username>` as a subprocess; parse JSON output.
- Each found platform → one `StrategySignal { signal_type: "social", value: platform, strength }`.
- Aggregate into `StrategyReport`; write one DB entry per platform found.
- Rate-limit: max 3 concurrent Sherlock probes; backoff on 429.

---

## ChatPanel ↔ InspectPanel ↔ CanvasMode Flow

```
ChatPanel  ──"inspect <url>"──▶  ChatDispatch
                                      │ CanvasMode::Inspect
                                      ▼
                               InspectPanel::inspect()
                                      │  InspectResult { canvas_mode, findings_count, nomx_preview }
                                      ▼
                               CanvasMode switch:
                                  GithubRepo  → canvas
                                  YouTube     → compose
                                  Person/Co.  → document
                                  Website     → editor
```

InspectPanel emits `InspectResult`; the shell reads `canvas_mode` and activates the matching layout.

---

## Output: Content-Addressed DB Rows

Every finding writes one row via `ContentStore::dedup_insert`:

```
hash  = FNV-1a(nomx_entry_string)
entry = "define <kind> that source(<url>) findings(<n>) provider(<adapter>)"
```

Duplicate inspections of the same URL are no-ops (hash collision → skip).  
Rows are queryable via `gitnexus_query` once indexed.

---

## Key Invariants

- Zero foreign-language names in `word` field; provenance stored in `entry_meta`.
- No wrapper layers — each adapter is native real work.
- Old adapters deleted in the same commit when replaced.
