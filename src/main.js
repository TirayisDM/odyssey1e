// odyssey1e — access test rig.
//
// This is a harness, not the app. Its job is to make the four-account
// RLS test from migration 001 fast to run and impossible to misread:
// every call is logged with its raw result, and a denial is shown as
// loudly as a success. Zero rows is the answer we are usually looking
// for, so it gets said out loud rather than rendering as an empty list.

const { invoke } = window.__TAURI__.core;

// TWO ENCOUNTER IDS, ON PURPOSE.
//
// `encounterId` is the ACTIVE encounter, owned by loadTargets: what the
// players can aim at, and what a roll is attributed to. `dmEncounterId`
// is whichever encounter the DM has open for editing, which is often a
// draft nobody can see yet.
//
// They were one variable for about ten minutes. Selecting a draft to
// enrol into, then refreshing the target list, silently snapped the
// DM's selection back to the active encounter — and a roll taken in
// between would have been attributed to the wrong one.
let state = {
  user: null, gameId: null, characterId: null, sheet: null, rolls: [],
  games: [], encounterId: null, dmEncounterId: null, dmTargets: [],
};

/* ---------- logging ---------- */

const logEl = () => document.querySelector("#log");

function log(cmd, value, isError) {
  const el = document.createElement("div");
  el.className = "entry" + (isError ? " err" : "");
  const head = document.createElement("div");
  head.className = "cmd";
  head.textContent = (isError ? "✗ " : "✓ ") + cmd;
  const body = document.createElement("div");
  body.className = "val";

  if (isError) {
    body.textContent = String(value);
  } else if (Array.isArray(value) && value.length === 0) {
    body.className = "val empty";
    body.textContent = "0 rows — nothing visible to this account";
  } else {
    body.textContent = JSON.stringify(value, null, 2);
  }

  el.append(head, body);
  logEl().prepend(el);
}

// A button that WRITES is not allowed to be re-entrant.
//
// Every roll is a fresh set of dice and a fresh row, so a second
// dispatch is not a harmless duplicate — it is a second swing nobody
// took. The death save is worse: two saves from one click, and a
// natural 1 among them is two failures toward dead.
//
// Disabling for the duration makes it impossible from this side, and a
// second dispatch that arrives while the first is in flight is dropped
// rather than queued.
function guarded(selector, fn) {
  const el = document.querySelector(selector);
  let busy = false;
  el.addEventListener("click", async () => {
    if (busy) return;
    busy = true;
    el.disabled = true;
    try {
      await fn();
    } finally {
      busy = false;
      el.disabled = false;
    }
  });
}

// Every command goes through here so nothing can fail silently.
//
// tryCall hands back WHY it failed; call throws that away and returns
// null, which is all most callers want. The DM panel wants the reason,
// because a refusal there is usually the DM being told something true
// about the game — "another encounter is already active" — rather than
// something being broken.
async function tryCall(cmd, args) {
  try {
    const out = await invoke(cmd, args || {});
    log(cmd, out, false);
    return { ok: true, value: out };
  } catch (e) {
    log(cmd, e, true);
    return { ok: false, error: String(e) };
  }
}

async function call(cmd, args) {
  const r = await tryCall(cmd, args);
  return r.ok ? r.value : null;
}

/* ---------- session ---------- */

// Fast login.
//
// Shown INSTEAD of the password form when a PIN is set on this device,
// and it names the account it is about to open - a PIN that silently
// signs you in as someone else is how a DM rolls as a player.
//
// "Use password" is always there. A stored token can expire or be
// revoked, and a door with no other way in is a trap.
async function paintUnlock() {
  const st = await call("pin_status");
  const unlock = document.querySelector("#unlock-panel");
  const auth = document.querySelector("#auth-panel");
  const set = !!(st && st.set) && !state.user;
  unlock.hidden = !set;
  auth.hidden = set;
  if (set) {
    document.querySelector("#unlock-who").textContent = st.email || "";
    document.querySelector("#pin").value = "";
    unlockSay("");
  }
  // Offering to set one only makes sense once there is a session to
  // store, so it lives under the password form and appears after.
  document.querySelector("#pin-setup").hidden = !state.user;
  if (st && st.set) {
    document.querySelector("#clear-pin").hidden = false;
  } else {
    document.querySelector("#clear-pin").hidden = true;
  }
}

// Say what happened to the PIN, beside the buttons that did it.
//
// Both outcomes used to go to the log pane, which is the right place to
// read what happened and the wrong one to NOTICE it. Save and Forget are
// adjacent, identically styled and irreversible in one direction, and
// neither acknowledged a click.
function pinSay(text, isError) {
  const el = document.querySelector("#pin-msg");
  el.textContent = text || "";
  el.className = "dm-msg" + (isError ? " err" : "");
  el.hidden = !text;
}

function unlockSay(text, isError) {
  const el = document.querySelector("#unlock-msg");
  el.textContent = text || "";
  el.className = "dm-msg" + (isError ? " err" : "");
  el.hidden = !text;
}

function paintUser() {
  const who = document.querySelector("#who");
  const out = document.querySelector("#signout");
  const auth = document.querySelector("#auth-panel");
  if (state.user) {
    who.textContent = state.user.email + "  ·  " + state.user.user_id.slice(0, 8);
    who.classList.remove("muted");
    out.hidden = false;
    auth.style.opacity = ".45";
  } else {
    who.textContent = "not signed in";
    who.classList.add("muted");
    out.hidden = true;
    auth.style.opacity = "1";
  }
}

function clearData() {
  state.gameId = null;
  state.characterId = null;
  state.sheet = null;
  state.games = [];
  state.encounterId = null;
  state.dmEncounterId = null;
  document.querySelector("#sheet-panel").hidden = true;
  document.querySelector("#equipment-panel").hidden = true;
  // Signing out as the DM must not leave the DM panel on screen for
  // whoever signs in next.
  for (const id of ["#run-panel", "#world-panel"]) {
    document.querySelector(id).hidden = true;
  }
  state.rolls = [];
  document.querySelector("#games").innerHTML = "";
  document.querySelector("#characters").innerHTML = "";
  document.querySelector("#rolls").innerHTML = "";
}

/* ---------- rendering ---------- */

function row(label, tag, onClick, selected) {
  const li = document.createElement("li");
  if (!onClick) li.className = "flat";
  if (selected) li.classList.add("sel");
  const a = document.createElement("span");
  a.textContent = label;
  const b = document.createElement("span");
  b.className = "tag";
  b.textContent = tag || "";
  li.append(a, b);
  if (onClick) li.addEventListener("click", onClick);
  return li;
}

// The engine's detail string is markdown, inherited from the Discord
// embeds the original posted: **20** for a natural, ~~5~~ for a die the
// keep threw away. dice.rs reproduces it exactly and should keep doing
// so — that string is parity-locked against diceroller.js.
//
// So the rendering happens HERE. Printing it raw put literal asterisks
// on every crit and fumble. Built as nodes rather than innerHTML: the
// string is engine-generated today, and it stays that way whatever ends
// up inside a label tomorrow.
function renderDetail(text) {
  const out = [];
  // One pass, both markers. The capture groups alternate so the split
  // yields plain, bold, plain, struck, plain...
  const parts = String(text).split(/(\*\*[^*]+\*\*|~~[^~]+~~)/g);
  for (const part of parts) {
    if (!part) continue;
    if (part.startsWith("**") && part.endsWith("**")) {
      const b = document.createElement("strong");
      b.className = "nat";
      b.textContent = part.slice(2, -2);
      out.push(b);
    } else if (part.startsWith("~~") && part.endsWith("~~")) {
      const d = document.createElement("s");
      d.className = "dropped";
      d.textContent = part.slice(2, -2);
      out.push(d);
    } else {
      out.push(document.createTextNode(part));
    }
  }
  return out;
}

// An ACTION card — one swing, however many rolls it took.
//
// A to-hit and its damage are one thing that happened, so they get one
// card. Rendering them as two rows would be the same mistake 012 exists
// to fix: nothing could say which damage belonged to which swing.
//
// Takes the action's rolls, to-hit first. The verdict and the prose come
// off the roll that was judged; the damage line sits underneath.
function rollCard(rolls) {
  const r = rolls[0];
  const li = document.createElement("li");
  li.className = "flat card";

  const head = document.createElement("div");
  head.className = "head";
  const who = document.createElement("span");
  who.textContent = (r.character_name || "Someone") + " · " + (r.label || r.request);
  const tag = document.createElement("span");
  tag.className = "tag";
  tag.textContent = r.status;
  head.append(who, tag);

  li.append(head);

  for (const part of rolls) {
    const dice = document.createElement("div");
    dice.className = "dice";
    const numbers =
      part.total === null || part.total === undefined
        ? part.detail || "—"
        : (part.detail ? part.detail + "  =  " : "") + part.total;
    // The damage line says so; the to-hit needs no prefix because the
    // card header already named the swing.
    dice.append(...renderDetail(part.role === "damage" ? "dmg  " + numbers : numbers));
    li.append(dice);
  }

  // The verdict, when the roll had a target. Absent is NOT failure —
  // an untargeted roll simply was not judged, so it says nothing.
  //
  // The reason is spelled out rather than left implied: "hit on a
  // natural 20" beats making someone work out how a total of 12 beat
  // an 18, and the same goes for a fumble that cleared the number.
  if (r.success !== null && r.success !== undefined) {
    const verdict = document.createElement("div");
    verdict.className = "verdict " + (r.success ? "hit" : "miss");

    const vs = r.target_label
      ? `${r.target_label} (${String(r.target_kind).toUpperCase()} ${r.target_value})`
      : `${String(r.target_kind).toUpperCase()} ${r.target_value}`;

    const by =
      r.reason === "auto_hit" ? "hit on a natural " + r.natural_roll
      : r.reason === "auto_miss" ? "missed on a natural " + r.natural_roll
      : r.margin === 0 ? "exactly"
      : r.margin > 0 ? "by " + r.margin
      : "by " + Math.abs(r.margin);

    verdict.textContent =
      (r.success ? "HIT" : "MISS") + " vs " + vs + " — " + by;
    li.append(verdict);
  }

  if (r.narrative) {
    const prose = document.createElement("div");
    prose.className = "prose";
    prose.textContent = r.narrative;
    li.append(prose);
  }

  return li;
}

async function loadGames() {
  const games = await call("list_games");
  const ul = document.querySelector("#games");
  ul.innerHTML = "";
  if (!Array.isArray(games)) return;
  state.games = games;
  for (const g of games) {
    ul.append(
      row(g.name, g.join_code, () => selectGame(g.id), g.id === state.gameId)
    );
  }
}

// Am I the DM of the game I am looking at?
//
// This decides whether to OFFER the DM side, and nothing more. The
// access rule itself lives in 011's policies and is enforced by
// Postgres; a second copy here would be a second place for it to be
// wrong. If this is somehow wrong, the worst case is a panel whose
// buttons come back "only the DM of this game can ...".
function amDM() {
  const g = (state.games || []).find((x) => x.id === state.gameId);
  return !!g && !!state.user && g.dm_uid === state.user.user_id;
}

async function selectGame(id) {
  state.gameId = id;
  state.dmEncounterId = null;
  await loadGames();
  await loadCharacters();
  await loadRolls();
  await loadTargets();
  // BEFORE loadDM, and outside it. The places are member-readable by
  // 033's policy, and a player dropping a torch needs somewhere to drop
  // it as much as the DM does. Loading them only inside the DM panel
  // would leave every player's drop offering nothing but "nowhere",
  // which is the leak this fixes rather than a cosmetic gap.
  await loadPlaces();
  await loadDM();
}

// The places, for anybody. loadWorld paints the DM's editor on top of
// the same list; this is the half a player needs.
async function loadPlaces() {
  state.locations = [];
  if (!state.gameId) return;
  state.locations = (await call("list_locations", { gameId: state.gameId })) || [];
}

// The active encounter's actors and challenges, as things to aim at.
//
// An actor's AC is resolved in Rust, not here — a character's is
// computed from what they are wearing by the same function their own
// sheet uses, so the two cannot disagree. The option carries the
// number so picking one supplies value, kind and label together and a
// half-filled target is impossible.
async function loadTargets() {
  const pick = document.querySelector("#target-pick");
  // Refreshing after a roll rebuilds this list so the hit points move,
  // which would otherwise drop the selection and make the player
  // re-pick the goblin between every swing.
  const wasPicked = pick.value;
  state.targets = [];
  state.encounterId = null;
  pick.innerHTML =
    '<option value="">no target</option><option value="manual">type a number…</option>';
  // Repopulating resets the selection to "no target", so the hand-typed
  // fields have to go back into hiding with it — otherwise switching
  // game leaves them on screen next to a select that says no target.
  document.querySelector("#manual-target").hidden = true;
  if (!state.gameId) return;

  const encounters = await call("list_encounters", { gameId: state.gameId });
  const active = (encounters || []).find((e) => e.status === "active");
  state.encounterId = active ? active.id : null;
  if (!active) return;

  const targets = await call("list_targets", { encounterId: active.id });
  state.targets = targets || [];
  if (!state.targets.length) return;

  for (const kind of ["actor", "challenge"]) {
    const group = state.targets.filter((t) => t.row === kind);
    if (!group.length) continue;
    const og = document.createElement("optgroup");
    og.label = kind === "actor" ? active.name : "Challenges";
    for (const t of group) {
      const o = document.createElement("option");
      o.value = t.id;
      // Hit points where the thing has any, so the goblin visibly drops
      // as it takes damage. A lock has none and says nothing.
      // Down, stable or dead reads on the name, where it belongs —
      // "Goblin 1 — down". Hit points only while they are still worth
      // counting: the event log keeps the honest total, but a dropdown
      // reading -7/7 hp is nonsense.
      const cond =
        t.condition && t.condition !== "conscious" ? " — " + t.condition : "";
      const hp =
        t.hp_current === null || t.hp_current === undefined || t.hp_current <= 0
          ? ""
          : " · " + t.hp_current + "/" + t.hp_max + " hp";
      // The tally, so a player can see how close it is either way.
      const saves =
        t.condition === "down" && (t.death_successes || t.death_failures)
          ? " · " + t.death_successes + "✓ " + t.death_failures + "✗"
          : "";
      o.textContent =
        t.label + cond + " · " + t.target_kind.toUpperCase() + " " + t.value + hp + saves;
      // Why this number is this number, on hover. Same instinct as the
      // proficiency badges: never show a figure with no account of it.
      o.title = t.source;
      og.append(o);
    }
    pick.append(og);
  }

  // Back to whatever was aimed at, if it is still in the encounter.
  if (wasPicked && [...pick.options].some((o) => o.value === wasPicked)) {
    pick.value = wasPicked;
  }
  paintDeathSave();
}

