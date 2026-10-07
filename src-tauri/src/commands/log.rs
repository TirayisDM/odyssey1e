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
            // 153. THE FK IS NAMED, and `rolls` has only ever had one
            // path back to `actions`. That is the point: naming it costs
            // nothing today and means a later migration adding a second
            // reference cannot turn this into a 300 the way 139 did to
            // `encounter_actors -> characters`. A bare embed is a bet
            // that the schema will not grow.
            (
                "select",
                "id,round,turn_actor_id,key,cost,request,label,created_at,\
                 actor_id,character_id,target_actor_id,target_challenge_id,\
                 rolls!rolls_action_id_fkey(id,role,label,request,formula,detail,total,natural_roll,\
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
            // 149. 060's stamp, which the log has painted since it
            // existed and the count can now read.
            turn_actor_id: a
                .get("turn_actor_id")
                .and_then(|v| v.as_str())
                .map(String::from),
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
            // STATUS.
            //
            // 153. NAMED, because 139 then did the same thing to this
            // pair: `encounter_actors.template_id` is a second reference
            // to `characters`, so a bare `characters(...)` stopped being
            // the unambiguous hop this comment used to claim it was.
            // The constraint name is the one spelling that stays right
            // however many more references get added.
            ("select", "id,character_id,characters!encounter_actors_character_id_fkey(game_id,npc_key)"),
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

    // 149. AND WHAT EACH ONE MAY DO OUT OF TURN, which lives on the
    // statblock and is read through `npc_key` - the same hop 121 makes
    // for a monster's resistances, for the same reason: it is a fact
    // about the TYPE and copying it onto every individual would be a
    // second place for it to be wrong.
    //
    // ONE QUERY FOR THE WHOLE ROSTER rather than one per actor, and
    // skipped entirely when nothing in the fight came off a statblock,
    // which is most fights.
    let keys: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            r.get("characters")
                .and_then(|c| c.get("npc_key"))
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .collect();
    let legendary = legendary_by_key(token, &keys)?;

    Ok(rows
        .iter()
        .filter_map(|r| {
            let actor = r.get("id").and_then(|v| v.as_str())?;
            let cid = r.get("character_id").and_then(|v| v.as_str())?;
            let allowed = r
                .get("characters")
                .and_then(|c| c.get("npc_key"))
                .and_then(|v| v.as_str())
                .and_then(|k| legendary.iter().find(|(key, _)| key == k))
                .map(|(_, n)| *n)
                .unwrap_or(0);
            Some((
                actor.to_string(),
                crate::spent::Budget::with_attacks(eff.attacks(cid)).with_legendary(allowed),
            ))
        })
        .collect())
}

/// The legendary allowance of each statblock named, by key.
///
/// 149. A GAME'S OWN ROW WINS over the shared one, the way every
/// tenanted read in this schema resolves - a DM who writes their own
/// goblin and gives it legendary actions gets them.
///
/// AN EMPTY LIST IS NOT A QUERY. Most fights have no monsters off a
/// statblock at all, and asking PostgREST for `key=in.()` is both a
/// wasted round trip and a request shaped like a mistake.
fn legendary_by_key(token: &str, keys: &[String]) -> Result<Vec<(String, i64)>, String> {
    let mut want: Vec<&String> = keys.iter().collect();
    want.sort();
    want.dedup();
    if want.is_empty() {
        return Ok(Vec::new());
    }
    let list = want
        .iter()
        .map(|k| k.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let rows = supabase::rest_get(
        token,
        "npcs",
        &[
            ("select", "key,game_id,legendary_actions"),
            ("key", &format!("in.({})", list)),
            // The game's own row sorts first - NULLs last - so the
            // first hit for a key is the one that should win.
            ("order", "game_id.asc.nullslast"),
        ],
    )?;
    let mut out: Vec<(String, i64)> = Vec::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let Some(key) = r.get("key").and_then(|v| v.as_str()) else {
            continue;
        };
        if out.iter().any(|(k, _)| k == key) {
            continue;
        }
        out.push((
            key.to_string(),
            r.get("legendary_actions").and_then(|v| v.as_i64()).unwrap_or(0),
        ));
    }
    Ok(out)
}
