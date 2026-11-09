// 298. THE RIG'S STAND-IN FOR THE TAURI BRIDGE.
//
// `main.js` reads `window.__TAURI__.core` at import time. This file
// defines it before the module runs - `tests/rig/serve.py` injects it
// one tag ahead - so the real frontend can be driven in a browser with
// nothing behind it. No database, no sign-in, no Rust.
//
// **IT IS A FAKE AND IT SHOULD LOOK LIKE ONE.** Every shape in here was
// copied off a Rust struct by hand and nothing keeps them in step. A row
// the app paints wrongly in the rig may be the rig's fault, and that is
// the cost of being able to drive the thing at all.
//
// WHAT IT BUYS, in cycles that have actually used it: 211 proved a throw
// in `loadPrayers` hid a wizard's spell tab; 290 drew the die; 295 proved
// the six sheet loaders stopped taking each other down; 297 proved
// `state.catalogue` held two shapes at once. None of those were visible
// from a test and none needed a database.
//
// WHAT IT DOES NOT BUY: the rules. Those are 1443 Rust tests, and
// nothing here should ever be asked to decide what a roll is worth.
//
// ----------------------------------------------------------------------
// COVERAGE, SAID PLAINLY
//
// `main.js` calls about 156 distinct commands. This answers about 35 -
// the reads that get screens painted. `tests/rig.smoke.mjs` prints both
// numbers on every `npm test` rather than asserting a floor, because a
// floor is a number somebody pads to.
//
// **EVERY OTHER COMMAND THROWS, ON PURPOSE.** It used to return `null`
// without throwing, which `tryCall` logged as `✓ give_item` with a null
// value - A GREEN TICK FOR A WRITE THAT DID NOTHING. That is the single
// most expensive kind of lie a harness can tell. Now `call` returns null,
// the log draws a red line naming the command, and 207's badge counts
// it. "Not in the rig" is a truthful answer; a silent success is not.
//
// ----------------------------------------------------------------------
// FLAGS
//
//   ?role=player     you are not the DM of this game
//   ?caster=0        the character cannot cast
//   ?break=<name>    make one loader fail - see BREAKS below
//
// `npm run rig`, then http://127.0.0.1:8790

const RIG = new URLSearchParams(location.search);
const DM = RIG.get("role") !== "player";
const BREAK = RIG.get("break") || "";
const GAME = "game-1";
const ME = { profile_id: DM ? "dm-1" : "pc-1", email: "rig@example.com" };

// 298. THE SEAM. `main.js` fills `window.__RIG__.state` if this object
// exists, so a harness can read the app's state instead of guessing at
// it through the DOM. In the real app `window.__RIG__` is undefined and
// that line does nothing.
window.__RIG__ = {};

/* ---------- the fixtures ---------- */

// 301. THE WORLD, AND IT IS A `let` SHAPE. See `list_locations`.
const PLACES = [
  { id: "l1", parent_id: null, name: "Tavern", kind: "room",
    description: null, depth: 0, path: ["Tavern"] },
  { id: "l2", parent_id: "l1", name: "Cellar", kind: "room",
    description: "Damp. Someone has been at the barrels.",
    depth: 1, path: ["Tavern", "Cellar"] },
];

const ABIL = (score) => ({ score, base: score, bonus: 0 });
const SAVE = (n, prof) => ({ natural: n, proficient: !!prof, mods: [], total: n + (prof ? 2 : 0) });

const CHAR = {
  id: "c1", name: "Luci", game_id: GAME, is_npc: false,
  species_key: "human", level: 3, hp: 18, hp_max: 22, ac: 14,
  caster: RIG.get("caster") !== "0",
  classes: [{ class_key: "cleric", level: 3 }],
  abilities: { str: 10, dex: 12, con: 13, int: 11, wis: 16, cha: 9 },
};