// The death save button belongs to whatever is selected and dying.
// Nothing that is conscious, stable or already dead has one to roll.
function paintDeathSave() {
  const picked = document.querySelector("#target-pick").value;
  const t = (state.targets || []).find((x) => x.id === picked);
  document.querySelector("#death-save").hidden = !(t && t.condition === "down");
}

// This character's row in the active encounter, if they are enrolled.
//
// actions.actor_id is the performer as a participant, which is not the
// same question as whose sheet it was — and it is what an experience
// system would count. Derived from the target list rather than queried,
// because that list already carries every actor's character_id.
function performerActorId() {
  if (!state.characterId) return null;
  const mine = (state.targets || []).find(
    (t) => t.row === "actor" && t.character_id === state.characterId
  );
  return mine ? mine.id : null;
}

async function loadCharacters() {
  const ul = document.querySelector("#characters");
  ul.innerHTML = "";
  if (!state.gameId) return;
  const chars = await call("list_characters", { gameId: state.gameId });
  if (!Array.isArray(chars)) return;
  for (const c of chars) {
    const mine = state.user && c.owner_uid === state.user.user_id;
    // Only your own character is selectable — the sheet editor writes,
    // and the policies would refuse anyway. Better to not offer it.
    ul.append(
      row(
        c.name,
        mine ? "mine" : "someone else's",
        mine ? () => selectCharacter(c.id) : null,
        c.id === state.characterId
      )
    );
  }
}

const ABILS = ["str", "dex", "con", "int", "wis", "cha"];

async function selectCharacter(id) {
  state.characterId = id;
  await loadCharacters();
  await loadSheet();
}

async function loadSheet() {
  const panel = document.querySelector("#sheet-panel");
  if (!state.characterId) { panel.hidden = true; return; }

  const sheet = await call("get_sheet", { characterId: state.characterId });
  if (!sheet) { panel.hidden = true; return; }
  state.sheet = sheet;
  panel.hidden = false;

  const pb = Math.floor((sheet.level - 1) / 4) + 2;

  // AC is the engine's computed number, never the export's flat field —
  // Rodnar's export says 14 and nobody should ever see a 14 here. HP is
  // the max only; current HP will be max less the damage events once
  // those exist, so there is deliberately nothing to show yet.
  const hp = sheet.vitals && sheet.vitals.hp_max ? " · HP " + sheet.vitals.hp_max : "";
  document.querySelector("#sheet-who").textContent =
    sheet.name + " · level " + sheet.level + " · PB +" + pb +
    " · AC " + sheet.armor_class + hp;
  document.querySelector("#level").value = sheet.level;

  // Abilities
  const box = document.querySelector("#abilities");
  box.innerHTML = "";
  for (const code of ABILS) {
    const a = sheet.abilities[code] || { score: 10, save_prof: false };
    const mod = Math.floor((a.score - 10) / 2);

    const el = document.createElement("div");
    el.className = "abil";

    const tag = document.createElement("b");
    tag.textContent = code.toUpperCase();

    const num = document.createElement("input");
    num.type = "number"; num.min = 1; num.max = 30; num.value = a.score;

    const m = document.createElement("span");
    m.className = "mod";
    m.textContent = (mod >= 0 ? "+" : "") + mod;

    const lab = document.createElement("label");
    const chk = document.createElement("input");
    chk.type = "checkbox"; chk.checked = a.save_prof;
    lab.append(chk, document.createTextNode("save"));

    const save = async () => {
      await call("set_ability", {
        characterId: state.characterId,
        ability: code,
        score: Number(num.value),
        saveProf: chk.checked,
      });
      await loadSheet();
    };
    num.addEventListener("change", save);
    chk.addEventListener("change", save);

    el.append(tag, num, m, lab);
    box.append(el);
  }

  // Skills, with the bonus each one currently gives
  const list = document.querySelector("#skills");
  list.innerHTML = "";
  for (const sk of sheet.skills) {
    const prof = sheet.profs[sk.key] ?? 0;
    const abilMod = Math.floor(((sheet.abilities[sk.ability]?.score ?? 10) - 10) / 2);
    const bonus = abilMod + Math.floor(prof * pb);

    const el = document.createElement("div");
    el.className = "skillrow";

    const nm = document.createElement("span");
    nm.textContent = sk.name + " (" + sk.ability.toUpperCase() + ")";

    const sel = document.createElement("select");
    for (const [v, t] of [[0, "—"], [0.5, "half"], [1, "prof"], [2, "exp"]]) {
      const o = document.createElement("option");
      o.value = v; o.textContent = t; o.selected = Number(prof) === v;
      sel.append(o);
    }
    sel.addEventListener("change", async () => {
      await call("set_skill_prof", {
        characterId: state.characterId,
        skillKey: sk.key,
        prof: Number(sel.value),
      });
      await loadSheet();
    });

    const b = document.createElement("span");
    b.className = "bonus";
    b.textContent = (bonus >= 0 ? "+" : "") + bonus;

    el.append(nm, sel, b);
    list.append(el);
  }

  await updatePreview();
  await loadInventory();
}

// The equipment panel.
//
// Every derived answer on a row comes from the engine. Nothing here
// recomputes proficiency or modes in JavaScript — that is the rule
// living in two places, which is the thing the port exists to stop. What
// the panel adds is the EVIDENCE: the character's proficiencies at the
// top and each item's class beside it, so a "not prof" can be read
// rather than taken on trust.
async function loadInventory() {
  const panel = document.querySelector("#equipment-panel");
  const list = document.querySelector("#inventory");
  if (!state.characterId) { panel.hidden = true; return; }

  const items = await call("list_inventory", { characterId: state.characterId });
  panel.hidden = false;
  list.innerHTML = "";

  const sheet = state.sheet || {};
  document.querySelector("#equip-profs").textContent =
    "trained: weapons " + ((sheet.weapon_profs || []).join(", ") || "none") +
    " · armor " + ((sheet.armor_profs || []).join(", ") || "none");

  if (sheet.game_id) await fillCatalogue(document.querySelector("#add-what"), sheet.game_id);

  if (!Array.isArray(items) || items.length === 0) {
    const li = document.createElement("li");
    li.className = "flat muted";
    li.textContent = "carrying nothing";
    list.append(li);
    return;
  }

  // Kept so a row can offer the OTHER rows as somewhere to put itself.
  state.inventory = items;
  for (const it of items) {
    list.append(inventoryRow(it));
  }
}

// Is this row a thing you can put things in?
//
// The catalogue's own `kind`, not a guess. A container is an object
// like any other - it is carried, dropped and stolen - and the only
// thing that marks it out is that since 031 it has an entity, so
// something can point INTO it.
function isContainer(it) {
  return it.item.kind === "container";
}

// Where this object could go: every container the character carries,
// except itself.
//
// Shallow on purpose. A container inside a container is legal and the
// database walks the whole chain to decide whose it is, but a picker
// that offers six levels of nesting is a tree widget, and this is a
// test rig.
function containerChoices(it) {
  return (state.inventory || []).filter((c) => isContainer(c) && c.id !== it.id);
}

// The catalogue, read once per campaign.
//
// It is ~50 global rows plus whatever the game overrides, and it does
// not change while the app is open. Re-reading it on every inventory
// paint would put a request on the wire for a list nobody edited.
async function catalogueFor(gameId) {
  state.catalogue = state.catalogue || {};
  if (!state.catalogue[gameId]) {
    state.catalogue[gameId] = (await call("list_catalogue", { gameId })) || [];
  }
  return state.catalogue[gameId];
}

// Fill a <select> with what can be added, grouped by kind.
//
// The groups are the catalogue's own `kind` column, not a classification
// invented here — the same value the engine branches on when it decides
// whether the one-armor rule applies.
async function fillCatalogue(sel, gameId) {
  const items = await catalogueFor(gameId);
  sel.innerHTML = "";
  const kinds = [];
  for (const it of items) if (!kinds.includes(it.kind)) kinds.push(it.kind);
  for (const k of kinds) {
    const g = document.createElement("optgroup");
    g.label = k;
    for (const it of items.filter((i) => i.kind === k)) {
      const o = document.createElement("option");
      o.value = it.key;
      o.textContent = it.name;
      g.append(o);
    }
    sel.append(g);
  }
  if (!items.length) {
    const o = document.createElement("option");
    o.value = "";
    o.textContent = "nothing in the catalogue";
    sel.append(o);
  }
}

// What can be done to one object, as opposed to WITH it.
//
// Name, drop, destroy. All three are new with 026 and none of them was
// expressible before it: a junction row keyed by type had no name to
// give, no identity to hand over, and nothing to destroy that was not
// also destroying everyone else's.
//
// Drop and destroy are kept apart on purpose. A dropped thing is still
// in the campaign with nobody holding it and can be picked back up; a
// destroyed one is gone, which is why it is the only control here that
// asks first.
function objectControls(it, onDone) {
  const wrap = document.createElement("div");
  wrap.className = "row obj-controls";

  const nameBox = document.createElement("input");
  nameBox.value = it.name || "";
  nameBox.placeholder = it.quantity > 1 ? "split one off to name it" : "name this one";
  nameBox.disabled = it.quantity > 1;

  const nameBtn = document.createElement("button");
  nameBtn.className = "tiny ghost";
  nameBtn.textContent = "name";
  nameBtn.disabled = it.quantity > 1;
  nameBtn.title = "blank clears the name and calls it by its type again";
  nameBtn.addEventListener("click", async () => {
    await call("rename_object", { objectId: it.id, name: nameBox.value });
    await onDone();
  });

  const qty = document.createElement("input");
  qty.type = "number";
  qty.min = "1";
  qty.max = String(it.quantity);
  qty.value = "1";
  qty.className = "narrow";
  qty.hidden = it.quantity <= 1;

  // WHERE IT GOES, not just that it goes.
  //
  // Dropping used to mean holder_id NULL, because until 033 there was
  // nowhere for a dropped thing to be. The cost was invisible until a
  // handaxe sat unheld for a day with nothing able to render it: an
  // object nobody holds and no place contains is not in the world, it
  // is only in the table.
  //
  // Blank still means nowhere. It is a real answer - the DM has not
  // built the room yet - but it is now a CHOICE rather than the only
  // outcome, and the lost-and-found in the World panel is where those
  // end up rather than nothing at all.
  const where = document.createElement("select");
  where.className = "wheretodrop";
  fillPlaces(where, "— nowhere —");

  const dropBtn = document.createElement("button");
  dropBtn.className = "tiny ghost";
  dropBtn.textContent = "drop";
  dropBtn.title = "put it down — pick a place, or nowhere";
  dropBtn.addEventListener("click", async () => {
    const n = it.quantity > 1 ? Number(qty.value) : null;
    // Two commands because they are two events: drop_here puts it in a
    // place, drop_object lets it go. Both split the same way.
    const r = where.value
      ? await tryCall("drop_here", { objectId: it.id, locationId: where.value, quantity: n })
      : await tryCall("drop_object", { objectId: it.id, quantity: n });
    if (!r.ok) log("drop", r.error, true);
    await onDone();
  });

  const killBtn = document.createElement("button");
  killBtn.className = "tiny ghost";
  killBtn.textContent = "destroy";
  killBtn.title = "gone for good — a drop is the reversible one";
  killBtn.addEventListener("click", async () => {
    const what = it.name || it.item.name;
    if (!confirm("Destroy " + what + (it.quantity > 1 ? " ×" + it.quantity : "") + "?")) return;
    await call("destroy_object", { objectId: it.id });
    await onDone();
  });

  wrap.append(nameBox, nameBtn, qty, where, dropBtn, killBtn);

  // Into a container. Only rendered when there is one to choose, so an
  // inventory with no bags looks exactly as it did before 032.
  const into = containerChoices(it);
  if (into.length) {
    const pick = document.createElement("select");
    pick.className = "into";
    for (const c of into) {
      const o = document.createElement("option");
      o.value = c.id;
      o.textContent = c.name || c.item.name;
      pick.append(o);
    }
    const put = document.createElement("button");
    put.className = "tiny ghost";
    put.textContent = "put in";
    put.title = "a coin purse takes only coins";
    put.addEventListener("click", async () => {
      // tryCall, not call: a refusal here is the rule working - the
      // purse saying no to a sword - and it is worth reading.
      const r = await tryCall("put_in_container", {
        objectId: it.id,
        containerId: pick.value,
      });
      if (!r.ok) { alert(r.error); return; }
      await onDone();
    });
    wrap.append(pick, put);
  }
  return wrap;
}

