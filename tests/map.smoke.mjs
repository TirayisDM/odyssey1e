// 242. DOES THE RENDERER RUN?
//
// `node --check src/map.js` passed on 240 and on 241. The file could
// not draw on either: 240 rewrote the top of it to add tile variants
// and deleted the colour table, `paint`, `isSolid` and `extent` on the
// way past. `draw` called `extent`, threw `ReferenceError`, and the
// canvas stayed blank.
//
// **`--check` PARSES. An undefined identifier is valid syntax.** The
// 1,251 Rust tests never touch a renderer, so nothing in the project
// could tell a working `map.js` from one with its middle removed, and
// the only thing that would have was opening the Map tab.
//
// So this RUNS it. Not a unit test of the drawing - canvas output is
// not worth asserting against - but a check that every function the
// renderer reaches exists and that the two answers which must agree
// still do.
//
// `npm test`.

import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";
import { join, dirname } from "node:path";

/* ---- the smallest canvas that map.js can be fooled by ----
   Every context call is a no-op; the properties it assigns are read
   back as strings. We are not checking what it drew, only that it
   could. */
globalThis.Image = class {
  set src(_) {} // never loads, so every tile falls back to a colour
};
const noop = () => {};
const ctx = new Proxy(
  {},
  {
    get: (_, k) =>
      k === "fillStyle" || k === "strokeStyle" || k === "font" ? "" : noop,
    set: () => true,
  }
);
const canvas = () => ({ width: 0, height: 0, getContext: () => ctx });

const here = dirname(new URL(import.meta.url).pathname.slice(1));
const { draw, squareAt, forgetArt } = await import(
  pathToFileURL(join(here, "..", "src", "map.js")).href
);

/* ---- a view with one of everything that changes a code path ---- */
const view = {
  width: 3,
  height: 2,
  z: 0,
  squares: [
    sq(0, 0, "stone_floor", null, { cost: 5 }),
    // A solid cover, which REPLACES the square.
    sq(1, 0, "stone_floor", "stone_wall", { blocks: true, cost: null }),
    // A layer cover on implied ground, turned, and outside the room.
    sq(2, 1, "grass", "snow", { facing: 2, drawn: false, outside: true }),
    // 232: a square the declared room does not contain is drawn, not
    // hidden - so the extent has to stretch to (9,9).
    sq(9, 9, "nonsense_word", null, { facing: 1, outside: true }),
  ],
  standing: [{ name: "Tarren", x: 0, y: 0, z: 0 }],
};

function sq(x, y, surface, cover, extra) {
  return {
    x,
    y,
    surface,
    cover,
    facing: 0,
    drawn: true,
    blocks: false,
    cost: 5,
    outside: false,
    ...extra,
  };
}

/* ---- it draws ---- */
const c = canvas();
const said = draw(c, view);
assert.match(said, /10x10/, "the extent must stretch to the stray square");
assert.match(said, /4 squares/);
assert.match(said, /1 from the recipe/);
assert.match(said, /2 outside the room/);
assert.match(said, /1 standing/);
assert.equal(c.width, 320, "ten squares at 32 pixels");
assert.equal(c.height, 320);

/* ---- nothing mapped is a SENTENCE, not a crash ---- */
const empty = canvas();
assert.match(
  draw(empty, { width: 0, height: 0, z: 0, squares: [], standing: [] }),
  /nothing is mapped/
);
assert.equal(empty.width, 1, "a 1x1 canvas, so the pane shows no stale map");

/* ---- 241. THE CLICK AND THE PICTURE MUST AGREE ----
   `squareAt` and `draw` both call `extent`, which is the whole reason
   they cannot drift. This asserts the consequence rather than the
   arrangement: a point inside a cell maps to that cell, and a point
   off the map maps to nothing. */
assert.deepEqual(squareAt(view, 0, 0), { x: 0, y: 0 });
assert.deepEqual(squareAt(view, 31, 31), { x: 0, y: 0 }, "still the first cell");
assert.deepEqual(squareAt(view, 32, 0), { x: 1, y: 0 });
assert.deepEqual(squareAt(view, 40, 40), { x: 1, y: 1 });
assert.equal(squareAt(view, 99999, 0), null, "off the right-hand edge");
assert.equal(squareAt(view, 0, 99999), null, "off the bottom");
assert.equal(
  squareAt({ width: 0, height: 0, squares: [], standing: [] }, 0, 0),
  null,
  "nothing mapped means no square is under the pointer"
);

/* ---- a negative origin, because a map may start left of zero ---- */
const negative = {
  width: 0,
  height: 0,
  z: 0,
  squares: [sq(-2, -2, "rock", null, {}), sq(-1, -1, "rock", null, {})],
  standing: [],
};
draw(canvas(), negative);
assert.deepEqual(
  squareAt(negative, 0, 0),
  { x: -2, y: -2 },
  "the top-left pixel is the top-left SQUARE, whatever it is numbered"
);

/* ---- 242. A WRONG-SIZED TILE IS NAMED, NOT SILENT ----
   `grass.png` was 58x57 and therefore not drawn, and the only place
   that said so was devtools. This replaces the never-loading stub with
   one that loads at the wrong size, and asserts the pane's own summary
   says which file it refused. */
globalThis.Image = class {
  set src(url) {
    this.naturalWidth = 58;
    this.naturalHeight = 57;
    // After the current turn, the way a real image does - the
    // renderer has to cope with art that lands later than the frame
    // that asked for it.
    setTimeout(() => this.onload && this.onload(), 0);
  }
};
// The refusals are the point of this phase; printing ten of them is
// not, and the last `forgetArt` below starts more that land after the
// script has finished asserting.
console.warn = noop;
forgetArt(); // empties the cache, so every tile is asked for again
draw(canvas(), view);
await new Promise((r) => setTimeout(r, 10));
const named = draw(canvas(), view);
assert.match(named, /NOT 32x32, so not drawn/);
assert.match(named, /stone_floor\.png/, "the refused file is named");

/* ---- 255. A SHAPE WITH NO SPRITE IS NAMED ----
   `shape.rs` admits a word before its art exists, so the only thing
   standing between a `center` wall and a square that silently draws
   a plain one is this line in the summary. */
forgetArt();
const shaped = {
  width: 1, height: 1, z: 0,
  squares: [sq(0, 0, "stone_floor", "stone_wall", { shape: "center" })],
  standing: [],
};
draw(canvas(), shaped);
await new Promise((r) => setTimeout(r, 10));
const named2 = draw(canvas(), shaped);
assert.match(named2, /no art yet, drawn plain/);
assert.match(named2, /stone_wall_center/, "it names the sprite it wanted");

/* ---- and forgetArt is the way back ---- */
forgetArt();
assert.doesNotMatch(
  draw(canvas(), view),
  /NOT 32x32/,
  "emptying the cache forgets the refusals too, or a fixed tile would " +
    "stay accused for the life of the session"
);

console.log("map.js runs: draw, squareAt, forgetArt, and they agree.");