// `character::Sheet`, by hand. The fields with no analogue in the rig
// are present and empty rather than absent, because absent is what the
// paint faults 295 was about look like.
const SHEET = {
  location_id: "l1", character_id: "c1", game_id: GAME, name: CHAR.name,
  level: 3, classes: [{ class_key: "cleric", level: 3 }], prof_bonus: null,
  abilities: {
    str: ABIL(10), dex: ABIL(12), con: ABIL(13),
    int: ABIL(11), wis: ABIL(16), cha: ABIL(9),
  },
  skills: [
    { key: "athletics", name: "Athletics", ability: "str" },
    { key: "insight", name: "Insight", ability: "wis" },
  ],
  profs: { insight: 1 },
  skill_mods: {
    athletics: { natural: 0, parts: [["dex", 0]], total: 0 },
    insight: { natural: 3, parts: [["wis", 3], ["proficiency", 2]], total: 5 },
  },
  narrative_pack: "default",
  weapon_profs: [], armor_profs: [], tool_profs: [], karma: null,
  species: {
    key: "human", name: "Human", playable: true, size: "medium",
    speed: 30, ability_bonuses: {}, prose: "The many.",
    languages: [], traits: [],
  },
  speed: { walk: 30, squares: 6, resolved: 30, parts: [] },
  body: {
    height_ft: 5.6, weight_lb: 140, hair: "black", skin: "brown",
    eyes: "grey", description: null, languages: [],
  },
  carry_size: "medium", loadout: [], techniques: [],
  vitals: {
    hp_max: 22, hp_temp: 0, hp_temp_max: 0, ac_mode: "auto",
    ac_override: null, death_successes: 0, death_failures: 0,
    exhaustion: 0, inspiration: false, size: "medium",
  },
  save_lines: {
    str: SAVE(0), dex: SAVE(1), con: SAVE(1),
    int: SAVE(0), wis: SAVE(3, true), cha: SAVE(-1, true),
  },
  creature_type: null, resistances: [], armor_class: 15,
  from_statblock: false,
  caster: CHAR.caster
    ? { class_key: "cleric", source: "prayer", caster_level: 3,
        casting_ability: "wis" }
    : null,
  gate_level: 2, spells: [],
};

// `equipment::Owned`, which nests an `Item`. 297's rig round found this
// the hard way: the flat shape an earlier stub used threw in
// `paintSlotLadder` on `it.item.kind`.
const HANDAXE = {
  id: "o1", name: null, quantity: 1, slot: null,
  equipped: false, attuned: false, proficient: false,
  grants: [], modes: [],
  item: {
    key: "handaxe", name: "Handaxe", kind: "weapon",
    base_item: null, weapon_class: "simple", grants: null,
    damage_number: 1, damage_denomination: 6,
    damage_types: ["slashing"], properties: ["light", "thrown"],
    range_reach: 5, range_value: 20, range_long: 60,
    armor_category: null, base_ac: null, dex_cap: null,
    size: "medium", holds_size: null, spell_levels: null,
    cost_cp: 500, weight_lb: 2,
  },
};

const CASTING = {
  source: "prayer", class_key: "cleric", caster_level: 3,
  casting_ability: "wis", save_dc: 13, attack_bonus: 5, top_slot: 2,
  slots: [4, 2], slots_left: [4, 1],
  prepared: [], known: [], catalogue: [],
  cantrips: [], cantrips_known: 3, prepared_max: 4,
};

/* ---------- the breaks ---------- */

// 295. A WAY TO MAKE ONE REGION FAIL, and the reason the shapes are
// wrong rather than the calls throwing: `call` turns a thrown command
// into `null` and logs it, so a refused command proves nothing about
// the paint. The faults worth reproducing are the ones a new column
// causes - a field that is simply not on the row.
//
// Each break is a function over the honest answer.
const BREAKS = {
  // `loadPrayers` does `got.casting_ability.toUpperCase()`.
  prayers: (a) => {
    const c = { ...a };
    delete c.casting_ability;
    return c;
  },
};

const breakIf = (name, answer) =>
  BREAK === name && BREAKS[name] ? BREAKS[name](answer) : answer;

/* ---------- the answers ---------- */