// What an item can do, as things to click.
//
// DERIVED, like the NPC roster's buttons and for the same reason: a
// weapon offers one attack per mode it has, so the Light Hammer offers
// Melee and Thrown because the item carries `thr`. Its techniques come
// off the sheet, already filtered to the EQUIPPED weapons by the engine
// and gated by level here — an unearned technique is not an error and
// not a fallback, so it simply is not offered.
//
// The `request` is the same string that would have been typed. That is
// the point: "Heavy Smash" should never need typing, but what runs is
// the identical path, so nothing can behave differently because it was
// clicked.
function itemActions(it) {
  if (it.item.kind !== "weapon") return [];
  const out = [];

  for (const m of it.modes) {
    const label = m === "melee" ? it.item.name : it.item.name + " (" + m + ")";
    out.push({
      request: label,
      label: m,
      hint: label + " — " + it.item.damage_number + "d" + it.item.damage_denomination +
            (it.proficient ? "" : ", NOT proficient"),
    });
  }

  const level = (state.sheet && state.sheet.level) || 1;
  for (const t of (state.sheet && state.sheet.techniques) || []) {
    if (t.item_key !== it.item.key) continue;
    if (!it.modes.includes(t.mode)) continue;
    if (t.min_level > level) continue;
    out.push({
      request: t.roll_name,
      label: t.name,
      technique: true,
      hint: t.name + " — " + t.dice + " in " + t.mode +
            (t.crit_min !== 20 || t.fumble_max !== 1
              ? ", crit " + t.crit_min + "+, fumble " + t.fumble_max + "-" : ""),
    });
  }

  return out;
}

function inventoryRow(it) {
  const li = document.createElement("li");
  li.className = "flat item" + (it.equipped ? " on" : "");

  // Equip toggle. The one-armor rule is enforced in Rust, so a refusal
  // arrives as an error in the log and the checkbox snaps back.
  const box = document.createElement("input");
  box.type = "checkbox";
  box.checked = it.equipped;
  box.title = "equipped";
  box.addEventListener("change", async () => {
    await call("set_item_equipped", {
      objectId: it.id,
      equipped: box.checked,
    });
    await loadSheet();
  });

  const actions = itemActions(it);

  const name = document.createElement("span");
  name.className = "nm has-actions";
  // ITS OWN NAME WHEN IT HAS ONE. Most swords are just swords, and
  // `name` is null for those; "Runt's Axe" prints as itself with the
  // type kept as a chip below, because what it can do still comes from
  // being a handaxe.
  name.textContent = (it.name || it.item.name) +
                     (it.quantity > 1 ? " ×" + it.quantity : "");
  // Activate the item to see what it can do. Clicking the NAME, not the
  // checkbox beside it — equipping and inspecting are different
  // questions and must not share a hit area.
  // Opens for EVERYTHING now, not just what can swing. A blanket has
  // no attacks and can still be named, dropped and destroyed, and a row
  // that does not open is a row with no way to do any of them.
  name.title = actions.length ? "show what this can do" : "show what can be done with it";
  name.addEventListener("click", () => {
    const open = li.classList.toggle("open");
    detail.hidden = !open;
  });

  // Toggle and name on one line, the engine's verdicts on the next. The
  // left column is narrow and chips wrap badly beside a flexible name.
  const head = document.createElement("div");
  head.className = "head";
  head.append(box, name);

  const tags = document.createElement("span");
  tags.className = "tags";

  // What the engine classified it as — the evidence behind the verdict.
  if (it.name) tags.append(chip(it.item.name, "cls"));
  const cls = it.item.weapon_class || it.item.armor_category;
  if (cls) tags.append(chip(cls, "cls"));

  for (const m of it.modes) tags.append(chip(m, "mode"));

  if (it.item.kind === "weapon" || it.item.kind === "armor") {
    // "flagged" means the source answered explicitly and the engine did
    // not derive anything; the tri-state column is the whole reason
    // those two cases must not look alike.
    const why = it.proficient_override === null ? "derived" : "flagged";
    tags.append(chip((it.proficient ? "proficient" : "not proficient") + " · " + why,
                     it.proficient ? "yes" : "no"));
  }

  if (it.attuned) tags.append(chip("attuned", "att"));
  if (it.uses_max !== null && it.uses_max !== undefined) {
    tags.append(chip((it.uses_max - it.uses_spent) + "/" + it.uses_max + " charges", "use"));
  }

  li.append(head);
  if (tags.childElementCount > 0) li.append(tags);

  // What the item can DO, hidden until asked for. Built now rather than
  // on open so `detail` exists for the click handler above.
  const detail = document.createElement("div");
  detail.className = "actions";
  detail.hidden = true;
  for (const a of actions) {
    const b = document.createElement("button");
    b.className = "tiny" + (a.technique ? " ghost" : "");
    b.textContent = a.label;
    b.title = a.hint;
    // SELECTS, does not roll. Advantage and the target live in the
    // Rolls box and a second set of them here would be two places to
    // get one swing wrong. This removes the typing, which was the ask.
    b.addEventListener("click", () => {
      document.querySelector("#named-request").value = a.request;
      updatePreview();
      document.querySelector("#roll-named").scrollIntoView({ block: "nearest" });
    });
    detail.append(b);
  }
  detail.append(objectControls(it, loadSheet));

  // WHAT IS INSIDE. Loaded when the row is opened rather than with the
  // inventory: a character with five bags would otherwise cost five
  // extra requests on every repaint, for lists nobody has looked at.
  if (isContainer(it)) {
    const inside = document.createElement("div");
    inside.className = "contents";
    inside.textContent = "…";
    detail.append(inside);
    let loaded = false;
    name.addEventListener("click", async () => {
      if (loaded || detail.hidden) return;
      loaded = true;
      await paintContents(inside, it);
    });
  }

  li.append(detail);
  return li;
}

// The inside of one container.
//
// Each line offers the way back out - into the character's own hands -
// which is the inverse move and the same command family. There is no
// nesting here: opening a purse inside a backpack is the purse's own
// row's job, and a rig does not need a tree.
async function paintContents(host, container) {
  host.innerHTML = "";
  const rows = await call("list_contents", { containerId: container.id });
  if (!Array.isArray(rows) || rows.length === 0) {
    host.append(chip("empty", "cls"));
    return;
  }
  for (const r of rows) {
    const line = document.createElement("div");
    line.className = "row inside-row";
    const label = document.createElement("span");
    label.className = "tags";
    label.append(chip((r.name || r.item.name) +
                      (r.quantity > 1 ? " ×" + r.quantity : ""), "mode"));
    const out = document.createElement("button");
    out.className = "tiny ghost";
    out.textContent = "take out";
    out.addEventListener("click", async () => {
      const res = await tryCall("take_from_container", {
        objectId: r.id,
        characterId: state.characterId,
      });
      if (!res.ok) { alert(res.error); return; }
      await loadSheet();
    });
    line.append(label, out);
    host.append(line);
  }
}

function chip(text, kind) {
  const s = document.createElement("span");
  s.className = "chip " + kind;
  s.textContent = text;
  return s;
}

/* ---------- the DM side ---------- */

// WHICH TAB IS SHOWING.
//
// The rig showed every panel for every role at once, which stopped
// being navigable somewhere around the DM screen. Three tabs, because
// there are three things somebody is doing: playing a character,
// running what is in front of the table, or building the world behind
// it.
//
// Show and hide only. No routing, no history, no state beyond which
// button is lit - the panes are the same markup they always were and
// every loader still fills them whether or not they are visible. That
// is deliberate: a tab that only loads when opened is a tab that can be
// stale, and this rig's whole value is that what it shows is what the
// database said.
function showTab(name) {
  for (const b of document.querySelectorAll("#tabs .tab")) {
    b.classList.toggle("on", b.dataset.tab === name);
  }
  for (const p of document.querySelectorAll(".pane")) {
    p.hidden = p.dataset.pane !== name;
  }
  state.tab = name;
}

// The sub-tab strips, one function three strips use.
//
// They differ only in the attribute they switch on, so this takes it as
// an argument rather than being copied three times - which is what the
// scene's version was on its way to becoming. It is still deliberately
// NOT the same function as showTab: the top strip switches panes and
// this one switches panels inside a pane, and one refactor merging them
// is one bug away from clicking PCs closing the Characters tab.
function showSub(strip, attr, name) {
  for (const b of document.querySelectorAll("#" + strip + " .tab")) {
    b.classList.toggle("on", b.dataset[attr] === name);
  }
  for (const p of document.querySelectorAll("." + attr + "-pane")) {
    p.hidden = p.dataset[attr] !== name;
  }
}

// Say it where the DM is looking.
//
// The log pane carries every call and its raw result, which is the right
// tool for reading what happened and the wrong one for noticing that
// something did not. A refused click looked identical to a click that
// did nothing at all — the explanation was there, on the other side of
// the screen, in a list the DM was not watching.
function dmSay(text, isError) {
  const el = document.querySelector("#dm-msg");
  el.textContent = text || "";
  el.className = "dm-msg" + (isError ? " err" : "");
  el.hidden = !text;
}

// The world, as a tree the DM can build.
//
// DEPTH IS THE WHOLE HIERARCHY. 033 stores parent_id and nothing else -
// no level, no path - and locations.rs works both out on the Rust side,
// so the indent here is a rendering of a derived number rather than a
// column anybody has to keep true. The original materialised both and
// its own data disagrees with its own convention.
//
// Selecting a place shows what is lying in it. That is one step, not a
// search: a chest in the room holds its own contents, and "what is on
// the floor" is a different question from "what is in this building
// somewhere".
async function loadWorld() {
  const ul = document.querySelector("#locations");
  const parent = document.querySelector("#loc-parent");
  ul.innerHTML = "";
  state.locations = [];
  parent.innerHTML = '<option value="">— top of the world —</option>';
  if (!state.gameId) return;

  const places = await call("list_locations", { gameId: state.gameId });
  state.locations = places || [];

  for (const l of state.locations) {
    // Two spaces a level. The list is already depth-first, so an indent
    // is all it takes to read as a tree.
    const indent = "\u00a0\u00a0".repeat(l.depth);
    const li = row(indent + l.name, l.kind, () => selectPlace(l.id), l.id === state.placeId);

    const del = document.createElement("button");
    del.className = "tiny ghost";
    del.textContent = "remove";
    del.addEventListener("click", async (ev) => {
      ev.stopPropagation();
      // 033 makes parent_id RESTRICT, so a place with somewhere inside
      // it refuses to go. The command turns that into a sentence.
      const r = await call("delete_location", { locationId: l.id });
      if (r !== null) {
        if (state.placeId === l.id) state.placeId = null;
        dmSay("removed " + l.name);
        await loadWorld();
      } else {
        dmSay("could not remove " + l.name + " — is something inside it?", true);
      }
    });
    li.append(del);
    ul.append(li);

    const o = document.createElement("option");
    o.value = l.id;
    o.textContent = "\u00a0\u00a0".repeat(l.depth) + l.name;
    parent.append(o);
  }

  fillPlaces(document.querySelector("#enc-where"), "— nowhere in particular —");
  // The detail picker too. Its value is set by selectEncounter rather
  // than kept: it shows where THIS encounter is, not what was last
  // chosen on the create form.
  fillPlaces(document.querySelector("#enc-place"), "— nowhere in particular —");
  if (state.dmEncounterId) syncEncPlace(state.dmEncounterId);
  // AND THE SCENE, if one is open. Every write on this tab can change
  // what is in the selected place - putting a loose thing away lands in
  // it, removing a place can close it - so repainting the tree without
  // repainting the scene leaves the lower half a moment out of date.
  await loadScene();
}

// Fill a select with every place, indented by depth.
//
// One function because three controls want the same list - the parent of
// a new place, where an encounter happens, and where a thing is put
// down - and three copies would drift.
function fillPlaces(sel, blankLabel) {
  if (!sel) return sel;
  const keep = sel.value;
  sel.innerHTML = "";
  const none = document.createElement("option");
  none.value = "";
  none.textContent = blankLabel;
  sel.append(none);
  for (const l of state.locations || []) {
    const o = document.createElement("option");
    o.value = l.id;
    o.textContent = "\u00a0\u00a0".repeat(l.depth) + l.name;
    sel.append(o);
  }
  if (keep && [...sel.options].some((o) => o.value === keep)) sel.value = keep;
  // Handed back so a caller can use it in an expression. Every existing
  // caller ignores it.
  return sel;
}

// Open a place. Selecting the one already open closes it again, which
// is the only way back to the bare tree.
async function selectPlace(id) {
  state.placeId = state.placeId === id ? null : id;
  await loadWorld();
}

// Which of the scene's three lists is showing.
//
// The same show-and-hide as showTab and deliberately not a generalised
// version of it: two tab strips sharing one function is one refactor
// away from the World tab closing itself when somebody clicks People.
function showSceneTab(name) {
  showSub("scene-tabs", "scene", name);
  state.sceneTab = name;
}

// What is in a place: who is here, what is happening here, what is
// lying here.
//
// THREE CALLS, NOT ONE. They are three tables with three policies, and
// a combined endpoint would have to decide what a partial answer means
// - a player who can see the room and its contents but not the whole
// roster is a real case once access control lands. Asked separately,
// each list is simply as full as that person is allowed to see.
//
// Counts go in the tab labels rather than only inside the lists,
// because the useful glance at a room is "is anyone in here" and that
// should not cost three clicks.
async function loadScene() {
  const wrap = document.querySelector("#scene");
  const id = state.placeId;
  const here = (state.locations || []).find((l) => l.id === id);
  // A place can vanish under an open scene - removed, or the game
  // switched - and a stale panel is worse than none.
  if (!id || !here) {
    wrap.hidden = true;
    return;
  }
  wrap.hidden = false;
  document.querySelector("#scene-where").textContent = here.path.join(" > ");

  const [roster, fights, things] = await Promise.all([
    call("who_is_where", { gameId: state.gameId }),
    call("encounters_here", { locationId: id }),
    call("location_contents", { locationId: id }),
  ]);

  paintPeople(roster || [], id);
  paintSceneEncounters(fights || []);
  paintThings(things || []);
}

function setCount(sel, n) {
  document.querySelector(sel).textContent = n ? "(" + n + ")" : "";
}

