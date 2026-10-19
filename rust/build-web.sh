#!/bin/sh
# Build the browser version into dist/. Serve dist/ with any static file server.
set -e
cd "$(dirname "$0")"
cargo build --release -p client --target wasm32-unknown-unknown
mkdir -p dist
cp -f target/wasm32-unknown-unknown/release/client.wasm web/index.html dist/
cp -f "$(find ~/.cargo/registry/src -path '*macroquad-0.4.*/js/mq_js_bundle.js' | sort | tail -1)" dist/
ln -sfn ../assets dist/assets
