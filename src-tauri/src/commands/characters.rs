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
            // 123. A TEMPLATE IS NOT A CREATURE IN THE WORLD.
            ("is_template", "is.false"),
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

/// Seven ability scores, 3d6 with every 1 rerolled. 066.
///
/// NO DATABASE AND NO CHARACTER. The dice are rolled on this device
/// before anything exists to put them on - the same decision 012 made
/// about `create_roll`, and the reason the engine is in Rust at all.
/// A player sees seven numbers and assigns six of them; the spare is
/// the point.
///
/// NOTHING IS RECORDED. A spread rerolled until it is liked would be a
/// row per attempt in a log nobody asked for, and whether that is
/// allowed is a table's business rather than this app's.
#[tauri::command]
pub fn roll_ability_spread() -> Vec<i64> {
    crate::generation::spread()
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
    // 066. Ability code to score, as the creation screen assigned
    // them. Absent leaves the ten the seed trigger writes, which is
    // what every character made before this got.
    abilities: Option<Vec<(String, i64)>>,
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
    // CHECKED BEFORE THE INSERT, so a bad assignment costs nothing. The
    // screen's dropdown makes a duplicate impossible by construction,
    // and `generation::complete` is the rule that does not depend on
    // there being a dropdown.
    if let Some(picks) = &abilities {
        crate::generation::complete(picks)?;
        // AND THE PEOPLE'S OWN CEILINGS. A Ny'ook cannot naturally
        // carry a Strength above 13 however the dice fell, so the
        // assignment is refused here rather than accepted and then
        // contradicted by every modifier derived from it.
        //
        // The spare roll is what makes this fair rather than punishing:
        // a player holding a 16 they cannot put on Strength has five
        // other places for it and one to discard - which is what 066
        // rolled seven for.
        for (code, score) in picks {
            species::within_natural_cap(people.as_ref(), code, *score)?;
        }
    }

    let size = size
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| people.as_ref().map(|sp| sp.size.clone()))
        .unwrap_or_else(|| "med".to_string());

    let mut row = json!({
        "game_id": game_id,
        "owner_uid": session.profile_id,
        "name": name,
        "token_name": token_name,
        "size": size,
    });

    if let Some(sp) = &people {
        row["species_key"] = json!(sp.key);
    }
    if let Some(c) = &chosen {
        // Written on the insert as well as into `character_classes`
        // below. 073's trigger would set it anyway; doing it here means
        // the row is never momentarily a classed character with no
        // class, which is a state a concurrent read could see.
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

    // 073. THE CLASS GETS ITS OWN ROW, which is what makes a second one
    // possible later. Level 1, because that is where a character
    // starts and `characters.level` defaults to the same.
    //
    // THIS ROW IS THE STARTING CLASS for the rest of their career - its
    // `added_at` is what `multiclass::hp` reads to decide whose whole
    // hit die pays for level one.
    if let Some(c) = &chosen {
        supabase::rest_upsert(
            token,
            "character_classes",
            &json!({ "character_id": id, "class_key": c.key, "level": 1 }),
            "character_id,class_key",
        )?;
    }

    // 078. AND THE SAVES THAT CLASS GRANTS. Best effort for the same
    // reason the skills below are: a save proficiency that fails to
    // write is worth less than refusing to make the character, and the
    // sheet shows it missing.
    let _ = apply_class_saves(token, id);

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

    // THE ROLLED SCORES, BEFORE THE HIT POINTS ARE WORKED OUT. The
    // order is the whole of it: `seed_character_abilities` writes ten
    // across the board on insert, and reading Constitution before
    // these land would give every character the hit points of a 10 -
    // a Barbarian who rolled 16 would be quietly short for their
    // whole career.
    if let Some(picks) = &abilities {
        for (code, score) in picks {
            supabase::rest_update(
                token,
                "character_abilities",
                &[
                    ("character_id", &format!("eq.{}", id)),
                    ("ability", &format!("eq.{}", code)),
                ],
                &json!({ "score": score }),
            )?;
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

/* ============================ RENAMING ============================ */

/// Change what a character is called.
///
/// 072. The sheet could set a level and could not set a name. Every
/// other name in the app had an editor - an object has `rename_object`,
/// an encounter actor has `rename_actor` - and the one a player types
/// first was fixed at creation and never again. A typo at the form was
/// permanent.
///
/// TWO COLUMNS, ONE RULE, AND THE RULE IS IN naming.rs. `token_name` is
/// what the roster and the roll cards actually print, so moving `name`
/// alone would change the sheet heading and nothing else. Whether the
/// short name follows is a judgement - it follows when it was a copy
/// and stays when it was a decision - and judgements belong somewhere
/// they can be tested, which a command is not.
///
/// NOT RETROSPECTIVE. Rolls and actions carry the name that was true
/// when they happened, by 001's design, and this does not go back and
/// rewrite them. An actor already standing in an encounter keeps its
/// own label too; `rename_actor` is the tool for that, and the two are
/// separate because a disguise is a real thing to want.
#[tauri::command]
pub fn rename_character(
    state: State<AppState>,
    character_id: String,
    name: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let wanted = crate::naming::clean(&name)?;

    // READ BEFORE WRITE, because the short name's fate depends on what
    // the long one used to be. One row, by id.
    let rows = supabase::rest_get(
        &token,
        "characters",
        &[
            ("select", "name,token_name"),
            ("id", &format!("eq.{}", character_id)),
        ],
    )?;
    let Some(row) = rows.as_array().and_then(|a| a.first()) else {
        return Err("no such character".to_string());
    };
    let was = row.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let token_was = row.get("token_name").and_then(|v| v.as_str());

    // NULL IS SENT EXPLICITLY when there was no short name, rather than
    // left out. The outcome is the same either way - but saying it means
    // this patch describes the whole of both name columns, and a reader
    // does not have to know PostgREST merge rules to see that.
    let short = crate::naming::renamed_token(was, token_was, &wanted);

    supabase::rest_update(
        &token,
        "characters",
        &[("id", &format!("eq.{}", character_id))],
        &json!({ "name": wanted, "token_name": short }),
    )
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

    let game_id = row.get("game_id").and_then(|v| v.as_str()).unwrap_or_default();

    // 073. EVERY CLASS THEY HOLD, in the order they took them, because
    // the first level's whole hit die belongs to the one they STARTED
    // as and is paid once in a career. This used to read `class_key`
    // and one level off the row above, which is the single-class case
    // of the same question.
    let taken = class::load_taken(token, game_id, character_id)?;
    if taken.is_empty() {
        // No class, no die. A monster levels through
        // `commands::dm::set_actor_level`, which has its own rule
        // because its die comes from size.
        return Ok(None);
    }

    // A class_key pointing at nothing. 055 cannot use a foreign key to
    // prevent this, so it is reported rather than guessed around -
    // `load_taken` keeps the row with a die of zero precisely so this
    // can say which class is missing instead of silently costing the
    // character the levels they put into it.
    if let Some(lost) = taken.iter().find(|t| t.hit_die <= 0) {
        return Err(format!("{} has an unknown class: {}", character_id, lost.key));
    }

    // THE EFFECTIVE CONSTITUTION, species bonus included - the same
    // number creation used. Reading the stored score here is how a
    // re-derivation would quietly disagree with the value it replaced.
    // Resolved here first, and correctly - which made it the second
    // copy of a rule that turned out to be wrong in three other
    // places. One loader answers it now for all four: this, the
    // target list, the initiative roll and the level button.
    let eff = crate::character::load_effective(token, game_id, &[character_id.to_string()])?;
    let hp = crate::multiclass::hp(&taken, eff.modifier(character_id, "con"));
    supabase::rest_update(
        token,
        "characters",
        &[("id", &format!("eq.{}", character_id))],
        &json!({ "hp_max": hp }),
    )?;
    Ok(Some(hp))
}

/// Turn on the saving throw proficiencies this character's STARTING
/// class grants.
///
/// 078. `classes.saving_throws` has existed since 055, is parsed into
/// `Class`, and is printed on the creation form's class picker - and
/// nothing has ever written it to a character. Every character in the
/// game has been rolling saves short by their proficiency bonus with
/// nothing on screen to explain it. The same shape as `price_override`,
/// which 049 added and nothing applied until 070: a column that exists
/// to serve a rule, and the rule never asks.
///
/// ADDITIVE, AND THAT IS DELIBERATE. This turns proficiencies ON and
/// never off. A species or a feat may grant a save this does not know
/// about and a DM may tick one by hand, and a re-derivation that
/// cleared the others would quietly undo both. The cost is that
/// swapping a starting class leaves the old class's saves behind, which
/// is a tick to undo rather than a wrong answer nobody can see.
///
/// SILENT WHEN THERE IS NOTHING TO GRANT, like `rederive_hp_max`: a
/// classless character and a monster both pass through writing nothing.
pub(crate) fn apply_class_saves(token: &str, character_id: &str) -> Result<Vec<String>, String> {
    let game_id = game_of(token, character_id)?;
    let taken = class::load_taken(token, &game_id, character_id)?;
    if taken.is_empty() {
        return Ok(Vec::new());
    }

    let catalogue = class::load_map(
        token,
        &game_id,
        &taken.iter().map(|t| t.key.clone()).collect::<Vec<_>>(),
    )?;
    // IN THE ORDER TAKEN, because the rule reads the first of them and
    // `load_taken` is already ordered by `added_at`. A class the
    // catalogue cannot find keeps its place with no saves rather than
    // being dropped, which would promote the class behind it into a
    // starting class it never was.
    let by_class: Vec<(String, Vec<String>)> = taken
        .iter()
        .map(|t| {
            let saves = catalogue
                .iter()
                .find(|c| c.key == t.key)
                .map(|c| c.saving_throws.clone())
                .unwrap_or_default();
            (t.key.clone(), saves)
        })
        .collect();

    let granted = crate::multiclass::saves_granted(&by_class);
    if granted.is_empty() {
        return Ok(granted);
    }

    supabase::rest_update(
        token,
        "character_abilities",
        &[
            ("character_id", &format!("eq.{}", character_id)),
            ("ability", &format!("in.({})", granted.join(","))),
        ],
        &json!({ "save_prof": true }),
    )?;
    Ok(granted)
}

/* ======================== WHAT THEY ARE ======================== */

/// Put a character at `level` in `class_key`, adding the class if they
/// did not have it.
///
/// 073. ONE BUTTON FOR THREE THINGS - take a class, change its level,
/// swap a slot for a different class - because on screen they are one
/// control: a dropdown saying which class, a number saying how far, and
/// a button that commits both. Splitting them into three commands would
/// make the screen decide which to call, which is the screen deciding a
/// rule.
///
/// `replacing` IS WHAT THAT SLOT USED TO HOLD. A player who had Fighter
/// in the first slot and picks Rogue has not become a Fighter/Rogue -
/// they have corrected what they are. Without this the only thing that
/// could happen is gaining a class, and nothing could ever be undone
/// except by removing it.
///
/// THE HIT POINTS FOLLOW, every time. `characters.level` follows too,
/// but through `sync_character_level` rather than from here - a trigger
/// cannot be forgotten and a command can.
#[tauri::command]
pub fn set_class_level(
    state: State<AppState>,
    character_id: String,
    class_key: String,
    level: i64,
    replacing: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    let key = class_key.trim();
    if key.is_empty() {
        return Err("choose a class".to_string());
    }

    let game_id = game_of(&token, &character_id)?;

    // A class named must be a class that exists - the same check
    // creation makes, for the same reason: writing an unknown key
    // produces a character whose die nothing can find.
    let found = class::load_map(&token, &game_id, &[key.to_string()])?;
    if !found.iter().any(|c| c.key == key) {
        return Err(format!("no such class: {}", key));
    }

    // THE CEILING IS ON THE TOTAL, not on the class. Checked against
    // what they hold now with this class's own levels taken back out,
    // so raising a Fighter 15 to 16 is one more level rather than
    // sixteen more - see multiclass::room_for.
    let mut held = class::load_taken(&token, &game_id, &character_id)?;
    if let Some(was) = replacing.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        held.retain(|t| t.key != was);
    }
    crate::multiclass::room_for(&held, key, level)?;

    // THE SWAP IS A DELETE AND AN INSERT, not an update of the key.
    // `added_at` is what decides the starting class and the tie for
    // which class leads, and a character who was never a Fighter should
    // not inherit the date they stopped being one.
    if let Some(was) = replacing.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if was != key {
            supabase::rest_delete(
                &token,
                "character_classes",
                &[
                    ("character_id", &format!("eq.{}", character_id)),
                    ("class_key", &format!("eq.{}", was)),
                ],
            )?;
        }
    }

    let written = supabase::rest_upsert(
        &token,
        "character_classes",
        &json!({ "character_id": character_id, "class_key": key, "level": level }),
        "character_id,class_key",
    )?;
    rederive_hp_max(&token, &character_id)?;
    // 078. A character who had NO class and now has one has just
    // acquired a starting class, and with it their saves. One who
    // already had a starting class gains nothing here, because
    // `saves_granted` reads the first row and this is not it.
    apply_class_saves(&token, &character_id)?;
    Ok(written)
}

/// Drop a class entirely.
///
/// A DELETE RATHER THAN A LEVEL OF ZERO, because zero levels in a class
/// is not a thing a character is - see the column comment in 073. The
/// trigger recomputes the total and promotes whatever leads now.
///
/// DROPPING THE LAST ONE LEAVES THEM CLASSLESS AT THE LEVEL THEY
/// REACHED, which is the state every character made before 055 is in,
/// and `rederive_hp_max` then has nothing to derive from and says so by
/// writing nothing.
#[tauri::command]
pub fn remove_class(
    state: State<AppState>,
    character_id: String,
    class_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_delete(
        &token,
        "character_classes",
        &[
            ("character_id", &format!("eq.{}", character_id)),
            ("class_key", &format!("eq.{}", class_key.trim())),
        ],
    )?;
    rederive_hp_max(&token, &character_id)?;
    Ok(json!({ "ok": true }))
}

/// Which game a character belongs to. One column, by id.
///
/// NEEDED BECAUSE THE CATALOGUE IS TENANTED: a class key resolves
/// against the global rows and this game's, and asking without the game
/// would let one game's homebrew class be taken by another.
fn game_of(token: &str, character_id: &str) -> Result<String, String> {
    let rows = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "game_id"),
            ("id", &format!("eq.{}", character_id)),
        ],
    )?;
    rows.as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("game_id"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| "no such character".to_string())
}