// Who is standing here, and the way to bring somebody else in.
//
// NPCS AND PLAYERS IN ONE LIST, unlike the character picker, which
// filters monsters out so a player's own list is not full of goblins.
// Most of who is in a room is monsters, so here they are the point -
// the tag says which is which.
function paintPeople(roster, id) {
  const ul = document.querySelector("#scene-people");
  ul.innerHTML = "";
  const here = roster.filter((c) => c.location_id === id);
  setCount("#n-people", here.length);

  if (!here.length) ul.append(row("nobody here", "", null));

  for (const c of here) {
    const li = row(c.token_name || c.name, c.is_npc ? "npc" : "player", null);
    if (c.dead) {
      const d = document.createElement("span");
      d.className = "tag";
      d.textContent = "dead";
      li.append(d);
    }
    // Out of the room without saying which room. Somewhere in
    // particular is what the picker in the destination is for.
    const out = document.createElement("button");
    out.className = "tiny ghost";
    out.textContent = "send off";
    out.addEventListener("click", async (ev) => {
      ev.stopPropagation();
      const r = await tryCall("move_character", {
        characterId: c.id,
        locationId: null,
      });
      dmSay(
        r.ok ? (c.token_name || c.name) + " is nowhere in particular" : r.error,
        !r.ok
      );
      await loadWorld();
    });
    li.append(out);
    ul.append(li);
  }

  // The picker names where each person currently is, so moving somebody
  // is not a blind guess at which of three goblins is the one in the
  // corridor.
  const who = document.querySelector("#bring-who");
  const keep = who.value;
  who.innerHTML = "";
  const none = document.createElement("option");
  none.value = "";
  none.textContent = "— who —";
  who.append(none);
  for (const c of roster) {
    if (c.location_id === id) continue;
    const at = (state.locations || []).find((l) => l.id === c.location_id);
    const o = document.createElement("option");
    o.value = c.id;
    o.textContent =
      (c.token_name || c.name) + (at ? " — in " + at.name : " — nowhere");
    who.append(o);
  }
  if (keep && [...who.options].some((o) => o.value === keep)) who.value = keep;
}

// What is happening here. 034 put location_id on encounters and until
// now only the Run tab read it.
function paintSceneEncounters(fights) {
  const ul = document.querySelector("#scene-encounters");
  ul.innerHTML = "";
  setCount("#n-events", fights.length);
  if (!fights.length) {
    ul.append(row("nothing happening here", "", null));
    return;
  }
  for (const e of fights) {
    // Clicking it opens it on the Run tab, because that is where it is
    // run from - the scene says what is here, not how to fight it.
    ul.append(
      row(e.name, e.status, async () => {
        showTab("run");
        await selectEncounter(e.id);
      })
    );
  }
}

// What is lying on the floor. One step, not a search: a chest in the
// room holds its own contents, and opening it is the container panel's
// job.
function paintThings(things) {
  const ul = document.querySelector("#scene-things");
  ul.innerHTML = "";
  setCount("#n-things", things.length);
  if (!things.length) {
    ul.append(row("nothing lying here", "", null));
    return;
  }
  for (const o of things) {
    ul.append(
      row(o.name || o.item_key, o.quantity > 1 ? "x" + o.quantity : "", null)
    );
  }
}

// Point the Where picker at the place this encounter is actually in.
//
// fillPlaces preserves whatever was selected, which is right for a
// create form and wrong here: this control REPORTS a fact and must not
// carry the last encounter's answer over to the next one.
function syncEncPlace(id) {
  const sel = document.querySelector("#enc-place");
  if (!sel) return;
  const e = (state.encounters || []).find((x) => x.id === id);
  sel.value = (e && e.location_id) || "";
}

/* ---------- the character manager ---------- */

// Who exists in this campaign, players and monsters.
//
// A DM TOOL, and behind the same gate as Run and World. A player has
// their own character on the sheet and no business with the roster.
//
// ONE CALL FOR BOTH LISTS. who_is_where already returns everybody with
// is_npc and where they are standing, so the two sub-tabs are two
// filters of one answer rather than two round trips that can disagree.
async function loadChars() {
  const panel = document.querySelector("#chars-panel");
  if (!state.gameId || !amDM()) {
    panel.hidden = true;
    return;
  }
  panel.hidden = false;
  document.querySelector("#chars-who").textContent = "you run this game";

  // Same distinction as loadObjects: "nobody" is an answer, and
  // "the question failed" is not.
  const roster = await tryCall("who_is_where", { gameId: state.gameId });
  if (!roster.ok) {
    dmSay(roster.error, true);
    return;
  }
  const folk = roster.value || [];
  paintFolk("#pc-list", "#n-pcs", folk.filter((c) => !c.is_npc));
  paintFolk("#npc-list", "#n-npcs", folk.filter((c) => c.is_npc));

  // The TYPES, which are a different table and a different idea from
  // the individuals above. 022 is the whole distinction.
  const types = (await call("list_npcs", { gameId: state.gameId })) || [];
  const ul = document.querySelector("#npc-types");
  ul.innerHTML = "";
  if (!types.length) ul.append(row("no statblocks yet", "", null));
  for (const t of types) {
    // A campaign's own row shadows the global one sharing its key, and
    // which one you are looking at decides whether you may edit it.
    const li = row(t.name, t.game_id ? "yours" : "shared", null);
    const bits = [t.species, t.class, t.ac ? "AC " + t.ac : null,
                  t.hp_max ? t.hp_max + " hp" : null]
      .filter(Boolean)
      .join(" \u00b7 ");
    if (bits) {
      const d = document.createElement("span");
      d.className = "tag";
      d.textContent = bits;
      li.append(d);
    }
    ul.append(li);
  }
}

// One list painter for both sub-tabs. They differ by a filter, not by
// shape, and two copies would drift the moment one gained a column.
function paintFolk(listSel, countSel, folk) {
  const ul = document.querySelector(listSel);
  ul.innerHTML = "";
  setCount(countSel, folk.length);
  if (!folk.length) {
    ul.append(row("nobody", "", null));
    return;
  }
  for (const c of folk) {
    const at = (state.locations || []).find((l) => l.id === c.location_id);
    const li = row(c.token_name || c.name, at ? at.name : "nowhere", null);
    if (c.dead) {
      const d = document.createElement("span");
      d.className = "tag";
      d.textContent = "dead";
      li.append(d);
    }
    if (!c.is_active) {
      const d = document.createElement("span");
      d.className = "tag";
      d.textContent = "retired";
      li.append(d);
    }
    // EDITING IS THE SHEET, which already exists and already writes
    // scores, level and skills. A second editor here would be a second
    // place for the same rules to be wrong - and since 022 an NPC IS a
    // character, so the goblin opens in exactly the same screen.
    const open = document.createElement("button");
    open.className = "tiny ghost";
    open.textContent = "open sheet";
    open.addEventListener("click", async (ev) => {
      ev.stopPropagation();
      await selectCharacter(c.id);
      showTab("play");
    });
    li.append(open);
    ul.append(li);
  }
}

/* ---------- the object manager ---------- */

// Every object in the game and who is holding it.
//
// THE VIEW THAT WORKS BACKWARDS. Every other object screen starts from
// a holder and asks what is in it; this one starts from the thing and
// asks where it went, which is the question an actual DM has - "where
// did that handaxe go" - and the one nothing could answer.
//
// The holder is resolved in Rust by holders::resolve, because a
// container is an object and therefore both the question and half the
// answer. It is a fold with tests rather than a join.
async function loadObjects() {
  const panel = document.querySelector("#objects-panel");
  if (!state.gameId || !amDM()) {
    panel.hidden = true;
    return;
  }
  panel.hidden = false;
  document.querySelector("#objects-who").textContent = "you run this game";

  // THE CATALOGUE FIRST. Every row's detail panel reads it to say what
  // a thing IS - damage, properties, the AC it sets - and painting
  // before it arrives would print objects with no facts attached.
  const [cat, objs] = await Promise.all([
    tryCall("list_catalogue", { gameId: state.gameId }),
    tryCall("list_objects", { gameId: state.gameId }),
  ]);

  // A QUESTION THAT FAILED IS NOT AN EMPTY ANSWER. `call` turns both
  // into null, and an empty list paints "no objects yet" - a sentence
  // about the game rather than about a broken query. That is how a
  // malformed select emptied this entire screen without anybody
  // reading it as a fault.
  if (!objs.ok || !cat.ok) {
    document.querySelector("#obj-list").innerHTML = "";
    document.querySelector("#cat-list").innerHTML = "";
    setCount("#n-objs", 0);
    setCount("#n-cat", 0);
    dmSay((objs.ok ? cat.error : objs.error), true);
    return;
  }

  state.catalogue = cat.value || [];
  state.objects = objs.value || [];
  // Who could take something back out of a container. Cheap, and the
  // alternative is the mover asking per row.
  state.roster = (await call("who_is_where", { gameId: state.gameId })) || [];

  paintObjects();
  paintCatalogue();
}

// Filtering happens here and not on the server: the whole list is
// already in hand, and a round trip per keystroke would be slower and
// no more correct.
function paintObjects() {
  const ul = document.querySelector("#obj-list");
  ul.innerHTML = "";
  const find = (document.querySelector("#obj-find").value || "").toLowerCase().trim();
  const where = document.querySelector("#obj-where").value;

  const shown = (state.objects || []).filter((o) => {
    if (where && o.holder_kind !== where) return false;
    if (!find) return true;
    return (
      (o.name || "").toLowerCase().includes(find) ||
      o.item_key.toLowerCase().includes(find) ||
      o.holder_name.toLowerCase().includes(find)
    );
  });

  // The count is of EVERYTHING, not of what survived the filter - a tab
  // label that changes as you type says nothing about the game.
  setCount("#n-objs", (state.objects || []).length);

  if (!shown.length) {
    ul.append(row(find || where ? "nothing matches" : "no objects yet", "", null));
    return;
  }
  for (const o of shown) ul.append(objectRow(o));
}

// One object, with everything you can do to it.
//
// FOUR ACTIONS AND THREE PANELS. The move picker used to sit open on
// every row, which put a select box and a button on thirty lines to
// serve the one being moved. Each action opens its own panel instead,
// and opening one closes the others - a row is doing one thing at a
// time.
function objectRow(o) {
  const li = document.createElement("li");
  li.className = "flat item";

  const head = document.createElement("div");
  head.className = "head";
  const nm = document.createElement("span");
  nm.className = "nm";
  nm.textContent = (o.name || o.item_key) + (o.quantity > 1 ? " x" + o.quantity : "");
  head.append(nm);

  for (const text of [
    o.holder_name,
    // On the row rather than only in the detail: size is the thing that
    // decides whether a move will be refused, and finding that out by
    // being refused is a worse way to learn it.
    sizeWord(o.size),
    o.equipped ? "worn" : null,
    o.is_container ? "holds " + (o.holds_size ? sizeWord(o.holds_size) : "any size") : null,
  ]) {
    if (!text) continue;
    const t = document.createElement("span");
    t.className = "tag";
    t.textContent = text;
    head.append(t);
  }

  // ON THE ROW, not only in the detail. How full a bag is is the thing
  // you want to know while scanning a list of them, and opening each
  // one to find out defeats the list.
  if (o.is_container) head.append(gauge(o.used_slots || 0, o.capacity_slots));

  const detail = panel("detail");
  const editor = panel("editor");
  const mover = panel("mover");
  const panels = [detail, editor, mover];

  function toggle(which, fill) {
    const opening = which.hidden;
    for (const p of panels) p.hidden = true;
    if (!opening) return;
    which.hidden = false;
    if (fill) fill();
  }

  head.append(
    action("view", () => toggle(detail, () => fillDetail(detail, o))),
    action("edit", () => toggle(editor, () => fillEditor(editor, o))),
    // Immediate. There is nothing to ask: the copy is anonymous and,
    // for a container, empty - both decided in clone_object, not here.
    action("clone", async () => {
      const r = await tryCall("clone_object", { objectId: o.id, quantity: 1 });
      dmSay(r.ok ? "another " + (o.item_key) + " made" : r.error, !r.ok);
      await loadObjects();
    }),
    action("move", () => toggle(mover, () => fillMover(mover, o)))
  );

  li.append(head, detail, editor, mover);
  return li;
}

// 036's ladder, and the only copy of it on this side.
//
// Short words in the database, long ones on screen. The database is
// what 010 already spelled for creatures and what every check
// constraint now names; a person reading a screen wants "Large".
const SIZES = [
  ["tiny", "Tiny"],
  ["sm", "Small"],
  ["med", "Medium"],
  ["lg", "Large"],
  ["huge", "Huge"],
  ["grg", "Gargantuan"],
];

function sizeWord(key) {
  const found = SIZES.find(([k]) => k === key);
  // An unrecognised word is PRINTED, not blanked. It means a row this
  // screen does not understand, and hiding it would hide the problem.
  return found ? found[1] : key || "";
}

// A picker over the ladder, with a blank that means something specific.
function sizeSelect(value, blankLabel) {
  const sel = document.createElement("select");
  const none = document.createElement("option");
  none.value = "";
  none.textContent = blankLabel;
  sel.append(none);
  for (const [k, word] of SIZES) {
    const o = document.createElement("option");
    o.value = k;
    o.textContent = word;
    sel.append(o);
  }
  sel.value = value || "";
  return sel;
}

// Pounds, as a number the screen can add up. PostgREST sends numeric as
// a string, and holders.rs keeps it as one deliberately - the rounding
// question belongs here, where there is something to round it for.
function poundsOf(weight, quantity) {
  const each = Number(weight);
  if (!weight || Number.isNaN(each)) return null;
  const total = each * (quantity || 1);
  // Arrows are 0.05lb. Two decimals keeps a quiver honest and stops a
  // longsword reading as "3.00".
  return (Math.round(total * 100) / 100).toString();
}

