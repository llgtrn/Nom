# NomCanvas IDE — 4 Remaining Gaps Implementation Plan

**Date:** 2026-04-17
**Scope:** 4 gaps, ~12 tasks across frontend (src/) and backend (src-tauri/src/)
**Estimated complexity:** MEDIUM-HIGH

---

## GAP 1: Canvas Rendering Layer (L)

Currently `SpatialCanvas` in `canvas.ts` uses DOM nodes + CSS `transform` for positioning. The 8 element types in `elements.ts` (rectangle, ellipse, diamond, text, arrow, line, connector, image) have hit-testing but no draw calls.

### Tasks

**1a.** Create `src/renderer.ts` — HTML5 Canvas 2D rendering engine.
- Add a `<canvas>` element sized to container, layered behind DOM blocks.
- Implement `drawElement(ctx: CanvasRenderingContext2D, el: CanvasElement, vp: Viewport)` with a switch on all 8 element types. Rectangle/ellipse/diamond as filled paths; text via `ctx.fillText`; arrow/line/connector via polyline + arrowhead; image via `ctx.drawImage`.
- Wire `requestAnimationFrame` loop that calls `Viewport.clientToModel` for coordinate transforms.
- **AC:** All 8 types render visually on a `<canvas>` at any zoom/pan level.

**1b.** Integrate renderer with `ElementStore` and `SpatialCanvas`.
- `SpatialCanvas` constructor creates the canvas overlay and instantiates the renderer.
- Selection state from `ElementStore.getSelected()` draws resize handles (use `getTransformHandles` from `elements.ts`).
- Viewport frustum culling via `boundsIntersect` to skip off-screen elements.
- **AC:** Elements added via `ElementStore.add()` appear on canvas; selection shows handles.

**1c.** Mouse interaction on canvas layer.
- Pointer events on the `<canvas>` element call `ElementStore.hitTest()` for selection.
- Drag-to-move updates element x/y and triggers re-render.
- Handle resize via transform handles; rotation via rotation handle.
- **AC:** Click-select, drag-move, resize, rotate work for rectangle and ellipse.

**Deps:** 1a -> 1b -> 1c
**Crate APIs:** None (pure frontend). Patterns from `elements.ts` hit-test + `canvas.ts` Viewport.

---

## GAP 2: LSP Client in Canvas (L)

`nom-lsp` (at `nom-compiler/crates/nom-lsp/`) already implements hover, goto-def, completion, diagnostics, and ReAct drill-through over stdio. The Prosemirror editor in `main.ts` has no LSP bridge.

### Tasks

**2a.** Add Tauri sidecar or command to spawn `nom-lsp` as a child process.
- In `src-tauri/src/lib.rs`, add a `start_lsp` command that spawns the nom-lsp binary with stdio pipes.
- Use Tauri's `sidecar` or `Command::new_sidecar` API to manage the process lifecycle.
- Forward JSON-RPC messages between frontend and the LSP process via Tauri events.
- **AC:** `nom-lsp` process starts on app launch, frontend can send/receive JSON-RPC.

**2b.** Create `src/lsp-client.ts` — minimal LSP JSON-RPC client.
- Implement `initialize`, `textDocument/hover`, `textDocument/completion`, `textDocument/definition`, `workspace/executeCommand` request methods.
- Wire Tauri event listeners for server-to-client messages (diagnostics, etc.).
- **AC:** `lspClient.hover(uri, position)` returns markdown content from nom-lsp.

**2c.** Integrate LSP client with Prosemirror editor.
- Create a Prosemirror plugin that sends `textDocument/didChange` on doc changes.
- On hover (tooltip), call `lspClient.hover()` and display markdown popup.
- On Ctrl+Space, call `lspClient.completion()` and render completion menu.
- On Ctrl+Click / F12, call `lspClient.definition()` and navigate.
- **AC:** Hovering a known nomtu word shows signature+contracts; completion shows keywords.

**Deps:** 2a -> 2b -> 2c
**Crate APIs:** `nom_lsp::serve_on_stdio()`, `nom_lsp::server_capabilities()`. Env vars: `NOM_DICT`, `NOM_GRAMMAR_DB`.

