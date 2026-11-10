// 305. IS ANY PANE EMPTY WHILE THE APP WAITS ON THE NETWORK?
//
// Dave: *"the screens seem really jumpy and glitchy when I am trying to
// do things - is this a performance issue from taxing system or just
// jagged programming."*
//
// Neither, as it turned out: not the machine and not the renderer. The
// order. Thirteen paints emptied their pane and THEN went to the
// network, so the pane sat at zero height for the whole round trip.
// The left column is `overflow-y:auto`, so a pane collapsing inside it
// reflows the column - everything below slides up, then slides back
// down when the content lands. Twice per action, in opposite
// directions. That is the jump.
//
// **WHAT IT BUYS:** the shape cannot come back silently. A new loader
// written the old way is named here by its function and its pane,
// before anybody has to notice the screen moving.
//
// **WHAT IT DOES NOT BUY:** this is a ratchet, not a proof. It reads
// text, because this project has no bundler and so no AST to walk -
// see `reach.smoke.mjs` on the same trade. It knows three shapes:
//
//   host.innerHTML = ...        the pane is emptied
//   host.replaceChildren()      the same, with no argument
//   host.append(...)            the pane is filled again
//
// A pane filled through a HELPER that takes the host is invisible to
// it. `fillMoves(el, o)` is exactly that, and it is why `fillDetail`
// is on the allowed list below rather than fixed: its pane is not
// blank, it GROWS in three stages, which is a different fault needing
// a different change.
//
// ---------------------------------------------------------------------
// WHY `innerHTML = ""` IS NOT ITSELF THE FAULT
// ---------------------------------------------------------------------
//
// Emptying a pane you fill in the same tick costs nothing - the browser
// never lays out the empty state. 100 of these exist in `main.js` and
// most are correct. What costs is emptying one and then AWAITING, and
// the two read identically at the call site. So this does not count
// `innerHTML`; it counts the gap between a clear and the fill, and
// asks whether a round trip happens inside it.

import assert from "node:assert";
import { readFileSync } from "node:fs";

const js = readFileSync(new URL("../src/main.js", import.meta.url), "utf8");
const lines = js.split("\n");

// Top-level functions only. A nested one is inside a handler, which
// runs later and is not part of any paint.
const funcs = [];
for (let i = 0; i < lines.length; i++) {
  if (!/^(async )?function [A-Za-z_$]/.test(lines[i])) continue;
  let depth = 0;
  for (let j = i; j < lines.length; j++) {
    depth += (lines[j].match(/\{/g) || []).length;
    depth -= (lines[j].match(/\}/g) || []).length;
    if (depth <= 0 && j > i) {
      funcs.push({ name: lines[i].split("(")[0].split(" ").pop(), i, j });
      break;
    }
  }
}

const CLEAR = /\b([A-Za-z_$][\w$]*)\.(?:innerHTML\s*=|replaceChildren\(\s*\))/;
const FILL = /\b([A-Za-z_$][\w$]*)\.(?:append|replaceChildren)\(\s*[^\s)]/;
// An await belonging to the function itself sits at one indent. One
// indented further is inside something the function only registers.
const OWN_AWAIT = /^ {2}[^ \t].*\bawait\b/;

const indentOf = (t) => t.length - t.trimStart().length;

const holes = [];
for (const f of funcs) {
  const open = new Map(); // host -> { line that emptied it, its indent }
  for (let k = f.i; k <= f.j; k++) {
    const line = lines[k];
    const at = indentOf(line);

    // A CLEAR INSIDE A BRANCH DIES WITH THE BRANCH. `if (!characterId)
    // { pane.replaceChildren(); return; }` empties a pane because there
    // is nothing to put in it, and owes no round trip - but textually
    // it sits above every await in the function. The dedent out of the
    // block is where it stops being this function's problem, and
    // indentation is the only thing a text scan has to read it by.
    // That is a real limit: a clear and an await at the SAME indent
    // inside one branch read as the function's own, which errs toward
    // reporting rather than toward silence.
    if (line.trim()) {
      for (const [host, c] of open) if (at < c.indent) open.delete(host);
    }

    const cleared = CLEAR.exec(line);
    if (cleared && !open.has(cleared[1])) {
      open.set(cleared[1], { at: k + 1, indent: at });
    }
    const filled = FILL.exec(line);
    if (filled) open.delete(filled[1]);
    if (open.size && OWN_AWAIT.test(line)) {
      for (const [host, c] of open) holes.push({ fn: f.name, host, at: c.at });
      open.clear(); // one report per pane, not one per await
    }
  }
}

// Known, with a reason each. A name comes off this list by being
// fixed, and goes on it only with the reason written down.
//
// EMPTY AFTER 305, AND THAT IS THE POINT OF SAYING SO. The number is
// zero today; the list exists so the next one has somewhere to be
// written down with its reason instead of being argued about.
const ALLOWED = {};

const found = holes.map((h) => `${h.fn}/${h.host}`).sort();
const allowed = Object.keys(ALLOWED).sort();
const unexpected = found.filter((n) => !allowed.includes(n));
const fixed = allowed.filter((n) => !found.includes(n));

assert.deepEqual(
  unexpected,
  [],
  "a pane is emptied and then waited on - the screen will collapse and " +
    "jump for the length of the round trip. Build into the fragment " +
    "`swapInto` hands you, or gather the answer before you empty " +
    "anything. See 305. Found: " +
    holes
      .filter((h) => unexpected.includes(`${h.fn}/${h.host}`))
      .map((h) => `${h.fn} empties ${h.host} at main.js:${h.at}`)
      .join("; ")
);

assert.deepEqual(
  fixed,
  [],
  "these are on the allowed list and no longer happen - take them off " +
    "it, because an expectation that is stale is worse than no " +
    "expectation (077). Fixed: " + fixed.join(", ")
);

// Minus its own definition, so the number counts call sites.
const swaps = (js.match(/\bswapInto\(/g) || []).length - 1;
const clears = (js.match(/\.innerHTML\s*=/g) || []).length;
console.log(
  `paint: ${swaps} panes are built before they are shown. ` +
    `${clears} pane clears remain and not one is followed by a round ` +
    `trip. ${found.length} known hole(s)${
      allowed.length ? ": " + allowed.join(", ") : ""
    }.`
);

// 305. WHAT THIS CHECK CANNOT SEE, written beside it rather than in a
// cycle note nobody opens:
//
//   - A PANE THAT GROWS. `fillDetail` appends its facts in the same
//     tick as the clear, then adds the moves section and the contents
//     over two more round trips. Never blank, so never a hole here -
//     but the pane still changes height three times. Closing it means
//     `fillMoves` not taking the host to hand on to `reopenDetail`:
//     three functions, and not an ordering change.
//   - A PANE FILLED BY A HELPER that takes the host as an argument.
//   - HOW LONG the gap is. `?slow=<ms>` in the rig is for that, and it
//     is driven by hand.
