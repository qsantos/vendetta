#!/bin/sh
# Build the browser version into dist/. Serve dist/ with any static file server; see
# serve.py for the caching headers to use.
set -e
cd "$(dirname "$0")"
cargo build --release -p client --target wasm32-unknown-unknown
mkdir -p dist
cp -f target/wasm32-unknown-unknown/release/client.wasm web/favicon.ico dist/
# The bundle's quad_net plugin assigns an undeclared `register_plugin`, which strict mode
# rejects: declare it.
# Its WebGL context is created with the defaults: antialiasing (a multisampled screen to
# resolve every frame, useless for pixel art), transparency (blended with the page) and a depth
# buffer (unused in 2D). Ask for none of them.
attributes='{antialias:false,alpha:false,depth:false,stencil:false,premultipliedAlpha:false}'
sed -e 's/function i(){}register_plugin=function(e){e.env.ws_connect/var register_plugin;function i(){}register_plugin=function(e){e.env.ws_connect/' \
    -e "s/getContext(\"webgl\")/getContext(\"webgl\",$attributes)/; s/getContext(\"webgl2\")/getContext(\"webgl2\",$attributes)/" \
    "$(find ~/.cargo/registry/src -path '*macroquad-0.4.*/js/mq_js_bundle.js' | sort | tail -1)" >dist/mq_js_bundle.js
grep -q 'var register_plugin;' dist/mq_js_bundle.js
grep -q 'getContext("webgl",{antialias' dist/mq_js_bundle.js
# Version the game and the loader by content, so a new build gets new URLs.
wasm=$(sha256sum dist/client.wasm | cut -c1-16)
bundle=$(sha256sum dist/mq_js_bundle.js | cut -c1-16)
sed -e "s|\"client.wasm\"|\"client.wasm?v=$wasm\"|" \
    -e "s|const build = \"dev\";|const build = \"$wasm\";|" \
    -e "s|src=\"mq_js_bundle.js\"|src=\"mq_js_bundle.js?v=$bundle\"|" \
    web/index.html > dist/index.html
grep -q "const build = \"$wasm\";" dist/index.html
ln -sfn ../assets dist/assets
