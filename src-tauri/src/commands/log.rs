//! What has happened in this fight.
//!
//! Thin, per commands/mod.rs. The counting rule is `src/spent.rs`
//! where it has tests; everything here is one read and a shape.
//!
//! WHY THE RUN TAB NEEDS ITS OWN READ. `list_rolls` is the Play tab's
//! log: the whole game, newest fifty, flat. A DM running a fight wants
//! the opposite selection - this encounter only, grouped by round, with
//! who swung at whom - and filtering the game-wide list in JavaScript
//! would mean the Run tab quietly showing "nothing yet" whenever the
//! fight's rolls fell off the end of those fifty.
//!
//! ONE REQUEST FOR THE DICE. The rolls come back embedded under their
//! action rather than as a second read joined by hand - 012 made the
//! action own its rolls precisely so they could be asked for together.

use serde_json::{json, Value};
use tauri::State;

use crate::spent::{self, Act};
use crate::supabase::{self, AppState};

/// Everything done in one encounter, newest first, with what each
/// creature has spent in the round now being played.
///
/// THE ROUND IS READ, NOT ACCEPTED. The caller knows the round - it is
/// on screen - but a client-supplied round is one a client can get
/// wrong, and the count built on it would be calmly false rather than
/// obviously broken. Same reasoning as 054's trigger, one layer up.
#[tauri::command]
pub fn encounter_log(
    state: State<AppState>,
    encounter_id: String,
    limit: Option<i64>,
) -> Result<Value, String> {
    let token = state.token()?;

    let enc = supabase::rest_get(
        &token,
        "encounters",
        &[
            ("select", "id,round"),
            ("id", &format!("eq.{}", encounter_id)),
        ],
    )?;
    let round = enc
        .as_array()
        .and_then(|a| a.first())
        .and_then(|e| e.get("round"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    // A fight that has run long enough to need paging is a fight the
    // DM is scrolling anyway; 200 is generous and bounded.
    let cap = limit.unwrap_or(200).clamp(1, 500).to_string();

    let rows = supabase::rest_get(
        &token,
        "actions",
        &[
            (
                "select",
                "id,round,turn_actor_id,key,cost,request,label,created_at,\
                 actor_id,character_id,target_actor_id,target_challenge_id,\
                 rolls(id,role,label,request,formula,detail,total,natural_roll,\
                 character_name,target_value,target_kind,target_label,\
                 success,reason,margin,face_outcome)",
            ),
            ("encounter_id", &format!("eq.{}", encounter_id)),
            ("order", "created_at.desc"),
            ("limit", &cap),
        ],
    )?;

    let mut actions = rows.as_array().cloned().unwrap_or_default();

    // WHETHER EACH ONE WAS TAKEN IN TURN, decided by `spent::out_of_turn`
    // rather than by the screen comparing two ids. It is one comparison,
    // and one comparison written on a screen is how the frontend came to
    // disagree with the engine about whether a goblin may heal itself.
    for a in actions.iter_mut() {
        let out = crate::spent::out_of_turn(
            a.get("actor_id").and_then(|v| v.as_str()),
            a.get("turn_actor_id").and_then(|v| v.as_str()),
        );
        a["out_of_turn"] = json!(out);
    }

    // The count is derived from exactly the rows that were just read,
    // so the log and the tally can never disagree with each other.
    let acts: Vec<Act> = actions
        .iter()
        .map(|a| Act {
            actor_id: a
                .get("actor_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            key: a
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("custom")
                .to_string(),
            round: a.get("round").and_then(|v| v.as_i64()),
            cost: a.get("cost").and_then(|v| v.as_str()).map(String::from),
        })
        .collect();

    Ok(json!({
        "round": round,
        "actions": actions,
        "spent": spent::this_round(&acts, round, &budgets_for(&token, &encounter_id)?),
    }))
}

/// What each creature in this fight is owed in a turn, keyed by the
/// ROSTER row rather than by the character.
///
/// THE KEY IS THE ACTOR because that is what an action records and what
/// the screen paints. Two goblins off one statblock are two actors and
/// two characters since 022, so the two ids do not collapse - but an
/// action points at the actor, and translating on the way in beats
/// making `spent` learn about characters.
///
/// A ROSTER WITH NO CLASSES COSTS ONE REQUEST, not four:
/// `load_effective` returns early on an empty id list, and a fight of
/// nothing but monsters supplies one. Every monster then falls through
/// to `Budget::default` - one swing - which is both true today and the
/// safe way to be wrong, since a statblock's multiattack is a
/// different mechanism nothing reads yet.
fn budgets_for(
    token: &str,
    encounter_id: &str,
) -> Result<Vec<(String, crate::spent::Budget)>, String> {
    let rows = supabase::rest_get(
        token,
        "encounter_actors",
        &[
            // THROUGH `characters`, not through `encounters`. 051 gave
            // encounters a second path back to this table and PostgREST
            // refuses an ambiguous embed outright - see the trap in
            // STATUS. This hop is the unambiguous one.
            ("select", "id,character_id,characters(game_id)"),
            ("encounter_id", &format!("eq.{}", encounter_id)),
        ],
    )?;
    let rows = rows.as_array().cloned().unwrap_or_default();

    let game_id = rows
        .iter()
        .find_map(|r| {
            r.get("characters")
                .and_then(|c| c.get("game_id"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or_default()
        .to_string();

    let ids: Vec<String> = rows
        .iter()
        .filter_map(|r| r.get("character_id").and_then(|v| v.as_str()))
        .map(String::from)
        .collect();

    let eff = crate::character::load_effective(token, &game_id, &ids)?;

    Ok(rows
        .iter()
        .filter_map(|r| {
            let actor = r.get("id").and_then(|v| v.as_str())?;
            let cid = r.get("character_id").and_then(|v| v.as_str())?;
            Some((
                actor.to_string(),
                crate::spent::Budget::with_attacks(eff.attacks(cid)),
            ))
        })
        .collect())
}
