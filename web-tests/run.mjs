// Rendering tests: plays scenes of the browser build in headless Chrome and compares their
// screenshots with references in `golden/`.
//
//   node web-tests/run.mjs [--update] [scene...]     (after ./build-web.sh, from the top directory)
//
// Each scene loads the game with `#test&…&play` (a still world, no storage, no FPS counter; see
// client/src/options.rs) and a fixed seed, then moves, clicks, presses keys. A scene passes when
// at most 0.1 % of its pixels differ by more than 24 in a channel. Failures write `out/<scene>.png`
// and `out/<scene>-diff.png` (differences in red). `--update` rewrites the references.
// Chrome is `$CHROME`, else google-chrome or chromium; it renders with SwiftShade so that
// results do not depend on the GPU. Needs Node 22+ (built-in WebSocket).

import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { deflateSync, inflateSync } from "node:zlib";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const SERVER_PORT = 8799;
const CHROME_PORT = 9399;
const [W, H] = [1024, 768];

// Steps: ["move", x, y], ["left", x, y], ["right", x, y], ["key", code], ["wheel", x, y, dy],
// ["wait", seconds], ["drag", x0, y0, x1, y1] (the left button); with a finger: ["tap", x, y], ["press", x, y] (kept down for the
// screenshot), ["swipe", x0, y0, x1, y1].
// A scene with `phone: [w, h]` plays on a touch screen of that size (the touch interface).
const SCENES = {
  // The game as it starts: the player in the castle, its trade window open.
  start: { hash: "seed=1", steps: [["move", 512, 600]] },
  inventory: { hash: "seed=1", steps: [["key", "KeyI"], ["move", 512, 600]] },
  resources: { hash: "seed=1", steps: [["key", "KeyR"], ["move", 512, 600]] },
  character: { hash: "seed=1", steps: [["key", "KeyP"], ["move", 250, 270], ["wait", 0.3]] },
  minimap: { hash: "seed=3&size=256&fiefs=6&villagers=20", steps: [["key", "Tab"], ["move", 600, 740]] },
  // The skills window, every skill shown (its 4th sort button).
  skills: { hash: "seed=1", steps: [["key", "KeyT"], ["left", 222, 437], ["move", 512, 740]] },
  bestiary: { hash: "seed=1", steps: [["key", "KeyU"], ["move", 512, 740]] },
  // The build buttons slid out, with a building's tooltip.
  build_tooltip: { hash: "seed=1", steps: [["move", 2, 265], ["wait", 1], ["move", 14, 290], ["wait", 0.5]] },
  // The radial buttons around a right click on the ground.
  radial: { hash: "seed=1", steps: [["right", 500, 600], ["move", 520, 560]] },
  // Zoomed out on a large map: the terrain overview.
  overview: { hash: "seed=2&size=512", steps: [["move", 500, 500], ...Array(6).fill(["wheel", 500, 500, 120]), ["wait", 0.5]] },
  // Many fiefs: the fief list keeps to 16 lines.
  // Temporary effect icons right of the shortcut slots: one fading in its last seconds, one
  // that never expires (no bar), and the tooltip of the first one.
  effects: { hash: "seed=1&effects=148+149:3+112", steps: [["move", 360, 16]] },
  // Recharging objects on the shortcut slots: their clocks.
  recharges: { hash: "seed=1&recharges=40:5+41:25+42:3", steps: [["move", 512, 600]] },
  // The epoch box while changing epoch, and the emperor's life bar under it.
  epoch: { hash: "seed=1&emperor&advance=40", steps: [["move", 512, 600]] },
  // The message log: a tip (F9).
  messages: { hash: "seed=1", steps: [["key", "F9"], ["move", 512, 600]] },
  // Holding an object over the own castle: the sell cursor and its bubble.
  sell: { hash: "seed=1&hold=0", steps: [["move", 330, 312]] },
  // Several human players: Page Down controls the one of the third fief (its own view, bars
  // and house).
  switch: { hash: "seed=1&players=1+3", steps: [["key", "PageDown"], ["move", 512, 600]] },
  // The bag shown is the controlled character's: the second player's is empty…
  switch_inventory: { hash: "seed=1&players=1+3&inventory=Hache+Marteau:2+40:3", steps: [["key", "KeyI"], ["key", "PageDown"], ["move", 512, 600]] },
  // …and Page Up comes back to the first player and its filled bag.
  switch_back: { hash: "seed=1&players=1+3&inventory=Hache+Marteau:2+40:3", steps: [["key", "KeyI"], ["key", "PageDown"], ["key", "PageUp"], ["move", 512, 600]] },
  // The map dragged away from the character: the button to follow it again, and the arrow
  // pointing at it.
  pan: { hash: "seed=1", steps: [["drag", 300, 650, 900, 150], ["move", 512, 400]] },
  // Space follows the character again.
  pan_back: { hash: "seed=1", steps: [["drag", 300, 650, 900, 150], ["key", "Space"], ["move", 512, 600]] },
  many_fiefs: { hash: "seed=3&size=256&fiefs=30&villagers=2", steps: [["move", 512, 600]] },
  // The touch interface on a phone held sideways: the bar on the right, the castle's window.
  touch_start: { hash: "seed=1&layout=touch", phone: [800, 360], steps: [] },
  // …held upright: the bar at the bottom.
  touch_portrait: { hash: "seed=1&layout=touch", phone: [360, 740], steps: [] },
  // The bar's panels, each entry with what its tooltip tells first.
  touch_build: { hash: "seed=1&layout=touch", phone: [800, 360], steps: [["tap", 770, 30]] },
  touch_move: { hash: "seed=1&layout=touch", phone: [800, 360], steps: [["tap", 770, 88]] },
  touch_windows: { hash: "seed=1&layout=touch", phone: [800, 360], steps: [["tap", 770, 146]] },
  // The build panel scrolled, a building out of reach tapped: its tooltip says why.
  touch_build_scroll: { hash: "seed=1&layout=touch", phone: [800, 360], steps: [["tap", 770, 30], ["swipe", 400, 300, 400, 100], ["tap", 300, 200]] },
  // A building being placed: dragged on the map, its tooltip at the top, buttons beside it
  // to confirm or cancel.
  touch_placing: { hash: "seed=1&layout=touch&god", phone: [800, 360], steps: [["tap", 770, 30], ["tap", 290, 30], ["swipe", 300, 250, 250, 200]] },
  // A finger held on the map: what a tap would do, above it.
  touch_press: { hash: "seed=1&layout=touch", phone: [800, 360], steps: [["tap", 490, 40], ["press", 370, 190]] },
  // Upright, the panels open above the bar.
  touch_portrait_move: { hash: "seed=1&layout=touch", phone: [360, 740], steps: [["tap", 92, 700]] },
  // The message log and temporary effects at the left edge (no trays there).
  touch_messages: { hash: "seed=1&layout=touch&effects=148+149:3", phone: [800, 360], steps: [["tap", 490, 40], ["key", "F9"]] },
  // Eating automatically: its button right of the bars, its panel, Vie turned on and Energie's
  // threshold raised.
  touch_auto: { hash: "seed=1&layout=touch", phone: [800, 360], steps: [["tap", 490, 40], ["tap", 147, 338], ["tap", 140, 160], ["tap", 270, 192]] },
  // The bar's Menu button, at its far end: the in-game menu, the solo game paused behind it.
  touch_menu: { hash: "seed=1&layout=touch", phone: [800, 360], steps: [["tap", 770, 315]] },
  // The map window (Fenêtres, then Carte); a finger dragging on the minimap moves the view.
  touch_minimap: { hash: "seed=3&size=256&fiefs=6&villagers=20&layout=touch", phone: [800, 360], steps: [["tap", 770, 146], ["tap", 430, 75], ["swipe", 330, 150, 360, 250]] },
  // A finger dragging the map: the same.
  touch_pan: { hash: "seed=1&layout=touch", phone: [800, 360], steps: [["tap", 490, 40], ["swipe", 150, 300, 700, 60]] },
  // …and Suivre follows the character again.
  touch_pan_back: { hash: "seed=1&layout=touch", phone: [800, 360], steps: [["tap", 490, 40], ["swipe", 150, 300, 700, 60], ["tap", 333, 50]] },
  // Lists scroll under a dragging finger.
  touch_scroll: { hash: "seed=1&layout=touch", phone: [800, 360], steps: [["swipe", 300, 300, 300, 150]] },
};

