//! Making a character, and listing the ones that exist.
//!
//! MOVED HERE BECAUSE IT WAS BEING WORKED ON. commands/mod.rs sets the
//! rule - "a group migrates when it is being worked on anyway, so the
//! diff that moves it is a diff somebody is already reading" - and 055
//! is a diff through the middle of `create_character`.
//!
//! WHAT WAS WRONG WITH IT. It took a name and inserted a name. Level
//! defaulted to 1 and everything else to NULL, which meant no size,
//! therefore no hit die, therefore NO HIT POINTS - not zero, null, an
//! empty space on the sheet where a character's life goes. Snot and
//! Unnamed stood that way for weeks and nothing anywhere complained,
//! because nothing was asking.
//!
//! A CHARACTER IS NOW BORN FINISHED, or as finished as the facts given
//! allow. A class supplies the hit die, the abilities the trigger seeds
//! supply the Constitution, and `vitality::pc_hp` turns those into a
//! maximum. Nothing is left for a later screen to remember to fill in,
//! because the evidence says a later screen does not.

use serde_json::{json, Value};
use tauri::State;

use crate::class;
use crate::species;
use crate::supabase::{self, AppState};
use crate::vitality;

/* ============================ READING ============================ */

#[tauri::command]
pub fn list_characters(state: State<AppState>, game_id: String) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_get(
        &token,
        "characters",
        &[
            ("select", "id,name,token_name,owner_uid,is_active,class_key,level"),
            ("game_id", &format!("eq.{}", game_id)),
            // PEOPLE ONLY. Since 022 a monster is a character too, and
            // without this a player's list fills with goblins. is_npc is
            // a label rather than a structure - it changes no rule, it
            // decides which list you are looking at.
            ("is_npc", "is.false"),
            ("order", "name.asc"),
        ],
    )
}

/// The classes this game can choose from.
///
/// BOTH SPACES IN ONE QUERY, collapsed in Rust - the 004 tenancy the
/// catalogue has always had, and the same two passes
/// `equipment::collapse_overrides` makes. A table that has written its
/// own Fighter sees theirs and not the SRD one.
#[tauri::command]
pub fn list_classes(state: State<AppState>, game_id: String) -> Result<Vec<class::Class>, String> {
    let token = state.token()?;
    let rows = supabase::rest_get(
        &token,
        "classes",
        &[
            ("select", class::CLASS_COLUMNS),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
        ],
    )?;
    Ok(class::collapse(rows.as_array().unwrap_or(&Vec::new())))
}

/// The peoples this game can choose from.
///
/// THIS CAMPAIGN HAS NO SRD LAYER - every species in it is custom, so
/// the global rows ARE the campaign's. The tenancy collapse runs
/// anyway, because a second campaign wanting its own Unt'garoth is
/// exactly what it is for.
#[tauri::command]
pub fn list_species(
    state: State<AppState>,
    game_id: String,
) -> Result<Vec<species::Species>, String> {
    let token = state.token()?;
    let rows = supabase::rest_get(
        &token,
        "species",
        &[
            ("select", species::SPECIES_COLUMNS),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
        ],
    )?;
    Ok(species::collapse(rows.as_array().unwrap_or(&Vec::new())))
}

/* ============================ MAKING ONE ============================ */

