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

    let size = size
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("med")
        .to_string();

    let mut row = json!({
        "game_id": game_id,
        "owner_uid": session.user_id,
        "name": name,
        "token_name": token_name,
        "size": size,
    });

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
    let Some(c) = chosen else {
        return Ok(created);
    };
    let Some(id) = created
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("id"))
        .and_then(|x| x.as_str())
    else {
        return Ok(created);
    };

    let con = load_con(token, id).unwrap_or(10);
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
