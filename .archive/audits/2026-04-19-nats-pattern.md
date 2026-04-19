# NATS Server Pattern Audit

**Date:** 2026-04-19  
**Reference Repo:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\nats-server`  
**License:** Apache-2.0  
**Language:** Go 1.25  

---

## 1. Pattern Summary

NATS Server is a high-performance pub/sub message broker built around **subject-based routing**, **account-isolated namespaces**, and **zero-allocation hot paths**. Three architectural layers are relevant to Nom:

| Layer | Core Abstraction | Key Pattern |
|-------|-----------------|-------------|
| **Core Server** | `Server` / `client` | Per-connection `client` struct multiplexes reads/writes over a single TCP socket; protocol parsing is state-machine driven (`parseState`). |
| **Pub/Sub Routing** | `Sublist` | Trie (prefix tree) over dot-separated subject tokens with two wildcard classes: `*` (single token) and `>` (recursive tail). Results are LRU-cached (`slCacheMax = 1024`). |
| **JetStream (Persistence)** | `stream` / `consumer` | Append-only `StreamStore` with configurable `RetentionPolicy` (`LimitsPolicy`, `InterestPolicy`, `WorkQueuePolicy`). Consumers track deliver/ack floor sequences. |
| **Clustering** | `jetStreamCluster` + `raft` | Custom Raft implementation (`raft.go`, ~5,165 LOC) replicates stream/consumer metadata. Streams are assigned to peer groups via `streamAssignment`; the meta-controller is itself a Raft group. |
| **Intra-Process Messaging** | `ipQueue[T]` | Generic lock + slice + `chan struct{}` queue used heavily inside JetStream to avoid channel overhead (`ipqueue.go`). |

**Notable routing behavior:**
- `Sublist.Match(subject)` returns `psubs` (plain subscriptions) and `qsubs` (queue groups, stored as `[][]*subscription`).
- `processInboundClientMsg` -> `processMsgResults` -> `deliverMsg` is the hot-path delivery pipeline.
- Inter-server traffic uses `route` connections with pooled sockets (`routesPoolSize`) and gossip-based topology discovery.

---

## 2. Key Source Files

| File | Lines | What It Holds |
|------|-------|---------------|
| `server/server.go` | 4,783 | `Server` struct (accounts, routes, listeners, JetStream pointer), `NewServerFromConfig`, reload logic. |
| `server/client.go` | 6,803 | `client` struct (connection state, `subs` map, outbound buffers), `subscription` struct, `processInboundMsg`, `deliverMsg`, `processMsgResults`. |
| `server/sublist.go` | 1,729 | `Sublist` trie, `level`/`node` structs, `Match`, `Insert`, `Remove`, cache sweeper. |
| `server/opts.go` | 6,483 | `Options` struct (JSON-tagged for config), `ClusterOpts`, `JetStream` toggles, `StreamConfig` defaults. |
| `server/stream.go` | 8,752 | `stream` struct (consumers, store, mirror/sources, clustered state), `StreamConfig`, `StreamStore` interface consumer. |
| `server/consumer.go` | 6,898 | `consumer` struct, `ConsumerConfig`, `ConsumerInfo`, deliver/ack policies (`DeliverPolicy`, `AckPolicy`), queue-group logic. |
| `server/jetstream.go` | 2,873 | `jetStream` struct, `jsAccount`, API dispatch (`apiSubs`), `EnableJetStream`. |
| `server/jetstream_cluster.go` | 10,923 | `jetStreamCluster`, `streamAssignment`, `consumerAssignment`, placement, peer removal, stream move operations. |
| `server/raft.go` | 5,165 | Custom `raft` struct (WAL, snapshots, quorum logic, highwayhash checksums). |
| `server/route.go` | 3,311 | `route` struct, route pooling, gossip modes, `connectInfo`, inter-server subscription propagation (`RS+`, `RS-`). |
| `server/ipqueue.go` | 286 | `ipQueue[T]` generic intra-process queue with pool-backed slice recycling. |
| `server/store.go` | 794 | `StreamStore` interface (`StoreMsg`, `LoadMsg`, `Purge`, `Snapshot`, etc.), `RetentionPolicy`, `DiscardPolicy`. |
| `server/filestore.go` | 13,609 | File-backed `StreamStore` implementation (block files, index, encryption, TTL). |

**Total core server:** ~70,000+ lines (excluding tests).

---

## 3. Nom Mapping

### Target: `nom-compose/src/stream.rs` (actual path: `nom-canvas/crates/nom-compose/src/streaming.rs`)

The current `streaming.rs` is **AI-token streaming**, not message-broker streaming:

- `SwitchableStream` wraps `AiGlueOrchestrator` and emits `StreamToken` { text, is_final }.
- It has no subject routing, no persistence, no queue groups, and no inter-agent distribution.

### Adjacent Nom Code

- `dispatch.rs` (`nom-canvas/crates/nom-compose/src/dispatch.rs`) provides a `BackendRegistry` that maps `kind` strings to `Backend` trait implementations. This is a static registry, not a dynamic pub/sub router.

### Conceptual Port (Go -> Rust)

| NATS Pattern | Nom Equivalent | Where to Adopt |
|--------------|---------------|----------------|
| `Sublist` (subject trie) | `SubjectRouter` struct with `HashMap` levels or `radix_trie` crate | New file: `nom-compose/src/subject_router.rs` |
| `*` and `>` wildcards | Tokenize on `.`, match single or greedy tail | Router logic for distributing compose jobs by artifact type (e.g., `video.frame.*`, `audio.>`) |
| `subscription` { client, subject, queue, sid } | `Subscription` { agent_id, subject_filter, queue_group, sid } | Per-agent subscription table in composer orchestrator |
| `ipQueue[T]` | `IpQueue<T>` using `VecDeque` + `std::sync::mpsc` or `crossbeam` | Internal queues between dispatcher and backends |
| `ConsumerConfig` { FilterSubject, DeliverPolicy, AckPolicy } | `JobConsumerConfig` { filter, retry_policy, ack_timeout } | Persistent job-consumer configuration for distributed agents |
| `StreamConfig` { Subjects, Retention, Replicas } | `EventStreamConfig` { event_patterns, retention_secs, replicas } | If Nom wants durable event logs for agent audit trails |
| `jetStreamCluster` + `raft` | Use `raft` crate or `openraft` | Only if Nom needs true distributed consensus for stream metadata; otherwise start with single-node |
| `route` (inter-server pooling) | `ComposerRoute` over QUIC or TCP | Future multi-node composer mesh |

**Immediate win:** Replace the static `BackendRegistry` in `dispatch.rs` with a `SubjectRouter` so composer agents can subscribe to patterns like `image.render.>` and receive relevant jobs without hard-coded `kind` strings.

---

## 4. Licensing / Complexity Notes

- **License:** Apache-2.0 (see `LICENSE`, header comments in every file). Compatible with Nom's licensing; attribution required if copying struct definitions or algorithms verbatim.
- **Go -> Rust conceptual port:** No direct translation is viable. The Go code relies on:
  - Interface-based polymorphism (`StreamStore`, `RaftNode`).
  - `sync.RWMutex` + `atomic` patterns that map to `parking_lot` or `std::sync` in Rust.
  - `net.Buffers` scatter/gather writes (no direct Rust std equivalent; `tokio::io::WriteV` or manual flattening needed).
  - Generics (`ipQueue[T]`) -- Go 1.25 generics translate cleanly to Rust generics.
- **Complexity hotspots:**
  - `filestore.go` (13,609 lines): Highly optimized log-structured storage with encryption, compression, and TTL. Not needed for a first port.
  - `raft.go` (5,165 lines): Custom Raft with snapshot installation, membership changes, and highwayhash. Use an existing Rust Raft library instead.
  - `jetstream_cluster.go` (10,923 lines): Stream assignment, consumer migration, peer removal, and stretch-cluster logic. Very high domain complexity.

---

## 5. Adoption Effort Estimate

| Scope | Effort | Description |
|-------|--------|-------------|
| **Minimal (subject router + in-memory pub/sub)** | 1-2 weeks | Port `Sublist` trie logic to Rust; add `SubjectRouter`, `Subscription`, and `deliverMsg`-style broadcast inside a single `nom-compose` process. No persistence, no clustering. |
| **Medium (add queue groups + ipQueue + acks)** | 3-5 weeks | Add queue-group round-robin (`qsubs`), generic `IpQueue<T>`, and at-least-once delivery with manual acks. Still single-node. |
| **Full (persistent streams + consumers)** | 2-3 months | Implement `StreamStore` trait with a simple file or SQLite backend; port `stream` and `consumer` state machines; single-node JetStream-lite. |
| **Distributed (Raft clustering)** | 6-12 months | Integrate a Rust Raft crate; replicate stream/consumer metadata; handle split-brain, peer removal, and catch-up. Comparable to NATS's own engineering investment. |

**Recommendation for Nom:** Adopt only the **Minimal** scope now. The `Sublist` trie and subject wildcard patterns are high-leverage, low-risk abstractions that immediately improve `nom-compose`'s agent orchestration. Avoid porting persistence or clustering until Nom has a concrete multi-node deployment requirement.

---

*Audit completed. No production code written.*