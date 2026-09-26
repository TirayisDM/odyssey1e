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
  // 055. The class catalogue for this game, globals plus any the
  // table has written for itself.
  classes: [],
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
  // 055: twelve rows, so the creation picker is ready before
  // anybody opens it.
  await loadClasses();
  await loadRolls();
  await loadTargets();
  // BEFORE loadDM, and outside it. The places are member-readable by
  // 033's policy and a player's own screen needs them: the drop
  // confirmation names the room they are standing in, and it can only
  // do that if the tree is loaded. It used to be needed for a place
  // PICKER on every inventory row, which is gone - that listed every
  // room in the world on a player's sheet, which is a DM act wearing a
  // player's control.
  await loadPlaces();
  await loadDM();
  // AFTER the roster loads, because the two pickers are built from it.
  // Not inside loadDM: trading is not a DM-only act, and a player
  // swapping a rope with another player is the case that would have
  // been quietly locked behind the DM panel.
  await loadTrade();
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

/* ============================ CLASSES ============================ */

// 055. Loaded once per game, because the catalogue is twelve rows that
// only move when a DM writes their own.
async function loadClasses() {
  const sel = document.querySelector("#char-class");
  if (!sel || !state.gameId) return;
  const rows = await invoke("list_classes", { gameId: state.gameId }).catch(() => []);
  state.classes = rows || [];
  const keep = sel.value;
  sel.innerHTML = "";
  sel.append(new Option("no class yet", ""));
  for (const c of state.classes) {
    sel.append(new Option(c.name + " \u00b7 d" + c.hit_die, c.key));
  }
  // A repaint must not silently change what somebody already chose.
  if (keep && state.classes.some((c) => c.key === keep)) sel.value = keep;
  paintClassNote();
}

