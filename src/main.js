// odyssey1e — access test rig.
//
// This is a harness, not the app. Its job is to make the four-account
// RLS test from migration 001 fast to run and impossible to misread:
// every call is logged with its raw result, and a denial is shown as
// loudly as a success. Zero rows is the answer we are usually looking
// for, so it gets said out loud rather than rendering as an empty list.

const { invoke } = window.__TAURI__.core;

let state = { user: null, gameId: null, characterId: null, sheet: null, rolls: [] };

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
  document.querySelector("#sheet-panel").hidden = true;
  document.querySelector("#equipment-panel").hidden = true;
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

// A roll card. Three lines, top to bottom: who did what, what the dice
// said, and the prose that narrates it. The narrative is chosen on the
// device and written into the row at insert, so it is already there the
// first time the log is read — nothing here waits for it or polls.
function rollCard(r) {
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

  const dice = document.createElement("div");
  dice.className = "dice";
  dice.textContent =
    r.total === null || r.total === undefined
      ? r.detail || "—"
      : (r.detail ? r.detail + "  =  " : "") + r.total;

  li.append(head, dice);

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
  for (const g of games) {
    ul.append(
      row(g.name, g.join_code, () => selectGame(g.id), g.id === state.gameId)
    );
  }
}

async function selectGame(id) {
  state.gameId = id;
  await loadGames();
  await loadCharacters();
  await loadRolls();
  await loadTargets();
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
  state.targets = [];
  pick.innerHTML =
    '<option value="">no target</option><option value="manual">type a number…</option>';
  // Repopulating resets the selection to "no target", so the hand-typed
  // fields have to go back into hiding with it — otherwise switching
  // game leaves them on screen next to a select that says no target.
  document.querySelector("#manual-target").hidden = true;
  if (!state.gameId) return;

  const encounters = await call("list_encounters", { gameId: state.gameId });
  const active = (encounters || []).find((e) => e.status === "active");
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
      o.textContent =
        t.label + " · " + t.target_kind.toUpperCase() + " " + t.value;
      // Why this number is this number, on hover. Same instinct as the
      // proficiency badges: never show a figure with no account of it.
      o.title = t.source;
      og.append(o);
    }
    pick.append(og);
  }
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
    el.textContent = r.label + "  →  " + r.formula;
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
  for (const r of state.rolls) {
    ul.append(rollCard(r));
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
  });

  document.querySelector("#roll-named").addEventListener("click", async () => {
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
      target = { value: t.value, kind: t.target_kind, label: t.label };
    }

    const r = await call("roll_named", {
      characterId: state.characterId,
      request: val("#named-request") || "insight",
      mode: document.querySelector("#mode").value,
      targetValue: target ? target.value : null,
      targetKind: target ? target.kind : null,
      targetLabel: target ? target.label : null,
    });
    if (r) await loadRolls();
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

  document.querySelector("#clear-log").addEventListener("click", () => {
    logEl().innerHTML = "";
  });
});
