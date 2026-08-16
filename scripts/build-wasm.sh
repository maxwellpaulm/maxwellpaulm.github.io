#!/bin/bash
# Builds each wasm demo crate and emits its JS bindings.
# Run before `cargo run -p site` so the generator can copy the artifacts.
set -euo pipefail

# Builds $1 (a crate name) to wasm32-unknown-unknown and runs wasm-bindgen
# over the result, writing artifacts to $2. $3 is the crate name with
# hyphens replaced by underscores, matching cargo's build output filename.
build_wasm_crate() {
  local crate=$1
  local out=$2
  local module=$3

  cargo build -p "$crate" --target wasm32-unknown-unknown --release

  mkdir -p "$out"
  wasm-bindgen \
    --target web \
    --no-typescript \
    --out-dir "$out" \
    "target/wasm32-unknown-unknown/release/${module}.wasm"

  echo "wasm artifacts in $out:"
  ls -la "$out"
}

build_wasm_crate reaction-diffusion static/demos/reaction-diffusion reaction_diffusion
build_wasm_crate aho-corasick-demo static/demos/aho-corasick aho_corasick_demo
# Parked with the ask terminal (see Route::ALL) so the site doesn't ship an
# unreachable wasm binary. The crate still builds and tests via `cargo test`.
# build_wasm_crate ask-terminal static/ask ask_terminal