/// Make a character.
///
/// `class_key` and `size` are OPTIONAL and the reason is not politeness
/// - it is that this command is also how a blank sheet gets made, and
/// refusing one would be refusing a workflow that already exists. What
/// changed is that giving a class now produces a finished character
/// instead of a named row.
///
/// SIZE DEFAULTS TO MEDIUM, and that is a real decision rather than a
/// shrug. Every playable species in the SRD is Small or Medium, the
/// difference between them changes no rule a character sheet reads
/// today, and the alternative - what was there before - is NULL, which
/// is what broke Snot. A stated default that is right nearly always
/// beats an absence that is useful never.
///
/// HIT POINTS ARE DERIVED, NOT ASKED FOR. `vitality::pc_hp` off the
/// class's die and the character's Constitution. Without a class there
/// is no die and `hp_max` stays NULL, which is honest: it says nobody
/// has decided what this character is yet, rather than inventing a d8.
#[tauri::command]
pub fn create_character(
    state: State<AppState>,
    game_id: String,
    name: String,
    token_name: Option<String>,
    class_key: Option<String>,
    species_key: Option<String>,
    size: Option<String>,
) -> Result<Value, String> {
    let session = state
        .current()?
        .ok_or_else(|| "not signed in".to_string())?;
    let token = &session.access_token;

    // A class named must be a class that exists. Writing an unknown key
    // would produce a character whose die nothing can find - the same
    // dangling-reference fault check_item_keys reports for objects, and
    // 055 cannot use a foreign key to prevent it.
    let chosen = match class_key.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(key) => {
            let rows = supabase::rest_get(
                token,
                "classes",
                &[
                    ("select", class::CLASS_COLUMNS),
                    ("key", &format!("eq.{}", key)),
                    ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
                ],
            )?;
            let found = class::collapse(rows.as_array().unwrap_or(&Vec::new()));
            match found.into_iter().next() {
                Some(c) => Some(c),
                None => return Err(format!("no such class: {}", key)),
            }
        }
        None => None,
    };

    // Same treatment as the class: named means it must exist, because
    // a key pointing at nothing produces a character whose bonuses
    // nothing can find and 056 cannot use a foreign key to stop it.
    let people = match species_key.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(key) => match crate::character::load_species(token, &game_id, key)? {
            Some(sp) => Some(sp),
            None => return Err(format!("no such species: {}", key)),
        },
        None => None,
    };

    // THE SPECIES DECIDES THE SIZE unless somebody says otherwise. An
    // Unt'garoth is Large, and that is a fact about the people rather
    // than a choice at the form - so an explicit argument still wins,
    // but the default comes from the row instead of from `med`.
    let size = size
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| people.as_ref().map(|sp| sp.size.clone()))
        .unwrap_or_else(|| "med".to_string());

    let mut row = json!({
        "game_id": game_id,
        "owner_uid": session.user_id,
        "name": name,
        "token_name": token_name,
        "size": size,
    });

    if let Some(sp) = &people {
        row["species_key"] = json!(sp.key);
    }
    if let Some(c) = &chosen {
        row["class_key"] = json!(c.key);
        // The class's saves and proficiencies, copied onto the
        // character. A SNAPSHOT, on purpose and on the same principle
        // 001 applied to a roll's names: what a character is proficient
        // with is a fact about the character, and a DM who rewrites the
        // class catalogue next month has not retrained anybody.
        row["weapon_profs"] = json!(c.weapon_profs);
        row["armor_profs"] = json!(c.armor_profs);
    }

    let created = supabase::rest_insert(token, "characters", &row)?;

    // HIT POINTS COME SECOND, because Constitution does not exist until
    // the row does - `characters_seed_abilities` is an AFTER INSERT
    // trigger, so there is no score to read before this point.
    let Some(id) = created
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("id"))
        .and_then(|x| x.as_str())
    else {
        return Ok(created);
    };

    // A SPECIES WITHOUT A CLASS IS STILL A SPECIES. The skills below are
    // granted either way; only the hit points need a die, so that is
    // what the class guard covers.
    let Some(c) = chosen else {
        if let Some(sp) = &people {
            for key in &sp.skill_profs {
                let _ = supabase::rest_upsert(
                    token,
                    "character_skills",
                    &json!({ "character_id": id, "skill_key": key, "prof": 1.0 }),
                    "character_id,skill_key",
                );
            }
        }
        return Ok(created);
    };

    // The species' granted skills. Unt'garoth get Athletics outright;
    // the DOUBLE proficiency their Enduring Might also grants applies
    // only to climbing, lifting and grappling, and nothing here can
    // tell which Athletics check is being made - so it stays a DM call
    // and the trait text says so.
    if let Some(sp) = &people {
        for key in &sp.skill_profs {
            // Best effort: a granted skill that fails to write is worth
            // less than refusing to make the character at all, and the
            // sheet will show it missing.
            let _ = supabase::rest_upsert(
                token,
                "character_skills",
                &json!({ "character_id": id, "skill_key": key, "prof": 1.0 }),
                "character_id,skill_key",
            );
        }
    }

    // CONSTITUTION WITH THE SPECIES BONUS IN IT, because hit points are
    // derived from the effective score and not the stored one. An
    // Unt'garoth's +1 is worth a point per level.
    let stored_con = load_con(token, id).unwrap_or(10);
    let con = match &people {
        Some(sp) => species::effective_score(
            stored_con,
            sp.bonus_for("con"),
            sp.maximum_for("con"),
        ),
        None => stored_con,
    };
    let hp = vitality::pc_hp(c.hit_die, 1, (con - 10).div_euclid(2));
    supabase::rest_update(
        token,
        "characters",
        &[("id", &format!("eq.{}", id))],
        &json!({ "hp_max": hp }),
    )
}