/// Errors the page reported, shown when a scene cannot load.
const errors = [];
const sleep = (s) => new Promise((r) => setTimeout(r, s * 1000));

function chromePath() {
  if (process.env.CHROME) return process.env.CHROME;
  for (const p of ["/usr/bin/google-chrome", "/usr/bin/chromium", "/usr/bin/chromium-browser"]) {
    if (existsSync(p)) return p;
  }
  throw new Error("no Chrome found: set $CHROME");
}

// ---------------------------------------------------------------- PNG (8-bit RGB/RGBA only)

function decodePng(buf) {
  let pos = 8, width, height, type;
  const idat = [];
  while (pos < buf.length) {
    const len = buf.readUInt32BE(pos);
    const kind = buf.toString("ascii", pos + 4, pos + 8);
    const data = buf.subarray(pos + 8, pos + 8 + len);
    if (kind === "IHDR") {
      [width, height, type] = [data.readUInt32BE(0), data.readUInt32BE(4), data[9]];
      if (data[8] !== 8 || (type !== 2 && type !== 6) || data[12] !== 0) throw new Error("unsupported PNG");
    } else if (kind === "IDAT") idat.push(data);
    pos += 12 + len;
  }
  const bpp = type === 6 ? 4 : 3;
  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * bpp;
  const out = Buffer.alloc(width * height * 4);
  let prev = Buffer.alloc(stride);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    const line = Buffer.from(raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1)));
    for (let i = 0; i < stride; i++) {
      const a = i >= bpp ? line[i - bpp] : 0, b = prev[i], c = i >= bpp ? prev[i - bpp] : 0;
      let p = 0;
      if (filter === 1) p = a;
      else if (filter === 2) p = b;
      else if (filter === 3) p = (a + b) >> 1;
      else if (filter === 4) {
        const pa = Math.abs(b - c), pb = Math.abs(a - c), pc = Math.abs(a + b - 2 * c);
        p = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      }
      line[i] = (line[i] + p) & 255;
    }
    for (let x = 0; x < width; x++) {
      for (let k = 0; k < 4; k++) out[(y * width + x) * 4 + k] = k < bpp ? line[x * bpp + k] : 255;
    }
    prev = line;
  }
  return { width, height, data: out };
}

