//! The spell catalogue, and what a cleric has chosen from it.
//!
//! 106. Plumbing. Every number - how many may be prepared, how many
//! cantrips are known, what slots exist, the save DC - is prayers.rs.
//!
//! THE CLERIC'S LEVEL, NEVER THE CHARACTER'S. A Fighter 4 / Cleric 1
//! prepares as a cleric 1, and reading their total of 5 would hand them
//! a 3rd-level slot they have not earned. The same rule
//! `features::held` and `uses::Context` already follow.

use serde_json::{json, Value};
use tauri::State;

use crate::prayers;
use crate::supabase::{self, AppState};

/// The spell catalogue for one class, global rows and this game's own.
///
/// BY CLASS RATHER THAN A TABLE PER CLASS. `spells.classes` is an
/// array, so the cleric list is a query and the wizard list is a seed
/// rather than a schema - see 101.
#[tauri::command]
pub fn list_spells(
    state: State<AppState>,
    game_id: String,
    class_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let rows = supabase::rest_get(
        &token,
        "spells",
        &[
            (
                "select",
                "key,game_id,name,level,cast_type,category,school,save_ability,dice,\
                 concentration,ritual,range,duration,casting_time,components,material,\
                 classes,description",
            ),
            ("classes", &format!("cs.{{{}}}", class_key)),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("order", "level.asc,name.asc"),
        ],
    )?;

    // The game's own row wins over the global one of the same key - the
    // precedence every tenanted catalogue in this schema uses.
    let all = rows.as_array().cloned().unwrap_or_default();
    let mut out: Vec<Value> = Vec::new();
    for r in &all {
        let Some(key) = r.get("key").and_then(|v| v.as_str()) else {
            continue;
        };
        let mine = r.get("game_id").and_then(|v| v.as_str()).is_some();
        match out
            .iter()
            .position(|o| o.get("key").and_then(|v| v.as_str()) == Some(key))
        {
            Some(i) if mine => out[i] = r.clone(),
            Some(_) => {}
            None => out.push(r.clone()),
        }
    }
    Ok(json!(out))
}

/// What this cleric can do today: their numbers, and what they hold.
///
/// REFUSES A CHARACTER WHO IS NOT A CLERIC, rather than answering with
/// zeroes. A fighter has no prepared list and saying "0 of 0" would
/// invite the question of how to raise it.
#[tauri::command]
pub fn list_prayers(state: State<AppState>, character_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;

    let Some(cleric) = sheet.classes.iter().find(|t| t.key == "cleric") else {
        return Err(format!("{} is not a cleric", sheet.name));
    };
    let level = cleric.level;
    let wis = sheet.ability_mod("wis");
    let pb = sheet.proficiency_bonus();

    let chosen = load_chosen(&token, &character_id)?;
    let prepared: Vec<String> = chosen
        .iter()
        .filter(|(_, p)| *p)
        .map(|(k, _)| k.clone())
        .collect();
    let cantrips: Vec<String> = chosen
        .iter()
        .filter(|(_, p)| !*p)
        .map(|(k, _)| k.clone())
        .collect();

    let slots = prayers::slots_at(level);
    let spent = load_slots(&token, &character_id)?;
    let left = prayers::slots_left(level, &spent);
    Ok(json!({
        "cleric_level": level,
        "wis_mod": wis,
        "save_dc": prayers::save_dc(pb, wis),
        "attack_bonus": prayers::attack_bonus(pb, wis),
        "prepared": prepared,
        "prepared_max": prayers::prepared_max(level, wis),
        "cantrips": cantrips,
        "cantrips_known": prayers::cantrips_known(level),
        "top_slot": prayers::top_slot(level),
        // 1st-level slots first. 107 made the spending real, so these
        // are three different facts and the tab needs all three: how
        // many they have, how many are gone, how many are left.
        "slots": slots,
        "slots_spent": spent,
        "slots_left": left,
    }))
}