/// This character's Constitution score.
///
/// DEFAULTED TO TEN BY THE CALLER RATHER THAN HERE, so a read that
/// fails and a score that is genuinely 10 stay distinguishable at the
/// one place that has to choose between them.
fn load_con(token: &str, character_id: &str) -> Option<i64> {
    let rows = supabase::rest_get(
        token,
        "character_abilities",
        &[
            ("select", "score"),
            ("character_id", &format!("eq.{}", character_id)),
            ("ability", "eq.con"),
        ],
    )
    .ok()?;
    rows.as_array()?
        .first()?
        .get("score")
        .and_then(|x| x.as_i64())
}

/* ======================== WHAT THEY LOOK LIKE ======================== */

/// Write a character's physical description. 058.
///
/// EVERY FIELD IS OPTIONAL AND EVERY FIELD IS SENT. An absent argument
/// leaves the column alone; an empty string CLEARS it. That is 036's
/// convention for overrides and 049's for the object editor, and the
/// reason is the same one: "unset it" and "do not touch it" are
/// different instructions and a form has to be able to give both.
///
/// HEIGHT AND WEIGHT ARE NOT VALIDATED AGAINST THE SPECIES. An
/// Unt'garoth of six feet is short for their people, not illegal, and
/// refusing them would be this app overruling a DM about their own
/// world. The panel SAYS when somebody sits outside their band, which
/// is the useful half of the same observation.
#[tauri::command]
pub fn set_description(
    state: State<AppState>,
    character_id: String,
    height_ft: Option<String>,
    weight_lb: Option<String>,
    hair: Option<String>,
    skin: Option<String>,
    eyes: Option<String>,
    description: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    let mut patch = json!({});

    for (field, given) in [
        ("height_ft", &height_ft),
        ("weight_lb", &weight_lb),
    ] {
        let Some(raw) = given.as_deref().map(str::trim) else { continue };
        if raw.is_empty() {
            patch[field] = Value::Null;
            continue;
        }
        let n: f64 = raw
            .parse()
            .map_err(|_| format!("{} wants a number, not '{}'", field, raw))?;
        if n <= 0.0 {
            return Err(format!("{} has to be more than nothing", field));
        }
        patch[field] = json!(n);
    }

    for (field, given) in [
        ("hair", &hair),
        ("skin", &skin),
        ("eyes", &eyes),
        ("description", &description),
    ] {
        let Some(raw) = given.as_deref().map(str::trim) else { continue };
        patch[field] = if raw.is_empty() { Value::Null } else { json!(raw) };
    }

    if patch.as_object().map(|o| o.is_empty()).unwrap_or(true) {
        return Err("nothing to change".to_string());
    }

    supabase::rest_update(
        &token,
        "characters",
        &[("id", &format!("eq.{}", character_id))],
        &patch,
    )
}

/* ======================= WHEN THE NUMBERS MOVE ======================= */

