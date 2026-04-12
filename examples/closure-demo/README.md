# Phase 4 Closure Demo

This directory is the acceptance milestone for **Phase 4 (DIDS — Distributed
Immutable Dictionary Store)**. It proves the three core claims of the
content-addressed architecture:

1. **Content-addressed storage** — every `.nom` declaration is hashed by its
   canonical AST. The same source always produces the same 64-hex-char id.
   `nom store add` is idempotent.

2. **Transitive closure walking** — `nom store closure <hash>` traverses
   `entry_refs` edges (populated from `use` statements at ingest time) and
   returns all transitive dependencies as a flat, deterministic list of ids.
   No dep-tree manifest is needed; the closure is the manifest.

3. **Hash-to-executable compilation** — `nom build <hash>` resolves the
   closure, concatenates body sources in dependency order, and compiles the
   result to LLVM IR and a native binary. The hash alone is the complete
   reproducible build spec.

## Files

| File | Role |
|------|------|
| `format.nom` | Leaf: `format_number(n) = n * 2` |
| `greet.nom`  | Depends on `format_number` via bare `use` |
| `main.nom`   | Depends on `greet`; entry point returns `greet(5) = 20` |

## Expected result

`main()` returns `greet(5) = format_number(5) + 10 = 10 + 10 = 20`, which
becomes the process exit code when the binary is run.

## Running the demo

```bash
export PATH="$PATH:/c/Program Files/LLVM/bin"
bash examples/closure-demo/run-demo.sh
```

The script uses a temporary SQLite file so it never touches `data/nomdict.db`.

## What it proves

> "Apps are hash closures, not dep trees."

Dependency resolution happens once at `store add` time and is recorded as
content-addressed graph edges. A downstream consumer only needs a single
root hash to reproduce the exact same closure — no version ranges, no
lockfiles, no network at build time.
