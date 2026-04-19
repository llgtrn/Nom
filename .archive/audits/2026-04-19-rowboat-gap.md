# Rowboat Pattern Gap Analysis — Nom Canvas

**Date:** 2026-04-19  
**Auditor:** Gap-analysis subagent  
**Reference repo:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\rowboat-main`  
**Nom assessment source:** `maria-hill-magik-sif.md` flags rowboat as **PARTIAL** — "UI models real, no LLM integration"  
**Scope:** LLM chat orchestration, tool inspector/cards, deep-think reasoning cards, right-dock architecture  

---

## 1. Pattern Summary

Rowboat is a full-stack AI-agent IDE with three deployable surfaces:

| Surface | Stack | Role |
|---------|-------|------|
| `apps/rowboat` | Next.js + MongoDB + Redis + DI container | Production agent platform (auth, billing, widget API) |
| `apps/rowboatx` | Next.js + shadcn/ui + `ai` SDK | Next-gen IDE (SSE streaming, reasoning cards, artifact editor) |
| `apps/python-sdk` | Python | Client SDK for external integrations |

The patterns Nom claims to adopt come from **rowboatx** (the IDE surface). The four patterns under audit are:

1. **LLM Chat Orchestration** — Server-Sent Events (SSE) streaming, multi-turn context, interrupt handling, status machine, message branching, attachment upload.
2. **Tool Inspector / Tool Cards** — MCP-based tool discovery, Zod-schema parameter validation, 7-state tool lifecycle (input-streaming → output-available | output-error), collapsible JSON input/output rendering.
3. **Deep-think Reasoning Cards** — Streaming reasoning delta injection, auto-open/close collapsible, duration tracking, confidence-less but time-aware "Thinking…" shimmer, `ChainOfThought` step visualization.
4. **Right-dock Architecture** — `SidebarProvider`/`SidebarInset` split layout, collapsible agent/config/run navigator, resource-selected artifact panel (JSON / Markdown editor), breadcrumb header, persistent `apiBase` + `selectedAgent` state.

---

## 2. Key Source Files (Rowboat)

| Pattern | File | Lines | What it does |
|---------|------|-------|--------------|
| Chat orchestration | `apps/rowboatx/app/page.tsx` | 1,045 | Main IDE page: `EventSource` SSE consumer, `handleEvent` switchboard (`run-processing-start`, `llm-stream-event`, `message`, `tool-invocation`, `tool-result`, `error`), `Conversation` + `Message` rendering, `PromptInput` with agent selector |
| Chat streaming API | `apps/rowboat/app/api/stream-response/[streamId]/route.ts` | 53 | `ReadableStream` over async generator `runCachedTurnController.execute()`, SSE envelope encoding |
| Chat REST API | `apps/rowboat/app/api/v1/[projectId]/chat/route.ts` | 83 | `POST` handler with `stream` flag; returns SSE stream or JSON turn via DI container `IRunTurnController` |
| Conversation scroll | `apps/rowboatx/components/ai-elements/conversation.tsx` | 100 | `useStickToBottomContext` with smooth scroll, `ConversationScrollButton` |
| Message primitives | `apps/rowboatx/components/ai-elements/message.tsx` | 453 | `Message`, `MessageContent`, `MessageResponse` (memoized `Streamdown`), `MessageBranch` with prev/next pagination, `MessageAttachment` for image/file drag-drop |
| Tool cards | `apps/rowboatx/components/ai-elements/tool.tsx` | 165 | `Tool` (Collapsible), `ToolHeader` with 7-state `Badge` + icon, `ToolInput` (JSON `CodeBlock`), `ToolOutput` (error/result conditional) |
| Reasoning cards | `apps/rowboatx/components/ai-elements/reasoning.tsx` | 180 | `ReasoningContext` provider, auto-open on streaming start, auto-close after `AUTO_CLOSE_DELAY`, `duration` stopwatch, `Streamdown` content, `Shimmer` "Thinking…" |
| Chain-of-thought | `apps/rowboatx/components/ai-elements/chain-of-thought.tsx` | 231 | `ChainOfThought` with `complete` / `active` / `pending` step states, vertical connector lines, `ChainOfThoughtSearchResult` badges |
| MCP client | `apps/rowboat/app/lib/mcp.ts` | 32 | Dual-transport MCP client (`StreamableHTTPClientTransport` → `SSEClientTransport` fallback) |
| Tool registry | `apps/rowboat/app/lib/default_tools.ts` | 36 | `getDefaultTools()` returning Zod-schema descriptors; env-gated (`NEXT_PUBLIC_HAS_GOOGLE_API_KEY`) |
| App sidebar | `apps/rowboatx/components/app-sidebar.tsx` | 348 | `Sidebar` with `SidebarProvider`, collapsible `Agents`/`Config`/`Runs` groups, `TeamSwitcher`, `NavUser`, `NavProjects` chat history |
| Panel primitive | `apps/rowboatx/components/ai-elements/panel.tsx` | 15 | `@xyflow/react` `Panel` wrapper with `bg-card` + border styling |

---

## 3. Gap Analysis

### 3.1 LLM Chat Orchestration

| Capability | Rowboat (full pattern) | Nom (current) | Gap severity |
|------------|------------------------|---------------|--------------|
| **Real LLM backend** | `IRunTurnController` in DI container; calls OpenAI / Gemini / Anthropic via provider abstraction | `AiGlueOrchestrator` in `nom-compose/src/glue.rs` has 4 adapters (`StubLlmFn`, `NomCliLlmFn`, `McpLlmFn`, `RealLlmFn`) — **all 3 non-stub are TODO stubs** returning hardcoded strings | 🔴 **Critical** |
| **Streaming protocol** | Native SSE (`EventSource`) with `llm-stream-event` delta types (`text-delta`, `reasoning-delta`, `tool-call`) | `SwitchableStream` in `nom-compose/src/streaming.rs` splits a pre-generated blueprint string by whitespace — fake token streaming | 🔴 **Critical** |
| **Multi-turn context** | `conversationId` + `runId` persisted server-side; `/runs/{id}/messages/new` API | `AiChatSession` in `nom-panels/src/right/chat.rs` stores `Vec<ChatMessage>` in memory only; no persistence, no server sync | 🔴 **Critical** |
| **Interrupt handling** | EventSource `close()` on cleanup; backend `TurnEvent` cancellation propagation | `InterruptSignal` in `nom-canvas-intent` exists and is wired to `DeepThinkPanel::trigger_interrupt()`, but **chat has no interrupt wiring** | 🟡 **High** |
| **Status machine** | Explicit states: `submitted` → `streaming` → `ready` / `error` | `ChatMessage::is_streaming` boolean + `ThinkState` enum (Idle/Streaming/Complete/Interrupted) — **no unified chat-level status** | 🟡 **High** |
| **Message branching** | `MessageBranch` with `currentBranch` / `totalBranches` navigation | **Not implemented** | 🟢 **Medium** |
| **Scroll-to-bottom** | `useStickToBottom` library with `ConversationScrollButton` | `ChatSidebarPanel::scroll_to_bottom` boolean flag exists but no actual smooth-scroll implementation | 🟡 **High** |
| **Attachment support** | `MessageAttachment` with image preview, file icon, remove button, drag-drop via `PromptInput` | `ChatAttachment` enum (`ImageBytes`, `WebUrl`, `FilePath`, `NomxSource`) exists but UI renders only a string label in `InspectDispatch`; **no attachment rendering in chat bubbles** | 🟡 **High** |
| **Markdown rendering** | `Streamdown` component (stream-aware markdown) | No markdown parser in chat; content is plain `String` painted as monochrome quads | 🟡 **High** |

**Key finding:** Nom has the *UI model* for streaming (`ChatMessage::is_streaming`, `append_delta`, `finalize`) but the *data layer* is entirely fake. There is no HTTP client, no SSE parser, no conversation persistence, and no real LLM call graph.

---

### 3.2 Tool Inspector / Tool Cards

| Capability | Rowboat (full pattern) | Nom (current) | Gap severity |
|------------|------------------------|---------------|--------------|
| **Tool discovery** | MCP client (`getMcpClient`) with HTTP + SSE fallback; `getDefaultTools()` Zod schema registry | **No MCP integration.** `NomInspector` in `nom-compose/src/inspector.rs` is a URL heuristic → stub findings generator | 🔴 **Critical** |
| **Tool lifecycle states** | 7 states: `input-streaming`, `input-available`, `approval-requested`, `approval-responded`, `output-available`, `output-error`, `output-denied` | `ToolCard` in `nom-panels/src/right/chat_sidebar.rs` has `pending_tool: Option<ToolCard>` + `complete()` — effectively 2 states (pending / done) | 🟡 **High** |
| **Rich rendering** | `ToolHeader` with `Badge` + icon per state, `ToolInput` with JSON `CodeBlock`, `ToolOutput` with conditional error/result styling | Painted as a single 20px colored strip (`fill_quad` with `tokens::BORDER`) — **no text, no JSON, no expand/collapse** | 🔴 **Critical** |
| **Tool execution** | Backend `tool-invocation` / `tool-result` SSE events update card state in real time | `begin_tool` / `complete_tool` are manual API calls on `ChatSidebarPanel`; no async execution backend | 🔴 **Critical** |
| **Parameter schema** | Zod `parameters` object with `type`, `properties`, `required` | `input_summary: String` — free-text only, no structured schema | 🟡 **High** |

**Key finding:** Nom has the *data shape* for a tool card (`ToolCard` struct) but zero of the surrounding infrastructure: no MCP, no schema, no rich rendering, no execution backend. The visual representation is a colored rectangle strip.

---

### 3.3 Deep-think Reasoning Cards

| Capability | Rowboat (full pattern) | Nom (current) | Gap severity |
|------------|------------------------|---------------|--------------|
| **Streaming reasoning deltas** | `reasoning-delta` SSE events appended to `currentReasoning` string; committed on `reasoning-end` | `DeepThinkStream` in `nom-compose/src/deep_think.rs` generates deterministic fake hypotheses (`hypothesis_{i}: intent_{hash}`) in a loop | 🔴 **Critical** |
| **Real LLM reasoning** | Actual model reasoning output (e.g., OpenAI o1 "reasoning" field) | `classify_with_react` in `nom-canvas-intent` is a stub heuristic; no real LLM invoked | 🔴 **Critical** |
| **Collapsible UI** | `Reasoning` + `Collapsible` with auto-open on stream start, auto-close after 1s delay | `DeepThinkPanel` in `nom-panels/src/right/deep_think.rs` has `is_expanded` on `ThinkingStep` but **no collapsible animation or auto-toggle logic** | 🟡 **High** |
| **Duration tracking** | `useEffect` stopwatch from `startTime` → `Date.now()`, displayed as "Thought for N seconds" | `duration_ms` on `ToolCard` only; **reasoning steps have no duration UI** | 🟡 **High** |
| **Confidence visualization** | Not in rowboatx reasoning component (time-based only) | `ThinkCard::confidence` 0-1 mapped to `edge_color_for_confidence` border color on quads — **Nom actually exceeds rowboat here** | 🟢 **Nom ahead** |
| **Chain-of-thought steps** | `ChainOfThoughtStep` with `complete`/`active`/`pending` status, vertical connector lines, search result badges | `ThinkingStep` + `HypothesisTree` + `HypothesisTreeNav` exist with depth-first traversal, but **no vertical connector rendering, no step status badges** | 🟡 **High** |
| **Animation FSM** | CSS transitions via Radix `Collapsible` | `AnimatedReasoningCard` in `nom-panels/src/right/reasoning_card.rs` has full `CardState` FSM (Hidden → Entering → Visible → Exiting) with `progress` and `display_opacity()` — **Nom exceeds rowboat here** | 🟢 **Nom ahead** |

**Key finding:** Nom has *better* low-level animation infrastructure (`AnimatedReasoningCard` FSM, confidence border colors) but the *content* is entirely synthetic. There is no real reasoning stream from an LLM, no auto-open/close behavior, and no duration display.

---

### 3.4 Right-dock Architecture

| Capability | Rowboat (full pattern) | Nom (current) | Gap severity |
|------------|------------------------|---------------|--------------|
| **Panel layout system** | `SidebarProvider` + `SidebarInset` + `Sidebar` from shadcn/ui; `Panel` from `@xyflow/react` | `Dock` in `nom-panels/src/dock.rs` with `DockPosition` (Left/Right/Bottom), `PanelEntry`, `PanelSizeState` (fixed/flex), frosted glass overlay | 🟡 **High** |
| **Collapsible sidebar groups** | `Collapsible` `Agents`/`Config`/`Runs` groups with `ChevronRight` rotate animation, `SidebarMenuButton` | `Dock::paint_scene` emits background quad + focus ring per active tab; **no collapsible group UI, no chevrons, no nested menus** | 🟡 **High** |
| **Resize handles** | CSS drag handles with `transition-[width,height]` | `ResizeHandle` in `nom-panels/src/layout_state.rs` has `start_drag`, `end_drag`, `delta` — **but `Dock` itself does not integrate resize interaction** | 🟡 **High** |
| **Size constraints** | Tailwind class constraints (`md:w-[70%]`, `max-w-4xl`) | `PanelState::resize` clamps to [80, 800]; `panel_min_width()` = 240, `panel_max_width()` = 600 | 🟢 **Nom adequate** |
| **State persistence** | `localStorage` for `apiBase`; server persistence for runs/agents | `LayoutSnapshot::capture` / `restore_widths` exist but **no serde, no file save/load, no localStorage equivalent** | 🟡 **High** |
| **Resource navigator** | `AppSidebar` fetches `/api/rowboat/summary` for agents/config/runs; click → `setSelectedResource` → artifact editor | **No equivalent.** Nom has `FileTreePanel`, `LibraryPanel`, `NodePalette` on the left; right dock is chat + deep-think + inspect only | 🟡 **High** |
| **Artifact editor panel** | Secondary right panel (`md:w-[70%]`) with `JsonEditor` / `TiptapMarkdownEditor` / `MarkdownViewer`, save/load API | **Not implemented.** No JSON/Markdown editor in right dock | 🟡 **High** |
| **Breadcrumb header** | `Breadcrumb` + `BreadcrumbList` + `BreadcrumbPage` in `SidebarInset` header | `HeaderPanel` / `TitleBarPanel` exist in `nom-panels/src/top/` but **no breadcrumb widget** | 🟢 **Medium** |

**Key finding:** Nom's `Dock` + `PanelSizeState` + `LayoutSnapshot` provide a solid Rust-native foundation, but the *interaction layer* (drag-to-resize, collapsible groups, state persistence) is incomplete. Rowboatx provides a much richer navigational sidebar with real data fetching; Nom's right dock is chat-only with no resource inspector.

---

## 4. Adoption Effort Estimate

### 4.1 Work breakdown by pattern

| Pattern | Files to touch | Effort | Blockers |
|---------|---------------|--------|----------|
| **LLM Chat Orchestration** | `nom-compose/src/glue.rs` (real adapters), `nom-compose/src/streaming.rs` (SSE parser), `nom-panels/src/right/chat_sidebar.rs` (EventSource integration), new `nom-compose/src/llm_client.rs` | **2–3 weeks** | Needs HTTP client choice (`reqwest` vs `hyper`), SSE parser crate, async runtime decision (tokio vs glib) |
| **Tool Inspector / Tool Cards** | New `nom-compose/src/mcp.rs` (MCP client port), `nom-panels/src/right/chat_sidebar.rs` (rich ToolCard rendering), `nom-compose/src/harness.rs` (tool skill library) | **2 weeks** | MCP SDK has no official Rust client; must port TypeScript SDK or use raw JSON-RPC over HTTP/SSE |
| **Deep-think Reasoning Cards** | `nom-compose/src/deep_think.rs` (replace fake loop with real LLM call), `nom-panels/src/right/deep_think.rs` (auto-open/close logic, duration tracking) | **1 week** | Depends on LLM client above |
| **Right-dock Architecture** | `nom-panels/src/dock.rs` (resize interaction), `nom-panels/src/layout_state.rs` (serde persistence), new `nom-panels/src/right/resource_sidebar.rs` | **1–2 weeks** | Needs file I/O for persistence; GPUI event system must support drag gestures |

### 4.2 Dependency graph

```
LLM Client (reqwest + SSE)
    ├─→ Chat Orchestration
    ├─→ Deep-think Reasoning
    └─→ Tool Inspector (needs LLM for parameter filling)

