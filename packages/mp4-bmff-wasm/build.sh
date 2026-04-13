#!/usr/bin/env bash
# Build the WASM crate twice (bundler + web targets) and emit wasm-bindgen
# glue into ./dist (bundler target, primary export) and ./dist-web (web
# target, secondary export reachable via `import "@kato-emb/mp4-bmff-wasm/web"`).
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli   # version must match Cargo.lock
#
# Usage:
#   ./build.sh           # release build (default; what `npm pack` runs)
#   ./build.sh debug     # debug build (faster compile, larger wasm)
#   ./build.sh release   # explicit release build

set -euo pipefail

PROFILE="${1:-release}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

CARGO_FLAGS=(-p mp4-bmff-wasm --target wasm32-unknown-unknown)
case "$PROFILE" in
  release)
    CARGO_FLAGS+=(--release)
    WASM_PATH="$WORKSPACE_ROOT/target/wasm32-unknown-unknown/release/mp4_bmff_wasm.wasm"
    ;;
  debug)
    WASM_PATH="$WORKSPACE_ROOT/target/wasm32-unknown-unknown/debug/mp4_bmff_wasm.wasm"
    ;;
  *)
    echo "usage: $0 [debug|release]" >&2
    exit 1
    ;;
esac

echo ">>> cargo build ${CARGO_FLAGS[*]}"
( cd "$WORKSPACE_ROOT" && cargo build "${CARGO_FLAGS[@]}" )

build_one() {
  local target="$1"
  local out="$2"
  echo ">>> wasm-bindgen --target $target --out-dir $out"
  rm -rf "$out"
  wasm-bindgen --target "$target" --out-dir "$out" "$WASM_PATH"
  # wasm-bindgen also emits its own package.json, .gitignore, and (for some
  # targets) a README.md. The `files` field of our manifest already lists
  # exactly what we ship, so these are harmless — but removing them keeps
  # the directory tidy and avoids confusion when inspecting the package.
  rm -f "$out/package.json" "$out/.gitignore" "$out/README.md"
}

build_one bundler "$SCRIPT_DIR/dist"
build_one web     "$SCRIPT_DIR/dist-web"

echo ">>> done"
echo ">>> bundler: $SCRIPT_DIR/dist"
echo ">>> web:     $SCRIPT_DIR/dist-web"
echo ""
echo "Inspect what npm would publish:"
echo "  (cd $SCRIPT_DIR && npm pack --dry-run)"