/// Recompute a character's hit point maximum from what it is derived
/// from, and write it.
///
/// THE BUG THIS EXISTS TO KILL. A maximum was computed once, at
/// creation, and never again. `set_level` wrote `{"level": n}` and
/// stopped; `set_ability` wrote a score and stopped. So Garn - a level
/// 5 Barbarian with Constitution 13, who should have 45 - sat on the
/// sheet with 12, which is what a d12 gives at level 1 with the 10 that
/// `seed_character_abilities` writes before anybody has chosen
/// anything. Both numbers the maximum comes from had moved and the
/// maximum had not heard about either.
///
/// 029 ASKED FOR EXACTLY THIS and got half of it: "a set level button
/// that allows a DM to add or subtract levels - this should ripple
/// through their HPs". `set_actor_level` does ripple, which is why a
/// goblin's maximum tracks its level and a player's did not. The rule
/// was written once for monsters and the characters never got it.
///
/// ONE PLACE, CALLED FROM BOTH WRITERS, for the reason the numeric
/// helper in supabase.rs exists: a rule with two copies is a rule with
/// two answers, and the second one is always the stale one.
///
/// RETURNS None WHEN THERE IS NOTHING TO DERIVE FROM, and writes
/// nothing in that case. A character with no class has no die, and a
/// stated maximum is a real thing - 029 made the same call for a
/// statblock whose hit points are copied from the book. Silence is the
/// honest answer; inventing a d8 is not.
pub(crate) fn rederive_hp_max(token: &str, character_id: &str) -> Result<Option<i64>, String> {
    let rows = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "id,level,class_key,game_id,species_key,is_npc"),
            ("id", &format!("eq.{}", character_id)),
        ],
    )?;
    let Some(row) = rows.as_array().and_then(|a| a.first()) else {
        return Ok(None);
    };

    // A MONSTER'S HIT POINTS ARE NOT A CHARACTER'S, and 064 is what
    // made saying so necessary. Before it every NPC had `class_key`
    // NULL and fell out of this function on the next check; now a
    // goblin is a Rogue, and without this guard the first touch of
    // their level would quietly recompute them on 061's `pc_hp` -
    // the Goblin Scout going from 28 hit points to 43 with nothing on
    // screen to explain it.
    //
    // 029 settled it: a monster is level times the die their SIZE
    // gives, because the Monster Manual writes 7 (2d6) and that is
    // what makes "set level" a button rather than a rewrite. They
    // level through `commands::dm::set_actor_level`, which applies
    // that rule. A class gives them an attack count and a name; it
    // does not give them a hit die.
    if row.get("is_npc").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Ok(None);
    }

    let Some(key) = row.get("class_key").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
    else {
        // No class, no die. A monster levels through
        // `commands::dm::set_actor_level`, which has its own rule
        // because its die comes from size.
        return Ok(None);
    };
    let level = row.get("level").and_then(|v| v.as_i64()).unwrap_or(1);
    let game_id = row.get("game_id").and_then(|v| v.as_str()).unwrap_or_default();

    let class_rows = supabase::rest_get(
        token,
        "classes",
        &[
            ("select", class::CLASS_COLUMNS),
            ("key", &format!("eq.{}", key)),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
        ],
    )?;
    let Some(c) = class::collapse(class_rows.as_array().unwrap_or(&Vec::new()))
        .into_iter()
        .next()
    else {
        // A class_key pointing at nothing. 055 cannot use a foreign key
        // to prevent this, so it is reported rather than guessed around.
        return Err(format!("{} has an unknown class: {}", character_id, key));
    };

    // THE EFFECTIVE CONSTITUTION, species bonus included - the same
    // number creation used. Reading the stored score here is how a
    // re-derivation would quietly disagree with the value it replaced.
    // Resolved here first, and correctly - which made it the second
    // copy of a rule that turned out to be wrong in three other
    // places. One loader answers it now for all four: this, the
    // target list, the initiative roll and the level button.
    let eff = crate::character::load_effective(token, game_id, &[character_id.to_string()])?;
    let hp = vitality::pc_hp(c.hit_die, level, eff.modifier(character_id, "con"));
    supabase::rest_update(
        token,
        "characters",
        &[("id", &format!("eq.{}", character_id))],
        &json!({ "hp_max": hp }),
    )?;
    Ok(Some(hp))
}