/// Hold a spell, or learn a cantrip.
///
/// ONE COMMAND FOR BOTH, because the difference is the spell's level
/// and the engine can see it. Asking the screen to pick between two
/// commands would be the screen deciding a rule.
#[tauri::command]
pub fn prepare_prayer(
    state: State<AppState>,
    character_id: String,
    spell_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;
    let Some(cleric) = sheet.classes.iter().find(|t| t.key == "cleric") else {
        return Err(format!("{} is not a cleric", sheet.name));
    };
    let level = cleric.level;
    let wis = sheet.ability_mod("wis");

    let spell = one_spell(&token, &sheet.game_id, &spell_key)?;
    if !spell
        .get("classes")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().any(|c| c.as_str() == Some("cleric")))
        .unwrap_or(false)
    {
        return Err("that is not a cleric spell".to_string());
    }
    let spell_level = spell.get("level").and_then(|v| v.as_i64()).unwrap_or(0);

    let chosen = load_chosen(&token, &character_id)?;
    let held: Vec<String> = chosen
        .iter()
        .filter(|(_, p)| *p)
        .map(|(k, _)| k.clone())
        .collect();

    // A CANTRIP IS A DIFFERENT BUDGET, so it is checked against its own
    // count rather than through may_prepare - which refuses cantrips
    // outright, and says so, because being prepared is not what a
    // cantrip does.
    if spell_level == 0 {
        let known: Vec<&String> = chosen
            .iter()
            .filter(|(_, p)| !*p)
            .map(|(k, _)| k)
            .collect();
        if known.iter().any(|k| **k == spell_key) {
            return Err("already known".to_string());
        }
        let max = prayers::cantrips_known(level);
        if known.len() as i64 >= max {
            return Err(format!(
                "that is {} cantrips and they know {} - forget one first",
                known.len(),
                max
            ));
        }
    } else {
        prayers::may_prepare(spell_level, level, wis, &held, &spell_key)?;
    }

    supabase::rest_upsert(
        &token,
        "character_prayers",
        &json!({
            "character_id": character_id,
            "spell_key": spell_key,
            "prepared": spell_level > 0,
        }),
        "character_id,spell_key",
    )
}

/// Put one down.
///
/// A DELETE, UNLIKE A CHOICE OR AN EFFECT. A prepared list is what you
/// hold TODAY and changes at every long rest; there is no history worth
/// keeping in which spells you held last Tuesday.
#[tauri::command]
pub fn forget_prayer(
    state: State<AppState>,
    character_id: String,
    spell_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_delete(
        &token,
        "character_prayers",
        &[
            ("character_id", &format!("eq.{}", character_id)),
            ("spell_key", &format!("eq.{}", spell_key)),
        ],
    )?;
    Ok(json!({ "ok": true }))
}

/// What this cleric can cast right now, as turn actions.
///
/// 110. PREPARED AND CANTRIPS ONLY, which is the whole point - the
/// catalogue is 106 long and what a cleric can do on their turn is the
/// dozen they are holding.
///
/// WHAT CANNOT BE CAST IS STILL LISTED, with the reason on it. Prayer
/// of Healing takes ten minutes and a fight is six seconds a round, so
/// it is shown as too long rather than hidden - a cleric reaching for
/// it should be told why rather than wondering where it went.
#[tauri::command]
pub fn castable(state: State<AppState>, character_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;
    let Some(cleric) = sheet.classes.iter().find(|t| t.key == "cleric") else {
        return Err(format!("{} is not a cleric", sheet.name));
    };
    let wis = sheet.ability_mod("wis");
    let pb = sheet.proficiency_bonus();

    let chosen = load_chosen(&token, &character_id)?;
    if chosen.is_empty() {
        return Ok(json!([]));
    }
    let keys: Vec<String> = chosen.iter().map(|(k, _)| k.clone()).collect();
    let rows = supabase::rest_get(
        &token,
        "spells",
        &[
            ("select", "key,name,level,cast_type,casting_time,save_ability,dice,range,duration,concentration"),
            ("key", &format!("in.({})", keys.join(","))),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", sheet.game_id)),
        ],
    )?;

    let spent = load_slots(&token, &character_id)?;
    let left = prayers::slots_left(cleric.level, &spent);

    let mut out: Vec<Value> = Vec::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let c = crate::spellcast::cast(
            r.get("key").and_then(|v| v.as_str()).unwrap_or(""),
            r.get("name").and_then(|v| v.as_str()).unwrap_or(""),
            r.get("level").and_then(|v| v.as_i64()).unwrap_or(0),
            r.get("cast_type").and_then(|v| v.as_str()).unwrap_or(""),
            r.get("casting_time").and_then(|v| v.as_str()),
            r.get("save_ability").and_then(|v| v.as_str()),
            r.get("dice").and_then(|v| v.as_str()),
            pb,
            wis,
        );
        // WHY IT CANNOT BE CAST, where it cannot. Two different
        // problems and they want different words: it takes too long,
        // or there is no slot left to carry it.
        let blocked = if !c.cost.in_a_fight() {
            Some(format!(
                "takes {}",
                r.get("casting_time").and_then(|v| v.as_str()).unwrap_or("too long")
            ))
        } else if c.needs_slot && left[(c.level - 1).max(0) as usize] == 0 {
            Some(format!("no level {} slots left", c.level))
        } else {
            None
        };

        out.push(json!({
            "key": c.key, "name": c.name, "level": c.level,
            "stance": c.stance.as_str(),
            "cost": c.cost.as_str(),
            "to_hit": c.to_hit,
            "save_dc": c.save_dc,
            "save_ability": c.save_ability,
            "dice": c.dice,
            "needs_slot": c.needs_slot,
            "label": crate::spellcast::label(&c),
            "range": r.get("range"),
            "duration": r.get("duration"),
            "concentration": r.get("concentration"),
            "blocked": blocked,
        }));
    }
    // Cantrips last: they cost nothing and are what is left when the
    // slots are gone, so the things that run out lead.
    out.sort_by_key(|v| {
        let lvl = v.get("level").and_then(|x| x.as_i64()).unwrap_or(0);
        (
            lvl == 0,
            lvl,
            v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        )
    });
    Ok(json!(out))
}

