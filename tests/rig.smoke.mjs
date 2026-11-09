// 298. IS THE RIG STILL POINTING AT THE APP THAT EXISTS?
//
// `tests/rig/` is the browser harness: `serve.py` serves `src/` as the
// document root the way Tauri does and injects `stub.js`, which stands
// in for the Tauri bridge. Four cycles have used it to prove things no
// test could reach - 211's hidden spell tab, 290's die, 295's sheet
// loaders, 297's two-shaped `state.catalogue`.
//
// IT ROTS IN TWO PLACES AND BOTH ARE MECHANICAL.
//
// **WHAT THIS BUYS:** a stub answering a command that no longer exists
// fails here, and so does `serve.py` losing the tag it injects the stub
// in front of - which would otherwise serve the app with no bridge at
// all and look like a hung page.
//
// **WHAT IT DOES NOT BUY, and nothing mechanical could:** whether a
// fixture's SHAPE still matches the Rust struct it was copied off by
// hand. Every one of those was transcribed by eye, and a field the
// engine renamed reads here as a field the rig forgot. The header of
// `stub.js` says so too.
//
// COVERAGE IS PRINTED, NOT ASSERTED. The rig answers a fraction of the
// commands the frontend calls, and that fraction is the honest state of
// a harness nobody has finished - 294 already learned what happens when
// a floor exists to be padded to.
//
// `npm test`.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";

const here = dirname(new URL(import.meta.url).pathname.slice(1));
const root = join(here, "..");
const read = (...p) => readFileSync(join(root, ...p), "utf8");

const stub = read("tests", "rig", "stub.js");
const serve = read("tests", "rig", "serve.py");
const html = read("src", "index.html");
const mainjs = read("src", "main.js");
const lib = read("src-tauri", "src", "lib.rs");

/* ---- the command registry, as generate_handler! lists it ----
   The registry rather than `commands/*.rs`, for `tour.smoke.mjs`'s
   reason: a command can exist in a source file and be unreachable from
   the frontend if nothing registered it. */
const open = lib.indexOf("generate_handler![");
assert.notEqual(open, -1, "lib.rs still has a generate_handler! list");
const close = lib.indexOf("])", open);
const registered = new Set(
  lib
    .slice(open, close)
    .split("\n")
    .map((l) => l.replace(/\/\/.*$/, "").trim())
    .filter((l) => /^[a-z_:]+,$/.test(l))
    .map((l) => l.replace(/,$/, "").split("::").pop())
);
assert.ok(
  registered.size > 140,
  `lib.rs registers ${registered.size} commands, expected 140+`
);

/* ---- what the stub answers ----
   Only the ANSWERS object. The fixtures above it are full of object
   literals whose keys look exactly like command names - `abilities`,
   `classes`, `speed` - and sweeping the whole file would mistake a
   sheet's fields for the rig's promises. Narrowing to the object that
   makes the promise beat keeping a list of exceptions, which is 288. */
const a0 = stub.indexOf("const ANSWERS = {");
assert.notEqual(a0, -1, "stub.js still has an ANSWERS object");
const a1 = stub.indexOf("\n};", a0);
assert.notEqual(a1, -1, "and it closes");
const answers = [
  ...stub.slice(a0, a1).matchAll(/^ {2}([a-z_][a-z0-9_]*):/gm),
].map((m) => m[1]);

assert.ok(answers.length > 25, `the stub answers ${answers.length}, expected 25+`);

const dead = answers.filter((c) => !registered.has(c));
assert.deepEqual(
  dead,
  [],
  "stub.js answers commands that are not registered in lib.rs. Either " +
    "the command was renamed - fix the stub - or it is gone and the rig " +
    "is faking an app that no longer exists. Found: " +
    dead.join(", ")
);

/* ---- what the frontend asks for ---- */
const called = new Set(
  [...mainjs.matchAll(/\b(?:tryCall|call|invoke)\("([a-z_][a-z0-9_]*)"/g)].map(
    (m) => m[1]
  )
);
assert.ok(called.size > 100, `main.js calls ${called.size} commands, expected 100+`);

/* ---- and nothing the app never asks for ----
   The other direction, and the cheaper half to get wrong. A stub
   answering a command the frontend does not call is a conversation the
   app never has, and it sits there reading like coverage. `list_tiles`
   was exactly that: registered in `lib.rs`, called from nowhere in
   `src/`, answered here. */
const unasked = answers.filter((c) => !called.has(c));
assert.deepEqual(
  unasked,
  [],
  "stub.js answers commands main.js never calls. Either the frontend " +
    "stopped asking - drop the answer - or the call moved somewhere " +
    "this check does not read, which is worth knowing either way. " +
    "Found: " +
    unasked.join(", ")
);

/* ---- the injection point serve.py depends on ---- */
const tag = serve.match(/^MODULE_TAG = '(.+)'$/m);
assert.ok(tag, "serve.py still declares MODULE_TAG");
assert.ok(
  html.includes(tag[1]),
  "serve.py injects the stub in front of MODULE_TAG and src/index.html " +
    "no longer contains it:\n  " +
    tag[1] +
    "\nWithout the stub, main.js throws on window.__TAURI__.core at " +
    "import time and the rig serves a blank page. Fix MODULE_TAG."
);

/* ---- and the seam the harness reads state through ---- */
assert.ok(
  /if \(window\.__RIG__\) window\.__RIG__\.state = state;/.test(mainjs),
  "src/main.js no longer hands the rig its state. That line is how a " +
    "harness reads the app's own state instead of guessing through the " +
    "DOM - 295 and 297 both needed it."
);
assert.ok(
  stub.includes("window.__RIG__ = {}"),
  "stub.js must define window.__RIG__ before main.js runs, or the seam " +
    "above is dead."
);

/* ---- coverage, printed ---- */
console.log(
  `rig: answers ${answers.length} of the ${called.size} commands main.js ` +
    `calls - all registered, all asked for. The other ` +
    `${called.size - answers.length} throw "not in the rig".`
);
