# agent_demo_vn — Vietnamese locale pack demo

**Bản tiếng Việt của agent_demo.** This is the Vietnamese-keyword form of
the English `examples/agent_demo/`, demonstrating that motivation 02's
locale-pack mandate is shipped: a Vietnamese-primary developer can author
the same agent composition entirely in Vietnamese ASCII transliterations
(`cai ham` for "the function", `khi` for "this works when", etc.).

The dictionary vocabulary (function names, prose contracts) stays in
English by design — only the structural keywords change. This matches
how natural multilingual development works: English remains the lingua
franca for shared APIs, while syntax is locale-friendly.

---

## Pipeline (same as English demo)

```sh
cd nom-compiler
cargo build -p nom-cli
./target/debug/nom store sync examples/agent_demo_vn
./target/debug/nom build status examples/agent_demo_vn
./target/debug/nom build status examples/agent_demo_vn --write-locks
```

After `--write-locks`:

- `agent.nom` is rewritten so each `cai ham <name> khop "..."` becomes
  `cai ham <name>@<64-hex> khop "..."`.
- `policy/safety.nom` is similarly rewritten for its one reference.
- Re-running sync + status confirms all words pinned (still exits 1 due to
  the intentional MECE collision — see below).

---

## What's the same / what's different

| Aspect | English `agent_demo/` | Vietnamese `agent_demo_vn/` |
|---|---|---|
| Tool count | 6 | 6 (giống) |
| Concepts | 2 (agent + safety) | 2 (giống) |
| MECE collision | security + speed | giống — kiểm tra rằng MECE validator chạy với mọi locale |
| Pipeline | sync → status → write-locks | giống |
| Final exit code | 1 (intentional MECE) | 1 (giống) |
| Keywords | English (`the`, `is`, `function`, …) | Vietnamese ASCII (`cai`, `la`, `ham`, …) |
| Word/prose | English | English (by design — shared dictionary) |

---

## Aliases reference

Doc 08 + motivation 02 alias table. Quick lookup:

| Vietnamese ASCII | English | Token |
|---|---|---|
| `cai` | `the` | article |
| `la` | `is` | Is |
| `ham` | `function` | Kind("function") |
| `khai_niem` | `concept` | Kind("concept") |
| `mo_dun` | `module` | Kind("module") |
| `dung` | `uses` | Uses |
| `khop` | `matching` | Matching |
| `bay_ra` | `exposes` | Exposes |
| `khi` | `this works when` | ThisWorksWhen (compressed) |
| `muc_dich` | `intended to` | IntendedTo (compressed) |
| `can` | `requires` | Requires |
| `bao_dam` | `ensures` | Ensures |
| `uu_tien` | `favor` | Favor |
| `roi` | `then` | Then |
| `ket_hop` | `composes` | Composes |
| `voi` | `with` | With |

(See `crates/nom-concept/src/lib.rs` lex module for the full table.)

---

## Why the MECE collision is intentional

`minimal_safe_agent` composes `agent_safety_policy`. Their objectives are:

- `minimal_safe_agent`: security → composability → speed
- `agent_safety_policy`: security → privacy → speed

Both name `security` and both name `speed`. The MECE-ME validator detects
two axis collisions and exits 1. **This is intentional** — it proves the
validator runs regardless of which locale the source was authored in. The
MECE check is keyword-agnostic: it sees the parsed AST, not the surface
syntax.

To make this demo build clean, change one duplicate objective
(e.g., rename `speed` to `latency` in safety.nom). This demo intentionally
leaves the collision in place.

---

## Sample: same concept in both locales

**English (`agent_demo/policy/safety.nom`)**:
```
the concept agent_safety_policy is
  intended to constrain what an agent may do during one task.

  uses the function read_file matching "read text from a workspace path".

  this works when the agent never writes outside the workspace root.
  this works when every web fetch uses https.

  favor security then privacy then speed.
```

**Vietnamese (`agent_demo_vn/policy/safety.nom`)**:
```
cai khai_niem agent_safety_policy muc_dich constrain what an agent may do during one task.

dung cai ham read_file khop "read text from a workspace path".

khi the agent never writes outside the workspace root.
khi every web fetch uses https.

uu_tien security roi privacy roi speed.
```

The token stream produced by the lexer is identical for both. The parser
sees the same AST. The concept name, prose, and contracts are unchanged.
Only the keywords differ.

---

## Files

```
agent_demo_vn/
  agent.nom               root concept: minimal_safe_agent (VN keywords)
  policy/
    safety.nom            concept: agent_safety_policy (VN keywords)
  tools/
    file_tools.nomtu      3 entities: read_file, write_file, list_dir (VN keywords)
    web_tools.nomtu       2 entities: fetch_url, search_web (VN keywords)
    shell_tools.nomtu     1 entity:  run_command (VN keywords)
```

End-to-end test: `crates/nom-cli/tests/agent_demo_vn_e2e.rs`.
