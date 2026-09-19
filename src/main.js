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
  games: [], encounterId: null, dmEncounterId: null,
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
async function call(cmd, args) {
  try {
    const out = await invoke(cmd, args || {});
    log(cmd, out, false);
    return out;
  } catch (e) {
    log(cmd, e, true);
    return null;
  }
}

/* ---------- session ---------- */

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
  document.querySelector("#dm-panel").hidden = true;
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
  await loadDM();
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

  if (!Array.isArray(items) || items.length === 0) {
    const li = document.createElement("li");
    li.className = "flat muted";
    li.textContent = "carrying nothing";
    list.append(li);
    return;
  }

  for (const it of items) {
    list.append(inventoryRow(it));
  }
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
      characterId: state.characterId,
      itemKey: it.item.key,
      equipped: box.checked,
    });
    await loadSheet();
  });

  const name = document.createElement("span");
  name.className = "nm";
  name.textContent = it.item.name + (it.quantity > 1 ? " ×" + it.quantity : "");

  // Toggle and name on one line, the engine's verdicts on the next. The
  // left column is narrow and chips wrap badly beside a flexible name.
  const head = document.createElement("div");
  head.className = "head";
  head.append(box, name);

  const tags = document.createElement("span");
  tags.className = "tags";

  // What the engine classified it as — the evidence behind the verdict.
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
  return li;
}

function chip(text, kind) {
  const s = document.createElement("span");
  s.className = "chip " + kind;
  s.textContent = text;
  return s;
}

/* ---------- the DM side ---------- */

// Everything an encounter needs was authored in SQL until now. The
// policies have been in place since 011; this is the screen that was
// missing.
//
// Shown only to the DM of the selected game — see amDM(). That is a
// courtesy, not a guard: every one of these commands is refused by
// Postgres for anyone else, and the panel would simply fill the log with
// "only the DM of this game can ...".
async function loadDM() {
  const panel = document.querySelector("#dm-panel");
  if (!state.gameId || !amDM()) {
    panel.hidden = true;
    return;
  }
  panel.hidden = false;
  document.querySelector("#dm-who").textContent = "you run this game";

  const encounters = await call("list_encounters", { gameId: state.gameId });
  const ul = document.querySelector("#encounters");
  ul.innerHTML = "";
  for (const e of encounters || []) {
    const li = row(e.name, e.status, () => selectEncounter(e.id), e.id === state.dmEncounterId);
    // The id rides on the element so selecting can re-mark the list
    // without refetching it. Rebuilding on every click would work and
    // would also throw away the DM's scroll position mid-setup.
    li.dataset.encId = e.id;
    // draft -> active -> ended, as a button rather than a dropdown: the
    // next state is nearly always the obvious one.
    const next = e.status === "draft" ? "active" : e.status === "active" ? "ended" : "draft";
    const b = document.createElement("button");
    b.className = "tiny ghost";
    b.textContent = "→ " + next;
    b.addEventListener("click", async (ev) => {
      ev.stopPropagation();
      await call("set_encounter_status", { encounterId: e.id, status: next });
      await loadDM();
      await loadTargets();
    });
    li.append(b);
    ul.append(li);
  }

  await loadStatblockPicker();
  await loadSkillPicker();
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

  const roster = await call("list_roster", { encounterId: id });
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
    head.append(tog, del);
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
  if (state.user) await loadGames();

  document.querySelector("#signin").addEventListener("click", async () => {
    const u = await call("sign_in", { email: val("#email"), password: val("#password") });
    if (u) { state.user = u; paintUser(); clearData(); await loadGames(); }
  });

  document.querySelector("#signup").addEventListener("click", async () => {
    const u = await call("sign_up", {
      email: val("#email"),
      password: val("#password"),
      displayName: val("#display") || "Adventurer",
    });
    if (u) { state.user = u; paintUser(); clearData(); await loadGames(); }
  });

  document.querySelector("#signout").addEventListener("click", async () => {
    await call("sign_out");
    state.user = null;
    paintUser();
    clearData();
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

  /* ---------- the DM side ---------- */
  // All writes, so all guarded: a double-click on Enrol is a second
  // goblin nobody asked for, and 018 will dutifully number it.

  guarded("#create-encounter", async () => {
    if (!state.gameId) return log("create_encounter", "select a game first", true);
    const r = await call("create_encounter", {
      gameId: state.gameId,
      name: val("#enc-name"),
    });
    if (r) document.querySelector("#enc-name").value = "";
    await loadDM();
  });

  guarded("#enrol", async () => {
    if (!state.dmEncounterId) return log("enrol_actor", "select an encounter first", true);
    const pick = document.querySelector("#enrol-what").value;
    const isNpc = pick.startsWith("npc:");
    const r = await call("enrol_actor", {
      encounterId: state.dmEncounterId,
      npcKey: isNpc ? pick.slice(4) : null,
      characterId: isNpc ? null : pick.slice(4),
      // Blank on purpose is the normal case: 018 names it.
      label: val("#enrol-name") || null,
    });
    if (r) document.querySelector("#enrol-name").value = "";
    await selectEncounter(state.dmEncounterId);
    await loadTargets();
  });

  guarded("#add-challenge", async () => {
    if (!state.dmEncounterId) return log("add_challenge", "select an encounter first", true);
    const r = await call("add_challenge", {
      encounterId: state.dmEncounterId,
      label: val("#chal-label"),
      dc: Number(val("#chal-dc") || 0),
      skillKey: document.querySelector("#chal-skill").value || null,
    });
    if (r) {
      document.querySelector("#chal-label").value = "";
      document.querySelector("#chal-dc").value = "";
    }
    await selectEncounter(state.dmEncounterId);
    await loadTargets();
  });

  guarded("#create-npc", async () => {
    if (!state.gameId) return log("create_npc", "select a game first", true);
    const r = await call("create_npc", {
      gameId: state.gameId,
      key: val("#npc-key"),
      name: val("#npc-name"),
      ac: Number(val("#npc-ac") || 0),
      hpMax: Number(val("#npc-hp") || 0),
      species: val("#npc-species") || null,
      class: val("#npc-class") || null,
    });
    if (r) {
      for (const id of ["#npc-key", "#npc-name", "#npc-species", "#npc-class", "#npc-ac", "#npc-hp"]) {
        document.querySelector(id).value = "";
      }
      await loadStatblockPicker();
    }
  });

  document.querySelector("#clear-log").addEventListener("click", () => {
    logEl().innerHTML = "";
  });
});