// How full a container is, as a bar and a number.
//
// THE NUMBERS COME FROM RUST and are the ones the refusal is computed
// from - holders.rs sums the contents with containers::slot_total, the
// same function `fits` uses. A gauge that did its own arithmetic would
// be a gauge that reads half empty while the container says no.
//
// NO CAPACITY RECORDED IS SAID, NOT DRAWN. 032 decided an absent
// capacity is an unfinished catalogue row rather than a bottomless bag,
// and an empty bar would show it as roomy - the opposite of the truth,
// since that container currently refuses everything.
function gauge(used, capacity) {
  const wrap = document.createElement("span");
  wrap.className = "gauge-wrap";

  if (capacity == null) {
    const t = document.createElement("span");
    t.className = "tag warn";
    t.textContent = "no capacity recorded";
    wrap.append(t);
    return wrap;
  }

  const pct = capacity > 0 ? (used / capacity) * 100 : 100;
  const bar = document.createElement("span");
  bar.className = "gauge";
  const fill = document.createElement("span");
  fill.className = "gauge-fill";
  // Clamped, because a bar wider than its track is a layout bug rather
  // than information. The NUMBERS below are not clamped, so an
  // overfull container - possible for rows that predate the check -
  // still says so.
  fill.style.width = Math.min(100, Math.max(0, pct)) + "%";
  if (pct > 100) fill.classList.add("over");
  else if (pct >= 90) fill.classList.add("full");
  bar.append(fill);

  const label = document.createElement("span");
  label.className = "gauge-text";
  // Rounded for reading, not for arithmetic. A purse of coins lands on
  // 4.8 rather than 5 and should not print as "4.800000000000001".
  label.textContent =
    trim(used) + "/" + trim(capacity) + " \u00b7 " + Math.round(pct) + "%";

  wrap.append(bar, label);
  return wrap;
}

// Drop a trailing .0 so whole slots read as whole numbers.
function trim(n) {
  const r = Math.round(n * 100) / 100;
  return String(r);
}

function panel(cls) {
  const d = document.createElement("div");
  d.className = cls;
  d.hidden = true;
  return d;
}

function action(label, fn) {
  const b = document.createElement("button");
  b.className = "tiny ghost";
  b.textContent = label;
  b.addEventListener("click", async (ev) => {
    ev.stopPropagation();
    await fn();
  });
  return b;
}

// What a thing IS, as opposed to where it is.
//
// The catalogue half comes from state.catalogue rather than a fetch:
// it is already loaded, it does not change while the tab is open, and
// asking again per row would be thirty requests to say "dagger".
async function fillDetail(el, o) {
  el.innerHTML = "";
  const item = (state.catalogue || []).find((i) => i.key === o.item_key);

  const facts = [
    ["key", o.item_key],
    ["quantity", String(o.quantity)],
    ["held by", o.holder_name + " (" + o.holder_kind + ")"],
    ["kind", item ? item.kind : "not in the catalogue"],
    // WHOSE ANSWER IT IS, said out loud. A size that came from the
    // catalogue and one a DM set on this particular object are
    // different facts, and a screen that prints them identically hides
    // an edit somebody made.
    [
      "size",
      sizeWord(o.size) +
        (item && o.size !== item.size ? " (set on this one)" : ""),
    ],
  ];

  const pounds = poundsOf(o.weight, o.quantity);
  if (pounds) {
    facts.push([
      "weight",
      o.quantity > 1 ? pounds + " lb for " + o.quantity : pounds + " lb",
    ]);
  }
  if (item && item.damage_number && item.damage_denomination) {
    facts.push([
      "damage",
      item.damage_number + "d" + item.damage_denomination +
        (item.damage_types && item.damage_types.length ? " " + item.damage_types.join("/") : ""),
    ]);
  }
  if (item && item.base_ac != null) facts.push(["armour", "AC " + item.base_ac]);
  if (o.is_container) {
    // THREE THINGS A CONTAINER REFUSES ON, and they are not the same
    // question: what it takes, how big, and how much. 036 added the
    // middle one; the other two have been enforced since 032 and were
    // never shown, so a refusal arrived with no way to have predicted
    // it.
    facts.push(["holds up to", o.holds_size ? sizeWord(o.holds_size) : "any size"]);
    if (item && item.accepts && item.accepts.length) {
      facts.push(["takes only", item.accepts.join(", ")]);
    }
  }
  if (item && item.properties && item.properties.length) {
    facts.push(["properties", item.properties.join(", ")]);
  }
  if (o.equipped) facts.push(["worn", "yes"]);

  for (const [k, v] of facts) {
    const line = document.createElement("div");
    line.className = "fact";
    const a = document.createElement("span");
    a.className = "muted";
    a.textContent = k;
    const b = document.createElement("span");
    b.textContent = v;
    line.append(a, b);
    el.append(line);
  }

  // The gauge rather than the bare capacity: "30 slots" never said how
  // many of them were left, which is the only part anybody was asking.
  // Built after the facts loop because it is an element, not a string.
  if (o.is_container) {
    const line = document.createElement("div");
    line.className = "fact";
    const k = document.createElement("span");
    k.className = "muted";
    k.textContent = "room";
    line.append(k, gauge(o.used_slots || 0, o.capacity_slots));
    el.append(line);
  }

  // A CONTAINER IS A DOOR. What is inside is a live question rather
  // than a property of the row, so it is asked when the row is opened.
  if (!o.is_container) return;
  const inside = await call("list_contents", { containerId: o.id });
  const sub = document.createElement("div");
  sub.className = "sub";
  sub.textContent = "Inside";
  el.append(sub);
  const ul = document.createElement("ul");
  ul.className = "list";
  if (!inside || !inside.length) {
    ul.append(row("empty", "", null));
  } else {
    for (const c of inside) {
      // `item_key` IS NOT A FIELD HERE. list_contents returns
      // equipment::Owned, which carries the whole catalogue row as
      // `item` rather than the bare key the way objects::Stack and
      // holders::Located do. Reading o.item_key off one of these gives
      // undefined, so anything WITHOUT a given name rendered as a blank
      // row - and a named one rendered fine, which is why a backpack
      // holding a Wool Blanket and a set of clothes looked like a
      // backpack holding a Wool Blanket and a ghost.
      const what = c.name || (c.item && c.item.name) || "unnamed";
      const tag = [
        c.quantity > 1 ? "x" + c.quantity : null,
        c.item ? sizeWord(c.item.size) : null,
      ]
        .filter(Boolean)
        .join(" · ");
      ul.append(row(what, tag, null));
    }
  }
  el.append(ul);
}

// Rename it, change how many there are, or destroy it.
//
// NAME AND QUANTITY GO TOGETHER in one write, because they constrain
// each other: a stack of seven cannot carry a name. edit_object checks
// the name against the NEW quantity, which is the only order that can
// refuse naming one dagger and raising it to three in the same breath.
function fillEditor(el, o) {
  el.innerHTML = "";

  const nameIn = document.createElement("input");
  nameIn.value = o.name || "";
  nameIn.placeholder = "name (blank = call it a " + o.item_key + ")";

  const qtyIn = document.createElement("input");
  qtyIn.type = "number";
  qtyIn.min = "1";
  qtyIn.value = String(o.quantity);
  qtyIn.className = "narrow";

  const line = document.createElement("div");
  line.className = "row";
  line.append(nameIn, qtyIn);

  // THE OVERRIDES, and the blank option is the point of them. 036 puts
  // size on the TYPE; this says THIS ONE is different - a giant's
  // dagger, a rodent's backpack - because there is no screen for
  // writing a catalogue row and otherwise the only way is a migration.
  //
  // The catalogue's own answer is named in the blank label, so leaving
  // it alone is an informed choice rather than an empty box.
  const item = (state.catalogue || []).find((i) => i.key === o.item_key);
  const sizeIn = sizeSelect(
    item && o.size === item.size ? "" : o.size,
    "size: as a " + o.item_key + (item ? " (" + sizeWord(item.size) + ")" : "")
  );
  const sizeLine = document.createElement("div");
  sizeLine.className = "row";
  sizeLine.append(sizeIn);

  let holdsIn = null;
  if (o.is_container) {
    const fromType = item && item.holds_size;
    holdsIn = sizeSelect(
      fromType && o.holds_size === fromType ? "" : o.holds_size || "",
      "holds: as a " + o.item_key +
        " (" + (fromType ? sizeWord(fromType) : "any size") + ")"
    );
    sizeLine.append(holdsIn);
  }

  const save = document.createElement("button");
  save.className = "tiny ghost";
  save.textContent = "save";
  save.addEventListener("click", async () => {
    const r = await tryCall("edit_object", {
      objectId: o.id,
      name: nameIn.value,
      quantity: Number(qtyIn.value) || o.quantity,
      // Always sent, because "" is how the command is told to CLEAR an
      // override rather than leave it alone. Omitting the field is what
      // means "do not touch".
      sizeOverride: sizeIn.value,
      holdsSizeOverride: holdsIn ? holdsIn.value : null,
    });
    dmSay(r.ok ? "saved" : r.error, !r.ok);
    if (r.ok) await loadObjects();
  });

  const kill = document.createElement("button");
  kill.className = "tiny ghost danger";
  kill.textContent = "destroy";
  kill.addEventListener("click", async () => {
    // ASKS FIRST. There is no undo and no soft delete - 030 keeps an
    // object alive when its HOLDER dies precisely so that losing one
    // is always somebody's decision.
    if (!confirm("Destroy " + (o.name || o.item_key) + "? This cannot be undone.")) return;
    const r = await tryCall("destroy_object", { objectId: o.id });
    dmSay(r.ok ? (o.name || o.item_key) + " destroyed" : r.error, !r.ok);
    if (r.ok) await loadObjects();
  });

  const buttons = document.createElement("div");
  buttons.className = "row";
  buttons.append(save, kill);
  el.append(line, sizeLine, buttons);
}

// Put it somewhere real, or inside something.
//
// TWO DESTINATIONS, because they are two different writes. A place is
// drop_here or place_object depending on whose the thing is; a
// container is put_in_container, which has three more questions to ask
// before it agrees.
function fillMover(el, o) {
  el.innerHTML = "";

  /* ---- into a place ---- */

  const to = fillPlaces(document.createElement("select"), "\u2014 move to \u2014");
  const go = action("put it there", async () => {
    if (!to.value) return dmSay("pick somewhere to put it", true);
    // WHICH COMMAND DEPENDS ON WHOSE IT IS. drop_here takes a thing out
    // of a pair of hands and splits a stack on the way; place_object
    // moves a whole loose row that nobody is holding. Both end with the
    // thing resting in a place, and they are not interchangeable.
    const held = o.holder_kind === "character" || o.holder_kind === "container";
    const r = await tryCall(held ? "drop_here" : "place_object", {
      objectId: o.id,
      locationId: to.value,
    });
    dmSay(r.ok ? (o.name || o.item_key) + " moved" : r.error, !r.ok);
    if (r.ok) await loadObjects();
  });
  const placeLine = document.createElement("div");
  placeLine.className = "row";
  placeLine.append(to, go);
  el.append(placeLine);

  /* ---- into a container ---- */

  // ONLY WHAT IT CAN ACTUALLY REACH. `reach` is the whole chain of
  // containers walked to its root, done in Rust and tested there - a
  // thing held by Rodnar and a chest on the floor come back with
  // different tokens, so the chest is never offered. An empty token
  // means the chain ran into something nobody can see, and then
  // nothing is offered at all.
  //
  // The rule is not copied here. One string compare IS the rule, which
  // is why holders.rs hands the screen a token rather than a pair of
  // fields to reason about.
  const reachable = (state.objects || []).filter(
    (c) =>
      c.is_container &&
      c.id !== o.id &&
      o.reach &&
      c.reach === o.reach &&
      // A thing already in this container has nowhere to go.
      c.entity_id !== o.holder_id
  );

  const into = document.createElement("select");
  const none = document.createElement("option");
  none.value = "";
  none.textContent = reachable.length
    ? "\u2014 put inside \u2014"
    : "no container within reach of " + (o.reach_name || "here");
  into.append(none);
  for (const c of reachable) {
    const opt = document.createElement("option");
    opt.value = c.id;
    // Say what it will take, because that is what the refusal will be
    // about: a purse that holds Tiny things is worth knowing before
    // the click rather than after it.
    // Say what it takes AND how much room is left, because those are
    // the two things the refusal will be about.
    const room =
      c.capacity_slots == null
        ? "no capacity recorded"
        : trim(Math.max(0, c.capacity_slots - (c.used_slots || 0))) + " free";
    opt.textContent =
      (c.name || c.item_key) +
      " \u2014 " +
      (c.holds_size ? "holds " + sizeWord(c.holds_size) + ", " : "") +
      room;
    into.append(opt);
  }
  into.disabled = !reachable.length;

  const put = action("put it in", async () => {
    if (!into.value) return dmSay("pick a container", true);
    const r = await tryCall("put_in_container", {
      objectId: o.id,
      containerId: into.value,
    });
    dmSay(r.ok ? (o.name || o.item_key) + " packed away" : r.error, !r.ok);
    if (r.ok) await loadObjects();
  });

  const intoLine = document.createElement("div");
  intoLine.className = "row";
  intoLine.append(into, put);
  el.append(intoLine);

  /* ---- back out of one ---- */

  // Only worth offering when it is IN something, and only into the
  // hands of whoever ultimately holds that container - which is the
  // same reach rule read backwards.
  if (o.holder_kind !== "container") return;
  const owner = (state.roster || []).find((c) => "c:" + c.entity_id === o.reach);
  if (!owner) return;

  const outLine = document.createElement("div");
  outLine.className = "row";
  outLine.append(
    action("take it out", async () => {
      const r = await tryCall("take_from_container", {
        objectId: o.id,
        characterId: owner.id,
      });
      dmSay(
        r.ok ? (owner.token_name || owner.name) + " takes the " + o.item_key : r.error,
        !r.ok
      );
      if (r.ok) await loadObjects();
    })
  );
  el.append(outLine);
}