const ANSWERS = {
  // who, and which game
  me: () => ME,
  pin_status: () => ({ set: false }),
  list_games: () => [{ id: GAME, name: "The Rig", dm_uid: DM ? "dm-1" : "dm-x" }],
  list_members: () => [
    { profile_id: ME.profile_id, display_name: "Rig", email: ME.email },
  ],
  // 292's colours are NAMES, not hex. `paintDieColours` builds a class
  // out of the value - `die-` plus it - and `styles.css` has
  // `.d20.die-red`, `.die-yellow`, `.die-white`. A hex here produced
  // the class `die-#f5f5f5`, which is not a valid selector, matched
  // nothing, and fell through to the base rule - so the die still
  // looked right. The fourth fixture this pair of cycles has caught
  // being wrong, and the fourth to look very nearly correct.
  // `?die=` and `?pip=` pick them.
  dice_prefs: () => ({
    die_colour: RIG.get("die") || "white",
    pip_colour: RIG.get("pip") || "black",
  }),

  // the sheet and the regions around it
  get_sheet: () => SHEET,
  list_characters: () => [CHAR],
  // Who the DM can enrol in a fight: the `characters` rows with
  // `is_template` false, which is a narrower select than
  // `list_characters` returns and so gets its own fixture.
  list_individuals: () => [
    { id: "c1", name: "Luci", token_name: null, is_npc: false,
      is_active: true, level: 3, class_key: "cleric",
      npc_key: null, creature_type: null },
  ],
  list_character_images: () => [],
  list_inventory: () => [HANDAXE],
  wallet: () => ({ said: "12 gp", worth: "1200 cp", cp: 0, sp: 0, gp: 12, pp: 0 }),
  encumbrance: () => ({ carried: 2, capacity: 150, burden: "unencumbered" }),
  list_features: () => [
    { key: "channel", name: "Channel Divinity", source: "cleric",
      text: "Once per rest.", uses: null },
  ],
  list_casting: () => breakIf("prayers", CASTING),
  castable: () => [],

  // the reference shelves
  list_classes: () => [{ key: "cleric", name: "Cleric", hit_die: 8 }],
  list_species: () => [
    { key: "human", name: "Human", playable: true, size: "medium",
      speed: 30, ability_bonuses: {}, prose: "The many." },
  ],
  // `{key, name, ability}`, which is what `select=key,name,ability`
  // returns - NOT bare strings, whatever the command's name suggests.
  // `loadSkillPicker` does `s.ability.toUpperCase()`, and the bare-string
  // version of this fixture took the DM's whole Encounter refresh down
  // with it. Reusing SHEET.skills so the two cannot disagree: the stub
  // has its own one-source-of-truth problem to avoid.
  list_skill_keys: () => SHEET.skills,
  list_spells: () => [
    { id: "s1", key: "fireball", name: "Fireball", level: 3,
      school: "evocation", classes: ["wizard"], text: "A bright streak." },
    { id: "s2", key: "cure-wounds", name: "Cure Wounds", level: 1,
      school: "evocation", classes: ["cleric"], text: "A touch." },
  ],
  list_catalogue: () => [
    { key: "handaxe", name: "Handaxe", kind: "weapon", cost_cp: 500, weight: 2 },
    { key: "torch", name: "Torch", kind: "gear", cost_cp: 1, weight: 1 },
  ],
  list_creatures: () => [],
  list_audiences: () => [],
  list_slots: () => [],
  list_conditions: () => [],

  // the world and what is in it
  //
  // `locations::Placed`, and `path` is not optional - `loadDM` does
  // `at.path.join(" > ")` to label an encounter's room. An earlier
  // version of this fixture left it out and the DM's Encounter tab
  // failed with `Cannot read properties of undefined (reading 'join')`,
  // which looked like an app fault for most of a cycle. **THAT IS THE
  // LIMIT THIS FILE'S HEADER WARNS ABOUT**: nothing keeps these shapes
  // in step with the Rust structs, and a field the rig forgot reads
  // exactly like a field the engine lost.
  //
  // 301. THESE TWO ARE THE ONLY FIXTURE THAT CAN BE WRITTEN TO, and
  // they have to be: the whole question `rename_location` raises is
  // whether the new name comes back on the next read, in the tree, the
  // breadcrumb and three pickers at once. A read-only fixture can only
  // show that the click did not throw.
  //
  // The Cellar carries prose on purpose. The fault 301 fixed destroys a
  // description, and a fixture where every description is null cannot
  // see it happen - which is also why every description WAS null here,
  // because every description in the real database still is.
  list_locations: () => PLACES.map((p) => ({ ...p })),
  // 301. A WRITE THAT ACTUALLY WRITES, and one of very few in here.
  //
  // BOTH FIELDS, BECAUSE THE COMMAND WRITES BOTH. Rust does
  // `json!({ "name": ..., "description": description })` in one
  // statement, so this assigns both unconditionally too. A stub that
  // helpfully kept the old description when none was sent would hide
  // the exact fault the cycle exists to fix.
  //
  // `path` is rebuilt because the breadcrumb is derived in Rust, not
  // stored, and a renamed room appears in its children's trails.
  rename_location: ({ locationId, name, description }) => {
    const p = PLACES.find((x) => x.id === locationId);
    if (!p) throw "no such place: " + locationId;
    if (!String(name || "").trim()) throw "a place needs a name";
    const was = p.name;
    p.name = String(name).trim();
    p.description = description === undefined ? null : description;
    for (const q of PLACES) {
      q.path = q.path.map((seg) => (seg === was ? p.name : seg));
    }
    return {};
  },

  list_objects: () => [
    { id: "o1", item_key: "handaxe", name: "Handaxe",
      holder_kind: "character", holder_id: "c1", holder_name: "Luci", qty: 1 },
  ],
  who_is_where: () => [
    { id: "c1", name: "Luci", is_npc: false, location_id: "l1",
      location_name: "Tavern", type: "humanoid" },
  ],
  list_npcs: () => [{ id: "n1", key: "goblin", name: "Goblin", game_id: null }],
  // 237/240's one round trip for six pickers. Answered here because it
  // runs on startup, so without it every rig session opens with a red
  // line that has nothing to do with whatever is being tested.
  terrain_words: () => [
    { key: "rock", sub: true, surface: true, cover: false },
    { key: "grass", sub: false, surface: true, cover: false },
    { key: "wall", sub: false, surface: false, cover: true },
  ],
  // There is no `list_tiles` answer. The command is registered in
  // `lib.rs` and called from nowhere in `src/` - the map paints through
  // `paint_tiles` and `erase_tiles` - so an answer for it would be the
  // rig faking a conversation the app never has. `rig.smoke.mjs`
  // asserts every answer here is actually asked for.

  // time, the fight, the feed
  game_clock: () => ({
    reading: "Luhnstag, 1 Aiberhan SA 2240 · 00:00",
    part: "night", season: "Aiberhan", steps: [],
  }),
  list_effects: () => [],
  // The condition keys that stop somebody acting - a bare array of
  // strings, out of `conditions::ALL`. Also a startup read, same
  // reason as `terrain_words`.
  halting_conditions: () => ["unconscious", "paralyzed", "stunned"],
  list_encounters: () => [
    { id: "e1", name: "Bar brawl", status: "ready", location_id: "l1" },
  ],
  fight_strip: () => ({ name: "", round: 0, order: [] }),
  // 299. `natural_roll` IS WHAT THE DICE PAD NEEDS. Without it
  // `paintDie` says "no d20 rolled yet" and the whole pane - twenty
  // faces, the spin, 292's colours - is unreachable in the rig, which
  // is the one pane nothing but eyes can check.
  //
  // `?nat=` picks the face so a particular one can be looked at, and
  // `face_outcome` is what paints crit green and fumble dark.
  list_rolls: () => {
    const nat = Number(RIG.get("nat")) || 14;
    return [
      { id: "r1", request: "insight", label: "Insight", total: nat + 5,
        natural_roll: nat,
        face_outcome: nat === 20 ? "crit" : nat === 1 ? "fumble" : null,
        character_name: "Luci",
        narrative: "She reads the room.",
        created_at: new Date().toISOString() },
    ];
  },
};

