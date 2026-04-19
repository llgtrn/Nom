# Temporal SDK Core — Pattern Extraction Audit
**Date:** 2026-04-19  
**Reference:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\temporal-sdk-core`  
**Target Nom mapping:** `nom-compose/src/durable.rs` (durable workflow execution)

---

## 1. Pattern Summary

Temporal SDK Core implements a **deterministic, replay-based durable workflow engine** with the following architectural pillars:

| Pattern | What Temporal Does |
|---------|-------------------|
| **History-driven replay** | Workflow state is not persisted directly; instead the server records an immutable event history. The worker replays history against deterministic workflow code to reconstruct state. |
| **Finite-state machines per command** | Every workflow command (schedule activity, start timer, child workflow, etc.) gets its own `StateMachine` instance. The aggregate `WorkflowMachines` orchestrator holds a `SlotMap<MachineKey, Machines>` of all active sub-machines. |
| **Command-event pairing** | When the SDK emits a command (e.g. `ScheduleActivityTask`), the server later returns a corresponding command event (`ActivityTaskScheduled`). The orchestrator matches queued commands to incoming events; mismatches produce nondeterminism errors. |
| **Sticky task queues / caching** | Workers cache workflow state in-memory. Subsequent workflow tasks for the same run are sent to a "sticky" task queue on that worker, carrying only incremental history. |
| **Local activities** | Activities executed locally inside the workflow worker use marker-record events in history so they can be resolved during replay without re-execution. |
| **Graceful shutdown & eviction** | `ManagedRun` buffers tasks, tracks outstanding activations, and supports LRU eviction when the workflow cache is full. |
| **Retry with exponential backoff** | `ValidatedRetryPolicy` enforces `initial_interval`, `backoff_coefficient`, `maximum_interval`, `maximum_attempts`, and `non_retryable_error_types`. |

---

## 2. Key Source Files

### Core workflow state machine & replay
| File | Key Types | Role |
|------|-----------|------|
| `crates/common/src/fsm_trait.rs` | `StateMachine`, `TransitionResult`, `MachineError` | Base trait for all FSMs: `on_event(event) -> Result<Vec<Command>, MachineError>` |
| `crates/sdk-core/src/worker/workflow/machines/workflow_machines.rs` | `WorkflowMachines`, `MachineResponse`, `CommandAndMachine` | Top-level orchestrator; applies history events to sub-machines, manages replay boundaries, queues commands |
| `crates/sdk-core/src/worker/workflow/machines/mod.rs` | `Machines` (enum-dispatched), `TemporalStateMachine`, `WFMachinesAdapter` | Enum-dispatch glue that bridges generic `StateMachine` impls to the workflow orchestrator |
| `crates/sdk-core/src/worker/workflow/managed_run.rs` | `ManagedRun`, `RunUpdateAct` | Per-run lifecycle manager: handles WFT arrival, activation completion, local-activity heartbeat timeouts, eviction |
| `crates/sdk-core/src/worker/workflow/workflow_stream.rs` | `WFStream`, `LocalInputs`, `WFStreamInput` | Stream processor that combines poll results, completions, local resolutions, and evictions into a pure state machine |

### Task queue polling & worker architecture
| File | Key Types | Role |
|------|-----------|------|
| `crates/sdk-core/src/worker/mod.rs` | `Worker`, `WorkerConfig`, `AllPermitsTracker` | Root worker struct; owns workflow, activity, nexus managers; configures poller behaviors and slot suppliers |
| `crates/sdk-core/src/worker/workflow/wft_poller.rs` | `make_wft_poller`, `WFTPollerShared` | Builds the workflow-task poll stream with sticky/non-sticky balancing and slot-based backpressure |
| `crates/sdk-core/src/worker/slot_provider.rs` / `slot_supplier.rs` | `SlotProvider`, `MeteredPermitDealer` | Concurrency-limiting abstraction; maps task types to semaphore-style slot suppliers |

### Activity execution & retry
| File | Key Types | Role |
|------|-----------|------|
| `crates/sdk-core/src/worker/workflow/machines/activity_state_machine.rs` | `ActivityMachine`, `SharedState`, `ActivityMachineCommand` | FSM for remote activities: states `Created → ScheduleCommandCreated → ScheduledEventRecorded → Started → {Completed, Failed, TimedOut, Canceled}` |
| `crates/sdk-core/src/worker/workflow/machines/local_activity_state_machine.rs` | `LocalActivityMachine`, `ResolveDat` | FSM for local activities with replay-aware marker recording and `RequestSent / WaitingMarkerEvent` states |
| `crates/sdk-core/src/retry_logic.rs` | `ValidatedRetryPolicy` | Validates `RetryPolicy` proto and computes `should_retry(attempt_number, application_failure) -> Option<Duration>` with exponential backoff |
| `crates/sdk-core/src/worker/activities.rs` | `WorkerActivityTasks`, `RemoteInFlightActInfo`, `ActivityTaskStream` | Manages in-flight remote activities: polls, heartbeats, eager activities, local timeouts, graceful-shutdown cancellation |

### Failure handling & compensation
| File | Key Types | Role |
|------|-----------|------|
| `crates/sdk-core/src/worker/workflow/machines/fail_workflow_state_machine.rs` | `FailWorkflowMachine`, `FailWFCommand` | Terminal FSM that emits `FailWorkflowExecution` command and waits for `WorkflowExecutionFailed` event |
| `crates/sdk-core/src/worker/workflow/machines/cancel_workflow_state_machine.rs` | `CancelWorkflowMachine` | Terminal FSM that emits `CancelWorkflowExecution` command and waits for `WorkflowExecutionCanceled` event |
| `crates/sdk/src/workflow_future.rs` | `WorkflowFuture`, `CommandID` | SDK-side future that polls user workflow code inside `panic::catch_unwind`, converts panics to `WorkflowActivationCompletion::fail`, and detects nondeterministic wakes |
| `crates/sdk-core/src/worker/workflow/machines/activity_state_machine.rs` (cancel paths) | `ScheduledActivityCancelCommandCreated`, `StartedActivityCancelCommandCreated` | Activity cancellation with three modes: `TryCancel`, `WaitCancellationCompleted`, `Abandon` |

---

## 3. Nom Mapping

**Current state of `nom-compose`**
- `workflow_compose.rs` — Static `WorkflowGraph` (nodes + edges) and `WorkflowComposer` builder. No execution semantics.
- `workflow_runner.rs` — Simple DAG executor (`WorkflowRunner`). Runs nodes in topological order via Kahn''s algorithm. Status enum: `Pending → Running → Success | Failed`. No replay, no history, no persistence.
- `task_queue.rs` — In-memory FIFO queue (`TaskQueue`). States: `Pending → Running → Completed | Failed(String) | Cancelled`. Supports `max_size` cap and basic dequeue/start/complete/cancel transitions.
- `orchestrator.rs` — Synchronous `ComposeOrchestrator` that delegates to `HybridResolver`. No worker loop, no polling, no durability.
- **`durable.rs` does not exist.**

**What a `nom-compose/src/durable.rs` would need to adopt Temporal patterns**

| Temporal Concept | Nom Equivalent (proposed) | Notes |
|------------------|---------------------------|-------|
| `WorkflowMachines` + `SlotMap<MachineKey, Machines>` | `DurableWorkflowEngine` holding `HashMap<NodeId, NodeStateMachine>` | Nom nodes (`WorkflowNode`) would each get a lightweight FSM mirroring `TemporalStateMachine`. |
| `StateMachine::on_event` | `NodeStateMachine::apply(event: HistoryEvent) -> Vec<Command>` | Transition logic per node type (Action, Condition, Loop). |
| History log / replay | `WorkflowHistory` (append-only event store) | Nom currently has no history concept. A SQLite or JSONL-backed event log is required before replay can exist. |
| `ActivityMachine` (remote activities) | `RemoteTaskStateMachine` | Map `TaskQueue` tasks to states: `Pending → Scheduled → Running → Completed | Failed | TimedOut | Cancelled`. |
| `LocalActivityMachine` (local activities) | `LocalTaskStateMachine` | For short tasks that run inside the compose process. Needs marker events so replay can skip re-execution. |
| `ValidatedRetryPolicy` | `RetryPolicy` + `BackoffCalculator` | Nom has no retry logic. Adopt the same fields: `initial_interval`, `backoff_coefficient`, `maximum_interval`, `maximum_attempts`, `non_retryable_error_types`. |
| `Worker` + `WorkerConfig` | `ComposeWorker` + `WorkerConfig` | Poll loop, slot limits, shutdown token. Nom''s `TaskQueue` is in-memory only; a worker would need a persistent task-queue backend or gRPC poll loop. |
| `wft_poller.rs` sticky queues | **Not applicable initially** | Sticky caching is an optimization; Nom can start with non-sticky full-history replay. |
| `ManagedRun` eviction & buffering | `RunHandle` with `BufferedTasks` | Needed if Nom supports concurrent workflow runs and cache limits. |
| `WorkflowFuture` panic catching | `catch_unwind` around node execution | Required to convert panics into `FailWorkflowExecution`-style events rather than crashing the worker. |
| Nondeterminism detection | **Optional v2** | Temporal detects non-SDK wakes. Nom can defer this until async node executors are introduced. |

**Minimal viable adoption path**
1. Introduce `WorkflowHistory` (append-only event log).
2. Convert `WorkflowRunner` from a one-shot DAG walker into a history-driven replay engine.
3. Add `NodeStateMachine` trait and per-node-type state machines (`ActionMachine`, `ConditionMachine`, `LoopMachine`).
4. Add `RetryPolicy` and a `BackoffCalculator` borrowed from `ValidatedRetryPolicy`.
5. Extend `TaskQueue` to emit `HistoryEvent`s on every state transition.
6. Wrap the runner in a `ComposeWorker` loop that processes `WorkflowTask` items (analogous to `WFStream`).

---

## 4. Licensing / Complexity Notes

- **License:** MIT License (Copyright 2021 Temporal Technologies, Inc.). Copying verbatim or adapting substantial portions is legally permissible with attribution.
- **Code volume:** `crates/sdk-core/src` alone contains **69 `.rs` files** totaling **~1.5 MB** of source code. This is a large, production-grade codebase.
- **Hidden complexity:** State machines are generated via the `temporalio_macros::fsm` procedural macro. The macro emits `StateMachine` impls, state structs, and transition boilerplate. A Nom port would either need to adopt a similar macro or write the boilerplate manually.
- **Proto dependencies:** Temporal is tightly coupled to Protobuf-generated types (`temporal::api::...`, `coresdk::...`). Nom would need its own event schema (or reuse a subset) to avoid pulling in the entire Temporal proto stack.
- **Testing surface:** Temporal has extensive integration tests (mock pollers, canned histories, transition coverage). Any Nom adoption should budget for a similarly rigorous test matrix around replay correctness.

---

## 5. Adoption Effort Estimate

| Component | Effort | Rationale |
|-----------|--------|-----------|
| Event history schema + storage | 2–3 days | Design `HistoryEvent` enum and a SQLite/JSONL append-only store. |
| `NodeStateMachine` trait + macros | 3–5 days | Port `fsm_trait.rs` plus a lightweight `fsm!` macro or hand-write boilerplate for 4–5 node types. |
| Replay engine (history → node states) | 5–7 days | Re-implement `WorkflowMachines::apply_next_wft_from_history` and command-event matching. |
| Retry / backoff logic | 1 day | `ValidatedRetryPolicy` is self-contained (~130 lines). Direct port feasible. |
| Task-level state machines (`RemoteTaskStateMachine`) | 3–4 days | Port `ActivityMachine` states and cancellation modes to Nom `TaskQueue` tasks. |
| Worker loop + polling abstraction | 4–6 days | Build `ComposeWorker` with shutdown tokens, slot semaphores, and a stream processor analogous to `WFStream`. |
| Local activity markers + heartbeat | 3–4 days | Port `LocalActivityMachine` and WFT-heartbeat timeout logic. |
| Failure / eviction / compensation | 2–3 days | Port `FailWorkflowMachine`, `CancelWorkflowMachine`, and `ManagedRun` eviction paths. |
| Testing (replay, nondeterminism, cancel) | 5–7 days | Write canned histories, mock pollers, and transition-coverage tests. |
| **Total** | **~28–40 engineer-days** | This assumes a single engineer familiar with Rust async and state machines. |

**Risk factors**
- Replay correctness is notoriously subtle; bugs often manifest only under specific interleavings of history events.
- Temporal''s macro-generated FSM code is dense; manual ports are error-prone.
- Nom currently has no Protobuf or gRPC infrastructure; adding a server-side task queue would expand scope significantly.

---
*Report generated by pattern-extraction analyst. Do not edit without re-auditing upstream reference.*
