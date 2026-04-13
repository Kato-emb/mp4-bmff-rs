#!/usr/bin/env bash
# Build the WASM crate and emit wasm-bindgen glue into ./pkg.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli   # version must match Cargo.lock
#
# Usage:
#   ./build.sh           # debug build
#   ./build.sh release   # release build (smaller wasm, slower compile)

set -euo pipefail

PROFILE="${1:-debug}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
OUT_DIR="$SCRIPT_DIR/pkg"

CARGO_FLAGS=(-p mp4-bmff-wasm --target wasm32-unknown-unknown)
if [[ "$PROFILE" == "release" ]]; then
  CARGO_FLAGS+=(--release)
  WASM_PATH="$WORKSPACE_ROOT/target/wasm32-unknown-unknown/release/mp4_bmff_wasm.wasm"
else
  WASM_PATH="$WORKSPACE_ROOT/target/wasm32-unknown-unknown/debug/mp4_bmff_wasm.wasm"
fi

echo ">>> cargo build ${CARGO_FLAGS[*]}"
( cd "$WORKSPACE_ROOT" && cargo build "${CARGO_FLAGS[@]}" )

echo ">>> wasm-bindgen --target web --out-dir $OUT_DIR"
wasm-bindgen --target web --out-dir "$OUT_DIR" "$WASM_PATH"

echo ">>> done. Serve with: (cd $SCRIPT_DIR && python3 -m http.server 8080)"