/* ---------- the bridge ---------- */

// Which commands were asked for and never answered. Printed on the
// console at the end of the first second, so an unstubbed read shows up
// as a list rather than as one puzzling blank panel.
const missing = new Set();

window.__TAURI__ = {
  core: {
    invoke: async (cmd, args) => {
      const fn = ANSWERS[cmd];
      if (!fn) {
        missing.add(cmd);
        // THROWS. See the header: a null here is a green tick in the
        // app's own log for a command that did nothing.
        throw "not in the rig: " + cmd;
      }
      return fn(args);
    },
  },
};

addEventListener("load", () => {
  setTimeout(() => {
    if (missing.size) {
      // Concatenated rather than formatted: a `%s` placeholder reads
      // literally in some consoles, which turns a useful list into
      // noise about the harness.
      console.log(
        "[rig] " + missing.size + " command(s) asked for and not " +
        "answered: " + [...missing].sort().join(", ")
      );
    }
  }, 1500);
});

// Straight past the door. The gate is a sign-in and this has no auth;
// every cycle that has used the rig wanted the tabs behind it.
addEventListener("DOMContentLoaded", () => {
  setTimeout(() => {
    for (const id of ["#gate", "#gate-wait", "#unlock-panel",
                      "#auth-panel", "#game-panel"]) {
      const el = document.querySelector(id);
      if (el) el.hidden = true;
    }
    document.body.classList.remove("gated");
  }, 400);
});
