#!/usr/bin/env bash
# Build nom-canvas for WASM target
set -e
cargo build -p nom-canvas-core --target wasm32-unknown-unknown --features wasm --no-default-features
echo "WASM build complete: nom-canvas-core"
