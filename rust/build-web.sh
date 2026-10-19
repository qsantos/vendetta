#!/bin/sh
# Build the browser version into dist/. Serve dist/ with any static file server; see
# serve.py for the caching headers to use.
set -e
cd "$(dirname "$0")"
cargo build --release -p client --target wasm32-unknown-unknown
mkdir -p dist
cp -f target/wasm32-unknown-unknown/release/client.wasm web/favicon.ico dist/
cp -f "$(find ~/.cargo/registry/src -path '*macroquad-0.4.*/js/mq_js_bundle.js' | sort | tail -1)" dist/
# Version the game and the loader by content, so a new build gets new URLs.
wasm=$(sha256sum dist/client.wasm | cut -c1-16)
bundle=$(sha256sum dist/mq_js_bundle.js | cut -c1-16)
sed -e "s|\"client.wasm\"|\"client.wasm?v=$wasm\"|" \
    -e "s|src=\"mq_js_bundle.js\"|src=\"mq_js_bundle.js?v=$bundle\"|" \
    web/index.html > dist/index.html
ln -sfn ../assets dist/assets