MCP Client (Rust port)
    └─→ Tool Inspector

Dock Resize + Persistence
    └─→ Right-dock Architecture (independent of LLM)
```

### 4.3 Total effort

| Scenario | Time | Outcome |
|----------|------|---------|
| **Minimal viable** (stub LLM + real SSE skeleton + rich tool rendering) | **3–4 weeks** | Chat feels real but still calls stub LLM; tool cards render JSON; reasoning shows fake deltas with real timing |
| **Full rowboat parity** (real Ollama/OpenAI adapters + MCP tool discovery + layout persistence) | **6–7 weeks** | End-to-end LLM chat, tool execution, reasoning stream, resizable persistent dock |
| **Nom exceeds rowboat** (confidence visualization + animated card FSM already built + real Rust GPUI) | **8+ weeks** | All of the above plus GPUI-native performance, no Electron bloat |

### 4.4 Riskiest items

1. **MCP in Rust** — No first-party Rust MCP SDK exists. Porting the TypeScript SDK (`@modelcontextprotocol/sdk`) to Rust is non-trivial (JSON-RPC 2.0 over SSE/HTTP, capability negotiation, tool schema reflection).
2. **Async SSE in GPUI** — Nom's GPUI is custom Rust (patterned after Zed). Integrating `reqwest` SSE streaming with GPUI's event loop requires careful threading or async bridge.
3. **Real LLM adapter wiring** — `AiGlueOrchestrator` has the trait (`ReActLlmFn`) but no production adapter. Selecting a default (Ollama local) and making it discoverable requires CLI/config changes.

---

## 5. Recommendations

| Priority | Action | Rationale |
|----------|--------|-----------|
| P0 | **Implement `RealLlmFn` using `reqwest` + async SSE** in `nom-compose/src/glue.rs` | Unblocks chat, reasoning, and tool execution simultaneously |
| P0 | **Add `serde` + file persistence** to `LayoutSnapshot` and `PanelState` | Quick win; makes dock feel production-grade |
| P1 | **Port MCP client to Rust** or use raw HTTP tool discovery | Required for real tool cards; without this, tool inspector remains a stub |
| P1 | **Add auto-open/close + duration tracking** to `DeepThinkPanel` | Small UI change; closes the reasoning-card gap vs rowboat |
| P2 | **Add `MessageBranch` equivalent** to `ChatSidebarPanel` | Differentiating feature; rowboat has it, Nom does not |
| P2 | **Build `ResourceSidebar`** for agents/config/runs | Rowboatx's `AppSidebar` is a major UX win; Nom currently has no right-side resource navigator |

---

*Report generated from direct source inspection of 18 files across rowboat-main and nom-canvas. All class/component names verified against actual source.*
