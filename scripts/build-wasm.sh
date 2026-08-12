#!/bin/bash
# Builds the reaction-diffusion demo to WebAssembly and emits the JS bindings.
# Run before `cargo run -p site` so the generator can copy the artifacts.
set -euo pipefail

OUT=static/demos/reaction-diffusion

cargo build -p reaction-diffusion --target wasm32-unknown-unknown --release

mkdir -p "$OUT"
wasm-bindgen \
  --target web \
  --no-typescript \
  --out-dir "$OUT" \
  target/wasm32-unknown-unknown/release/reaction_diffusion.wasm

echo "wasm artifacts in $OUT:"
ls -la "$OUT"
