#!/usr/bin/env bash
# Builds the gallery for the browser into site/public/wasm.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="$root/site/public/wasm"

cargo build --manifest-path "$root/Cargo.toml" -p gallery --bin gallery \
  --target wasm32-unknown-unknown --profile release-wasm

wasm-bindgen "$root/target/wasm32-unknown-unknown/release-wasm/gallery.wasm" \
  --out-dir "$out" --target web --no-typescript

if [ -x "$root/site/node_modules/.bin/wasm-opt" ]; then
  "$root/site/node_modules/.bin/wasm-opt" -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
    "$out/gallery_bg.wasm" -o "$out/gallery_bg.wasm"
fi

ls -l "$out"
