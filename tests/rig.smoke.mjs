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
// 302. AND WHETHER THE FIXTURES STILL HAVE THE ENGINE'S SHAPE, which
// the paragraph that used to sit here said nothing mechanical could do.
// It can, for half of them, and the half is the interesting part.
//
// ----------------------------------------------------------------------
// WHY THERE IS NO GENERATOR
//
// 298 carried a note saying "generating them from the structs is the
// only real fix". Measured: **119 of the 165 commands return
// `serde_json::Value`.** Of the 36 the stub answers, NINE have a Rust
// struct. The rest pass PostgREST rows straight through, so their shape
// lives in the database and not in Rust, and a generator would cover a
// quarter of this surface while missing `list_skill_keys` and
// `dice_prefs` - two of the four faults that prompted the note.
//
// There is no generator here. There is a check, against two sources:
//
//   a Rust struct            9 of 36   list_locations -> Vec<Placed>
//   a ("select", "...")      9 of 36   list_skill_keys -> key,name,ability
//   neither                 18 of 36   dice_prefs, wallet, encumbrance
//
// The select string is the exact column list PostgREST returns, sitting
// in the command's own body. `list_skill_keys` carries
// `("select", "key,name,ability")` - the three keys 298's fixture got
// wrong. An untyped command may also ADD keys after the read -
// `who_is_where` inserts `creature` - so the allowed set is the select
// plus every bare string literal in the body. Deliberately permissive:
// this check must never accuse a fixture that is right.
//
// ----------------------------------------------------------------------
// WHY ONLY ONE DIRECTION IS ASSERTED
//
// **AN INVENTED KEY FAILS, WITH NO ALLOWLIST AT ALL.** A key the struct
// or the select does not have is something production can never send.
// That is unambiguous, so it needs no judgement and has no list to rot -
// unlike 300's six reasons, which somebody has to reread.
//
// **A MISSING KEY IS PRINTED, NOT ASSERTED.** The first draft asserted
// both directions. Measured before writing it down: **72 fields would
// have been flagged.** `Item` has 30 fields and the fixture carries 5 ON
// PURPOSE - the thinness is the rig's design. A 72-entry allowlist is
// not a list anybody reads, and 294 already learned what a floor is for.
//
// ----------------------------------------------------------------------
// WHAT IT STILL DOES NOT BUY
//
// **A VALUE IN THE WRONG DOMAIN.** 299's `dice_prefs` returned
// `#f5f5f5` where the app wanted `slate`, and that is a valid String in
// a correctly-shaped answer. No shape check catches it, this one
// included; it was caught by an eye and the next one will be too.
//
// **NESTED SHAPES.** Only the top level of each row is checked.
// `Owned.item` is an `Item` and nothing here looks inside it. 298's
// un-nested inventory row was still caught, because flattening a nest
// shows up at the top level as invented keys.
//
// **THE 18 WITH NO CHECKABLE SHAPE.** Printed, every run, as a number.
//
// COVERAGE IS PRINTED, NOT ASSERTED. The rig answers a fraction of the
// commands the frontend calls, and that fraction is the honest state of
// a harness nobody has finished - 294 already learned what happens when
// a floor exists to be padded to.
//
// `npm test`.

import assert from "node:assert/strict";
import vm from "node:vm";
import { readFileSync, readdirSync } from "node:fs";
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

/* ======================================================================
   302. THE SHAPES
   ====================================================================== */

/* ---- every .rs file, as one string ---- */
const rustFiles = [];
(function walk(dir) {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) walk(p);
    else if (e.name.endsWith(".rs")) rustFiles.push(p);
  }
})(join(root, "src-tauri", "src"));
const rust = rustFiles.map((f) => readFileSync(f, "utf8")).join("\n\n");

/* ---- THE ASSUMPTION THIS WHOLE CHECK RESTS ON ----
   Field names are read out of the struct declaration, so a
   `#[serde(rename = "...")]` on a field would make every answer below
   quietly wrong rather than loudly wrong. There are none today - only
   `rename_all` on enums, which is a different thing and does not reach
   a struct's field names. If one ever lands, this fails first and says
   why, instead of the check silently going blind. */
assert.ok(
  !/#\[serde\(rename\s*=/.test(rust),
  "a #[serde(rename = ...)] appeared in src-tauri/src. This check reads " +
    "field names straight off the struct, so a renamed field would read " +
    "as an invented key in every fixture that has it right. Teach the " +
    "parser about renames before adding one."
);

/* ---- each command's return type and body ----
   Brace-matched rather than line-based: a signature spans lines and a
   body is full of braces. */