---

## GAP 3: Unstub 3 Tauri Commands (M)

`build_artifact`, `resolve_intent`, `platform_spec` are stubs because `nom-llvm` (via inkwell) requires LLVM C headers. `nom-intent` also depends on `nom-llvm`.

### Tasks

**3a.** Feature-gate LLVM in nom-canvas Cargo.toml.
- Add `nom-intent = { path = "../../nom-compiler/crates/nom-intent", optional = true }` and `nom-llvm = { ..., optional = true }` to `Cargo.toml`.
- Create feature `llvm = ["dep:nom-llvm", "dep:nom-intent"]`.
- In `lib.rs`, gate `build_artifact` and `resolve_intent` bodies with `#[cfg(feature = "llvm")]` — return real results when available, current stub otherwise.
- **AC:** `cargo build` succeeds without LLVM; `cargo build --features llvm` compiles the full pipeline.

**3b.** Wire real `resolve_intent` behind the feature gate.
- When `llvm` feature is on: call `nom_intent::classify(input, &IntentCtx::default(), &stub_llm)`.
- Map `NomIntent` variants to `IntentResult { action, confidence, tools_used }`.
- **AC:** `resolve_intent("cache a function")` returns action="define" with confidence > 0.

**3c.** Wire real `platform_spec`.
- This does not need LLVM. Read the target string ("windows", "wasm", "linux") and return the appropriate launch command from a static table.
- For "wasm": `launch_command = "wasm-bindgen-runner"`. For "windows": `launch_command = ".\\artifact.exe"`. For "linux": `launch_command = "./artifact"`.
- **AC:** `platform_spec("windows")` returns non-empty launch_command.

**Deps:** 3a -> 3b (parallel with 3c). 3c is independent.
**Crate APIs:** `nom_intent::classify`, `nom_llvm::compile` (behind feature gate).

---

## GAP 4: Credential/Secret Management (S)

No secure storage for API keys needed by v4 intelligence features.

### Tasks

**4a.** Create `src/credentials.ts` — encrypt/decrypt credential store.
- Use Web Crypto API (`crypto.subtle.encrypt/decrypt`) with AES-GCM.
- Derive encryption key from a user-provided master password via PBKDF2.
- Store encrypted blob via Tauri `fs` plugin to `$APPDATA/nom-canvas/credentials.enc`.
- API: `saveCredential(name: string, value: string)`, `loadCredential(name: string): string | null`, `listCredentials(): string[]`, `deleteCredential(name: string)`.
- **AC:** Round-trip test: save + load returns original value; raw file is not readable as plaintext.

**4b.** Add Tauri backend support for secure storage.
- Add `tauri-plugin-store` or use the OS keyring via a Tauri command for the master key.
- Create `store_credential` and `load_credential` Tauri commands that delegate to OS-level secure storage (Windows Credential Manager via `keyring` crate).
- **AC:** API keys persist across app restarts; master key in OS keyring.

**Deps:** 4a and 4b can be built in parallel; 4a is the frontend API, 4b is the backend persistence. Integration requires both.
**Crate APIs:** None from nom-compiler. External: `keyring` crate (Rust), Web Crypto API (JS).

---

## Dependency Graph (cross-gap)

```
GAP 3a (feature-gate) ─── no deps on other gaps
GAP 4  ─────────────────── no deps on other gaps
GAP 1  ─────────────────── no deps on other gaps
GAP 2a ─────────────────── needs nom-lsp binary built (outside nom-canvas)
GAP 2c ─────────────────── benefits from GAP 1 (canvas elements visible for goto-def)
```

**Recommended execution order:** GAP 3 (unblocks CI) -> GAP 4 (small, parallel) -> GAP 1 (rendering) -> GAP 2 (LSP, depends on stable editor).

## Success Criteria

- [ ] All 8 element types render on HTML5 Canvas at arbitrary zoom
- [ ] Prosemirror editor shows hover/completion/goto-def from nom-lsp
- [ ] `cargo build` succeeds without LLVM; all 16 Tauri commands non-stub
- [ ] Credentials encrypted at rest, master key in OS keyring
- [ ] Zero new npm/cargo warnings introduced
