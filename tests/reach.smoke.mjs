// 300. CAN THE APP REACH EVERY COMMAND IT REGISTERS?
//
// `generate_handler!` is the list of what the frontend may call. A name
// on it that nothing calls is a command that compiles, passes its
// tests, and cannot be got at from the running application - and
// nothing anywhere said which ones those were. 298 found `list_tiles`
// by accident while writing a different check and carried a note
// saying there were probably more. There were seven more.
//
// **WHAT IT BUYS:** the answer to "what did we build and never wire
// up", kept current, with a reason beside every exception.
//
// **WHAT IT DOES NOT BUY:** it cannot tell a command nobody wants from
// a command somebody meant to finish. That is what the reasons in
// ALLOWED are for, and they are written by hand because the difference
// is a judgement and not a fact.
//
// ---------------------------------------------------------------------
// WHY IT MATCHES A STRING AND NOT A CALL SITE
// ---------------------------------------------------------------------
//
// This is the opposite instrument to `tour.smoke.mjs`, which narrowed
// to one table column on purpose, and the two would otherwise look
// like they disagree about method.
//
// A scan of `call("...")` call sites reports SIXTEEN unreachable
// commands and is wrong about half of them, because a command's name
// reaches Rust three ways:
//
//   tryCall(held ? "drop_here" : "place_object", ...)   a ternary
//   slotMove("spend_spell_slot", lvl)                   a helper's arg
//   onSquare("damage_square", { amount }, ...)          the same
//
// `clear_slot`, `spend_slot`, `drop_here`, `place_object`,
// `spend_spell_slot`, `restore_spell_slot`, `damage_square` and
// `set_square_hp` are all live and all invisible to a call-site check.
//
// So: a WIDER pattern plus a short named list beats a precise pattern
// plus eight wrong answers. 294 narrowed because its false positives
// were in prose it did not own; this widens because its false
// positives are real calls it must not accuse.
//
// It costs one thing, said plainly: a command whose name appears only
// in a comment would pass. That is cheap to notice by eye and the
// alternative is accusing working code.
//
// `npm test`.

import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";

const here = dirname(new URL(import.meta.url).pathname.slice(1));
const root = join(here, "..");
const read = (...p) => readFileSync(join(root, ...p), "utf8");

/* ---- REGISTERED AND DELIBERATELY UNREACHABLE ----
   Every entry is a command `generate_handler!` exposes that no file in
   `src/` names, and every one carries the reason - 195's rule for
   `security_doors()`' allowlist, for the same reason: a bare name on a
   list like this is an exception nobody can judge later.

   **NONE OF THESE IS DEAD CODE.** Seven of the eight 300 found were
   finished engine work with no button, which is a different thing from
   rot and wants a different answer. Two were not, and they are gone:
   `list_tiles` is deleted (superseded by `map_at`) and `standing_in`
   is unregistered (an internal helper with two Rust callers that never
   needed to be a command).

   Adding a name here is saying "built, and nothing can reach it, and
   that is correct for now". If that is not true, wire it up instead. */
const ALLOWED = {
  roll_dice:
    "DELIBERATE. Resolves a formula without touching the database - " +
    "its own comment calls it the fastest way to eyeball a formula by " +
    "hand. A developer's command, and giving it a button would put a " +
    "second roller in front of players beside the one that records.",

  rename_location:
    "BUILT, NOT WIRED. Renames a room or rewrites its description. " +
    "A character, an object, an encounter and an actor can all be " +
    "renamed; a room cannot. The most user-visible of these and the " +
    "one worth doing first.",

  move_location:
    "BUILT, NOT WIRED. Re-parents a room. Postgres refuses putting " +
    "the Inn inside its own cupboard, so the hard half is done.",

  remove_class:
    "BUILT, NOT WIRED. Undoes a multiclass. 073 gave the sheet a way " +
    "to add a class and never one to take it off again.",

  copy_from_scroll:
    "BUILT, NOT WIRED. Scribes from a scroll and destroys it. The " +
    "rest of scribing is reachable - `scribe_options` and " +
    "`scribe_spell` are both called - so this is one button short.",

  quote_object:
    "BUILT, NOT WIRED. A merchant's price for ONE object. Trade " +
    "reaches `quote_trade` for a whole basket; this is the single-row " +
    "version nothing asks for yet.",
};