function paintCatalogue() {
  const ul = document.querySelector("#cat-list");
  ul.innerHTML = "";
  const find = (document.querySelector("#cat-find").value || "").toLowerCase().trim();
  const all = state.catalogue || [];
  setCount("#n-cat", all.length);

  const shown = all.filter(
    (i) =>
      !find ||
      i.name.toLowerCase().includes(find) ||
      i.key.toLowerCase().includes(find) ||
      (i.kind || "").toLowerCase().includes(find)
  );
  if (!shown.length) {
    ul.append(row("nothing matches", "", null));
    return;
  }
  for (const i of shown) {
    const li = row(i.name, i.kind || "", null);
    const bits = [sizeWord(i.size)];
    // Seeded by 027 and 032 from the book, and read by nothing until
    // now - equipment.rs said as much in its own header.
    const lb = poundsOf(i.weight, 1);
    if (lb) bits.push(lb + " lb");
    if (i.holds_size) bits.push("holds " + sizeWord(i.holds_size));
    if (i.damage_number && i.damage_denomination) {
      bits.push(i.damage_number + "d" + i.damage_denomination);
    }
    if (i.base_ac != null) bits.push("AC " + i.base_ac);
    if (i.properties && i.properties.length) bits.push(i.properties.join(" "));
    if (bits.length) {
      const d = document.createElement("span");
      d.className = "tag";
      d.textContent = bits.join(" \u00b7 ");
      li.append(d);
    }
    ul.append(li);
  }
}

// Everything an encounter needs was authored in SQL until now. The
// policies have been in place since 011; this is the screen that was
// missing.
//
// Shown only to the DM of the selected game — see amDM(). That is a
// courtesy, not a guard: every one of these commands is refused by
// Postgres for anyone else, and the panel would simply fill the log with
// "only the DM of this game can ...".
async function loadDM() {
  // TWO PANELS NOW, one gate. Running a fight and building the world
  // are different jobs on different tabs; they are still the same
  // permission, and 011's policies are what actually enforce it.
  const run = document.querySelector("#run-panel");
  const world = document.querySelector("#world-panel");
  if (!state.gameId || !amDM()) {
    run.hidden = true;
    world.hidden = true;
    // The managers gate themselves the same way, and are called here so
    // a player switching games does not keep the last DM's roster on a
    // tab they can still click.
    await loadChars();
    await loadObjects();
    return;
  }
  run.hidden = false;
  world.hidden = false;
  document.querySelector("#dm-who").textContent = "you run this game";
  document.querySelector("#world-who").textContent = "you run this game";

  // BEFORE the managers, because both of them print where somebody or
  // something is by looking up state.locations - which loadWorld is
  // what fills.
  await loadWorld();
  await loadChars();
  await loadObjects();

  const encounters = await call("list_encounters", { gameId: state.gameId });
  // Kept so selectEncounter can say WHICH encounter is being edited
  // without refetching the list to find out its name and status.
  state.encounters = encounters || [];
  const ul = document.querySelector("#encounters");
  ul.innerHTML = "";
  for (const e of encounters || []) {
    const li = row(e.name, e.status, () => selectEncounter(e.id), e.id === state.dmEncounterId);
    // The id rides on the element so selecting can re-mark the list
    // without refetching it. Rebuilding on every click would work and
    // would also throw away the DM's scroll position mid-setup.
    li.dataset.encId = e.id;

    // Where it happens, when it happens anywhere.
    if (e.location_id) {
      const at = (state.locations || []).find((l) => l.id === e.location_id);
      if (at) {
        const w = document.createElement("span");
        w.className = "tag";
        w.textContent = at.path.join(" > ");
        li.append(w);
      }
    }
    // draft -> active -> ended, as a button rather than a dropdown: the
    // next state is nearly always the obvious one.
    // ended and cancelled both go back to draft: a finished fight and
    // an abandoned one are equally re-openable, and 034 keeps the two
    // words apart for what they will trigger, not for what they allow.
    const next =
      e.status === "draft" ? "active" : e.status === "active" ? "ended" : "draft";
    const b = document.createElement("button");
    b.className = "tiny ghost";
    b.textContent = "→ " + next;
    // CANCELLED IS NOT ENDED. 034 says why: ended is what will trigger
    // the experience review and the journal entry, and a called-off
    // encounter must earn nobody anything. Offered only while there is
    // something to call off.
    if (e.status === "draft" || e.status === "active") {
      const x = document.createElement("button");
      x.className = "tiny ghost";
      x.textContent = "cancel";
      x.addEventListener("click", async (ev) => {
        ev.stopPropagation();
        const r = await tryCall("set_encounter_status", {
          encounterId: e.id,
          status: "cancelled",
        });
        dmSay(r.ok ? e.name + " called off — nobody earns anything for it" : r.error, !r.ok);
        await loadDM();
      });
      li.append(x);
    }
    b.addEventListener("click", async (ev) => {
      ev.stopPropagation();
      dmSay("");
      const r = await tryCall("set_encounter_status", {
        encounterId: e.id,
        status: next,
      });
      // "Only one active per game" is not an error the DM caused by
      // clicking wrong — it is the rule, and the next move is obvious
      // once it is said out loud.
      if (!r.ok) dmSay(r.error, true);
      else dmSay(e.name + " is now " + next);
      await loadDM();
      await loadTargets();
    });
    li.append(b);
    ul.append(li);
  }

  await loadStatblockPicker();
  await loadSkillPicker();

  // Open on something rather than on nothing.
  //
  // Everything below the encounter list belongs to a selected encounter,
  // so with none selected the panel looks like it only creates them —
  // there is no visible hint that Enrol is one click away. The active
  // encounter is what the table is looking at, so it is the right guess;
  // failing that, the newest, which is what a DM who just hit Create is
  // about to fill.
  //
  // An explicit selection is never overridden: this only fires when
  // nothing is chosen, or when what was chosen has gone.
  const list = encounters || [];
  const stillThere = list.some((e) => e.id === state.dmEncounterId);
  if (!stillThere) {
    const active = list.find((e) => e.status === "active");
    state.dmEncounterId = (active || list[0] || {}).id || null;
  }
  await selectEncounter(state.dmEncounterId);
}

async function selectEncounter(id) {
  state.dmEncounterId = id;

  // Show which one is being edited. The list is built by loadDM and not
  // rebuilt on a click, so the highlight has to be moved here or it
  // never moves at all.
  for (const li of document.querySelectorAll("#encounters li")) {
    li.classList.toggle("sel", !!id && li.dataset.encId === id);
  }

  const detail = document.querySelector("#enc-detail");
  detail.hidden = !id;
  if (!id) return;
  syncEncPlace(id);

  // SAY WHICH ENCOUNTER THIS IS, AND WHETHER ANYONE CAN SEE IT.
  //
  // The panel edits whatever is SELECTED; the players' target list
  // follows whatever is ACTIVE. Those are different encounters more
  // often than not, and the only cue was an outline on a row and a word
  // in small grey text — which was not enough to stop a goblin and two
  // challenges being built into an ended encounter, where nobody could
  // reach them. The form now says so at the point of use.
  const e = (state.encounters || []).find((x) => x.id === id);
  const banner = document.querySelector("#enc-editing");
  if (e) {
    const note =
      e.status === "active" ? "in front of the table"
      : e.status === "draft" ? "players cannot see this yet"
      : "players cannot see this";
    banner.textContent = "editing: " + e.name + " — " + e.status + " · " + note;
    banner.className = "editing" + (e.status === "active" ? " live" : "");
    banner.hidden = false;
  } else {
    banner.hidden = true;
  }

  const roster = await call("list_roster", { encounterId: id });

  // Targets for THIS encounter, not the active one.
  //
  // state.targets belongs to loadTargets and follows whatever is live,
  // which is usually not what the DM has open. A goblin in a draft
  // attacks the people in that draft. Making this a second list rather
  // than reusing the first is the same lesson dmEncounterId taught.
  const dmTargets = await call("list_targets", { encounterId: id });
  state.dmTargets = Array.isArray(dmTargets) ? dmTargets : [];

  const rl = document.querySelector("#roster");
  rl.innerHTML = "";
  for (const a of roster || []) {
    const li = document.createElement("li");
    li.className = "flat item" + (a.active ? " on" : "");

    const head = document.createElement("div");
    head.className = "head";
    const nm = document.createElement("span");
    nm.className = "nm";
    nm.textContent = a.label;
    head.append(nm);

    // Deactivate rather than delete for anything that has rolled: the
    // rolls point at it. Delete is for a mis-click during setup, and the
    // foreign keys refuse it when it is not.
    const tog = document.createElement("button");
    tog.className = "tiny ghost";
    tog.textContent = a.active ? "hide" : "show";
    tog.addEventListener("click", async () => {
      await call("set_actor_active", { actorId: a.id, active: !a.active });
      await selectEncounter(id);
      await loadTargets();
    });
    const del = document.createElement("button");
    del.className = "tiny ghost";
    del.textContent = "remove";
    del.addEventListener("click", async () => {
      await call("remove_actor", { actorId: a.id });
      await selectEncounter(id);
      await loadTargets();
    });
    // View. Opens the same sheet a player gets, because since 022 a
    // goblin IS a character - see viewRow.
    const see = document.createElement("button");
    see.className = "tiny ghost";
    see.textContent = "view";
    see.addEventListener("click", async () => {
      const open = li.classList.toggle("open");
      view.hidden = !open;
      if (open && !view.dataset.loaded) {
        await paintActorView(view, a, id);
        view.dataset.loaded = "1";
      }
    });

    head.append(see, tog, del);
    li.append(head);

    const tags = document.createElement("span");
    tags.className = "tags";
    if (a.npc_key) tags.append(chip(a.npc_key, "cls"));
    if (a.character_id) tags.append(chip("character", "cls"));
    // name_base set means 018 named it rather than a person.
    if (a.name_base) tags.append(chip("auto-named", "use"));
    if (a.dead) tags.append(chip("dead", "no"));
    else if (a.death_failures > 0 || a.death_successes > 0)
      tags.append(chip(a.death_successes + "/" + a.death_failures + " saves", "no"));
    if (tags.childElementCount) li.append(tags);

    // A monster acts. Characters do not get buttons here — their player
    // rolls for them from the sheet, which is the whole point of a
    // player having one.
    if (a.npc_key && a.active && !a.dead) {
      li.append(await attackRow(a, id));
    }

    const view = document.createElement("div");
    view.className = "actorview";
    view.hidden = true;
    li.append(view);

    rl.append(li);
  }

  const challenges = await call("list_challenges", { encounterId: id });
  const cl = document.querySelector("#challenges");
  cl.innerHTML = "";
  for (const c of challenges || []) {
    const li = row(c.label, "DC " + c.dc + (c.skill_key ? " · " + c.skill_key : ""), null);
    const tog = document.createElement("button");
    tog.className = "tiny ghost";
    tog.textContent = c.active ? "hide" : "show";
    tog.addEventListener("click", async () => {
      await call("set_challenge_active", { challengeId: c.id, active: !c.active });
      await selectEncounter(id);
      await loadTargets();
    });
    li.append(tog);
    cl.append(li);
  }
}