function encodePng({ width, height, data }) {
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
  });
  const crc = (b) => {
    let c = 0xffffffff;
    for (const x of b) c = crcTable[(c ^ x) & 255] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  };
  const chunk = (kind, body) => {
    const head = Buffer.alloc(8);
    head.writeUInt32BE(body.length, 0);
    head.write(kind, 4, "ascii");
    const tail = Buffer.alloc(4);
    tail.writeUInt32BE(crc(Buffer.concat([head.subarray(4), body])), 0);
    return Buffer.concat([head, body, tail]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr.set([8, 6, 0, 0, 0], 8);
  const rows = Buffer.alloc(height * (width * 4 + 1));
  for (let y = 0; y < height; y++) data.copy(rows, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4);
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  return Buffer.concat([sig, chunk("IHDR", ihdr), chunk("IDAT", deflateSync(rows)), chunk("IEND", Buffer.alloc(0))]);
}

/// Pixels differing by more than 24 in a channel, and an image of them in red over a faded copy.
function compare(a, b) {
  if (a.width !== b.width || a.height !== b.height) return { differing: Infinity };
  const diff = Buffer.alloc(a.data.length);
  let differing = 0;
  for (let i = 0; i < a.data.length; i += 4) {
    const off = [0, 1, 2].some((k) => Math.abs(a.data[i + k] - b.data[i + k]) > 24);
    differing += off;
    const grey = (a.data[i] + a.data[i + 1] + a.data[i + 2]) / 12 + 128;
    diff.set(off ? [255, 0, 0, 255] : [grey, grey, grey, 255], i);
  }
  return { differing, diff: { width: a.width, height: a.height, data: diff } };
}

// ---------------------------------------------------------------- Chrome

async function connect() {
  for (let i = 0; i < 100; i++) {
    try {
      const targets = await (await fetch(`http://127.0.0.1:${CHROME_PORT}/json`)).json();
      const page = targets.find((t) => t.type === "page");
      if (page) {
        const ws = new WebSocket(page.webSocketDebuggerUrl);
        await new Promise((r, e) => ((ws.onopen = r), (ws.onerror = e)));
        let id = 0;
        const pending = new Map();
        ws.onmessage = (ev) => {
          const m = JSON.parse(ev.data);
          if (m.id && pending.has(m.id)) pending.get(m.id)(m), pending.delete(m.id);
          if (m.method === "Runtime.exceptionThrown") errors.push(m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text);
          if (m.method === "Runtime.consoleAPICalled" && m.params.type === "error") errors.push(m.params.args.map((a) => a.value ?? a.description).join(" "));
        };
        const send = (method, params = {}) =>
          new Promise((r) => {
            pending.set(++id, r);
            ws.send(JSON.stringify({ id, method, params }));
          });
        return { ws, send };
      }
    } catch {}
    await sleep(0.1);
  }
  throw new Error("Chrome did not start");
}

async function play(send, scene) {
  await send("Page.navigate", { url: "about:blank" });
  await send("Page.navigate", { url: `http://127.0.0.1:${SERVER_PORT}/#test&${scene.hash}&play` });
  const evaluate = async (expression) => (await send("Runtime.evaluate", { expression, returnByValue: true })).result?.result?.value;
  for (let i = 0; ; i++) {
    if (i > 600) throw new Error(`the game did not load: ${await evaluate("document.body.innerText.slice(0, 300)")} ${errors.join("; ")}`);
    await sleep(0.1);
    if ((await evaluate("document.readyState === 'complete' && !document.getElementById('loading')")) === true) break;
  }
  await sleep(1);
  const mouse = (type, x, y, extra = {}) => send("Input.dispatchMouseEvent", { type, x, y, ...extra });
  const finger = (type, points) => send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([x, y]) => ({ x, y })) });
  for (const [kind, ...a] of scene.steps) {
    if (kind === "move") await mouse("mouseMoved", a[0], a[1]);
    else if (kind === "left" || kind === "right") {
      const button = kind;
      await mouse("mouseMoved", a[0], a[1]);
      await mouse("mousePressed", a[0], a[1], { button, clickCount: 1 });
      await sleep(0.1);
      await mouse("mouseReleased", a[0], a[1], { button, clickCount: 1 });
    } else if (kind === "key") {
      await send("Input.dispatchKeyEvent", { type: "rawKeyDown", code: a[0], key: a[0].replace(/^Key/, "").toLowerCase() });
      await sleep(0.1);
      await send("Input.dispatchKeyEvent", { type: "keyUp", code: a[0], key: a[0].replace(/^Key/, "").toLowerCase() });
    } else if (kind === "wheel") await mouse("mouseWheel", a[0], a[1], { deltaX: 0, deltaY: a[2] });
    else if (kind === "wait") await sleep(a[0]);
    else if (kind === "drag") {
      await mouse("mouseMoved", a[0], a[1]);
      await mouse("mousePressed", a[0], a[1], { button: "left", clickCount: 1 });
      for (let k = 1; k <= 10; k++) {
        await sleep(0.03);
        await mouse("mouseMoved", a[0] + ((a[2] - a[0]) * k) / 10, a[1] + ((a[3] - a[1]) * k) / 10, { buttons: 1 });
      }
      await mouse("mouseReleased", a[2], a[3], { button: "left", clickCount: 1 });
    }
    else if (kind === "tap") {
      await finger("touchStart", [[a[0], a[1]]]);
      await sleep(0.1);
      await finger("touchEnd", []);
    } else if (kind === "press") {
      await finger("touchStart", [[a[0], a[1]]]);
      await sleep(0.3);
    } else if (kind === "swipe") {
      await finger("touchStart", [[a[0], a[1]]]);
      for (let k = 1; k <= 10; k++) {
        await sleep(0.03);
        await finger("touchMove", [[a[0] + ((a[2] - a[0]) * k) / 10, a[1] + ((a[3] - a[1]) * k) / 10]]);
      }
      await sleep(0.1);
      await finger("touchEnd", []);
    }
    await sleep(0.2);
  }
  // let the interface's slides and tooltips settle
  await sleep(0.5);
  const shot = await send("Page.captureScreenshot", { format: "png" });
  // a finger left down would still be down on the next scene's page
  if (scene.phone) await finger("touchEnd", []);
  return Buffer.from(shot.result.data, "base64");
}