const CMD = new Map();
for (const m of rust.matchAll(/#\[tauri::command\]\s*(?:pub\s+)?fn\s+(\w+)\s*\(/g)) {
  const argOpen = m.index + m[0].length - 1;
  let d = 0;
  let argClose = argOpen;
  for (let i = argOpen; i < rust.length; i++) {
    if (rust[i] === "(") d++;
    else if (rust[i] === ")" && --d === 0) { argClose = i; break; }
  }
  const bodyOpen = rust.indexOf("{", argClose);
  let b = 0;
  let bodyClose = bodyOpen;
  for (let i = bodyOpen; i < rust.length; i++) {
    if (rust[i] === "{") b++;
    else if (rust[i] === "}" && --b === 0) { bodyClose = i; break; }
  }
  CMD.set(m[1], {
    // TRIM BEFORE STRIPPING THE ARROW. The slice starts at the closing
    // paren, so it opens with a space and `^->` does not match - which
    // sent every typed command down the select path and made this check
    // report four faults that were not there. Caught by `list_locations`
    // being flagged for `depth` and `path`, which 301 had just proved
    // correct.
    ret: rust
      .slice(argClose + 1, bodyOpen)
      .replace(/\s+/g, " ")
      .trim()
      .replace(/^->\s*/, "")
      .trim(),
    body: rust.slice(bodyOpen, bodyClose),
  });
}
assert.ok(CMD.size > 140, `parsed ${CMD.size} commands out of the Rust, expected 140+`);

/* ---- source one: a Rust struct ----
   `Result<Vec<crate::equipment::Owned>, String>` is an `Owned`. A
   return type that names no struct we can find - `Value`, `String`,
   `usize` - falls through to source two, which is the correct answer
   for 119 of the 165 commands. */
function structNamed(ret) {
  let t = ret.replace(/^Result<([\s\S]*),\s*String>$/, "$1").trim();
  for (let prev = ""; t !== prev; ) {
    prev = t;
    t = t.replace(/^(?:Vec|Option)<([\s\S]+)>$/, "$1").trim();
  }
  const path = t.split("::").map((s) => s.trim());
  const name = path.pop();
  // `crate::equipment::Owned` says which file to look in; a bare
  // `Placed` does not, and that is what the ambiguity guard is for.
  const module = path.filter((s) => s && s !== "crate").pop() || null;
  return /^[A-Z]\w*$/.test(name) ? { name, module } : null;
}

/* A NAME IS NOT UNIQUE, which is the trap in resolving one by regex.
   There are THREE `Placed` in this tree - `locations.rs`, `slots.rs`
   and a private one in `commands/tiles.rs` - and reading the fields of
   the wrong one would hand this check a wrong allowed set and let it
   accuse a fixture that is right. So: collect every file that declares
   the name, prefer the module the return type points at, and DECLINE
   when it is still ambiguous rather than guessing. */
const STRUCTS = new Map();
function structFields(name, modulePath) {
  const memo = name + "|" + modulePath;
  if (STRUCTS.has(memo)) return STRUCTS.get(memo);
  const re = new RegExp(
    "((?:#\\[[^\\]]*\\]\\s*)*)pub struct " + name + "\\s*\\{([\\s\\S]*?)\\n\\}"
  );
  let hits = rustFiles
    .map((f) => ({ file: f, m: readFileSync(f, "utf8").match(re) }))
    .filter((h) => h.m);
  if (hits.length > 1 && modulePath) {
    const narrowed = hits.filter((h) => h.file.replace(/\\/g, "/").includes("/" + modulePath + ".rs"));
    if (narrowed.length) hits = narrowed;
  }
  let out = null;
  // A `rename_all` on the struct itself rewrites every field name on the
  // way out, so that declines too.
  if (hits.length === 1 && !/rename/.test(hits[0].m[1])) {
    out = hits[0].m[2]
      .split("\n")
      .map((l) => l.trim())
      .filter((l) => l && !l.startsWith("//") && !l.startsWith("#"))
      .map((l) => (l.match(/^(?:pub )?([a-z_][a-z0-9_]*)\s*:/) || [])[1])
      .filter(Boolean);
    if (!out.length) out = null;
  }
  STRUCTS.set(memo, out);
  return out;
}

/* ---- source two: the select, plus anything the body adds afterwards ----
   `("select", "key,name,ability")` is the exact column list PostgREST
   sends back. An untyped command may then ADD keys - `who_is_where`
   inserts `creature` after the read - so every bare string literal in
   the body joins the allowed set.

   DELIBERATELY PERMISSIVE. A literal that is really a table name or a
   query parameter widens the set and lets a wrong fixture through; the
   alternative accuses a fixture that is right, and that is the failure
   that gets a check deleted. 300 chose the same way round. */
const selectColumns = (body) => [
  ...new Set(
    [...body.matchAll(/\("select",\s*"([^"]+)"/g)]
      .flatMap((m) => m[1].split(","))
      .map((c) => c.trim())
      .filter(Boolean)
  ),
];
const bodyLiterals = (body) =>
  new Set([...body.matchAll(/"([a-z_][a-z0-9_]*)"/g)].map((m) => m[1]));

/* ---- the stub, run for real ----
   Evaluated in a vm with a five-line shim rather than read as text,
   because half these fixtures are built by functions and spreads and
   the literal in the file is not the answer the app receives.

   A FRESH CONTEXT PER COMMAND. `rename_location` writes to the world
   fixture, so one shared context would make this check depend on the
   order it asked its questions. */
const stubSrc = read("tests", "rig", "stub.js");
function bridge() {
  const ctx = {
    console,
    URLSearchParams,
    setTimeout,
    location: { search: "" },
    addEventListener() {},
    document: { querySelector: () => null },
  };
  ctx.window = ctx;
  ctx.globalThis = ctx;
  vm.createContext(ctx);
  vm.runInContext(stubSrc, ctx);
  return ctx.window.__TAURI__.core;
}

// Enough of an argument bag for any answer that reads one. A fixture
// that still cannot be called is reported by name below, never skipped
// quietly - a silent skip is how a check stops covering something.
const ARGS = {
  gameId: "game-1", characterId: "c1", locationId: "l1", encounterId: "e1",
  objectId: "o1", actorId: "a1", profileId: "dm-1", id: "c1",
  key: "goblin", npcKey: "goblin", spellKey: "bless", name: "rig", level: 1,
};

const invented = [];
const checked = [];
const uncheckable = [];
const uncallable = [];

for (const cmd of answers) {
  const meta = CMD.get(cmd);
  if (!meta) continue; // already asserted registered; nothing to shape-check

  let value;
  try {
    value = await bridge().invoke(cmd, ARGS);
  } catch (e) {
    uncallable.push(`${cmd} (${String((e && e.message) || e).slice(0, 48)})`);
    continue;
  }

  const st = structNamed(meta.ret);
  const fields = st ? structFields(st.name, st.module) : null;
  const columns = fields ? null : selectColumns(meta.body);
  if (!fields && !(columns && columns.length)) {
    uncheckable.push(cmd);
    continue;
  }
  const declared = fields || columns;
  const allowed = new Set(fields || [...columns, ...bodyLiterals(meta.body)]);
  const source = fields ? st.name : "the select in " + cmd;

  const row = Array.isArray(value) ? value[0] : value;
  if (row === undefined || row === null) {
    checked.push([cmd, source, 0, []]);
    continue;
  }
  if (typeof row !== "object") {
    // 298's `list_skill_keys` was a list of bare strings where the
    // engine sends objects. Not a wrong key - a wrong KIND of answer.
    invented.push(
      `${cmd}: rows are ${typeof row}s, but the engine sends objects ` +
        `(${declared.slice(0, 4).join(", ")}…)`
    );
    continue;
  }

  const have = Object.keys(row);
  const bad = have.filter((k) => !allowed.has(k));
  if (bad.length) {
    invented.push(`${cmd}: ${bad.join(", ")} — not in ${source}`);
  }
  // The other direction, for the printout only. A field name that
  // appears in src/ is a cheap over-approximation of "the frontend
  // reads it" - 300's instrument, reused.
  const missed = declared.filter(
    (k) => !have.includes(k) && new RegExp("\\.\\b" + k + "\\b").test(mainjs)
  );
  checked.push([cmd, source, have.length, missed]);
}

assert.deepEqual(
  invented,
  [],
  "a rig fixture carries keys the engine does not send. Production can " +
    "never produce them, so every screen the rig paints from one is a " +
    "screen nobody will ever see. Fix the fixture, not this check:\n  " +
    invented.join("\n  ")
);

assert.deepEqual(
  uncallable,
  [],
  "a stub answer threw when this check called it. Either it needs an " +
    "argument ARGS does not carry - add it - or the fixture is broken " +
    "and the rig has been painting from it. Found: " + uncallable.join(", ")
);

/* ---- the other direction, as a worklist ----
   A number per command rather than the whole list, because `Species`
   alone would print twenty-one names and a line nobody finishes reading
   is a line nobody reads. Thin is usually CORRECT - `fight_strip` omits
   `id` and `status` because its fixture is an idle strip - so this is
   somewhere to look, never a verdict. */
const thin = checked.filter(([, , , m]) => m.length);
if (thin.length) {
  console.log(
    "rig: fixtures thinner than the engine, in fields src/ names -\n" +
      thin
        .map(
          ([c, , , m]) =>
            `      ${c}: ${m.length} (${m.slice(0, 3).join(", ")}` +
            `${m.length > 3 ? ", …" : ""})`
        )
        .join("\n")
  );
}

/* ---- coverage, printed ---- */
console.log(
  `rig: answers ${answers.length} of the ${called.size} commands main.js ` +
    `calls - all registered, all asked for. The other ` +
    `${called.size - answers.length} throw "not in the rig".`
);
console.log(
  `rig: ${checked.length} of those ${answers.length} have a shape to be ` +
    `held to and none invents a key; the other ${uncheckable.length} ` +
    `return an untyped Value with no select to read.`
);