function paintClassNote() {
  const note = document.querySelector("#char-class-note");
  const sel = document.querySelector("#char-class");
  if (!note || !sel) return;
  const c = (state.classes || []).find((x) => x.key === sel.value);
  if (!c) {
    note.textContent = "no class means no hit die, and therefore no hit points";
    return;
  }
  // The level-1 maximum, worked the same way vitality::pc_hp works it -
  // the full die, because 5e maxes the first one. Constitution is not
  // known until the row exists and its abilities are seeded, so this
  // says what the die gives and the sheet says what they ended up with.
  const saves = (c.saving_throws || []).join("/");
  note.textContent =
    "d" + c.hit_die + " \u00b7 " + c.hit_die + " hp at level 1 before CON" +
    (saves ? " \u00b7 saves " + saves : "") +
    (c.skill_choices ? " \u00b7 " + c.skill_choices + " skills" : "");
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
  // NONE IS WORTH SAYING LOUDLY. A character trained with nothing is
  // not proficient with anything they pick up, and every attack comes
  // back short by the proficiency bonus with no explanation on screen.
  // Three characters in this game were in that state.
  const weapons = sheet.weapon_profs || [];
  const armour = sheet.armor_profs || [];
  const text = document.querySelector("#profs-text");
  text.textContent =
    "trained: weapons " + (weapons.join(", ") || "none") +
    " \u00b7 armor " + (armour.join(", ") || "none");
  text.classList.toggle("warn", !weapons.length && !armour.length);

  // The boxes hold what is there, so editing is a correction rather
  // than a retype.
  document.querySelector("#prof-weapons").value = weapons.join(" ");
  document.querySelector("#prof-armour").value = armour.join(" ");
  profSay("");

  if (sheet.game_id) await fillCatalogue(document.querySelector("#add-what"), sheet.game_id);

  // WHAT THEY ARE CARRYING. Asked here rather than read off the sheet:
  // it walks everything held at any depth, which is the right cost for
  // an inventory screen and the wrong one ahead of a d20.
  // COIN, beside the weight. Both are sums over everything held at any
  // depth, and a purse in a backpack is the case that makes them worth
  // asking the engine for rather than adding up on screen.
  const money = await call("wallet", { characterId: state.characterId });
  const purseEl = document.querySelector("#purse");
  // What is in it, with what it is worth on hover. Two different
  // questions - 18 gp and 8 sp is 1880 cp, which normalises to "1 pp,
  // 8 gp, 8 sp" and names a coin nobody is carrying.
  purseEl.textContent = money ? "purse: " + money.said : "";
  if (money) purseEl.title = "worth " + money.worth;

  const load = await call("encumbrance", { characterId: state.characterId });
  const burdenEl = document.querySelector("#burden");
  if (load) {
    burdenEl.textContent = "carrying " + load.carried + " lb of " + load.capacity +
                           " · " + load.burden;
    burdenEl.className = "muted trained burden " +
      (load.burden === "unencumbered" ? "ok" : "over");
  } else {
    burdenEl.textContent = "";
  }

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
// "Rodnar is dropping 3 Rations at The Frostvalley Inn."
//
// A GUESS, and the engine's own sentence replaces it in the log once
// the drop lands. This one exists to be read BEFORE, which means it has
// to be composed from what the screen already has - the sheet's name
// and location, and the place picker's own labels.
function dropLine(it, n) {
  const sheet = state.sheet || {};
  const who = sheet.name || "This character";
  const thing = it.name || it.item.name;
  const many = n && n > 1 ? n + " " : "";

  // WHERE THEY STAND, and nowhere else. The only question left is
  // whether they are standing anywhere at all, which 035 keeps as a
  // real answer rather than a misfiling.
  const here = (state.locations || []).find((l) => l.id === sheet.location_id);
  const place = here ? here.path.join(" > ") : "nowhere in particular";
  return who + " is dropping " + many + thing + " at " + place + ".";
}

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
  // DROPPING IS PUTTING IT DOWN WHERE YOU ARE. There used to be a
  // place picker here and it listed EVERY room in the world, on a
  // player's own sheet, beside the word "drop" - which reads as an
  // offer to put a carried sword down in a tavern nobody is standing
  // in. That is a DM act and it already has a home: Objects -> move,
  // which does places, containers and quantities together.
  //
  // So the picker is gone rather than defaulted. Two controls for one
  // act, one of them on the wrong screen, is the duplication this repo
  // keeps removing.
  //
  // A character standing nowhere still drops into nowhere, which 035
  // keeps as a real answer.
  const dropBtn = document.createElement("button");
  dropBtn.className = "tiny ghost";
  dropBtn.textContent = "drop";
  dropBtn.title = "put it down where you are standing";
  dropBtn.addEventListener("click", async () => {
    const n = it.quantity > 1 ? Number(qty.value) : null;

    // SAY IT BEFORE DOING IT. The floor is derived from the character
    // rather than chosen, so the sentence names it before the write.
    if (!confirm(dropLine(it, n))) return;

    const r = await tryCall("drop_object", { objectId: it.id, quantity: n });
    if (!r.ok) { log("drop", r.error, true); return; }
    // What the ENGINE says happened, which is the authority on which
    // floor it landed on. The line above was this screen's guess.
    if (r.value && r.value.said) log("drop", r.value.said, false);
    await onDone();
  });

  // ATTUNEMENT, and only where it could apply. Offered on anything
  // flagged `mgc` and on anything already attuned - the catalogue has
  // no "requires attunement" column, so `mgc` is the closest honest
  // signal and an already-attuned thing must always be releasable.
  const magical = (it.item.properties || []).includes("mgc");
  if (magical || it.attuned) {
    const att = document.createElement("button");
    att.className = "tiny ghost";
    att.textContent = it.attuned ? "break attunement" : "attune";
    att.title = "three at once, across everything you carry";
    att.addEventListener("click", async () => {
      const r = await tryCall("set_item_attuned", {
        objectId: it.id,
        attuned: !it.attuned,
      });
      if (!r.ok) { alert(r.error); return; }
      await onDone();
    });
    wrap.append(att);
  }

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

  wrap.append(nameBox, nameBtn, qty, dropBtn, killBtn);

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
    tags.append(chip((it.proficient ? "proficient" : "not proficient") + " \u00b7 " + why,
                     it.proficient ? "yes" : "no"));

    // THE ESCAPE HATCH, cycling through all THREE states rather than
    // toggling two. instantiate_npc stamps TRUE on every item in a
    // statblock's kit, so without a way back to "derive" a goblin's
    // weapon is proficient for ever - and "derive" is not the same
    // answer as "no", because training the character later changes one
    // and not the other.
    const next =
      it.proficient_override === null ? true : it.proficient_override ? false : null;
    const word = next === null ? "derive" : next ? "yes" : "no";
    const cycle = document.createElement("button");
    cycle.className = "tiny ghost";
    cycle.textContent = "set " + word;
    cycle.title = "override this one item, or hand the question back to the rule";
    cycle.addEventListener("click", async (ev) => {
      ev.stopPropagation();
      const r = await tryCall("set_object_proficient", {
        objectId: it.id,
        proficient: next,
      });
      if (!r.ok) return profSay(r.error, true);
      await loadInventory();
    });
    tags.append(cycle);
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


/* ---------- trade ---------- */

// TWO SIDES, ONE SCREEN. Buy, sell, barter and give are the same
// exchange with different columns filled in, so there are no modes
// here - what makes it a purchase is that only the right column has
// anything in it, and what makes it a shop is that the counterparty
// has a markup.
//
// Nothing is priced on this side. Every number comes from quote_trade,
// because the valuation is asymmetric (a shop buys at 0.40 and sells at
// markup) and a screen that worked that out for itself would be a
// second pricing rule to keep in step.

function tradeSay(text, isError) {
  const el = document.querySelector("#trade-msg");
  el.textContent = text || "";
  el.className = "dm-msg" + (isError ? " err" : "");
  el.hidden = !text;
}

async function loadTrade() {
  if (!state.gameId) return;
  const who = (await call("who_is_where", { gameId: state.gameId })) || [];
  for (const id of ["#trade-a", "#trade-b"]) {
    const sel = document.querySelector(id);
    const keep = sel.value;
    sel.innerHTML = "";
    const blank = document.createElement("option");
    blank.value = "";
    blank.textContent = id === "#trade-a" ? "— who is trading —" : "— with whom —";
    sel.append(blank);
    for (const c of who) {
      const o = document.createElement("option");
      o.value = c.id;
      o.textContent = c.name;
      sel.append(o);
    }
    if (keep) sel.value = keep;
  }
  await paintTrade();
}

// One side's goods, as things to offer.
//
// Reuses list_inventory, which is the same call the equipment panel
// makes - a merchant's stock is an inventory like anyone else's, which
// is the whole reason a shop needed almost no new schema.
async function paintTradeSide(listId, characterId) {
  const host = document.querySelector(listId);
  host.innerHTML = "";
  if (!characterId) return;
  const items = (await call("list_inventory", { characterId })) || [];
  if (!items.length) {
    const li = document.createElement("li");
    li.className = "flat muted";
    li.textContent = "carrying nothing";
    host.append(li);
    return;
  }
  for (const it of items) {
    const li = document.createElement("li");
    li.className = "flat offer";

    const box = document.createElement("input");
    box.type = "checkbox";
    box.dataset.object = it.id;
    box.addEventListener("change", refreshBalance);

    const name = document.createElement("span");
    name.className = "nm";
    name.textContent = it.name || it.item.name;

    // A stack can be offered in part - eight of twenty arrows, six of
    // eighteen gold. A sword cannot, and does not get the box.
    const qty = document.createElement("input");
    qty.type = "number";
    qty.min = "1";
    qty.max = String(it.quantity);
    qty.value = String(it.quantity);
    qty.className = "narrow";
    qty.hidden = it.quantity <= 1;
    qty.dataset.count = "1";
    qty.addEventListener("change", refreshBalance);

    li.append(box, name, qty);
    host.append(li);
  }
}

// What each side has ticked.
function offersFrom(listId) {
  const out = [];
  for (const li of document.querySelectorAll(listId + " li")) {
    const box = li.querySelector("input[type=checkbox]");
    if (!box || !box.checked) continue;
    const qty = li.querySelector("input[type=number]");
    out.push({
      object: box.dataset.object,
      count: qty && !qty.hidden ? Number(qty.value) : null,
    });
  }
  return out;
}

async function paintTrade() {
  const a = val("#trade-a");
  const b = val("#trade-b");
  document.querySelector("#trade-a-name").textContent =
    a ? (document.querySelector("#trade-a").selectedOptions[0].textContent + " gives") : "this side gives";
  document.querySelector("#trade-b-name").textContent =
    b ? (document.querySelector("#trade-b").selectedOptions[0].textContent + " gives") : "that side gives";
  await paintTradeSide("#trade-a-items", a);
  await paintTradeSide("#trade-b-items", b);
  state.haggle = null;
  document.querySelector("#haggle-said").textContent = "";
  await refreshBalance();
}

// The balance, from the engine.
//
// Called on every tick and every quantity change, which is a round trip
// per click - acceptable on a screen somebody is deliberating over, and
// the alternative is this file learning what a shop's margin is.
async function refreshBalance() {
  const a = val("#trade-a");
  const b = val("#trade-b");
  const el = document.querySelector("#trade-balance");
  const kind = document.querySelector("#trade-kind");
  if (!a || !b) {
    el.textContent = "pick two sides";
    el.className = "balance";
    kind.textContent = "";
    return;
  }
  const r = await tryCall("quote_trade", {
    aId: a,
    bId: b,
    aGives: offersFrom("#trade-a-items"),
    bGives: offersFrom("#trade-b-items"),
    haggleMargin: state.haggle,
  });
  if (!r.ok) {
    el.textContent = r.error;
    el.className = "balance err";
    kind.textContent = "";
    return;
  }
  const q = r.value;
  kind.textContent = q.counterparty === "merchant"
    ? "a shop · " + q.disposition
    : "between two people — list price both ways";
  el.textContent = q.said;
  el.className = "balance " + (q.owed_cp > 0 ? "owe" : q.owed_cp < 0 ? "gain" : "even");
  // Haggling only means something against a shop.
  document.querySelector("#haggle-row").hidden = q.counterparty !== "merchant";
}

async function doTrade(free) {
  const a = val("#trade-a");
  const b = val("#trade-b");
  if (!a || !b) return tradeSay("pick two sides first", true);
  tradeSay("");
  const r = await tryCall("execute_trade", {
    aId: a,
    bId: b,
    aGives: offersFrom("#trade-a-items"),
    bGives: offersFrom("#trade-b-items"),
    haggleMargin: state.haggle,
    free: !!free,
  });
  if (!r.ok) return tradeSay(r.error, true);
  tradeSay(r.value.said);
  await paintTrade();
  // The sheet's purse and weight both moved.
  await loadSheet();
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


// SHOPKEEPING, on the row where you'd look for it.
//
// 040 gave characters a markup and a disposition to serve store::quote
// and left nothing that could write them, so every merchant existed
// only inside a rolled-back probe. This is that door.
//
// ON EVERY ROW. This was NPC-only for one commit, on the reasoning
// that a player character running a shop was a case nobody had. The
// first thing that happened was a character called Merchant 1, made
// through "Create character" - which is the obvious way to make a
// person, and a shopkeeper is a person. The guess cost one screenshot.
//
// A BLANK MARKUP IS THE OFF SWITCH, matching 040's nullable column:
// "not a merchant" is the absence of a price, not a price of zero.
function shopControl(c, onDone) {
  const wrap = document.createElement("div");
  wrap.className = "row shop";

  const markup = document.createElement("input");
  markup.type = "number";
  markup.step = "0.05";
  markup.min = "0";
  markup.className = "narrow";
  markup.placeholder = "markup";
  markup.title = "1.0 is list price · blank means not a merchant";
  if (c.markup !== null && c.markup !== undefined) markup.value = c.markup;

  const disp = document.createElement("select");
  for (const [v, label] of [
    ["", "— neutral —"],
    ["allied", "allied · 0.80"],
    ["friendly", "friendly · 0.90"],
    ["warm", "warm · 0.95"],
    ["neutral", "neutral · 1.00"],
    ["unfriendly", "unfriendly · 1.10"],
    ["hostile", "hostile · 1.25"],
    ["sworn_enemy", "sworn enemy · refuses"],
  ]) {
    const o = document.createElement("option");
    o.value = v;
    o.textContent = label;
    disp.append(o);
  }
  if (c.disposition) disp.value = c.disposition;

  const save = document.createElement("button");
  save.className = "tiny ghost";
  save.textContent = "shop";
  let busy = false;
  save.addEventListener("click", async () => {
    if (busy) return;
    busy = true; save.disabled = true;
    try {
      dmSay("");
      const r = await tryCall("set_merchant", {
        characterId: c.id,
        markup: markup.value === "" ? null : Number(markup.value),
        disposition: disp.value || null,
      });
      if (!r.ok) { dmSay(r.error, true); return; }
      const m = Number(markup.value);
      dmSay(markup.value === ""
        ? (c.name + " is no longer a merchant")
        : (c.name + " sells at " + markup.value + "x" +
           // A markup under 1 is a DISCOUNT on list, which is legal and
           // is usually a slip for 1.0. Said out loud rather than
           // refused: a charitable shop is a real thing.
           (m < 1 ? " — " + Math.round((1 - m) * 100) + "% BELOW list" : "") +
           (disp.value ? " · " + disp.value : "")));
      await onDone();
    } finally { busy = false; save.disabled = false; }
  });

  wrap.append(markup, disp, save);
  return wrap;
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

    // WHICH LIST IT BELONGS IN, and it is a label rather than a rule -
    // 022 is explicit about that. A merchant can be a shopkeeper the DM
    // runs or somebody's character who keeps a shop, and nothing but
    // this sorts them.
    const move = document.createElement("button");
    move.className = "tiny ghost";
    move.textContent = c.is_npc ? "make a PC" : "make an NPC";
    move.title = c.is_npc
      ? "show in every player's character picker"
      : "hide from every player's character picker";
    move.addEventListener("click", async (ev) => {
      ev.stopPropagation();
      const r = await tryCall("set_is_npc", { characterId: c.id, isNpc: !c.is_npc });
      if (!r.ok) return dmSay(r.error, true);
      dmSay((c.token_name || c.name) + (c.is_npc ? " is a player character" : " is an NPC"));
      // A character that just became an NPC leaves list_characters, so
      // the strip picker has to be repainted or it keeps offering one
      // that is no longer there.
      await loadCharacters();
      await loadChars();
    });
    li.append(move);

    // EVERY row, not just monsters. This was NPC-only for one commit on
    // the reasoning that a player character running a shop was a case
    // nobody had - and the first thing anybody did was make a character
    // called Merchant 1. A shopkeeper is a person before they are a
    // monster, and "Create character" is the obvious way to make one.
    if (c.markup !== null && c.markup !== undefined) {
      const tag = document.createElement("span");
      tag.className = "tag shop-tag";
      tag.textContent = "shop " + c.markup + "x";
      li.append(tag);
    }
    const wrap = document.createElement("div");
    wrap.className = "folk-row";
    wrap.append(li, shopControl(c, loadChars));
    ul.append(wrap);
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

// "3 coin_gp" or "the Wool Blanket" - what actually left, so a partial
// move reads as one.
function moved(o, n) {
  const what = o.name || o.item_key;
  const all = n == null || n >= o.quantity;
  if (all && o.quantity === 1) return what;
  return (all ? o.quantity : n) + " " + what;
}

function profSay(text, isError) {
  const el = document.querySelector("#profs-msg");
  el.textContent = text || "";
  el.className = "dm-msg" + (isError ? " err" : "");
  el.hidden = !text;
}

// The special attacks this weapon offers, and whether THIS one can
// still reach them.
//
// THE TECHNIQUES BELONG TO THE TYPE AND THE MODES BELONG TO THE OBJECT.
// 043 pinned every technique to a mode; equipment::modes derives an
// object's modes from its properties; and 049 lets an object override
// those. So a greatsword reforged without `thr` really loses its thrown
// techniques, and object_techniques works that out with the same merge
// the sheet and the manager use.
//
// UNREACHABLE ONES ARE SHOWN, NOT HIDDEN, which is 043's own argument
// turned into a screen: "a technique written in an unreachable mode is
// not an error anywhere - it is simply never offered, which is the
// worst kind of bug to find."
//
// AND 050 MADE THEM EDITABLE PER OBJECT. Three scopes - the rulebook,
// this campaign, this sword - and the scope tag says which one is
// answering, because "the global Zwerchhau" and "this sword's own" look
// identical once merged and a DM about to change one should know which.
async function fillMoves(el, o) {
  const moves = await call("object_techniques", { objectId: o.id });
  if (moves === null) return;

  const sub = document.createElement("div");
  sub.className = "sub";
  sub.textContent = "Special attacks";
  el.append(sub);

  const ul = document.createElement("ul");
  ul.className = "list";
  for (const m of moves) ul.append(moveRow(el, o, m));
  if (!moves.length) ul.append(row("no special attacks", "", null));
  el.append(ul);

  // A MOVE NOTHING ELSE HAS. The whole point of 050's third scope, and
  // it needs its own control because every other button on this panel
  // acts on a move that already exists.
  const add = action("add a move", () => {
    const form = el.querySelector(".move-new");
    form.hidden = !form.hidden;
  });
  const wrap = document.createElement("div");
  wrap.className = "row";
  wrap.append(add);
  el.append(wrap);

  const form = moveForm(o, null, () => reopenDetail(el, o));
  form.classList.add("move-new");
  form.hidden = true;
  el.append(form);
}

// Repaint the whole detail panel after a write, because a change to one
// move can alter another - striking out an override restores the
// inherited one underneath it, which is a different row.
async function reopenDetail(el, o) {
  await loadObjects();
  const fresh = (state.objects || []).find((x) => x.id === o.id) || o;
  el.innerHTML = "";
  await fillDetail(el, fresh);
}

function moveRow(panelEl, o, m) {
  const t = m.technique;
  const li = document.createElement("li");
  li.className = "flat item" + (m.offered ? "" : " unreachable");

  const head = document.createElement("div");
  head.className = "head";
  const nm = document.createElement("span");
  nm.className = "nm";
  nm.textContent = t.name;
  head.append(nm);

  for (const [text, cls] of [
    [t.dice, "tag"],
    ["level " + t.min_level, "tag"],
    [m.needs, "tag"],
    // Only when the house rule has moved them off 20 and 1, because
    // the default on every row is noise.
    [t.crit_min < 20 ? "crit " + t.crit_min + "+" : null, "tag"],
    [t.fumble_max > 1 ? "fumble " + t.fumble_max + "-" : null, "tag"],
    // WHOSE ANSWER IT IS. Only worth saying when it is not the
    // rulebook's - "global" on every row would be noise, and the two
    // that matter are the ones somebody edited.
    [t.scope === "object" ? "this one only" : t.scope === "game" ? "this campaign" : null, "tag"],
    [m.offered ? null : "this one cannot: no " + m.needs, "tag warn"],
  ]) {
    if (!text) continue;
    const g = document.createElement("span");
    g.className = cls;
    g.textContent = text;
    head.append(g);
  }

  const form = moveForm(o, t, () => reopenDetail(panelEl, o));
  form.hidden = true;

  head.append(
    action("edit", () => {
      form.hidden = !form.hidden;
    }),
    action("remove", async () => {
      if (!confirm("Take " + t.name + " away from this one?")) return;
      const r = await tryCall("remove_object_technique", { objectId: o.id, key: t.key });
      dmSay(r.ok ? t.name + " struck from this one" : r.error, !r.ok);
      if (r.ok) await reopenDetail(panelEl, o);
    })
  );

  // Only an overridden move has something to go back TO. Offering
  // "revert" on the rulebook's own row would be a button that does
  // nothing.
  if (t.scope === "object") {
    head.append(
      action("revert", async () => {
        const r = await tryCall("clear_object_technique", { objectId: o.id, key: t.key });
        dmSay(r.ok ? t.name + " back to the type" : r.error, !r.ok);
        if (r.ok) await reopenDetail(panelEl, o);
      })
    );
  }

  li.append(head);
  if (t.special_text) {
    const prose = document.createElement("div");
    prose.className = "prose-line";
    prose.textContent = t.special_text;
    li.append(prose);
  }
  li.append(form);
  return li;
}

// One form for editing a move and for adding one.
//
// `t` null means a new move: the key is left for the command to mint,
// so naming it after an existing one ADDS rather than silently
// overriding. Editing passes the key, which is what makes it an
// override of exactly that move.
function moveForm(o, t, done) {
  const form = document.createElement("div");
  form.className = "editor";

  const cell = labelledCell;

  const name = cell("name", t ? t.name : "", "wide");
  name.input.placeholder = "e.g. Descending Cut";
  const dice = cell("dice", t ? t.dice : "");
  dice.input.placeholder = "2d6";
  const one = document.createElement("div");
  one.className = "row cells";
  one.append(name, dice);

  const mode = document.createElement("select");
  for (const val of ["melee", "thrown", "ranged"]) {
    const opt = document.createElement("option");
    opt.value = val;
    opt.textContent = val;
    mode.append(opt);
  }
  mode.value = t ? t.mode : "melee";
  const modeWrap = labelledControl("mode", mode);

  // The three that were unreadable. "crit 19+" and "fumble 1-" are how
  // they read everywhere else, so the labels say which end each is.
  const level = cell("from level", t ? t.min_level : 1);
  const crit = cell("crit on", t ? t.crit_min : 20);
  const fumble = cell("fumble up to", t ? t.fumble_max : 1);
  const two = document.createElement("div");
  two.className = "row cells";
  two.append(modeWrap, level, crit, fumble);

  const prose = cell("what the table adjudicates", t ? t.special_text : "", "wide");
  const three = document.createElement("div");
  three.className = "row cells";
  three.append(prose);

  const save = action("save", async () => {
    const r = await tryCall("save_object_technique", {
      objectId: o.id,
      // Editing overrides THIS key; adding leaves it for the command
      // to mint, so a new move named after an old one does not quietly
      // replace it.
      key: t ? t.key : null,
      name: name.input.value,
      mode: mode.value,
      dice: dice.input.value,
      minLevel: Number(level.input.value) || 1,
      critMin: Number(crit.input.value) || 20,
      fumbleMax: Number(fumble.input.value) || 1,
      specialText: prose.input.value,
    });
    dmSay(r.ok ? (t ? name.input.value + " changed for this one" : name.input.value + " added") : r.error, !r.ok);
    if (r.ok) await done();
  });
  const four = document.createElement("div");
  four.className = "row";
  four.append(save);

  form.append(one, two, three, four);
  return form;
}

// A LABELLED CONTROL. The caption sits ABOVE the box and stays there.
//
// The object editor had captions to the LEFT with a 3.2rem gutter, and
// at four fields a row that reads as the caption belonging to the
// PREVIOUS box: "6 lb  price" looks like a price of six pounds. A
// placeholder cannot rescue it either, because a box with a value in it
// stops showing one.
//
// One helper for both editors rather than two conventions, since the
// second one is how the two drift.
function labelledControl(label, control, cls) {
  const wrap = document.createElement("label");
  wrap.className = "cell" + (cls ? " " + cls : "");
  const cap = document.createElement("span");
  cap.textContent = label;
  wrap.append(cap, control);
  // The control itself, so a caller reads .input.value rather than
  // reaching through the wrapper.
  wrap.input = control;
  return wrap;
}

function labelledCell(label, value, cls) {
  const i = document.createElement("input");
  i.value = value == null ? "" : String(value);
  return labelledControl(label, i, cls);
}

// Whose turn, which round, and what the button will do.
//
// THE BUTTON NAMES THE ACT rather than the state: "Begin" before the
// order starts and "Next: Goblin 2" after, because a button that says
// "next turn" makes you look at the list to find out what that means.
function paintTurnBar(encounterId, turns) {
  const say = document.querySelector("#turn-now");
  const next = document.querySelector("#next-turn");
  const enc = (turns && turns.encounter) || {};
  const order = (turns && turns.order) || [];
  const up = turns && turns.up_next;

  const current = order.find((a) => a.is_current);
  // ROUND 0 IS NOT ROUND 1. 051 keeps them apart because an encounter
  // that is built and rolled for but not started is a real state, and
  // claiming round one would be a claim nobody made.
  say.textContent = !enc.round
    ? order.length
      ? "not started"
      : "nobody enrolled"
    : "round " + enc.round + " \u00b7 " + (current ? current.label : "nobody");

  const waiting = order.filter((a) => a.initiative == null).length;
  if (waiting) {
    say.textContent += " \u00b7 " + waiting + " still to roll";
  }

  if (!up) {
    next.textContent = "Begin";
    next.disabled = true;
    next.title = "nobody can take a turn — roll for initiative first";
    return;
  }
  next.disabled = false;
  const who = order.find((a) => a.id === up.actor_id);
  next.textContent = !enc.round
    ? "Begin with " + (who ? who.label : "the top")
    : "Next: " + (who ? who.label : "?") + (up.new_round ? " (round " + (enc.round + 1) + ")" : "");
  next.title = "hand the turn on";
}

function withSign(n) {
  return (n < 0 ? "" : "+") + n;
}

// Everything in this fight that a roll can be aimed at.
//
// THREE GROUPS, ONE RESOLVER. A creature answers with an AC, a
// difficulty with a DC, and since 052 so does a THING in the room - a
// challenge that points at an object rather than a third kind
// resolution.rs would have to learn. Only the grouping differs, which
// is what `row` is for.
//
// The DM already had all of this; it was spread across a roster that
// showed names, a challenge list that showed difficulties, and nothing
// at all for the objects. One list is what makes "who can hit what"
// answerable at a glance.
function paintTargets(encounterId) {
  const ul = document.querySelector("#targets");
  ul.innerHTML = "";
  const all = state.dmTargets || [];
  if (!all.length) {
    ul.append(row("nothing to aim at yet", "", null));
    return;
  }

  const groups = [
    ["creatures", "actor"],
    ["things here", "object"],
    ["difficulties", "challenge"],
  ];
  for (const [title, row_kind] of groups) {
    const mine = all.filter((t) => t.row === row_kind);
    if (!mine.length) continue;

    const head = document.createElement("li");
    head.className = "flat group";
    head.textContent = title;
    ul.append(head);

    for (const t of mine) {
      const li = row(
        t.label,
        (t.target_kind === "ac" ? "AC " : "DC ") + t.value,
        null
      );
      // WHERE THE NUMBER CAME FROM, which load_targets has always
      // returned and nothing ever showed - "why is the goblin 15 and
      // Rodnar 16" was a question you had to open the database for.
      const why = document.createElement("span");
      why.className = "tag";
      why.textContent = t.source;
      li.append(why);
      if (t.condition && t.condition !== "conscious") {
        const c = document.createElement("span");
        c.className = "tag warn";
        c.textContent = t.condition;
        li.append(c);
      }
      ul.append(li);
    }
  }
}

// The things lying where the fight is happening, for the difficulty
// form. Empty is ordinary: 034 lets an encounter have no location, and
// a room can be bare.
async function fillEncounterObjects(encounterId) {
  const sel = document.querySelector("#chal-object");
  if (!sel) return;
  sel.innerHTML = "";
  const none = document.createElement("option");
  none.value = "";
  none.textContent = "\u2014 nothing in particular \u2014";
  sel.append(none);

  const things = await call("encounter_objects", { encounterId });
  for (const o of things || []) {
    const opt = document.createElement("option");
    opt.value = o.id;
    opt.textContent = (o.name || o.item_key) + (o.quantity > 1 ? " x" + o.quantity : "");
    sel.append(opt);
  }
  sel.disabled = !(things || []).length;
  if (!(things || []).length) none.textContent = "nothing lying here";
}

// Looking at a fight and changing one are different jobs. One screen
// doing both is how the forms came to outnumber the facts.
// Open a fight, in one of its two modes.
async function openEncounter(id, mode) {
  showEncMode(mode);
  // THROUGH loadDM, because the list itself changes: opening a fight
  // narrows it to that one. selectEncounter alone would fill the panel
  // and leave the other names above it.
  state.dmEncounterId = id;
  await loadDM();
}

function showEncMode(name) {
  showSub("enc-modes", "encmode", name);
  state.encMode = name;
}

// The initiative order as one line, at the top, because it is what a DM
// looks at between every roll. Names and numbers only - the roster
// below carries the detail.
// The Edit pane's forms, and the View pane's description.
//
// FILLED FROM THE ROW ALREADY IN HAND. state.encounters is what
// loadDM read a moment ago, so opening a fight costs no request - the
// same reason selectEncounter keeps that list at all.
function paintEncounterEditor(e) {
  if (!e) return;

  document.querySelector("#enc-name").value = e.name || "";
  document.querySelector("#enc-story").value = e.narrative || "";

  // The description, in View. 053 is explicit that this is a human's
  // prose and not narrative_lines - nothing generates it and the engine
  // will never read it.
  const told = document.querySelector("#enc-narrative");
  told.textContent = e.narrative || "";
  told.hidden = !e.narrative;

  // STATUS, IN EDIT. It used to be a button on every row of the list,
  // which made the list of fights the control panel for all of them.
  const bar = document.querySelector("#enc-status");
  bar.innerHTML = "";

  const now = document.createElement("span");
  now.className = "tag";
  now.textContent = e.status;
  bar.append(now);

  // draft -> active -> ended, and both endings go back to draft: a
  // finished fight and an abandoned one are equally re-openable, and
  // 034 keeps the two words apart for what they will TRIGGER rather
  // than for what they allow.
  const next =
    e.status === "draft" ? "active" : e.status === "active" ? "ended" : "draft";
  bar.append(
    action("\u2192 " + next, async () => {
      const r = await tryCall("set_encounter_status", {
        encounterId: e.id,
        status: next,
      });
      // "Only one active per game" is the rule rather than a mistake,
      // and the next move is obvious once it is said out loud.
      dmSay(r.ok ? e.name + " is now " + next : r.error, !r.ok);
      if (r.ok) {
        await loadDM();
        await loadTargets();
      }
    })
  );

  // CANCELLED IS NOT ENDED. 034: ended is what will trigger the
  // experience review and the journal entry, and a called-off encounter
  // must earn nobody anything. Offered only while there is something to
  // call off.
  if (e.status === "draft" || e.status === "active") {
    bar.append(
      action("cancel", async () => {
        const r = await tryCall("set_encounter_status", {
          encounterId: e.id,
          status: "cancelled",
        });
        dmSay(r.ok ? e.name + " called off \u2014 nobody earns anything for it" : r.error, !r.ok);
        if (r.ok) await loadDM();
      })
    );
  }

  paintBranches(e);
}

// WHAT HAS HAPPENED IN THIS FIGHT, newest first, grouped by round.
//
// The attack buttons are on this tab and their results were not: a DM
// swung, the goblin lost hit points, and the only account of it was on
// the Play tab behind the whole game's roll log. This is the same
// material, cut to this encounter - which is also why it is a separate
// read rather than a filter over `state.rolls`. That list is the newest
// fifty rolls in the GAME, so a filter would quietly empty itself the
// moment a fight scrolled off the end of it.
function paintEncLog(done, turns) {
  const ul = document.querySelector("#enc-log");
  ul.innerHTML = "";

  // A FAILED READ IS NOT AN EMPTY FIGHT. Saying "nothing yet" over a
  // refused query is the fault this codebase keeps producing - a calm
  // sentence about the game standing in for an error about the query.
  if (!done.ok) {
    const bad = row("could not read the log - " + done.error, "", null);
    bad.classList.add("warn");
    ul.append(bad);
    return;
  }

  const actions = done.value.actions || [];
  if (!actions.length) {
    ul.append(row("nothing has been rolled in this encounter yet", "", null));
    return;
  }

  const now = done.value.round;
  let round;
  let first = true;
  for (const a of actions) {
    if (first || a.round !== round) {
      round = a.round;
      first = false;
      const head = document.createElement("li");
      head.className = "flat group";
      // THREE DIFFERENT ANSWERS, and they must not print alike.
      //
      //   null   written before 054 existed. Inside an encounter that
      //          is the ONLY way to have no round, because the trigger
      //          stamps every one since. 70 rows in this database.
      //   0      051's "the order has not started" - swung before
      //          anybody rolled, which is a real and ordinary thing.
      //   n      a round.
      //
      // Filing the first two under "round 1" would invent a fight that
      // had not begun, which is 054's whole argument for NULL.
      head.textContent =
        a.round == null
          ? "before rounds were recorded"
          : a.round === 0
          ? "before the first turn"
          : "round " + a.round + (a.round === now ? " · now" : "");
      ul.append(head);
    }
    ul.append(logLine(a));
  }
}

// One action: who, what, at whom, and how it went.
//
// READ OFF THE ROLLS, not off the action. `character_name` and
// `target_label` are snapshots taken when the dice landed - see 001 and
// 013 - so a creature renamed or removed since still reads correctly in
// the account of what it did.
function logLine(a) {
  const rolls = a.rolls || [];
  const hit = rolls.find((r) => r.role === "to_hit") || rolls.find((r) => r.role === "check");
  const dmg = rolls.find((r) => r.role === "damage");

  const li = document.createElement("li");
  li.className = "flat item logline";
  const head = document.createElement("div");
  head.className = "head";

  const who = document.createElement("span");
  who.className = "nm";
  who.textContent = (hit && hit.character_name) || "Someone";
  head.append(who);

  const what = document.createElement("span");
  what.className = "tag";
  what.textContent = a.label || a.request;
  head.append(what);

  if (hit && hit.total !== null && hit.total !== undefined) {
    const n = document.createElement("span");
    n.className = "tag";
    n.textContent = "rolled " + hit.total;
    n.title = hit.detail || "";
    head.append(n);
  }

  // THE VERDICT ONLY WHERE THERE WAS ONE. An untargeted roll was not
  // judged, and printing a MISS over it would invent a failure nobody
  // claimed - the same distinction rollCard makes.
  if (hit && hit.success !== null && hit.success !== undefined) {
    const v = document.createElement("span");
    v.className = "tag " + (hit.success ? "yes" : "no");
    const at = hit.target_label
      ? hit.target_label + " " + String(hit.target_kind).toUpperCase() + " " + hit.target_value
      : String(hit.target_kind).toUpperCase() + " " + hit.target_value;
    v.textContent = (hit.success ? "hit " : "missed ") + at;
    v.title =
      hit.reason === "auto_hit" ? "hit on a natural " + hit.natural_roll
      : hit.reason === "auto_miss" ? "missed on a natural " + hit.natural_roll
      : hit.margin === 0 ? "exactly"
      : "by " + Math.abs(hit.margin);
    head.append(v);
  } else if (hit && hit.target_label) {
    const at = document.createElement("span");
    at.className = "tag";
    at.textContent = "at " + hit.target_label;
    head.append(at);
  }

  if (dmg && dmg.total !== null && dmg.total !== undefined) {
    const d = document.createElement("span");
    d.className = "tag hp";
    d.textContent = dmg.total + " damage";
    d.title = dmg.detail || "";
    head.append(d);
  }

  li.append(head);
  return li;
}

function paintOrderStrip(turns) {
  const el = document.querySelector("#order-strip");
  el.innerHTML = "";
  const order = (turns && turns.order) || [];
  if (!order.length) {
    el.textContent = "nobody enrolled yet";
    el.className = "orderstrip muted";
    return;
  }
  el.className = "orderstrip";
  order.forEach((a, i) => {
    const chip = document.createElement("span");
    chip.className =
      "ochip" +
      (a.is_current ? " now" : "") +
      (a.takes_turns ? "" : " out");
    // A count on the strip, because the strip is what a DM reads
    // between swings and "who has already gone" is the question it is
    // being read for.
    const sp = (state.encSpent || []).find((x) => x.actor_id === a.id);
    chip.textContent =
      (a.initiative == null ? "—" : a.initiative) + " " + a.label +
      (sp && sp.actions ? " ×" + sp.actions : "");
    if (sp && sp.beyond_one_turn) chip.classList.add("again");
    chip.title = a.takes_turns
      ? a.is_current
        ? "acting now"
        : "in the order"
      : a.initiative == null
      ? "has not rolled"
      : a.dead
      ? "dead"
      : "withdrawn";
    el.append(chip);
    if (i < order.length - 1) {
      const sep = document.createElement("span");
      sep.className = "osep";
      sep.textContent = "\u203a";
      el.append(sep);
    }
  });
}

// WHERE ELSE TO LOOK. An encounter points at a room, the creatures in
// it are characters, and the things in it are objects - all three live
// on other tabs, and until now finding them meant remembering the name
// and going searching.
function paintBranches(enc) {
  const row = document.querySelector("#enc-branches");
  row.innerHTML = "";

  const place = (state.locations || []).find((l) => l.id === enc.location_id);
  row.append(
    action(place ? "the " + place.name + " \u2192 World" : "no place set", async () => {
      if (!place) return dmSay("this encounter happens nowhere in particular", true);
      showTab("world");
      await selectPlace(place.id);
    })
  );

  row.append(
    action("who is here \u2192 Characters", () => {
      showTab("chars");
    }),
    // The filter is set so the tab opens on the things in this room
    // rather than on all thirty objects in the campaign.
    action("things here \u2192 Objects", () => {
      const find = document.querySelector("#obj-find");
      if (find && place) {
        find.value = place.name;
        paintObjects();
      }
      showTab("objects");
    })
  );
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
  // FROM THE OBJECT, NOT THE TYPE. These used to read off the
  // catalogue, so an edited greatsword displayed 2d6 while the sheet
  // rolled the 2d8 somebody gave it - the two places problem, on
  // screen. holders::resolve applies the overrides with the same
  // function the sheet uses and reports the result.
  if (o.damage_number && o.damage_denomination) {
    facts.push([
      "damage",
      o.damage_number + "d" + o.damage_denomination +
        (o.damage_types && o.damage_types.length ? " " + o.damage_types.join("/") : "") +
        (item && (o.damage_number !== item.damage_number ||
                  o.damage_denomination !== item.damage_denomination)
          ? " (set on this one)" : ""),
    ]);
  }
  if (o.base_ac != null) facts.push(["armour", "AC " + o.base_ac]);
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
  if (o.properties && o.properties.length) {
    facts.push(["properties", o.properties.join(", ")]);
  } else if (item && item.properties && item.properties.length) {
    // The type has some and this one has none, which is an override
    // saying so rather than a gap.
    facts.push(["properties", "none (set on this one)"]);
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

  // WHAT IT CAN DO. Painted before the container block, which returns
  // early - a weapon is never a container, and doing this first makes
  // that irrelevant rather than lucky.
  await fillMoves(el, o);

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


// The seven fields of 049, with their current values and the type's
// underneath as placeholder.
//
// PLACEHOLDER IS THE TYPE, VALUE IS THE OVERRIDE. An empty box showing
// "2d6" means "as a greatsword"; typing 8 into the die box means this
// one is different. That is the whole tri-state, made visible without
// a second control saying whether the first one counts.
function editorOverrides(o, item) {
  const wrap = document.createElement("div");
  wrap.className = "overrides";

  function box(label, value, hint, wide) {
    const cell = labelledCell(label, value, wide ? "wide" : null);
    // The TYPE's answer as the placeholder, so an empty box still says
    // what leaving it empty will mean.
    cell.input.placeholder = hint;
    cell.input.title = label + " — blank is as the catalogue says";
    wrap.append(cell);
    return cell.input;
  }

  const t = item || {};
  const fields = {
    el: wrap,
    weight: box("weight", o.weight_override, t.weight != null ? t.weight + " lb" : "lb"),
    price:  box("price",  o.price_override,  t.price != null ? String(t.price) : "cost"),
    dmgN:   box("dice",   o.damage_number_override, t.damage_number != null ? String(t.damage_number) : "n"),
    dmgD:   box("die",    o.damage_denomination_override, t.damage_denomination != null ? "d" + t.damage_denomination : "dN"),
    dmgT:   box("damage", o.damage_types_override ? o.damage_types_override.join(", ") : "",
                (t.damage_types || []).join(", ") || "slashing, fire", true),
    props:  box("props",  o.properties_override ? o.properties_override.join(", ") : "",
                (t.properties || []).join(", ") || "hvy, two", true),
    ac:     box("AC",     o.base_ac_override, t.base_ac != null ? String(t.base_ac) : "armour"),
  };

  // `none` is how a list is emptied, because blank already means
  // "revert to the type" and an empty override is a different fact -
  // it is how a greatsword loses `hvy`.
  const note = document.createElement("div");
  note.className = "muted ov-note";
  note.textContent = "blank = as the catalogue says · type none to empty a list";
  wrap.append(note);
  return fields;
}

// Rename it, change how many there are, or destroy it.
//
// NAME AND QUANTITY GO TOGETHER in one write, because they constrain
// each other: a stack of seven cannot carry a name. edit_object checks
// the name against the NEW quantity, which is the only order that can
// refuse naming one dagger and raising it to three in the same breath.
function fillEditor(el, o) {
  el.innerHTML = "";

  const nameCell = labelledCell("name", o.name || "", "wide");
  nameCell.input.placeholder = "blank = call it a " + o.item_key;
  const nameIn = nameCell.input;

  const qtyCell = labelledCell("how many", o.quantity);
  qtyCell.input.type = "number";
  qtyCell.input.min = "1";
  const qtyIn = qtyCell.input;

  const line = document.createElement("div");
  line.className = "row cells";
  line.append(nameCell, qtyCell);

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
    "as a " + o.item_key + (item ? " (" + sizeWord(item.size) + ")" : "")
  );
  const sizeLine = document.createElement("div");
  sizeLine.className = "row cells";
  sizeLine.append(labelledControl("size", sizeIn, "wide"));

  let holdsIn = null;
  if (o.is_container) {
    const fromType = item && item.holds_size;
    holdsIn = sizeSelect(
      fromType && o.holds_size === fromType ? "" : o.holds_size || "",
      "as a " + o.item_key +
        " (" + (fromType ? sizeWord(fromType) : "any size") + ")"
    );
    sizeLine.append(labelledControl("holds up to", holdsIn, "wide"));
  }

  // EVERYTHING ELSE ABOUT THIS ONE. 049's overrides, all on the same
  // convention as 036's two: blank CLEARS and reverts to the type, a
  // value sets it on this object alone. A greatsword given 2d8 here is
  // still a greatsword - 026 settled that an edited thing is a unique
  // OBJECT rather than a new catalogue row.
  const over = editorOverrides(o, item);
  el.append(over.el);

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
      weightOverride: over.weight.value,
      priceOverride: over.price.value,
      damageNumberOverride: over.dmgN.value,
      damageDenominationOverride: over.dmgD.value,
      damageTypesOverride: over.dmgT.value,
      propertiesOverride: over.props.value,
      baseAcOverride: over.ac.value,
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

  /* ---- how many ---- */

  // ONE COUNT FOR ALL THREE DESTINATIONS. A thing goes to a place, into
  // a container or out of one, and "how many" means the same in each,
  // so it is asked once rather than three times.
  //
  // DEFAULTS TO ALL OF THEM, which is what moving a thing has always
  // meant - objects::split reads None as the whole stack, and this
  // control exists to say LESS than that. A default of 1 would make
  // every ordinary move a two-step.
  //
  // Absent entirely for a single thing. A box that can only say "1" is
  // a box asking a question with one answer.
  let howMany = null;
  if (o.quantity > 1) {
    const line = document.createElement("div");
    line.className = "row count";
    const label = document.createElement("span");
    label.className = "muted";
    label.textContent = "how many of " + o.quantity;
    howMany = document.createElement("input");
    howMany.type = "number";
    howMany.min = "1";
    howMany.max = String(o.quantity);
    howMany.value = String(o.quantity);
    howMany.className = "narrow";
    line.append(label, howMany);
    el.append(line);
  }

  // Null means all of it, which is what every one of these commands
  // reads as "the whole stack". Out-of-range is NOT clamped here: the
  // rule is objects::split, it says "only 18 to move, not 20" in words,
  // and a second copy of it in the browser would be a second answer.
  const count = () => {
    if (!howMany || howMany.value === "") return null;
    // NOT `|| null`. Zero is falsy, so that would turn "move 0" into
    // "move all of it" - the loudest possible misreading of the
    // quietest possible typo. A 0 goes through and objects::split
    // refuses it by name: "moving none of something is not moving it".
    return Number(howMany.value);
  };

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
      quantity: count(),
    });
    dmSay(r.ok ? moved(o, count()) + " moved" : r.error, !r.ok);
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
      quantity: count(),
    });
    dmSay(r.ok ? moved(o, count()) + " packed away" : r.error, !r.ok);
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
        quantity: count(),
      });
      dmSay(
        r.ok
          ? (owner.token_name || owner.name) + " takes " + moved(o, count())
          : r.error,
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

  // A SIMPLE LIST. Every encounter used to carry its own status button
  // and a cancel beside it, so the list of fights was also the control
  // panel for all of them - four buttons a row, and the names hard to
  // read between them. Status now lives in Edit, where the fight being
  // changed is the one that is open.
  //
  // ARCHIVED ONES ARE OFF IT, not gone. 053 has no delete because rolls
  // point at encounters and actions point at rolls, so removing one
  // would either cascade through a session or be refused - and the
  // refusal is the honest answer, so the screen never asks.
  const shelved = (encounters || []).filter((e) => e.archived);
  const live = (encounters || []).filter((e) => !e.archived);

  // ONE AT A TIME. With a fight open the other names sat above its
  // order, its roster and its forms, so the longest thing on the screen
  // was a list of encounters nobody was running. Opening one narrows
  // the list to it; the way back is a button rather than a scroll.
  const open = state.dmEncounterId;
  if (open) {
    const back = document.createElement("li");
    back.className = "flat item back";
    back.append(
      action("← all encounters", async () => {
        // loadDM ends by selecting whatever is left, which is nothing.
        state.dmEncounterId = null;
        await loadDM();
      })
    );
    ul.append(back);
  }

  for (const e of open ? live.filter((e) => e.id === open) : live) {
    const li = row(e.name, e.status, null, e.id === state.dmEncounterId);
    li.dataset.encId = e.id;

    if (e.location_id) {
      const at = (state.locations || []).find((l) => l.id === e.location_id);
      if (at) {
        const w = document.createElement("span");
        w.className = "tag";
        w.textContent = at.path.join(" > ");
        li.append(w);
      }
    }

    li.append(
      action("view", () => openEncounter(e.id, "view")),
      action("edit", () => openEncounter(e.id, "edit"))
    );
    ul.append(li);
  }

  // The shelf is part of the list, so it goes away with the rest of it.
  if (shelved.length && !open) {
    const head = document.createElement("li");
    head.className = "flat group";
    head.textContent = "retired (" + shelved.length + ")";
    ul.append(head);
    for (const e of shelved) {
      const li = row(e.name, e.status, null, false);
      li.classList.add("shelved");
      li.append(
        action("bring back", async () => {
          const r = await tryCall("set_encounter_archived", {
            encounterId: e.id,
            archived: false,
          });
          dmSay(r.ok ? e.name + " is back on the list" : r.error, !r.ok);
          if (r.ok) await loadDM();
        })
      );
      ul.append(li);
    }
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
  // NOTHING IS OPENED FOR YOU. This used to fall back to the active
  // encounter, or failing that the first one, so the panel always had
  // something in it. That was right when the list and the detail sat
  // side by side; now that opening a fight NARROWS the list to it, an
  // automatic selection would mean the list could never be seen at all.
  //
  // A selection that has gone - archived, or removed - is dropped, which
  // is the one case that still has to be handled here.
  const list = encounters || [];
  const stillThere = list.some(
    (e) => e.id === state.dmEncounterId && !e.archived
  );
  if (!stillThere) state.dmEncounterId = null;
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

  // IN ORDER, not in enrolment order. turn_order returns the same
  // rows list_roster did, sorted by initiative.rs and carrying three
  // derived fields: the DEX that broke the tie, whether this one takes
  // turns at all, and whether it is up now.
  const turns = await call("turn_order", { encounterId: id });
  const roster = (turns && turns.order) || [];
  paintTurnBar(id, turns);
  paintEncounterEditor(e);

  // WHAT HAS BEEN DONE, and what each creature has spent doing it.
  //
  // tryCall rather than call: an empty log and a failed read look
  // identical on screen, and "nothing has happened yet" is a sentence
  // about the fight rather than about a broken query. That is the
  // defect class STATUS.md names, and it has cost two screens already.
  const done = await tryCall("encounter_log", { encounterId: id });
  state.encLog = done.ok ? done.value.actions || [] : [];
  state.encSpent = done.ok ? done.value.spent || [] : [];
  paintOrderStrip(turns);
  paintEncLog(done, turns);

  // Targets for THIS encounter, not the active one.
  //
  // state.targets belongs to loadTargets and follows whatever is live,
  // which is usually not what the DM has open. A goblin in a draft
  // attacks the people in that draft. Making this a second list rather
  // than reusing the first is the same lesson dmEncounterId taught.
  const dmTargets = await call("list_targets", { encounterId: id });
  state.dmTargets = Array.isArray(dmTargets) ? dmTargets : [];
  paintTargets(id);
  fillEncounterObjects(id);

  // TWO LISTS, ONE ROSTER. View shows who is in the fight and how they
  // are doing; Edit shows the same creatures with the controls that
  // change them. Painting one list and moving it between panes would
  // mean the mode decided what a DM can SEE rather than what they can
  // change.
  const rl = document.querySelector("#roster");
  const el = document.querySelector("#edit-roster");
  rl.innerHTML = "";
  el.innerHTML = "";
  for (const a of roster) {
    const li = document.createElement("li");
    li.className =
      "flat item" + (a.active ? " on" : "") + (a.is_current ? " acting" : "");

    const head = document.createElement("div");
    head.className = "head";
    const nm = document.createElement("span");
    nm.className = "nm";
    nm.textContent = a.label;
    head.append(nm);

    // THE NUMBER, AND THE DIFFERENCE BETWEEN NONE AND ZERO. 011 is
    // explicit that NULL means "has not rolled" and is not a rolled
    // zero, so the two must not print alike - one is a prompt and the
    // other is a bad result somebody has to live with.
    const score = document.createElement("span");
    score.className = "tag init" + (a.initiative == null ? " warn" : "");
    score.textContent =
      a.initiative == null
        ? "not rolled"
        : a.initiative + (a.dex_mod ? " (dex " + withSign(a.dex_mod) + ")" : "");
    head.append(score);

    // WHAT IT TAKES TO HIT IT, AND HOW IT IS DOING. Both were already
    // loaded for the target list and the roster showed neither, so the
    // DM had two places to look at one creature.
    const t = (state.dmTargets || []).find((x) => x.id === a.id);
    if (t) {
      const ac = document.createElement("span");
      ac.className = "tag";
      ac.textContent = "AC " + t.value;
      head.append(ac);
      if (t.hp_max != null) {
        const hp = document.createElement("span");
        hp.className = "tag hp";
        hp.textContent = t.hp_current + "/" + t.hp_max;
        head.append(hp);
      }
      // conscious is the ordinary case and saying so on every row is
      // noise; the other three are the ones a DM is watching for.
      if (t.condition && t.condition !== "conscious") {
        const c = document.createElement("span");
        c.className = "tag warn";
        c.textContent = t.condition;
        head.append(c);
      }
    }

    // WHAT THIS ONE HAS SPENT THIS ROUND. The count comes from Rust
    // with the round already applied - see spent.rs - so the screen
    // adds no arithmetic of its own.
    //
    // IT SAYS, IT DOES NOT REFUSE. Two actions in a round is normal:
    // Extra Attack, haste, an action surge, or a DM allowing it. The
    // ask was to be able to SEE that a character has swung again and
    // again, which nothing anywhere could say before this.
    const sp = (state.encSpent || []).find((x) => x.actor_id === a.id);
    if (sp && sp.actions) {
      const gone = document.createElement("span");
      gone.className = "tag spent" + (sp.beyond_one_turn ? " warn" : "");
      gone.textContent =
        sp.actions === 1 ? "acted" : "acted " + sp.actions + " times";
      gone.title =
        sp.attacks + (sp.attacks === 1 ? " attack" : " attacks") +
        " of " + sp.actions + " this round" +
        (sp.beyond_one_turn ? " - more than one turn's worth" : "");
      head.append(gone);
    }

    // ROLLING, SETTING, HIDING AND REMOVING ARE EDITS and live in the
    // other pane - see the edit roster below. What stays here is what
    // RUNS the fight: looking at a creature, and a monster swinging.
    //
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

    head.append(see);
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

    /* ---------- the same creature, in Edit ---------- */

    const er = document.createElement("li");
    er.className = "flat item" + (a.active ? " on" : "");
    const eh = document.createElement("div");
    eh.className = "head";
    const en = document.createElement("span");
    en.className = "nm";
    en.textContent = a.label;
    eh.append(en);

    const num = document.createElement("span");
    num.className = "tag init" + (a.initiative == null ? " warn" : "");
    num.textContent = a.initiative == null ? "not rolled" : String(a.initiative);
    eh.append(num);

    // A monster already rolled on enrolment, so in practice this is the
    // players' button and the DM's re-roll.
    eh.append(
      action(a.initiative == null ? "roll" : "re-roll", async () => {
        const r = await tryCall("roll_initiative", { actorId: a.id });
        dmSay(r.ok ? a.label + ": " + r.value.said : r.error, !r.ok);
        if (r.ok) await selectEncounter(id);
      })
    );

    // And typing one, because the table rolled real dice and 051 stores
    // no tiebreak for the DM to adjudicate with.
    const byHand = document.createElement("input");
    byHand.className = "narrow";
    byHand.type = "number";
    byHand.placeholder = "set";
    byHand.addEventListener("change", async () => {
      const v = byHand.value === "" ? null : Number(byHand.value);
      const r = await tryCall("set_initiative", { actorId: a.id, initiative: v });
      dmSay(r.ok ? a.label + " set to " + (v == null ? "not rolled" : v) : r.error, !r.ok);
      if (r.ok) await selectEncounter(id);
    });
    eh.append(byHand);

    // HIDDEN RATHER THAN REMOVED for anything that has rolled: the
    // rolls point at it. Remove is for a mis-click during setup, and
    // the foreign keys refuse it when it is not - 011 made the same
    // call, and 053 made it again for the encounter itself.
    eh.append(
      action(a.active ? "hide" : "show", async () => {
        await call("set_actor_active", { actorId: a.id, active: !a.active });
        await selectEncounter(id);
        await loadTargets();
      }),
      action("remove", async () => {
        if (!confirm("Take " + a.label + " out of this fight?")) return;
        const r = await tryCall("remove_actor", { actorId: a.id });
        if (!r.ok) return dmSay(r.error, true);
        await selectEncounter(id);
        await loadTargets();
      })
    );

    er.append(eh);
    el.append(er);
  }

  if (!roster.length) {
    rl.append(row("nobody in this fight yet", "", null));
    el.append(row("nobody to change", "", null));
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

  // THE PICKER SAYS WHO IS SWINGING. Unlabelled, it sat in the row of
  // the creature it belongs to and read as one more fact about that
  // creature - a bare "Merchant 1 - AC 11" under the goblin, which is
  // the opposite of what it means. Every button below sends THIS
  // actor's attack at whatever is chosen here, so the caption names
  // both ends of that sentence.
  wrap.append(labelledControl(actor.label + " attacks", pick, "wide aim"));

  // ONE BLOCK PER WEAPON PER MODE. A goblin with a light hammer has
  // fifteen buttons, and they are one weapon in two modes - 007 exists
  // because that hammer carries seven melee techniques and six thrown.
  // In one undivided row they read as fifteen unrelated verbs.
  //
  // Grouped on what the command now says rather than on a guess:
  // `weapon` used to carry the TECHNIQUE'S name, so every button
  // claimed a different weapon and no grouping was possible.
  const groups = [];
  for (const atk of attacks) {
    const key = atk.weapon + "\u0000" + atk.mode;
    let g = groups.find((x) => x.key === key);
    if (!g) {
      g = { key, weapon: atk.weapon, mode: atk.mode, attacks: [] };
      groups.push(g);
    }
    g.attacks.push(atk);
  }

  for (const g of groups) {
    const line = document.createElement("div");
    line.className = "attack-line";

    const cap = document.createElement("span");
    cap.className = "attack-of";
    // The mode only when it is not the ordinary one - "Light Hammer"
    // and "Light Hammer thrown" rather than "Light Hammer melee".
    cap.textContent = g.weapon + (g.mode === "melee" ? "" : " " + g.mode);
    // The plain swing carries the proficiency; a technique inherits it
    // from the same weapon, so asking the first is asking all of them.
    if (g.attacks.some((a) => a.proficient === false)) {
      cap.classList.add("warn");
      cap.title = "not proficient — no proficiency bonus";
    }
    line.append(cap);

    for (const atk of g.attacks) line.append(buildAttackButton(atk));
    wrap.append(line);
  }

  // A closure over the picker and the actor, which is why it is nested
  // rather than a sibling of attackRow.
  function buildAttackButton(atk) {
    const b = document.createElement("button");
    b.className = "tiny" + (atk.technique ? " technique" : "");
    b.textContent = atk.label || atk.request;
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
    return b;
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
      // Blank is a real answer - see the note in the markup. The
      // command reads it as "no class decided" and leaves hp_max null
      // rather than inventing a d8.
      classKey: val("#char-class") || null,
      // Nothing asks for size yet; the command defaults it to medium,
      // which is the fix for what made Snot and Unnamed hollow.
      size: null,
    });
    if (c) await loadCharacters();
  });

  // Saying what the choice MEANS, at the moment it is made. A class is
  // a die and two saves before it is a name, and the sheet would
  // otherwise be the first place anybody learns what they picked.
  document.querySelector("#char-class").addEventListener("change", paintClassNote);

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

  /* ---------- trade ---------- */

  document.querySelector("#trade-a").addEventListener("change", paintTrade);
  document.querySelector("#trade-b").addEventListener("change", paintTrade);

  document.querySelector("#do-haggle").addEventListener("click", async () => {
    const p = Number(val("#haggle-persuasion"));
    const i = Number(val("#haggle-insight"));
    if (!p || !i) return tradeSay("both totals are needed to haggle", true);
    const r = await call("haggle_margin", { persuasion: p, insight: i });
    if (!r) return;
    state.haggle = r.margin;
    document.querySelector("#haggle-said").textContent = r.said;
    await refreshBalance();
  });

  // Guarded: a double-click is a second trade, and the second one
  // would price against an inventory the first one already changed.
  guarded("#do-trade", async () => { await doTrade(false); });
  guarded("#do-give", async () => { await doTrade(true); });

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
      // 052. Blank means the difficulty is about nothing in particular,
      // which is most of them - "notice the tripwire" has no object.
      objectId: document.querySelector("#chal-object").value || null,
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

  document.querySelector("#edit-profs").addEventListener("click", () => {
    const row = document.querySelector("#profs-edit");
    row.hidden = !row.hidden;
  });

  guarded("#save-profs", async () => {
    if (!state.characterId) return profSay("pick a character first", true);
    // BOTH FIELDS ALWAYS, because this REPLACES the lists rather than
    // adding to them - and an empty box is a legitimate answer meaning
    // "trained with none of it".
    const r = await tryCall("set_proficiencies", {
      characterId: state.characterId,
      weaponProfs: val("#prof-weapons"),
      armorProfs: val("#prof-armour"),
    });
    if (!r.ok) return profSay(r.error, true);
    profSay("trained");
    // The SHEET carries the lists and the inventory reads proficiency
    // off them, so both have to come back or the chips still say what
    // was true a moment ago.
    await loadSheet();
    await loadInventory();
  });

  guarded("#next-turn", async () => {
    if (!state.dmEncounterId) return dmSay("pick an encounter first", true);
    const r = await tryCall("advance_turn", { encounterId: state.dmEncounterId });
    if (!r.ok) return dmSay(r.error, true);
    await selectEncounter(state.dmEncounterId);
  });

  guarded("#reset-order", async () => {
    if (!state.dmEncounterId) return dmSay("pick an encounter first", true);
    // The initiatives SURVIVE a reset - re-rolling is a separate
    // decision, and losing everyone's number because the DM wanted to
    // restart the order would be the expensive kind of helpful.
    if (!confirm("Put the fight back before the first turn? Initiatives are kept.")) return;
    const r = await tryCall("reset_order", { encounterId: state.dmEncounterId });
    dmSay(r.ok ? "back before the first turn" : r.error, !r.ok);
    if (r.ok) await selectEncounter(state.dmEncounterId);
  });

  for (const b of document.querySelectorAll("#enc-modes .tab")) {
    b.addEventListener("click", () => showEncMode(b.dataset.encmode));
  }

  guarded("#save-encounter", async () => {
    if (!state.dmEncounterId) return dmSay("open an encounter first", true);
    const r = await tryCall("edit_encounter", {
      encounterId: state.dmEncounterId,
      name: val("#enc-name"),
      narrative: document.querySelector("#enc-story").value,
    });
    dmSay(r.ok ? "saved" : r.error, !r.ok);
    if (r.ok) await loadDM();
  });

  guarded("#archive-encounter", async () => {
    if (!state.dmEncounterId) return dmSay("open an encounter first", true);
    // RETIRED, NOT DELETED, and the wording says so. 053 has no delete
    // because rolls point at encounters and actions point at rolls.
    if (!confirm("Take this off the list? It is kept, with its rolls, and can be brought back.")) {
      return;
    }
    const r = await tryCall("set_encounter_archived", {
      encounterId: state.dmEncounterId,
      archived: true,
    });
    if (!r.ok) return dmSay(r.error, true);
    dmSay("retired from the list");
    state.dmEncounterId = null;
    await loadDM();
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
