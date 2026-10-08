#!/usr/bin/env bash
# Build the Rust core to WASM and embed it as base64 in core.wasm.js
# (so index.html also works when opened directly from disk).
set -euo pipefail
cd "$(dirname "$0")"
rustup target add wasm32-unknown-unknown >/dev/null
cargo build --release --manifest-path core/Cargo.toml --target wasm32-unknown-unknown
WASM=core/target/wasm32-unknown-unknown/release/abfindung_core.wasm
{ printf 'window.CORE_WASM_B64="'; base64 -w0 "$WASM"; printf '";\n'; } > core.wasm.js
echo "wrote core.wasm.js ($(stat -c %s "$WASM") bytes wasm)"
