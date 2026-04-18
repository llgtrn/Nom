# Nom — Integrated Development Workflow

Three systems work together. Route to the right one.

## System Roles

| System | Role | When |
|--------|------|------|
| **GitNexus** | Code knowledge graph — structure, relationships, blast radius | Understanding code, pre-edit safety, refactoring, debugging traces |
| **Ruflo** | Agent orchestration — parallel workers, memory, swarm coordination | Multi-file changes, parallel tasks, long-running work, agent memory |
| **Superpowers** | Disciplined workflows — brainstorming, TDD, debugging, plans | Before any creative/implementation work, process discipline |

## Routing Rules

### Before touching code
1. **Superpowers brainstorming** — if creating features, building components, adding functionality
2. **GitNexus impact** — MUST run `gitnexus_impact({target, direction: "upstream"})` before editing any symbol
3. **GitNexus query** — use `gitnexus_query({query: "concept"})` to understand execution flows (not grep)

### Planning
1. **Superpowers writing-plans** — for multi-step tasks, produces plans in `docs/superpowers/plans/`
2. **Superpowers brainstorming** — explore approaches, produces specs in `docs/superpowers/specs/`
3. **GitNexus context** — `gitnexus_context({name: "symbol"})` for 360-degree view of what you'll change

### Executing
1. **Ruflo agent spawn** — for 2+ independent tasks that can run in parallel
2. **Superpowers executing-plans** — for sequential plan execution with review checkpoints
3. **Superpowers subagent-driven-development** — for independent tasks within current session
4. **Superpowers TDD** — before writing implementation code, write tests first

### Debugging
1. **Superpowers systematic-debugging** — MUST use before proposing any fix
2. **GitNexus query** — `gitnexus_query({query: "<error>"})` to find related execution flows
3. **GitNexus context** — `gitnexus_context({name: "<suspect>"})` for callers/callees
4. **GitNexus detect_changes** — for regressions: `gitnexus_detect_changes({scope: "compare", base_ref: "main"})`

### Reviewing / Finishing
1. **Superpowers verification-before-completion** — MUST run before claiming work is done
2. **Superpowers requesting-code-review** — before merging or creating PRs
3. **GitNexus detect_changes** — `gitnexus_detect_changes({scope: "staged"})` confirms only expected scope changed
4. **Superpowers finishing-a-development-branch** — guides merge/PR/cleanup decision