// One creature, opened out: rename, vitals, scores, kit.
//
// THE SAME SHEET A PLAYER GETS. 022 made a monster a character, so
// view_actor calls the same load_sheet Rodnar's panel uses and this
// renders what comes back. There is no NPC-shaped view and no second
// set of rules - if a rule changes for a player it has already changed
// for the goblin.
//
// Rendered once and kept: reopening a row should not cost a round trip,
// and nothing in here moves on its own. A rename reloads the roster,
// which rebuilds it.
async function paintActorView(host, actor, encounterId) {
  host.innerHTML = "";
  const data = await call("view_actor", { actorId: actor.id });
  if (!data || !data.sheet) {
    host.append(chip("could not read this one", "no"));
    return;
  }
  const sh = data.sheet;

  // --- name ---
  const nameRow = document.createElement("div");
  nameRow.className = "row";
  const nameBox = document.createElement("input");
  nameBox.value = actor.label;
  nameBox.placeholder = "name";
  const save = document.createElement("button");
  save.className = "tiny";
  save.textContent = "rename";
  let busy = false;
  save.addEventListener("click", async () => {
    if (busy) return;
    busy = true; save.disabled = true;
    try {
      dmSay("");
      const r = await tryCall("rename_actor", { actorId: actor.id, name: nameBox.value });
      if (!r.ok) { dmSay(r.error, true); return; }
      dmSay(actor.label + " is now " + nameBox.value.trim());
      // The roster and the target list both carry the old name.
      await selectEncounter(encounterId);
      await loadTargets();
    } finally { busy = false; save.disabled = false; }
  });
  nameRow.append(nameBox, save);
  host.append(nameRow);

  // --- level, which is HIT DICE ---
  //
  // Not an annotation. A goblin at 2 is 2d6 and a goblin at 6 is 6d6,
  // so moving this rewrites what the creature is - its hit points and
  // its proficiency bonus both fall out of it. The command says what it
  // did rather than the panel guessing, because the hit dice are the
  // fact the other two come from and the one nothing stores.
  //
  // WOUNDS SURVIVE IT. Hit points are a log, so raising the maximum by
  // seven leaves a creature at 3 of 7 sitting at 10 of 14 - still down
  // by four, which is what levelling up means.
  const lvlRow = document.createElement("div");
  lvlRow.className = "row";
  const lvlBox = document.createElement("input");
  lvlBox.type = "number";
  lvlBox.min = "1";
  lvlBox.value = sh.level;
  lvlBox.className = "narrow";
  lvlBox.title = "level is hit dice";
  const setLvl = document.createElement("button");
  setLvl.className = "tiny";
  setLvl.textContent = "set level";
  let lvlBusy = false;
  setLvl.addEventListener("click", async () => {
    if (lvlBusy) return;
    lvlBusy = true; setLvl.disabled = true;
    try {
      dmSay("");
      const r = await tryCall("set_actor_level", {
        actorId: actor.id,
        level: Number(lvlBox.value) || 0,
      });
      if (!r.ok) { dmSay(r.error, true); return; }
      const v = r.value || {};
      dmSay(actor.label + " is level " + v.level + " · " + v.hit_dice +
            " · " + v.hp_max + " hp · PB +" + v.prof_bonus);
      await paintActorView(host, actor, encounterId);
      // The target list prints hit points, so it is now stale.
      await loadTargets();
    } finally { lvlBusy = false; setLvl.disabled = false; }
  });
  lvlRow.append(lvlBox, setLvl);
  host.append(lvlRow);

  // --- vitals, and where each number came from ---
  const pb = sh.prof_bonus !== null && sh.prof_bonus !== undefined
    ? sh.prof_bonus : Math.floor((sh.level - 1) / 4) + 2;
  const vitals = document.createElement("div");
  vitals.className = "tags";
  vitals.append(chip("level " + sh.level, "cls"));
  vitals.append(chip("PB +" + pb + (sh.prof_bonus !== null && sh.prof_bonus !== undefined
    ? " stated" : " from level"), "cls"));
  vitals.append(chip("AC " + sh.armor_class +
    (sh.vitals && sh.vitals.ac_mode === "flat" ? " flat" : " from armour"), "cls"));
  if (data.hp_current !== null && data.hp_current !== undefined) {
    vitals.append(chip(data.hp_current + "/" + (data.hp_max ?? "?") + " hp",
                       data.hp_current > 0 ? "use" : "no"));
  }
  if (actor.npc_key) vitals.append(chip("from " + actor.npc_key, "cls"));
  if (actor.name_ordinal) vitals.append(chip("#" + actor.name_ordinal + " of its kind", "cls"));
  host.append(vitals);

  // --- scores ---
  const abils = document.createElement("div");
  abils.className = "abils";
  for (const code of ABILS) {
    const score = (sh.abilities[code] || {}).score ?? 10;
    const mod = Math.floor((score - 10) / 2);
    const el = document.createElement("div");
    el.className = "abil";
    const tag = document.createElement("b");
    tag.textContent = code.toUpperCase();
    const num = document.createElement("span");
    num.textContent = score;
    const m = document.createElement("span");
    m.className = "mod";
    m.textContent = (mod >= 0 ? "+" : "") + mod;
    el.append(tag, num, m);
    abils.append(el);
  }
  host.append(abils);

  // --- kit, with the engine's verdicts, exactly as the player panel
  // shows them.
  //
  // NO LONGER READ-ONLY. It said equipping a goblin was a different
  // question without an answer; 026 answered it by making an object an
  // object, and `give_item` never asked whether the holder was a player
  // — a goblin has been a characters row since 022, so the same command
  // arms both. What stops a player arming the goblin is the policy, not
  // this panel.
  if (sh.character_id) {
    const add = document.createElement("div");
    add.className = "row";
    const pick = document.createElement("select");
    const go = document.createElement("button");
    go.className = "tiny ghost";
    go.textContent = "give";
    let giving = false;
    go.addEventListener("click", async () => {
      if (giving) return;
      giving = true; go.disabled = true;
      try {
        dmSay("");
        const r = await tryCall("give_item", {
          characterId: sh.character_id,
          itemKey: pick.value,
        });
        if (!r.ok) { dmSay(r.error, true); return; }
        // Unequipped on arrival, so the attack list does not change
        // until somebody says it should.
        dmSay(actor.label + " now carries " + (pick.selectedOptions[0] || {}).textContent);
        await paintActorView(host, actor, encounterId);
      } finally { giving = false; go.disabled = false; }
    });
    add.append(pick, go);
    host.append(add);
    await fillCatalogue(pick, sh.game_id);
  }

  // THE WHOLE INVENTORY, not sh.loadout. A sheet's loadout is what is
  // EQUIPPED, and a DM who has just handed over a longsword would watch
  // the panel repaint without it: give_item arms nobody, deliberately,
  // so the new thing has to be visible before it can be equipped.
  const kit = sh.character_id
    ? (await call("list_inventory", { characterId: sh.character_id })) || []
    : sh.loadout || [];

  if (kit.length) {
    for (const it of kit) {
      host.append(actorKitRow(it, () => paintActorView(host, actor, encounterId)));
    }
  } else {
    host.append(chip("carrying nothing", "cls"));
  }
}

// One thing a monster is carrying, with the DM's controls on it.
//
// NOT inventoryRow. That one offers techniques, and it reads them off
// `state.sheet` — the signed-in player's sheet — so rendering a
// goblin's axe with it would offer Rodnar's Heavy Smash to the goblin.
// The attack buttons a monster actually has are built from its own
// statblock in `attackRow`, which is where they belong.
function actorKitRow(it, onDone) {
  const line = document.createElement("div");
  line.className = "kit";

  const box = document.createElement("input");
  box.type = "checkbox";
  box.checked = it.equipped;
  box.title = "equipped";
  box.addEventListener("change", async () => {
    dmSay("");
    const r = await tryCall("set_item_equipped", { objectId: it.id, equipped: box.checked });
    if (!r.ok) { dmSay(r.error, true); box.checked = !box.checked; return; }
    await onDone();
  });

  const tags = document.createElement("span");
  tags.className = "tags";
  tags.append(chip((it.name || it.item.name) + (it.quantity > 1 ? " ×" + it.quantity : ""),
                   "mode"));
  if (it.name) tags.append(chip(it.item.name, "cls"));
  for (const m of it.modes) tags.append(chip(m, "cls"));
  if (it.item.kind === "weapon" || it.item.kind === "armor") {
    tags.append(chip(it.proficient ? "proficient" : "not proficient",
                     it.proficient ? "yes" : "no"));
  }

  const head = document.createElement("div");
  head.className = "head";
  head.append(box, tags);
  line.append(head, objectControls(it, onDone));
  return line;
}

// One row of attack buttons and a target, for one monster.
//
// The buttons are DERIVED, not configured: list_npc_attacks reads the
// statblock's loadout and offers one per weapon per mode, so a handaxe
// gives Handaxe and Handaxe (Thrown) because it is `thr`. Nothing here
// knows what a handaxe is.
//
// Every button is guarded against a second click for the same reason
// the roll buttons are: a duplicate is not a harmless repeat, it is a
// swing nobody took.
async function attackRow(actor, encounterId) {
  const wrap = document.createElement("div");
  wrap.className = "attacks";

  const attacks = await call("list_npc_attacks", { actorId: actor.id });
  if (!Array.isArray(attacks) || !attacks.length) {
    const none = document.createElement("span");
    none.className = "chip";
    none.textContent = "no weapon";
    wrap.append(none);
    return wrap;
  }

  // EVERYTHING IN THIS ENCOUNTER, INCLUDING THE ATTACKER.
  //
  // This filtered out the monster itself, on the assumption a goblin
  // should not hit itself. That was a game rule decided in JavaScript,
  // and the wrong one: plenty of actions target the actor taking them —
  // healing, a buff, a defensive stance — and the discriminator is the
  // KIND of action, not who is pointing at whom. A player's roll box
  // never filtered self either, so the two disagreed.
  //
  // Allowed here until there is a type to filter on. See the note in
  // STATUS.md under architecture decisions.
  const pick = document.createElement("select");
  for (const t of state.dmTargets) {
    const o = document.createElement("option");
    o.value = JSON.stringify({
      id: t.id, row: t.row, kind: t.target_kind,
      value: t.value, label: t.label, characterId: t.character_id,
    });
    o.textContent =
      t.label + " · " + String(t.target_kind).toUpperCase() + " " + t.value +
      (t.hp_current !== null && t.hp_current !== undefined
        ? " · " + t.hp_current + "/" + t.hp_max + "hp" : "");
    pick.append(o);
  }
  if (!pick.childElementCount) {
    const o = document.createElement("option");
    o.textContent = "nothing to attack";
    pick.append(o);
    pick.disabled = true;
  }
  wrap.append(pick);

  for (const atk of attacks) {
    const b = document.createElement("button");
    b.className = "tiny";
    b.textContent = atk.request;
    if (atk.proficient === false) b.title = "not proficient — no proficiency bonus";
    let busy = false;
    b.addEventListener("click", async () => {
      if (busy || pick.disabled) return;
      busy = true;
      b.disabled = true;
      try {
        const t = JSON.parse(pick.value);
        dmSay("");
        const r = await tryCall("npc_attack", {
          actorId: actor.id,
          request: atk.request,
          mode: "normal",
          targetValue: t.value,
          targetKind: t.kind,
          targetLabel: t.label,
          encounterId,
          targetId: t.id,
          targetRow: t.row,
          targetCharacterId: t.characterId || null,
        });
        if (!r.ok) dmSay(r.error, true);
        else dmSay(actor.label + " attacks " + t.label);
        await loadRolls();
        await loadTargets();
        await selectEncounter(encounterId);
      } finally {
        busy = false;
        b.disabled = false;
      }
    });
    wrap.append(b);
  }

  return wrap;
}

// What can be enrolled: every statblock, then every character in the
// game. One picker, because enrolment is one question.
async function loadStatblockPicker() {
  const sel = document.querySelector("#enrol-what");
  sel.innerHTML = "";
  const npcs = await call("list_npcs", { gameId: state.gameId });
  for (const n of npcs || []) {
    const o = document.createElement("option");
    o.value = "npc:" + n.key;
    const bits = [n.species, n.class].filter(Boolean).join(" ");
    o.textContent = n.name + " · AC " + n.ac + " · " + n.hp_max + "hp" +
      (bits && bits !== n.name ? " · " + bits : "");
    sel.append(o);
  }
  const chars = await call("list_characters", { gameId: state.gameId });
  for (const c of chars || []) {
    const o = document.createElement("option");
    o.value = "chr:" + c.id;
    o.textContent = c.name + " · character";
    sel.append(o);
  }
}

async function loadSkillPicker() {
  const sel = document.querySelector("#chal-skill");
  sel.innerHTML = "";
  const none = document.createElement("option");
  none.value = "";
  none.textContent = "no suggested skill";
  sel.append(none);
  const skills = await call("list_skill_keys", { gameId: state.gameId });
  for (const s of skills || []) {
    const o = document.createElement("option");
    o.value = s.key;
    o.textContent = s.name + " (" + s.ability.toUpperCase() + ")";
    sel.append(o);
  }
}

/// Show what the request would roll, before committing to it.
async function updatePreview() {
  const el = document.querySelector("#preview");
  const req = val("#named-request");
  if (!state.characterId || !req) {
    el.textContent = "—";
    el.classList.remove("live");
    return;
  }
  try {
    const r = await invoke("preview_request", {
      characterId: state.characterId,
      request: req,
      mode: document.querySelector("#mode").value,
    });
    // An attack shows its damage and, when it applies, the reason its
    // to-hit is lower than the character might expect. A number with no
    // account of itself is the thing this app keeps refusing to show.
    let line = r.label + "  →  " + r.formula;
    if (r.attack) {
      line += "   dmg " + r.attack.damage;
      line += r.attack.proficient
        ? "   " + r.attack.ability.toUpperCase() + " +" + r.attack.ability_mod +
          ", prof +" + r.attack.proficiency_bonus
        : "   " + r.attack.ability.toUpperCase() + " +" + r.attack.ability_mod +
          ", NOT proficient";
      if (r.attack.crit_min !== 20 || r.attack.fumble_max !== 1) {
        line += "   crit " + r.attack.crit_min + "+, fumble " + r.attack.fumble_max + "-";
      }
    }
    el.textContent = line;
    el.classList.add("live");
  } catch (e) {
    el.textContent = String(e);
    el.classList.remove("live");
  }
}

async function loadRolls() {
  const ul = document.querySelector("#rolls");
  ul.innerHTML = "";
  if (!state.gameId) return;
  const rolls = await call("list_rolls", { gameId: state.gameId });
  state.rolls = Array.isArray(rolls) ? rolls : [];

  // Group by action, preserving the newest-first order the query gave
  // us. A roll written before 012 has no action_id and stands alone,
  // which is the third legal state the schema allows for.
  const groups = [];
  const byAction = new Map();
  for (const r of state.rolls) {
    if (!r.action_id) {
      groups.push([r]);
      continue;
    }
    const seen = byAction.get(r.action_id);
    if (seen) {
      seen.push(r);
    } else {
      const g = [r];
      byAction.set(r.action_id, g);
      groups.push(g);
    }
  }

  // Damage reads under its to-hit, never above it. The two land in the
  // same insert so created_at cannot be relied on to order them.
  const order = { to_hit: 0, check: 0, damage: 1 };
  for (const g of groups) {
    g.sort((a, b) => (order[a.role] ?? 0) - (order[b.role] ?? 0));
    ul.append(rollCard(g));
  }
}

/* ---------- wiring ---------- */

const val = (id) => document.querySelector(id).value.trim();

