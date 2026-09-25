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
                "id,round,key,request,label,created_at,actor_id,character_id,\
                 target_actor_id,target_challenge_id,\
                 rolls(id,role,label,request,formula,detail,total,natural_roll,\
                 character_name,target_value,target_kind,target_label,\
                 success,reason,margin,face_outcome)",
            ),
            ("encounter_id", &format!("eq.{}", encounter_id)),
            ("order", "created_at.desc"),
            ("limit", &cap),
        ],
    )?;

    let actions = rows.as_array().cloned().unwrap_or_default();

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
        })
        .collect();

    Ok(json!({
        "round": round,
        "actions": actions,
        "spent": spent::this_round(&acts, round),
    }))
}