/// Cast one: spend the slot, and say what to roll.
///
/// THE SLOT IS SPENT HERE AND THE DICE ARE ROLLED BY THE ROLL PATH,
/// deliberately. A cast is a cost plus a roll, and the roll already has
/// somewhere to go. Rolling here would be a second way to make a d20
/// happen.
///
/// `at_level` CARRIES AN UPCAST. Casting Cure Wounds with a 3rd-level
/// slot is 5e's own rule, and the reason 107 tracks slots by level
/// rather than by spell.
#[tauri::command]
pub fn cast_prayer(
    state: State<AppState>,
    character_id: String,
    spell_key: String,
    at_level: Option<i64>,
) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;
    let Some(cleric) = sheet.classes.iter().find(|t| t.key == "cleric") else {
        return Err(format!("{} is not a cleric", sheet.name));
    };

    let held = load_chosen(&token, &character_id)?;
    if !held.iter().any(|(k, _)| *k == spell_key) {
        return Err("they do not have that prepared".to_string());
    }

    let spell = one_spell_full(&token, &sheet.game_id, &spell_key)?;
    let c = crate::spellcast::cast(
        &spell_key,
        spell.get("name").and_then(|v| v.as_str()).unwrap_or(""),
        spell.get("level").and_then(|v| v.as_i64()).unwrap_or(0),
        spell.get("cast_type").and_then(|v| v.as_str()).unwrap_or(""),
        spell.get("casting_time").and_then(|v| v.as_str()),
        spell.get("save_ability").and_then(|v| v.as_str()),
        spell.get("dice").and_then(|v| v.as_str()),
        sheet.proficiency_bonus(),
        sheet.ability_mod("wis"),
    );

    if !c.cost.in_a_fight() {
        return Err(format!(
            "{} takes {} - not something to do on a turn",
            c.name,
            spell.get("casting_time").and_then(|v| v.as_str()).unwrap_or("too long")
        ));
    }

    // A CANTRIP COSTS NOTHING, which is the whole of what makes it one.
    let mut used: Option<i64> = None;
    if c.needs_slot {
        let want = at_level.unwrap_or(c.level);
        if want < c.level {
            return Err(format!(
                "{} is a level {} spell - a level {} slot will not carry it",
                c.name, c.level, want
            ));
        }
        let spent = load_slots(&token, &character_id)?;
        prayers::may_spend_slot(cleric.level, &spent, want)?;
        write_slot(&token, &character_id, want, spent[(want - 1) as usize] + 1)?;
        used = Some(want);
    }

    Ok(json!({
        "name": c.name,
        "label": crate::spellcast::label(&c),
        "stance": c.stance.as_str(),
        "cost": c.cost.as_str(),
        "to_hit": c.to_hit,
        "save_dc": c.save_dc,
        "save_ability": c.save_ability,
        "dice": c.dice,
        "slot_used": used,
    }))
}

