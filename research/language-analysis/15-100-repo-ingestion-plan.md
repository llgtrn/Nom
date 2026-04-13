# 15 — 100-repo corpus ingestion plan

**Date:** 2026-04-14
**Purpose:** Execute the NON-NEGOTIABLE directive to test/train the nom-compiler against 100 real upstream repos. Each repo gets run end-to-end through the ingestion pipeline; DB1/DB2 rows land for every function/concept/entity discovered; placeholder rows materialize for symbols referenced but not yet defined; per-repo outcomes recorded in §3 below.

> **Status 2026-04-14:** Plan + report skeleton; no repos ingested yet under this harness. First execution cycle should run 1-3 small repos to validate the `nom corpus ingest-parent` path, then scale up.

---

## 1. Infrastructure check

CLI command already exists — [nom-cli/src/main.rs:1195](../../nom-compiler/crates/nom-cli/src/main.rs#L1195) wires `cmd_corpus_ingest_parent(path, dict, reset_checkpoint, json)` to walk every child repo under a parent directory. Good: 228 upstream repos at `C:\Users\trngh\Documents\APP\Accelworld\upstreams` is ready-made parent input.

Adjacent commands ([1190-1213](../../nom-compiler/crates/nom-cli/src/main.rs#L1190)):

- `corpus::cmd_corpus_scan(path, json)` — dry-run inventory
- `corpus::cmd_corpus_ingest(path, dict, json)` — single repo
- `corpus::cmd_corpus_clone_ingest(url, dict, json)` — clone + ingest
- `corpus::cmd_corpus_clone_batch(list, dict, json)` — batch clone + ingest
- `corpus::cmd_corpus_ingest_pypi(top, dict, json)` — PyPI top-N

The ingest-parent path already **streams-and-discards** per doc 04 §5.17's disk discipline (no intermediate source survival), so running on 100 repos does not require extra disk headroom beyond the biggest repo × 2.

## 2. Placeholder semantics

Per the directive's "placeholder for one that still not exist": when ingestion encounters a referenced symbol (call-target, import-target, composes-target) that the current DB does not have, emit a stub row immediately rather than failing:

```
words_v2 row:
  hash         = <synthetic "placeholder:" + reference-fingerprint>
  word         = <the referenced name>
  kind         = <best-guess from context, else "unknown">
  status       = "placeholder"
  body_hash    = NULL
  origin_ref   = <ingesting repo / caller file:line>
  authored_in  = NULL
```

When the actual symbol lands later (same or later batch), an **upsert-and-replace** per the graph-durability NodeUid + upsert pattern ([nom-graph/src/upsert.rs:421f902](../../nom-compiler/crates/nom-graph/src/upsert.rs)) fills in the real body and keeps the same UID so downstream edges don't snap.

**Implementation sketch (not yet landed):**

- `WordV2Row::status: Option<String>` — new column; migrates with default NULL for pre-existing rows.
- `find_words_v2_by_kind` already orders by hash; placeholder rows' synthetic hashes sort after real ones (prefix `placeholder:` > any hex), so real rows win in alphabetical tiebreak automatically — no resolver change needed.
- Call site: `nom-corpus::ingest_repo` — when a reference can't be resolved, call a new `upsert_placeholder(dict, name, kind, origin_ref)` helper.

This is a doc-level plan; the wedge is queued as **W7 placeholder rows** (see doc 10 Next actions on the next refresh).

## 3. Per-repo report table (to fill during execution)

Format: one row per repo ingested. Columns:

| # | Repo | Size (LOC) | Stage reached | Rows added (fn) | Rows added (concept) | Placeholders | Outcome | Fix commit |
|---|------|-----------:|---------------|----------------:|---------------------:|-------------:|---------|-----------|
| 1 | *(first run pending)* | — | — | — | — | — | — | — |

Stage legend:
- `scan` — `corpus scan` completed; repo structure mapped
- `parse` — source parsed without crashing the lexer / parser
- `extract` — `nom-extract` produced kind-classified entities
- `dict` — rows committed to `words_v2`
- `graph` — NomtuGraph constructed without panic
- `concept` — at least one concept row landed in DB1 (when applicable)

Outcome legend: `ok` / `warn:<reason>` / `skip:<reason>` / `crash:<stage>:<error>`.

## 4. Picking the first batch (by safety, not by size)

Priority order — start small + clean, escalate:

1. **Tiny Rust** — `bumpalo`, `bat` (sub-repo via `bat/build/`), `atuin`, `fd`
2. **Tiny Python** — `langchain-master/libs/core` subset, airflow `utils/`
3. **Tiny C** — `aircrack-ng/lib/crypto/`, `apt` select
4. **Tiny Go** — `gvisor/pkg/abi/`
5. **Tiny TS** — `bolt.new-main/app/components/`

After the first 20 clean, move to medium (1k-10k LOC): `dioxus`, `hickory-dns`, `kube-rs`, `helix`. Reserve giants (`linux`, `cpython`, `llvm-project`) for last.

## 5. Success criteria

- **100 repos** through at least the `parse` stage — no crashes past that gate.
- **≥ 80 repos** through `dict` — actual rows in DB2.
- **≥ 50 repos** through `graph` — edges in NomtuGraph.
- **Zero** uncaught panics from `nom-cli`; every failure caught + rendered as a `--json`-structured error.
- **Per-repo report row** filled in §3 for every run.
- **Compiler fix commits** land whenever a repeat-crash pattern emerges (don't advance past a known bug).

## 6. Dependencies + unblockers

- `nom corpus ingest-parent` works today — no blocker.
- Placeholder semantics (§2) is NOT a blocker for the initial runs; the pipeline can be exercised without it and the gaps surface as "N unresolved refs" stats. Placeholder-row wedge lands after the first ~5 repos expose how many references go unresolved in practice.
- GitNexus index stays fresh per `npx gitnexus analyze` after-each-push discipline.

## 7. Cycle cadence

- Each /loop cycle appends to §3 for whichever repos were ingested that cycle.
- Every N repos (N=5 or 10 depending on crash density) the loop pauses for a compiler-fix wedge before advancing.
- Commits per cycle: one `corpus-report: N repos ingested` commit bundling the §3 update + any fix commits from that batch.

## 8. Relation to other docs

- Doc 04 §5.17 — the original mass-corpus design; this doc is the execution artifact.
- Doc 10 §Next actions — grows a W7 placeholder-rows wedge + a W8 100-repo harness wedge.
- Doc 14 — translation examples feed back into doc 15 when a translation pattern surfaces a gap only the 100-repo run can confirm.
- Memory `feedback_100_repo_corpus_test_train.md` — the permanent directive reminder.
