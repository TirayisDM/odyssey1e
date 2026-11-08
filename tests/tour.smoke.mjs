// 294. IS THE TOUR STILL POINTING AT THINGS THAT EXIST?
//
// `TOUR.md` is a walkable path through every feature, and every step
// names where it lives: an element id in `src/index.html` and a Tauri
// command registered in `src-tauri/src/lib.rs`. That column is what
// makes the file useful for examining the code rather than only for
// using the app, and it is also the half of the file that rots on its
// own - a control gets renamed in the cycle that improves it and no
// human rereads a 600-line document to notice.
//
// So this reads the document and checks the names. **WHAT IT BUYS:**
// a renamed or deleted control, and an unregistered command, both fail
// here rather than under the next reader's hands.
//
// **WHAT IT DOES NOT BUY, and the file says so in its own words:** it
// cannot tell whether a step is in the right order, whether the
// "proves it" column still describes what happens, or whether the
// path through the tabs is still the path. Nothing mechanical can.
// Nobody has walked the tour end to end; this is the floor under it,
// not a substitute for the walk.
//
// ONLY THE LAST COLUMN IS SCANNED, deliberately. The prose cites
// `current_profile()`, which is a Postgres function and not a Tauri
// command, and would fail a naive sweep for `something()`. Narrowing
// to the column that makes the promise is better than keeping a list
// of exceptions, which would be a second place to be wrong - 288.
//
// `npm test`.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";

const here = dirname(new URL(import.meta.url).pathname.slice(1));
const root = join(here, "..");
const read = (...p) => readFileSync(join(root, ...p), "utf8");

const tour = read("TOUR.md");
const html = read("src", "index.html");
const lib = read("src-tauri", "src", "lib.rs");

/* ---- what the app actually offers ----
   Ids as the markup spells them, and the command registry as
   `generate_handler!` lists it. A path into a submodule registers
   exactly like a bare name and the command is still its last segment
   on the wire - see commands/mod.rs. */
const ids = new Set(
  [...html.matchAll(/\bid="([A-Za-z0-9_-]+)"/g)].map((m) => m[1])
);

const open = lib.indexOf("generate_handler![");
assert.notEqual(open, -1, "lib.rs still has a generate_handler! list");
const close = lib.indexOf("])", open);
assert.notEqual(close, -1, "and the list closes");
const commands = new Set(
  lib
    .slice(open, close)
    .split("\n")
    .map((l) => l.replace(/\/\/.*$/, "").trim())
    .filter((l) => /^[a-z_:]+,$/.test(l))
    .map((l) => l.replace(/,$/, "").split("::").pop())
);

assert.ok(ids.size > 300, `index.html has ${ids.size} ids, expected 300+`);
assert.ok(
  commands.size > 140,
  `lib.rs registers ${commands.size} commands, expected 140+`
);

/* ---- the step rows, and the column that makes the promise ----
   A step is a table row whose first cell is a number like 4.2. The
   last cell is "lives in". A row that is missing a cell is a
   half-written step and is worth failing on: the document's whole
   claim is that each step says what proves it. */
const steps = [];
for (const line of tour.split("\n")) {
  if (!line.startsWith("|")) continue;
  const cells = line.split("|").slice(1, -1).map((c) => c.trim());
  if (cells.length < 2) continue;
  if (!/^\d+\.\d+$/.test(cells[0])) continue;
  assert.equal(
    cells.length,
    4,
    `step ${cells[0]} has ${cells.length} cells, expected 4 ` +
      "(number, do, proves it, lives in)"
  );
  const [num, doing, proves, lives] = cells;
  assert.ok(doing, `step ${num} says what to do`);
  assert.ok(proves, `step ${num} says what proves it`);
  assert.ok(lives, `step ${num} says where it lives`);
  steps.push({ num, lives });
}

assert.ok(steps.length > 70, `TOUR.md has ${steps.length} steps, expected 70+`);

/* ---- and every name in that column resolves ---- */
const badIds = [];
const badCommands = [];
let seenIds = 0;
let seenCommands = 0;

for (const { num, lives } of steps) {
  for (const [, id] of lives.matchAll(/`#([A-Za-z0-9_-]+)`/g)) {
    seenIds += 1;
    if (!ids.has(id)) badIds.push(`${num}: #${id}`);
  }
  for (const [, name] of lives.matchAll(/`([a-z_][a-z0-9_]*)\(\)`/g)) {
    seenCommands += 1;
    if (!commands.has(name)) badCommands.push(`${num}: ${name}()`);
  }
}

assert.deepEqual(
  badIds,
  [],
  "TOUR.md names controls that are not in src/index.html. Either the " +
    "control was renamed - fix the tour - or the step is gone and the " +
    "tour is describing an app that no longer exists."
);

assert.deepEqual(
  badCommands,
  [],
  "TOUR.md names commands that are not registered in lib.rs. A command " +
    "can exist in commands/*.rs and still be unreachable from the " +
    "frontend if it is not in generate_handler!, which is the whole " +
    "point of checking the registry rather than the source files."
);

/* ---- a gutted document must not pass ----
   Every assertion above is satisfied by a file with no steps in it, so
   the floors are the part that keeps this test honest. */
// The floors are deliberately below the real figures rather than at
// them: a step whose control is built in JavaScript and has no id
// cites only a command, which is correct, so padding the document to
// hit a round number would be writing for the test.
assert.ok(seenIds > 60, `only ${seenIds} ids cited, expected 60+`);
assert.ok(
  seenCommands > 80,
  `only ${seenCommands} commands cited, expected 80+`
);

const sections = [...tour.matchAll(/^## \d+\. /gm)].length;
assert.ok(sections >= 11, `TOUR.md has ${sections} numbered parts, expected 11+`);

console.log(
  `TOUR.md: ${steps.length} steps across ${sections} parts, ` +
    `${seenIds} ids and ${seenCommands} commands, all of them real.`
);