window.addEventListener("DOMContentLoaded", async () => {
  state.user = await call("me");
  paintUser();
  // Decide which door to show before anything else is drawn.
  await paintUnlock();
  if (state.user) await loadGames();

  document.querySelector("#signin").addEventListener("click", async () => {
    const u = await call("sign_in", { email: val("#email"), password: val("#password") });
    if (u) { state.user = u; paintUser(); clearData(); await paintUnlock(); await loadGames(); }
  });

  document.querySelector("#signup").addEventListener("click", async () => {
    const u = await call("sign_up", {
      email: val("#email"),
      password: val("#password"),
      displayName: val("#display") || "Adventurer",
    });
    if (u) { state.user = u; paintUser(); clearData(); await paintUnlock(); await loadGames(); }
  });

  document.querySelector("#signout").addEventListener("click", async () => {
    await call("sign_out");
    state.user = null;
    paintUser();
    clearData();
    // Signing out returns to the unlock screen if a PIN is set. The PIN
    // is NOT forgotten - signing out is "not me right now", and
    // forgetting the device is a separate, deliberate button.
    await paintUnlock();
  });

  document.querySelector("#refresh-games").addEventListener("click", loadGames);

  document.querySelector("#create-game").addEventListener("click", async () => {
    const g = await call("create_game", { name: val("#game-name") || "Untitled campaign" });
    if (g) await loadGames();
  });

  document.querySelector("#join-game").addEventListener("click", async () => {
    const id = await call("join_game", { code: val("#join-code") });
    if (id) await selectGame(id);
  });

  document.querySelector("#create-char").addEventListener("click", async () => {
    if (!state.gameId) return log("create_character", "select a game first", true);
    const c = await call("create_character", {
      gameId: state.gameId,
      name: val("#char-name") || "Unnamed",
      tokenName: null,
    });
    if (c) await loadCharacters();
  });

  document.querySelector("#create-roll").addEventListener("click", async () => {
    if (!state.gameId) return log("create_roll", "select a game first", true);
    // Attach to my own character in this game when I have one — that is
    // what makes the character_name snapshot trigger do its job.
    const chars = await invoke("list_characters", { gameId: state.gameId }).catch(() => []);
    const mine = (chars || []).find((c) => state.user && c.owner_uid === state.user.user_id);
    const r = await call("create_roll", {
      gameId: state.gameId,
      characterId: mine ? mine.id : null,
      request: val("#roll-request") || "insight",
      label: null,
      // Blank formula leaves the row pending; a formula is rolled on
      // this device before the insert, so the row arrives resolved.
      formula: val("#roll-formula") || null,
    });
    if (r) await loadRolls();
  });

  document.querySelector("#patch-roll").addEventListener("click", async () => {
    if (!state.rolls.length) return log("patch_roll_narrative", "no rolls loaded", true);
    const newest = state.rolls[0];
    const out = await call("patch_roll_narrative", {
      rollId: newest.id,
      narrative: val("#patch-narr") || "The stone holds the truth.",
    });
    if (out) await loadRolls();
  });

  document.querySelector("#save-level").addEventListener("click", async () => {
    if (!state.characterId) return log("set_level", "select a character first", true);
    const ok = await call("set_level", {
      characterId: state.characterId,
      level: Number(val("#level")),
    });
    if (ok) await loadSheet();
  });

  // Preview updates as you type — the point is to see the modifier
  // before you commit, not after.
  document.querySelector("#named-request").addEventListener("input", updatePreview);
  document.querySelector("#mode").addEventListener("change", updatePreview);

  // The hand-typed fields only exist for the case with no encounter to
  // pick from, so they stay out of the way until asked for.
  document.querySelector("#target-pick").addEventListener("change", (e) => {
    document.querySelector("#manual-target").hidden = e.target.value !== "manual";
    paintDeathSave();
  });

  // One death saving throw for whatever is selected and dying.
  guarded("#death-save", async () => {
    const picked = document.querySelector("#target-pick").value;
    const t = (state.targets || []).find((x) => x.id === picked);
    if (!t) return log("death_save", "pick something that is down first", true);
    const r = await call("death_save", {
      gameId: state.gameId,
      actorId: t.id,
      encounterId: state.encounterId || null,
    });
    if (r) {
      await loadRolls();
      await loadTargets();
      paintDeathSave();
    }
  });

  guarded("#roll-named", async () => {
    if (!state.characterId) return log("roll_named", "select a character first", true);
    // Three cases: nothing picked, a real target off the encounter, or
    // a hand-typed one. Half a target is refused in Rust rather than
    // guessed at, so there is no point assembling one here.
    const picked = document.querySelector("#target-pick").value;
    let target = null;

    if (picked === "manual") {
      const kind = document.querySelector("#target-kind").value;
      const raw = val("#target-value");
      if (raw === "")
        return log("roll_named", "type a number to beat, or choose no target", true);
      target = { value: Number(raw), kind, label: val("#target-label") || null };
    } else if (picked) {
      const t = (state.targets || []).find((x) => x.id === picked);
      if (!t) return log("roll_named", "that target is no longer in the encounter", true);
      target = {
        value: t.value,
        kind: t.target_kind,
        label: t.label,
        // The link half. The three above are the snapshot half, which is
        // what the roll keeps; these are how the damage finds its way to
        // the right goblin.
        id: t.id,
        row: t.row,
        characterId: t.character_id || null,
      };
    }

    const r = await call("roll_named", {
      characterId: state.characterId,
      request: val("#named-request") || "insight",
      mode: document.querySelector("#mode").value,
      targetValue: target ? target.value : null,
      targetKind: target ? target.kind : null,
      targetLabel: target ? target.label : null,
      // Which encounter this happened in, so the action can be found
      // again by scene rather than only by time.
      encounterId: state.encounterId || null,
      targetId: target ? target.id || null : null,
      targetRow: target ? target.row || null : null,
      targetCharacterId: target ? target.characterId || null : null,
      // Who swung, as a participant. Found from the target list, which
      // already knows which actor row belongs to this character, so it
      // costs no extra query.
      actorId: performerActorId(),
    });
    if (r) {
      await loadRolls();
      // Damage just landed, so the target list is stale — the goblin has
      // fewer hit points than the dropdown is showing.
      await loadTargets();
    }
  });

  // The integrity check no foreign key can do. An empty list is the
  // passing answer, and the log says so out loud rather than rendering
  // as nothing — same convention as a zero-row RLS result.
  document.querySelector("#check-keys").addEventListener("click", async () => {
    if (!state.gameId) return log("check_item_keys", "select a game first", true);
    const faults = await call("check_item_keys", { gameId: state.gameId });
    if (Array.isArray(faults) && faults.length === 0) {
      log("check_item_keys", "every technique item_key and mode resolves", false);
    }
  });

  // Guarded, because a double-click is a second longsword. It merges
  // into the same stack rather than making a second row, which makes it
  // quieter than a duplicate enrolment and no more wanted.
  guarded("#add-item", async () => {
    if (!state.characterId) return log("give_item", "select a character first", true);
    const key = val("#add-what");
    if (!key) return log("give_item", "nothing selected", true);
    const r = await tryCall("give_item", {
      characterId: state.characterId,
      itemKey: key,
      quantity: Number(val("#add-qty")) || 1,
    });
    if (r.ok) await loadSheet();
  });

  /* ---------- the DM side ---------- */
  // All writes, so all guarded: a double-click on Enrol is a second
  // goblin nobody asked for, and 018 will dutifully number it.

  guarded("#create-encounter", async () => {
    if (!state.gameId) return log("create_encounter", "select a game first", true);
    dmSay("");
    const r = await tryCall("create_encounter", {
      gameId: state.gameId,
      name: val("#enc-name"),
      locationId: document.querySelector("#enc-where").value || null,
    });
    if (r.ok) document.querySelector("#enc-name").value = "";
    else dmSay(r.error, true);
    await loadDM();
  });

  guarded("#enrol", async () => {
    if (!state.dmEncounterId) return log("enrol_actor", "select an encounter first", true);
    const pick = document.querySelector("#enrol-what").value;
    const isNpc = pick.startsWith("npc:");
    dmSay("");
    const r = await tryCall("enrol_actor", {
      encounterId: state.dmEncounterId,
      npcKey: isNpc ? pick.slice(4) : null,
      characterId: isNpc ? null : pick.slice(4),
      // Blank on purpose is the normal case: 018 names it.
      label: val("#enrol-name") || null,
    });
    if (r.ok) document.querySelector("#enrol-name").value = "";
    else dmSay(r.error, true);
    await selectEncounter(state.dmEncounterId);
    await loadTargets();
  });

  guarded("#add-challenge", async () => {
    if (!state.dmEncounterId) return log("add_challenge", "select an encounter first", true);
    dmSay("");
    const r = await tryCall("add_challenge", {
      encounterId: state.dmEncounterId,
      label: val("#chal-label"),
      dc: Number(val("#chal-dc") || 0),
      skillKey: document.querySelector("#chal-skill").value || null,
    });
    if (!r.ok) dmSay(r.error, true);
    if (r.ok) {
      document.querySelector("#chal-label").value = "";
      document.querySelector("#chal-dc").value = "";
    }
    await selectEncounter(state.dmEncounterId);
    await loadTargets();
  });

  guarded("#create-location", async () => {
    if (!state.gameId) return log("create_location", "select a game first", true);
    dmSay("");
    const r = await tryCall("create_location", {
      gameId: state.gameId,
      name: val("#loc-name"),
      kind: document.querySelector("#loc-kind").value,
      // Blank is the top of the world, not a missing answer.
      parentId: document.querySelector("#loc-parent").value || null,
      description: null,
    });
    if (r.ok) {
      document.querySelector("#loc-name").value = "";
      await loadWorld();
      dmSay("added");
    } else {
      // A cycle and a cross-game parent are both refused by 033's
      // triggers, and both arrive here as the sentence they raised.
      dmSay(r.error, true);
    }
  });

  guarded("#create-npc", async () => {
    if (!state.gameId) return log("create_npc", "select a game first", true);
    dmSay("");
    const r = await tryCall("create_npc", {
      gameId: state.gameId,
      key: val("#npc-key"),
      name: val("#npc-name"),
      ac: Number(val("#npc-ac") || 0),
      // null, not 0: blank means "derive it", and 0 would be a stated
      // maximum the command is right to refuse.
      hpMax: val("#npc-hp") ? Number(val("#npc-hp")) : null,
      size: val("#npc-size") || null,
      level: val("#npc-level") ? Number(val("#npc-level")) : null,
      species: val("#npc-species") || null,
      class: val("#npc-class") || null,
      weaponProfs: val("#npc-wprof") || null,
      armorProfs: val("#npc-aprof") || null,
    });
    if (r.ok) {
      for (const id of ["#npc-key", "#npc-name", "#npc-species", "#npc-class", "#npc-ac", "#npc-hp"]) {
        document.querySelector(id).value = "";
      }
      await loadStatblockPicker();
      dmSay("statblock written");
    } else {
      dmSay(r.error, true);
    }
  });

  /* ---------- fast login ---------- */

  guarded("#do-unlock", async () => {
    unlockSay("");
    const r = await tryCall("unlock", { pinCode: val("#pin") });
    if (!r.ok) {
      unlockSay(r.error, true);
      document.querySelector("#pin").value = "";
      return;
    }
    state.user = r.value;
    paintUser();
    await paintUnlock();
    await loadGames();
  });

  // Always available. A stored token that has expired must not lock
  // someone out of their own app.
  document.querySelector("#use-password").addEventListener("click", async () => {
    document.querySelector("#unlock-panel").hidden = true;
    document.querySelector("#auth-panel").hidden = false;
  });

  guarded("#save-pin", async () => {
    const r = await tryCall("set_pin", { pinCode: val("#new-pin") });
    if (!r.ok) {
      // AND THE BOX KEEPS ITS CONTENTS. It was cleared before the
      // result was checked, so a PIN rejected for being three digits
      // vanished without a word and looked exactly like one that saved.
      pinSay(r.error, true);
      return;
    }
    document.querySelector("#new-pin").value = "";
    pinSay("PIN saved \u2014 it will be offered next time the app opens");
    await paintUnlock();
  });

  guarded("#clear-pin", async () => {
    // ASKS FIRST. It sits beside Save, looks identical to it, and
    // throws away the stored token for good - after which the only way
    // back in is the password.
    if (!confirm("Forget the PIN on this device? You will need your password next time.")) {
      return;
    }
    const r = await tryCall("forget_pin");
    pinSay(r.ok ? "PIN forgotten \u2014 sign in with your password" : r.error, !r.ok);
    await paintUnlock();
  });

  // Enter submits, because four digits and a reach for the mouse is not
  // faster than a password.
  document.querySelector("#pin").addEventListener("keydown", (e) => {
    if (e.key === "Enter") document.querySelector("#do-unlock").click();
  });

  for (const b of document.querySelectorAll("#tabs .tab")) {
    b.addEventListener("click", () => showTab(b.dataset.tab));
  }

  // 034 built set_encounter_location and nothing ever called it. The
  // create form's picker was the only way to say where a fight
  // happened, so every encounter made before that migration was stuck
  // nowhere - which is why the scene's Encounters tab was empty and
  // right to be.
  guarded("#set-enc-place", async () => {
    if (!state.dmEncounterId) return dmSay("pick an encounter first", true);
    const to = document.querySelector("#enc-place").value || null;
    const r = await tryCall("set_encounter_location", {
      encounterId: state.dmEncounterId,
      locationId: to,
    });
    if (!r.ok) return dmSay(r.error, true);
    const at = (state.locations || []).find((l) => l.id === to);
    dmSay(at ? "happens in " + at.path.join(" > ") : "happens nowhere in particular");
    // loadDM, not loadWorld: the encounter list carries location_id and
    // the scene reads it, so both halves have to come back.
    await loadDM();
    await selectEncounter(state.dmEncounterId);
  });

  for (const b of document.querySelectorAll("#scene-tabs .tab")) {
    b.addEventListener("click", () => showSceneTab(b.dataset.scene));
  }

  for (const b of document.querySelectorAll("#chars-tabs .tab")) {
    b.addEventListener("click", () => showSub("chars-tabs", "chars", b.dataset.chars));
  }

  for (const b of document.querySelectorAll("#objects-tabs .tab")) {
    b.addEventListener("click", () =>
      showSub("objects-tabs", "objects", b.dataset.objects)
    );
  }

  // Filtering is local - the whole list is already here - so it repaints
  // on every keystroke without asking the database anything.
  document.querySelector("#obj-find").addEventListener("input", paintObjects);
  document.querySelector("#obj-where").addEventListener("change", paintObjects);
  document.querySelector("#cat-find").addEventListener("input", paintCatalogue);

  guarded("#bring-here", async () => {
    const who = document.querySelector("#bring-who");
    if (!who.value) return dmSay("pick somebody to bring", true);
    if (!state.placeId) return dmSay("open a place first", true);
    const r = await tryCall("move_character", {
      characterId: who.value,
      locationId: state.placeId,
    });
    dmSay(r.ok ? "moved" : r.error, !r.ok);
    await loadWorld();
  });

  document.querySelector("#clear-log").addEventListener("click", () => {
    logEl().innerHTML = "";
  });
});
