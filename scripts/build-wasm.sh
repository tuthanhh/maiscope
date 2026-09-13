#!/usr/bin/env bash
# Build the Bevy engine to wasm and emit JS bindings the host app imports.
#
# Output lands in apps/host/src/wasm/ (gitignored). The host loads it lazily
# from the visualizer page: `import init, { load_song } from '~/wasm/maiscope_viewer.js'`.
#
# wasm-bindgen CLI version MUST match the wasm-bindgen crate version in
# engine/Cargo.toml exactly, or you'll get a "schema version" mismatch at bindgen time.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ENGINE_DIR="$WORKSPACE_DIR/engine"
OUT_DIR="$WORKSPACE_DIR/apps/host/src/wasm"
PROFILE="${1:-dev}" # dev (fast) | release (size-optimized, slow)

# NOTE: cargo resolves engine against the root workspace, so build artifacts
# land in the workspace target/ dir, not engine/target/.
TARGET_ROOT="$WORKSPACE_DIR/target/wasm32-unknown-unknown"

if [ "$PROFILE" = "release" ]; then
  # `wasm-release` (workspace root manifest) inherits release and adds
  # opt-level=z, fat LTO, strip and panic=abort. Cargo puts custom-profile output
  # in target/<triple>/<profile-name>/, not release/.
  CARGO_FLAGS="--profile wasm-release"
  TARGET_SUBDIR="wasm-release"
else
  CARGO_FLAGS=""
  TARGET_SUBDIR="debug"
fi

echo ">> cargo build ($PROFILE) -> wasm32-unknown-unknown"
( cd "$ENGINE_DIR" && cargo build $CARGO_FLAGS --target wasm32-unknown-unknown )

WASM_IN="$TARGET_ROOT/$TARGET_SUBDIR/maiscope_viewer.wasm"

echo ">> wasm-bindgen -> $OUT_DIR"
mkdir -p "$OUT_DIR"
wasm-bindgen "$WASM_IN" --out-dir "$OUT_DIR" --target web

# Sprite textures are loaded at runtime by Bevy's AssetServer, which on wasm
# fetches them from "/assets/..." (see ASSET_ROOT in engine/src/plugins/mod.rs).
# Vite serves apps/host/public/ at the origin root, so mirror the sprites
# there. (Charts + audio are NOT copied — they're pushed in as bytes via
# wasm_bridge; only sprites go through the file-fetching AssetServer.)
ASSETS_OUT="$WORKSPACE_DIR/apps/host/public/assets"
echo ">> sync sprites -> $ASSETS_OUT/sprites"
rm -rf "$ASSETS_OUT/sprites"
mkdir -p "$ASSETS_OUT"
cp -R "$ENGINE_DIR/assets/sprites" "$ASSETS_OUT/sprites"

echo ">> done. host imports from ~/wasm/maiscope_viewer.js"
