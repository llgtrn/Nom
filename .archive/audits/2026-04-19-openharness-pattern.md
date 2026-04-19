# OpenHarness Pattern Audit Report

Date: 2026-04-19
Source: `C:\Users\trngh\Documents\APP\Accelworld\upstreams\OpenHarness-main`
Target mapping: `nom-compose/src/harness.rs`

---

## 1. Pattern Summary

OpenHarness is a Python-based agent harness with a clean three-layer architecture:

- **Tool layer**: ~34 built-in tools + dynamic MCP adapters, all implementing `BaseTool` with Pydantic input schemas and async `execute()`.
- **Skill layer**: Markdown skill definitions loaded from bundled, user (`~/.openharness/skills/`), and plugin directories, exposed to the LLM via a `skill` tool.
- **Engine layer**: `QueryEngine` drives the conversation loop (`run_query`), handling streaming, tool dispatch (sequential or concurrent), auto-compaction, permission gating, and hook execution.

Key design choices:
- **Registry pattern**: `ToolRegistry` is a plain `dict[str, BaseTool]`. Tools are registered by instance in `create_default_tool_registry()`.
- **Schema-driven**: Every tool declares a Pydantic `input_model`; `to_api_schema()` emits Anthropic Messages API JSON schemas automatically.
- **Context passing**: `ToolExecutionContext` carries `cwd` and a `metadata` bag (includes `tool_registry`, `ask_user_prompt`, and caller-supplied metadata).
- **Memory as prompt injection**: Project memory lives in `~/.openharness/data/memory/<project-hash>/`. Relevant memories are retrieved via token-heuristic search and injected into the system prompt at runtime.
- **Provider abstraction**: `ProviderProfile` + `AuthManager` normalize credentials across Anthropic, OpenAI, Copilot, Moonshot, etc., with keyring/file fallback and env-var precedence.

---

## 2. Key Source Files

| File | Responsibility |
|------|---------------|
| `src/openharness/tools/base.py` | `BaseTool` (ABC), `ToolRegistry`, `ToolExecutionContext`, `ToolResult` |
| `src/openharness/tools/__init__.py` | `create_default_tool_registry()` — hard-codes all 34 built-in tools plus MCP dynamic registration |
| `src/openharness/tools/bash_tool.py` | `BashTool` — shell execution with timeout, sandbox fallback, output truncation |
| `src/openharness/tools/web_search_tool.py` | `WebSearchTool` — DuckDuckGo HTML scraping, regex parsing, read-only |
| `src/openharness/tools/agent_tool.py` | `AgentTool` — spawns sub-agents via `swarm` backend (`subprocess` executor) |
| `src/openharness/tools/mcp_tool.py` | `McpToolAdapter` — wraps external MCP tools into `BaseTool` with auto-generated Pydantic input models |
| `src/openharness/skills/registry.py` | `SkillRegistry` — simple dict of `SkillDefinition` |
| `src/openharness/skills/loader.py` | `load_skill_registry()` — loads bundled, user, and plugin skills from `SKILL.md` files with YAML frontmatter |
| `src/openharness/skills/types.py` | `SkillDefinition` dataclass (`name`, `description`, `content`, `source`, `path`) |
| `src/openharness/memory/manager.py` | `add_memory_entry()`, `remove_memory_entry()` — file-based memory CRUD |
| `src/openharness/memory/search.py` | `find_relevant_memories()` — token-scored heuristic search (metadata 2x weight) |
| `src/openharness/memory/paths.py` | `get_project_memory_dir()` — SHA-1 hashed per-project storage |
| `src/openharness/prompts/context.py` | `build_runtime_system_prompt()` — assembles system prompt, skills list, `claude.md`, issue context, and memory injection |
| `src/openharness/engine/query_engine.py` | `QueryEngine` — owns history, API client, tool registry, cost tracking |
| `src/openharness/engine/query.py` | `run_query()` — core agent loop with auto-compact, tool dispatch, permission checks, hooks |
| `src/openharness/config/settings.py` | `Settings` (Pydantic), `ProviderProfile`, `ResolvedAuth`, credential resolution with env/file/keyring precedence |
| `src/openharness/auth/manager.py` | `AuthManager` — central authority for provider auth state, profile switching, credential storage |
| `src/openharness/auth/storage.py` | `store_credential()`, `load_credential()` — file-based JSON (`credentials.json`, mode 600) + optional keyring |
| `src/openharness/bridge/manager.py` | `BridgeSessionManager` — spawns and tracks child agent subprocesses |

---

## 3. Nom Mapping

### Current Nom State (`nom-compose/src/harness.rs`)

Nom’s `ToolHarness` is a minimal `HashMap<String, Box<dyn Fn(&str) -> Result<String, String>>>` with 11 stub tools:

- `search`, `calculate`, `read_file`, `write_file`, `http_get`, `parse_json`, `format_text`, `summarize`, `translate`, `execute_nomx`

Tools take raw `&str` input, have no schema validation, no async execution, no permission gating, and no read-only classification.

### OpenHarness → Nom Gap Analysis