---

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **Nom** (21510 symbols, 55711 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## When Debugging

1. `gitnexus_query({query: "<error or symptom>"})` — find execution flows related to the issue
2. `gitnexus_context({name: "<suspect function>"})` — see all callers, callees, and process participation
3. `READ gitnexus://repo/Nom/process/{processName}` — trace the full execution flow step by step
4. For regressions: `gitnexus_detect_changes({scope: "compare", base_ref: "main"})` — see what your branch changed

## When Refactoring

- **Renaming**: MUST use `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` first. Review the preview — graph edits are safe, text_search edits need manual review. Then run with `dry_run: false`.
- **Extracting/Splitting**: MUST run `gitnexus_context({name: "target"})` to see all incoming/outgoing refs, then `gitnexus_impact({target: "target", direction: "upstream"})` to find all external callers before moving code.
- After any refactor: run `gitnexus_detect_changes({scope: "all"})` to verify only expected files changed.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Tools Quick Reference

| Tool | When to use | Command |
|------|-------------|---------|
| `query` | Find code by concept | `gitnexus_query({query: "auth validation"})` |
| `context` | 360-degree view of one symbol | `gitnexus_context({name: "validateUser"})` |
| `impact` | Blast radius before editing | `gitnexus_impact({target: "X", direction: "upstream"})` |
| `detect_changes` | Pre-commit scope check | `gitnexus_detect_changes({scope: "staged"})` |
| `rename` | Safe multi-file rename | `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` |
| `cypher` | Custom graph queries | `gitnexus_cypher({query: "MATCH ..."})` |

## Impact Risk Levels

| Depth | Meaning | Action |
|-------|---------|--------|
| d=1 | WILL BREAK — direct callers/importers | MUST update these |
| d=2 | LIKELY AFFECTED — indirect deps | Should test |
| d=3 | MAY NEED TESTING — transitive | Test if critical path |

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/Nom/context` | Codebase overview, check index freshness |
| `gitnexus://repo/Nom/clusters` | All functional areas |
| `gitnexus://repo/Nom/processes` | All execution flows |
| `gitnexus://repo/Nom/process/{name}` | Step-by-step execution trace |

## Self-Check Before Finishing

Before completing any code modification task, verify:
1. `gitnexus_impact` was run for all modified symbols
2. No HIGH/CRITICAL risk warnings were ignored
3. `gitnexus_detect_changes()` confirms changes match expected scope
4. All d=1 (WILL BREAK) dependents were updated

## Keeping the Index Fresh

After committing code changes, the GitNexus index becomes stale. Re-run analyze to update it:

```bash
npx gitnexus analyze
```

If the index previously included embeddings, preserve them by adding `--embeddings`:

```bash
npx gitnexus analyze --embeddings
```

To check whether embeddings exist, inspect `.gitnexus/meta.json` — the `stats.embeddings` field shows the count (0 means no embeddings). **Running analyze without `--embeddings` will delete any previously generated embeddings.**

> Claude Code users: A PostToolUse hook handles this automatically after `git commit` and `git merge`.

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->

---

## Ruflo — Agent Orchestration

Ruflo runs as the `claude-flow` MCP server. Use it for parallel agent work and persistent memory.

### When to Use Ruflo

- **2+ independent implementation tasks** — spawn agents instead of doing sequentially
- **Long-running builds/tests** — agent workers with progress tracking
- **Cross-session memory** — store/retrieve patterns and decisions
- **Swarm coordination** — when multiple agents need to coordinate on shared state

### Core Commands

| Task | Tool |
|------|------|
| Spawn a worker | `agent_spawn({type: "coder", task: "..."})` |
| Check agent status | `agent_status({id})` or `agent_list()` |
| Store a pattern | `memory_store({key, value, type: "pattern"})` |
| Search memory | `memory_search({query: "..."})` |
| Create a task | `task_create({title, description, assignee})` |
| Track progress | `progress_check()` or `task_summary()` |

### Ruflo + GitNexus Integration

Before spawning agents for code changes:
1. Run `gitnexus_impact` on all symbols the agents will touch
2. Include the blast radius in the agent's task description
3. After agents complete, run `gitnexus_detect_changes()` to verify scope

### Ruflo + Superpowers Integration

- Use **superpowers:dispatching-parallel-agents** skill to decide what to parallelize
- Each spawned agent should follow **superpowers:TDD** when writing code
- Use **superpowers:verification-before-completion** before marking any agent task complete

---

## Superpowers — Workflow Discipline

Superpowers are Claude Code skills invoked via the `Skill` tool. They enforce process discipline.

### Mandatory Skills (always use)

| Skill | When | Invoke |
|-------|------|--------|
| `superpowers:brainstorming` | Before ANY creative/implementation work | `Skill("superpowers:brainstorming")` |
| `superpowers:systematic-debugging` | Before proposing ANY bug fix | `Skill("superpowers:systematic-debugging")` |
| `superpowers:verification-before-completion` | Before claiming work is done | `Skill("superpowers:verification-before-completion")` |

### Planning Skills

| Skill | When | Output |
|-------|------|--------|
| `superpowers:writing-plans` | Multi-step task from spec/requirements | `docs/superpowers/plans/<date>-<name>.md` |
| `superpowers:executing-plans` | Executing a written plan | Checkboxes in plan file |
| `superpowers:subagent-driven-development` | Independent tasks in current session | Parallel agent dispatch |

### Implementation Skills

| Skill | When |
|-------|------|
| `superpowers:test-driven-development` | Before writing implementation code |
| `superpowers:dispatching-parallel-agents` | 2+ independent tasks |
| `superpowers:using-git-worktrees` | Feature work needing isolation |

### Review Skills

| Skill | When |
|-------|------|
| `superpowers:requesting-code-review` | Before merging or creating PRs |
| `superpowers:receiving-code-review` | When getting review feedback |
| `superpowers:finishing-a-development-branch` | Implementation complete, deciding how to integrate |

### Superpowers Output Directory

```
docs/superpowers/
├── plans/     # Implementation plans (from writing-plans skill)
├── specs/     # Design specs (from brainstorming skill)
```

---

## Workflow Cheat Sheet

### "Add a new feature"
1. `superpowers:brainstorming` → spec in `docs/superpowers/specs/`
2. `gitnexus_query` → understand related code
3. `superpowers:writing-plans` → plan in `docs/superpowers/plans/`
4. `gitnexus_impact` → blast radius for each symbol to change
5. `superpowers:TDD` → write tests first
6. Implement (use Ruflo agents if 2+ independent tasks)
7. `gitnexus_detect_changes` → verify scope
8. `superpowers:verification-before-completion` → confirm done
9. `superpowers:finishing-a-development-branch` → merge/PR

### "Fix a bug"
1. `superpowers:systematic-debugging` → root cause analysis
2. `gitnexus_query({query: "<error>"})` → find related flows
3. `gitnexus_context({name: "<suspect>"})` → callers/callees
4. `gitnexus_impact` → blast radius of fix
5. `superpowers:TDD` → write failing test, then fix
6. `gitnexus_detect_changes` → verify only expected scope changed
7. `superpowers:verification-before-completion`

### "Understand how X works"
1. `gitnexus_query({query: "X"})` → execution flows
2. `gitnexus_context({name: "X"})` → 360-degree view
3. `READ gitnexus://repo/Nom/process/{name}` → step-by-step trace

### "Refactor X"
1. `gitnexus_context({name: "X"})` → all refs
2. `gitnexus_impact({target: "X", direction: "upstream"})` → blast radius
3. `superpowers:brainstorming` → approach
4. `gitnexus_rename` for renames (never find-and-replace)
5. `gitnexus_detect_changes` → verify scope
6. `superpowers:verification-before-completion`