// ---------------------------------------------------------------- main

const args = process.argv.slice(2);
const update = args.includes("--update");
const names = args.filter((a) => !a.startsWith("--"));
const scenes = names.length ? names : Object.keys(SCENES);
for (const n of scenes) if (!SCENES[n]) throw new Error(`unknown scene ${n}`);
if (!existsSync(join(root, "dist/index.html"))) throw new Error("no dist/: run ./build-web.sh first");

const server = spawn("python3", ["serve.py", String(SERVER_PORT), "dist"], { cwd: root, stdio: "ignore" });
const profile = mkdtempSync(join(tmpdir(), "vendetta-web-tests-"));
const chrome = spawn(
  chromePath(),
  [
    "--headless=new",
    "--no-first-run",
    "--mute-audio",
    "--hide-scrollbars",
    "--use-angle=swiftshader-webgl",
    "--enable-unsafe-swiftshade",
    "--ozone-platform=headless",
    `--user-data-dir=${profile}`,
    `--remote-debugging-port=${CHROME_PORT}`,
    `--window-size=${W},${H}`,
    "about:blank",
  ],
  { stdio: "ignore" },
);
let failed = 0;
try {
  const { ws, send } = await connect();
  await send("Runtime.enable");
  mkdirSync(join(here, "golden"), { recursive: true });
  for (const name of scenes) {
    const phone = SCENES[name].phone;
    const [width, height] = phone ?? [W, H];
    await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: !!phone });
    await send("Emulation.setTouchEmulationEnabled", { enabled: !!phone, maxTouchPoints: 5 });
    const png = await play(send, SCENES[name]);
    const golden = join(here, "golden", `${name}.png`);
    if (update || !existsSync(golden)) {
      writeFileSync(golden, png);
      console.log(`${name}: reference ${update ? "updated" : "created"}`);
      continue;
    }
    const { differing, diff } = compare(decodePng(png), decodePng(readFileSync(golden)));
    const share = differing / (width * height);
    if (share <= 0.001) {
      console.log(`${name}: ok (${differing} pixels differ)`);
    } else {
      failed++;
      mkdirSync(join(here, "out"), { recursive: true });
      writeFileSync(join(here, "out", `${name}.png`), png);
      if (diff) writeFileSync(join(here, "out", `${name}-diff.png`), encodePng(diff));
      console.log(`${name}: FAILED, ${(share * 100).toFixed(2)} % of the pixels differ (see web-tests/out/)`);
    }
  }
  ws.close();
} finally {
  chrome.kill();
  server.kill();
  await sleep(0.3);
  rmSync(profile, { recursive: true, force: true });
}
process.exit(failed ? 1 : 0);