/// Spend one.
///
/// BY LEVEL AND NOT BY SPELL. A slot is a slot - a 3rd-level slot can
/// carry a 1st-level spell, and asking which spell it was for would
/// make upcasting unrepresentable.
#[tauri::command]
pub fn spend_spell_slot(
    state: State<AppState>,
    character_id: String,
    slot_level: i64,
) -> Result<Value, String> {
    let token = state.token()?;
    let level = cleric_level(&token, &character_id)?;
    let spent = load_slots(&token, &character_id)?;
    prayers::may_spend_slot(level, &spent, slot_level)?;

    let i = (slot_level - 1) as usize;
    write_slot(&token, &character_id, slot_level, spent[i] + 1)?;
    Ok(json!({ "left": prayers::slots_left(level, &spent)[i] - 1 }))
}

/// Give one back, for a misclick or a DM's say-so.
#[tauri::command]
pub fn restore_spell_slot(
    state: State<AppState>,
    character_id: String,
    slot_level: i64,
) -> Result<Value, String> {
    let token = state.token()?;
    let level = cleric_level(&token, &character_id)?;
    let spent = load_slots(&token, &character_id)?;
    if !(1..=9).contains(&slot_level) {
        return Err("a spell slot is level 1 to 9".to_string());
    }
    let i = (slot_level - 1) as usize;
    if spent[i] <= 0 {
        return Err("nothing to give back".to_string());
    }
    write_slot(&token, &character_id, slot_level, spent[i] - 1)?;
    Ok(json!({ "left": prayers::slots_left(level, &spent)[i] + 1 }))
}

/* ======================== THE WORKING PARTS ======================== */

/// What this character has spent, 1st-level first.
pub(crate) fn load_slots(token: &str, character_id: &str) -> Result<[i64; 9], String> {
    let rows = supabase::rest_get(
        token,
        "character_slots",
        &[
            ("select", "slot_level,spent"),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )?;
    let mut out = [0i64; 9];
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let Some(lvl) = r.get("slot_level").and_then(|v| v.as_i64()) else {
            continue;
        };
        if (1..=9).contains(&lvl) {
            out[(lvl - 1) as usize] = r.get("spent").and_then(|v| v.as_i64()).unwrap_or(0);
        }
    }
    Ok(out)
}

fn write_slot(token: &str, character_id: &str, level: i64, spent: i64) -> Result<(), String> {
    supabase::rest_upsert(
        token,
        "character_slots",
        &json!({
            "character_id": character_id,
            "slot_level": level,
            "spent": spent.max(0),
        }),
        "character_id,slot_level",
    )?;
    Ok(())
}

/// The cleric level, refusing anybody who is not one.
fn cleric_level(token: &str, character_id: &str) -> Result<i64, String> {
    let sheet = crate::character::load_sheet(token, character_id)?;
    sheet
        .classes
        .iter()
        .find(|t| t.key == "cleric")
        .map(|t| t.level)
        .ok_or_else(|| format!("{} is not a cleric", sheet.name))
}

/// Hand every slot back. Called by the rest path.
///
/// A DELETE RATHER THAN A ZERO because the rows carry nothing else -
/// an unspent level has no fact worth a row, and clearing them keeps
/// the table the size of what is actually spent.
pub(crate) fn clear_slots(token: &str, character_id: &str) -> Result<(), String> {
    supabase::rest_delete(
        token,
        "character_slots",
        &[("character_id", &format!("eq.{}", character_id))],
    )
}

/// (spell key, prepared) for this character.
fn load_chosen(token: &str, character_id: &str) -> Result<Vec<(String, bool)>, String> {
    let rows = supabase::rest_get(
        token,
        "character_prayers",
        &[
            ("select", "spell_key,prepared"),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )?;
    Ok(rows
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|r| {
            Some((
                r.get("spell_key")?.as_str()?.to_string(),
                r.get("prepared").and_then(|v| v.as_bool()).unwrap_or(true),
            ))
        })
        .collect())
}

/// One catalogue row with everything casting needs.
fn one_spell_full(token: &str, game_id: &str, key: &str) -> Result<Value, String> {
    let rows = supabase::rest_get(
        token,
        "spells",
        &[
            ("select", "key,name,level,cast_type,casting_time,save_ability,dice"),
            ("key", &format!("eq.{}", key)),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("order", "game_id.asc.nullslast"),
        ],
    )?;
    rows.as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| format!("no spell with key '{}'", key))
}

/// One catalogue row, this game's before the global one.
fn one_spell(token: &str, game_id: &str, key: &str) -> Result<Value, String> {
    let rows = supabase::rest_get(
        token,
        "spells",
        &[
            ("select", "key,name,level,classes"),
            ("key", &format!("eq.{}", key)),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("order", "game_id.asc.nullslast"),
        ],
    )?;
    rows.as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| format!("no spell with key '{}'", key))
}