/* ---- what is registered ---- */
const lib = read("src-tauri", "src", "lib.rs");
const open = lib.indexOf("generate_handler![");
assert.notEqual(open, -1, "lib.rs still has a generate_handler! list");
const close = lib.indexOf("])", open);
assert.notEqual(close, -1, "and the list closes");
const registered = [
  ...new Set(
    lib
      .slice(open, close)
      .split("\n")
      .map((l) => l.replace(/\/\/.*$/, "").trim())
      .filter((l) => /^[a-z_:]+,$/.test(l))
      .map((l) => l.replace(/,$/, "").split("::").pop())
  ),
];
assert.ok(
  registered.length > 140,
  `lib.rs registers ${registered.length} commands, expected 140+`
);

/* ---- every name the frontend mentions ----
   The whole of `src/`, as text. See the header for why this is a
   string match and not a call-site match. */
const front = readdirSync(join(root, "src"))
  .filter((f) => f.endsWith(".js") || f.endsWith(".html"))
  .map((f) => read("src", f))
  .join("\n");
assert.ok(front.length > 100000, "src/ read as less text than expected");

const mentions = (cmd) => new RegExp(`"${cmd}"`).test(front);

/* ---- the two directions ---- */
const unreachable = registered.filter((c) => !mentions(c) && !ALLOWED[c]);
assert.deepEqual(
  unreachable,
  [],
  "lib.rs registers commands no file in src/ names, and they are not " +
    "on the allowlist. Either wire one up, or add it to ALLOWED with " +
    "the reason it is correct for nothing to reach it. Found: " +
    unreachable.join(", ")
);

// AND THE ALLOWLIST MUST NOT ROT EITHER. An entry for a command that
// has since been wired up, renamed or deleted is a stale exception,
// and a stale exception is how an allowlist stops meaning anything.
const stale = Object.keys(ALLOWED).filter(
  (c) => !registered.includes(c) || mentions(c)
);
assert.deepEqual(
  stale,
  [],
  "ALLOWED names commands that are no longer registered, or that src/ " +
    "now reaches. Delete those entries - the exception has served its " +
    "purpose. Found: " +
    stale.join(", ")
);

for (const [cmd, why] of Object.entries(ALLOWED)) {
  assert.ok(
    why && why.length > 40,
    `ALLOWED.${cmd} needs a reason, not a bare name on a list`
  );
}

/* ---- and nothing is called that does not exist ----
   The cheaper direction and the one with teeth: a name in `src/` that
   is not registered fails at runtime, the first time that code path
   runs, which may be months after it was written. Only names that look
   like a call are checked - the markup is full of quoted words. */
const js = readdirSync(join(root, "src"))
  .filter((f) => f.endsWith(".js"))
  .map((f) => read("src", f))
  .join("\n");
const called = new Set(
  [...js.matchAll(/\b(?:tryCall|call|invoke)\(\s*"([a-z_][a-z0-9_]*)"/g)].map(
    (m) => m[1]
  )
);
const missing = [...called].filter((c) => !registered.includes(c)).sort();
assert.deepEqual(
  missing,
  [],
  "src/ calls commands that are not registered in generate_handler!. " +
    "Each one throws the first time its code path runs. Found: " +
    missing.join(", ")
);

const reached = registered.filter((c) => mentions(c)).length;
console.log(
  `reach: ${reached} of ${registered.length} registered commands are ` +
    `named in src/; the other ${registered.length - reached} are ` +
    `allowed, with reasons. Nothing calls a command that is not there.`
);