| OpenHarness Feature | Nom Gap | Adoption Notes |
|---------------------|---------|---------------|
| `BaseTool` + Pydantic `input_model` | No schema layer | Introduce a `Tool` trait with associated `Input` type (or serde JSON schema generation). |
| `ToolRegistry` (dict of instances) | `HashMap` of closures | Replace closure map with a `HashMap<String, Box<dyn Tool>>` implementing a common trait. |
| `ToolExecutionContext` | No context object | Add a context struct carrying `cwd`, `tool_registry`, and `ask_user_prompt` references. |
| `is_read_only()` gating | No read-only flag | Add a trait method for permission classification before execution. |
| `to_api_schema()` | No schema emission | Generate JSON schemas (e.g., via `schemars`) for each tool’s input struct. |
| 34 built-in tools | Only 11 stubs | Port high-value tools first: `BashTool`, `WebSearchTool`, `WebFetchTool`, `GlobTool`, `GrepTool`, `FileEditTool`, `TodoWriteTool`. |
| MCP adapter (`McpToolAdapter`) | No external tool discovery | Build an MCP client layer that dynamically registers external tools at startup. |
| `SkillRegistry` + `SKILL.md` loader | No skill system | Add markdown skill loading from `.kimi/skills/` or `.claude/skills/` directories. |
| Memory prompt injection | No memory integration | Add `~/.nom/memory/<project-hash>/MEMORY.md` index + `find_relevant_memories()` heuristic, inject into system prompt. |
| `QueryEngine` / `run_query` loop | No agent loop | Implement a conversation engine with max-turns, streaming, and concurrent tool dispatch. |
| `AuthManager` / `ProviderProfile` | No credential abstraction | Add provider profiles and a credential store (keyring or encrypted file). |
| Permission checker | No permission system | Add a lightweight permission evaluator before tool execution. |
| Hooks (`HookExecutor`) | No pre/post hooks | Add pre/post tool hooks for logging, blocking, or telemetry. |

### Suggested Nom 43+ Tool Roadmap

1. **Core file tools** (6): `read_file`, `write_file`, `file_edit`, `glob`, `grep`, `notebook_edit`
2. **Shell/system tools** (2): `bash`, `sleep`
3. **Web tools** (2): `web_search`, `web_fetch`
4. **Knowledge tools** (2): `skill`, `tool_search`
5. **Project tools** (3): `todo_write`, `enter_plan_mode`, `exit_plan_mode`
6. **Task/swarm tools** (6): `task_create`, `task_get`, `task_list`, `task_stop`, `task_output`, `task_update`
7. **Team tools** (3): `agent`, `team_create`, `team_delete`, `send_message`
8. **Config/tools** (2): `config`, `brief`
9. **Cron tools** (4): `cron_create`, `cron_list`, `cron_delete`, `cron_toggle`
10. **MCP tools** (dynamic): `list_mcp_resources`, `read_mcp_resource`, plus one adapter per MCP server tool
11. **LSP tool** (1): `lsp`
12. **User interaction** (1): `ask_user_question`
13. **Worktree** (2): `enter_worktree`, `exit_worktree`
14. **Remote trigger** (1): `remote_trigger`

Total built-in: ~35; plus dynamic MCP tools brings the count past 43 easily.

---

## 4. Licensing/Complexity Notes

- **License**: MIT (Copyright 2025 OpenHarness Contributors). Safe to study, borrow patterns, and port.
- **External dependencies**: Heavy reliance on `pydantic`, `anthropic`, `httpx`, and optional `keyring`. Nom’s Rust stack would replace these with `serde`, `schemars`, `reqwest`, and a Rust keyring crate (e.g., `keyring`).
- **Complexity hotspots**:
  - `engine/query.py` (324 lines) — the core loop has nuanced retry, compaction, permission prompt, and concurrent dispatch logic.
  - `config/settings.py` (770 lines) — provider profile migration from flat settings is verbose but self-contained.
  - `auth/manager.py` (436 lines) — many provider-specific branches; porting only the providers Nom needs would reduce this.
  - `tools/mcp_tool.py` — dynamic Pydantic model creation from JSON schema is tricky in Rust; a `serde_json::Value` fallback or `jsonschema` validation may be simpler.
- **Test coverage**: Extensive (`tests/`). Porting tests alongside code is recommended.

---

## 5. Adoption Effort Estimate

| Module | Effort | Notes |
|--------|--------|-------|
| Tool trait + registry refactor | **1–2 days** | Replace closure map with trait objects; add schema generation via `schemars`. |
| Built-in tool expansion (11 → 35) | **3–5 days** | Port high-value tools first; many are thin wrappers around `std::fs`, `tokio::process`, or `reqwest`. |
| MCP adapter | **2–3 days** | Requires JSON-schema-to-Rust mapping or `serde_json::Value` generic input; needs MCP client (stdio/SSE). |
| Skill loader + registry | **1 day** | Walk directories, parse YAML frontmatter, store in `HashMap`. |
| Memory integration | **1–2 days** | File I/O + token search + prompt concatenation; straightforward. |
| Agent loop (`QueryEngine`) | **3–5 days** | Async streaming, turn limiting, concurrent tool dispatch, conversation compaction. |
| Auth / credential injection | **2–3 days** | Simplify to 2–3 providers (Anthropic + OpenAI); keyring crate integration. |
| Permission system | **1–2 days** | Read-only check + path rules + user confirmation prompt. |
| Hooks | **1 day** | Pre/post trait hooks around tool execution. |
| **Total** | **~15–25 developer-days** | Depending on how many providers and tools Nom actually needs. |

**Risk**: The dynamic MCP schema mapping and the full conversation compaction/summarization loop are the two highest-complexity items. Deferring MCP dynamic schemas to a generic JSON-input tool would cut ~2 days and reduce runtime complexity.

---

*Report generated by pattern-extraction analyst. Do not edit without re-auditing against upstream source.*
