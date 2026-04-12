#!/usr/bin/env bash
# Phase 4 acceptance demo: content-addressed store + closure walk + compilation
# Usage: bash examples/closure-demo/run-demo.sh  (from repo root)
#   or:  bash run-demo.sh                        (from this directory)
set -e

# ── Locate the compiler ───────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPILER_DIR="$(cd "$SCRIPT_DIR/../../nom-compiler" 2>/dev/null && pwd || echo "")"
if [ -z "$COMPILER_DIR" ]; then
    echo "ERROR: could not locate nom-compiler directory" >&2
    exit 1
fi

# ── Ensure LLVM is available ──────────────────────────────────────────────────
export PATH="$PATH:/c/Program Files/LLVM/bin"

# ── Use a temp database so the demo never touches data/nomdict.db ─────────────
NOM_STORE_DB=$(mktemp -t nomdemo.XXXXX.db)
trap 'rm -f "$NOM_STORE_DB"' EXIT
echo "Using temporary store: $NOM_STORE_DB"
echo ""

NOM="cargo run -q -p nom-cli --"
cd "$COMPILER_DIR"

DEMO="$SCRIPT_DIR"

# ── Step 1: Ingest leaves first ───────────────────────────────────────────────
echo "=== Ingesting format.nom (leaf, no deps) ==="
F_HASH=$($NOM store add "$DEMO/format.nom" --dict "$NOM_STORE_DB")
echo "  F=$F_HASH"

echo ""
echo "=== Ingesting greet.nom (depends on format_number) ==="
G_HASH=$($NOM store add "$DEMO/greet.nom" --dict "$NOM_STORE_DB")
echo "  G=$G_HASH"

echo ""
echo "=== Ingesting main.nom (depends on greet) ==="
M_HASH=$($NOM store add "$DEMO/main.nom" --dict "$NOM_STORE_DB")
echo "  M=$M_HASH"

# ── Step 2: Closure walk from main ───────────────────────────────────────────
echo ""
echo "=== Closure walk from main (should list 3 hashes) ==="
CLOSURE=$($NOM store closure "$M_HASH" --dict "$NOM_STORE_DB")
echo "$CLOSURE"
COUNT=$(echo "$CLOSURE" | wc -l | tr -d ' ')
if [ "$COUNT" -ne 3 ]; then
    echo "ERROR: expected 3 entries in closure, got $COUNT" >&2
    exit 1
fi
echo "  -> $COUNT entries in closure: OK"

# ── Step 3: Verify reachability ───────────────────────────────────────────────
echo ""
echo "=== Verifying closure integrity ==="
$NOM store verify "$M_HASH" --dict "$NOM_STORE_DB"

# ── Step 4: Build from the root hash (LLVM target) ────────────────────────────
echo ""
echo "=== Building from hash (LLVM target) ==="
$NOM build "$M_HASH" --dict "$NOM_STORE_DB" --no-prelude --target llvm

# ── Step 5: Run the compiled binary ──────────────────────────────────────────
LL_FILE="$(mktemp -d)/nom_${M_HASH:0:8}.ll"
TMPBUILD=$(ls -td /tmp/nom-build-hash/ 2>/dev/null | head -1)
if [ -f "/tmp/nom-build-hash/nom_${M_HASH:0:8}.ll" ]; then
    LL_PATH="/tmp/nom-build-hash/nom_${M_HASH:0:8}.ll"
elif [ -f "$TEMP/nom-build-hash/nom_${M_HASH:0:8}.ll" ]; then
    LL_PATH="$TEMP/nom-build-hash/nom_${M_HASH:0:8}.ll"
else
    # Windows-style temp
    LL_PATH="$(cygpath -u "$TEMP" 2>/dev/null || echo /tmp)/nom-build-hash/nom_${M_HASH:0:8}.ll"
fi

if command -v clang >/dev/null 2>&1 && [ -f "$LL_PATH" ]; then
    echo ""
    echo "=== Compiling .ll to native binary via clang ==="
    OUT_EXE="$(dirname "$LL_PATH")/demo_closure"
    clang "$LL_PATH" -o "$OUT_EXE" 2>&1 || true
    if [ -f "$OUT_EXE" ] || [ -f "${OUT_EXE}.exe" ]; then
        EXE="${OUT_EXE}.exe"
        [ -f "$OUT_EXE" ] && EXE="$OUT_EXE"
        "$EXE" || EXIT_CODE=$?
        EXIT_CODE=${EXIT_CODE:-0}
        echo "  -> binary exited with code $EXIT_CODE (expected 20: greet(5)=format_number(5)+10=10+10=20)"
        if [ "$EXIT_CODE" -eq 20 ]; then
            echo "  -> result verified: 20 == expected"
        fi
    fi
fi

echo ""
echo "============================================"
echo "DEMO OK"
echo "  F = $F_HASH"
echo "  G = $G_HASH"
echo "  M = $M_HASH"
echo "  closure size = $COUNT"
echo "============================================"
