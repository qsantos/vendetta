#!/bin/sh
# Build the browser version into dist/. Serve dist/ with any static file server; see
# serve.py for the caching headers to use. Needs ImageMagick, and Python with Pillow, NumPy,
# SciPy, fontTools and the Noto Sans and Noto Sans CJK fonts (for the translations and their
# loading screens).
set -e
cd "$(dirname "$0")"
cargo build --release -p client --target wasm32-unknown-unknown
mkdir -p dist
cp -f target/wasm32-unknown-unknown/release/client.wasm web/favicon.ico dist/
# The bundle's quad_net plugin assigns an undeclared `register_plugin`, which strict mode
# rejects: declare it.
# A sound the browser cannot decode never counts as loaded, and `load_sound_from_bytes` waits
# for it forever: make it a silent one instead, and say its size, to tell which file it was
# (once: the promise decodeAudioData also returns fails too).
# Its WebGL context is created with the defaults: antialiasing (a multisampled screen to
# resolve every frame, useless for pixel art), transparency (blended with the page) and a depth
# buffer (unused in 2D). Ask for none of them.
attributes='{antialias:false,alpha:false,depth:false,stencil:false,premultipliedAlpha:false}'
sed -e 's/function i(){}register_plugin=function(e){e.env.ws_connect/var register_plugin;function i(){}register_plugin=function(e){e.env.ws_connect/' \
    -e 's/function(e){console.error("Failed to decode audio buffer",e)})/function(x){console.error("Failed to decode audio buffer ("+s+" octets)",x),n.set(i,e.createBuffer(1,1,e.sampleRate))}).catch(function(){})/' \
    -e "s/getContext(\"webgl\")/getContext(\"webgl\",$attributes)/; s/getContext(\"webgl2\")/getContext(\"webgl2\",$attributes)/" \
    "$(find ~/.cargo/registry/src -path '*macroquad-0.4.*/js/mq_js_bundle.js' | sort | tail -1)" >dist/mq_js_bundle.js
grep -q 'var register_plugin;' dist/mq_js_bundle.js
grep -q 'n.set(i,e.createBuffer(1,1,e.sampleRate))' dist/mq_js_bundle.js
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
# The translations beyond the original's French and English (see tools/translate.py).
python3 -I tools/translate.py apply assets
# The files the game loads as it starts, in a few bundles (see bundle.py), and their index,
# versioned like the game.
python3 bundle.py assets dist
index=$(sha256sum dist/bundles.json | cut -c1-16)
sed -i "s|\"bundles.json\"|\"bundles.json?v=$index\"|" dist/index.html
grep -q "bundles.json?v=$index" dist/index.html
# The loading screen's artworks, as lossy WebP (about a tenth of the PNGs, the first thing
# shown; the transparency of their black, which shows the bar through their bottom band, kept
# exact), for the page to pick one at once. In the other languages of `Langues.ini`, their
# "Chargement en cours" is redrawn (`<artwork>-<suffix>.png`, see tools/loading_text.py).
python3 -I tools/loading_text.py build assets/Donnees/Langues.ini assets/Images/Chargement target/chargement
mkdir -p dist/art
arts=
for png in assets/Images/Chargement/*.png target/chargement/*.png; do
    name=$(basename "$png" .png)
    case $name in *-??) lang=${name##*-} ;; *) lang=FR ;; esac
    webp="dist/art/$name.webp"
    [ "$webp" -nt "$png" ] || magick "$png" -quality 82 -define webp:method=6 -define webp:alpha-quality=100 "$webp"
    url="art/$name.webp?v=$(sha256sum "$webp" | cut -c1-16)"
    arts="$arts${arts:+,}[\"$lang\",\"$(echo "$url" | sed 's/ /%20/g')\"]"
done
sed -i "s|const arts = \[/\* build-web.sh \*/\];|const arts = [$arts];|" dist/index.html
grep -q 'const arts = \[\["FR","art/' dist/index.html
