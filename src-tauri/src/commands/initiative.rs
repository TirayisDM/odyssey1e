//! Rolling for initiative, and walking the turn around.
//!
//! Thin, per commands/mod.rs. The order and the walk are
//! `src/initiative.rs` where they have tests; everything here is
//! reading rows, writing two columns and turning a d20 into a number.
//!
//! WHO MAY DO WHAT is 011's policies, unchanged and not restated. The
//! DM owns the encounter, so advancing the turn and typing somebody's
//! initiative are refused by Postgres for anyone else. Rolling YOUR OWN
//! character's initiative is the exception the enrolment prompt needs,
//! and `encounter_actors`' update policy already allows it.
//!
//! NOTHING HERE GATES A ROLL. A creature can attack out of turn and the
//! rig will let it - see initiative.rs for why that is a decision
//! rather than an omission.

use serde_json::{json, Value};
use tauri::State;

use crate::initiative::{self, Contender};
use crate::supabase::{self, AppState};

/// Turn a policy refusal into a sentence. Same helper as dm.rs, same
/// reason.
fn denied(e: String, what: &str) -> String {
    if e.contains("(401)") || e.contains("(403)") || e.contains("42501") {
        format!("only the DM of this game can {} — you are signed in as a player", what)
    } else {
        e
    }
}

/// Everyone in the fight, in order, with the round and whose turn it is.
///
/// FOUR READS, because DEX lives on the character and the character is
/// one hop from the actor. The alternative is a view, and 011 kept the
/// roster a plain table on purpose.
#[tauri::command]
pub fn turn_order(state: State<AppState>, encounter_id: String) -> Result<Value, String> {
    let token = state.token()?;

    let enc = supabase::rest_get(
        &token,
        "encounters",
        &[
            ("select", "id,name,status,round,turn_actor_id"),
            ("id", &format!("eq.{}", encounter_id)),
        ],
    )?;
    let enc = enc
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| "that encounter is not visible to you".to_string())?;

    let rows = supabase::rest_get(
        &token,
        "encounter_actors",
        &[
            (
                "select",
                "id,label,character_id,npc_key,initiative,active,dead,enrolled_at",
            ),
            ("encounter_id", &format!("eq.{}", encounter_id)),
        ],
    )?;
    let actors = rows.as_array().cloned().unwrap_or_default();
    if actors.is_empty() {
        return Ok(json!({
            "encounter": enc, "order": [], "current": Value::Null
        }));
    }

    let dex = dex_by_character(&token, &actors)?;

    let contenders: Vec<Contender> = actors
        .iter()
        .map(|a| Contender {
            id: as_text(a, "id"),
            name: as_text(a, "label"),
            initiative: a.get("initiative").and_then(|v| v.as_i64()),
            dex: a
                .get("character_id")
                .and_then(|v| v.as_str())
                .and_then(|c| dex.get(c))
                .copied()
                .unwrap_or(0),
            active: a.get("active").and_then(|v| v.as_bool()).unwrap_or(true),
            dead: a.get("dead").and_then(|v| v.as_bool()).unwrap_or(false),
        })
        .collect();

    let ordered = initiative::order(&contenders);
    let current = enc.get("turn_actor_id").and_then(|v| v.as_str());

    // The screen wants the actor rows, not just the ids - so the sorted
    // ids are used to reorder what was already read rather than read
    // again.
    let sorted: Vec<Value> = ordered
        .iter()
        .filter_map(|c| {
            let row = actors.iter().find(|a| as_text(a, "id") == c.id)?;
            let mut row = row.clone();
            row["dex_mod"] = json!(c.dex);
            row["takes_turns"] = json!(c.takes_turns());
            row["is_current"] = json!(Some(c.id.as_str()) == current);
            Some(row)
        })
        .collect();

    Ok(json!({
        "encounter": enc,
        "order": sorted,
        // What the NEXT press would do, so the button can say it.
        "up_next": initiative::next_turn(&ordered, current),
    }))
}

/// Roll one creature's initiative: a d20 and its DEX.
///
/// NOT AN ACTION, and that is the one interesting decision here. 012
/// made every roll an action because "a miss spends an initiative slot
/// the same as a hit" - but an initiative roll does not spend a slot,
/// it DECIDES the slots. Writing it as an action would put a row in the
/// log that no turn paid for and that the XP review would later have to
/// learn to ignore.
///
/// So it writes the column and returns the dice for the screen to
/// show. The number is auditable because it is on the roster where
/// anybody can see and change it.
#[tauri::command]
pub fn roll_initiative(state: State<AppState>, actor_id: String) -> Result<Value, String> {
    let token = state.token()?;
    roll_for(&token, &actor_id)
}

/// The roll itself, without a command around it.
///
/// Split out because enrolment rolls for a statblock the moment it is
/// made - nobody wants to roll for eight goblins - and a second copy of
/// "d20 plus DEX, written to the column" is a second place for the
/// house rule to change and be missed.
pub(crate) fn roll_for(token: &str, actor_id: &str) -> Result<Value, String> {
    let dex = dex_of_actor(token, actor_id)?;
    let r = crate::dice::roll_formula("1d20")?;
    let total = initiative::score(r.total, dex);

    supabase::rest_update(
        token,
        "encounter_actors",
        &[("id", &format!("eq.{}", actor_id))],
        &json!({ "initiative": total }),
    )
    .map_err(|e| denied(e, "roll for that creature"))?;

    Ok(json!({
        "initiative": total,
        "d20": r.total,
        "dex_mod": dex,
        "said": format!("1d20 [{}] {}{} = {}", r.total,
                        if dex < 0 { "-" } else { "+" }, dex.abs(), total),
    }))
}

/// Type a number in instead of rolling one.
///
/// The physical table rolled real dice and the app should not argue.
/// Also the way a DM fixes a tie they have adjudicated, since 051
/// deliberately stores no tiebreak.
#[tauri::command]
pub fn set_initiative(
    state: State<AppState>,
    actor_id: String,
    initiative: Option<i64>,
) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_update(
        &token,
        "encounter_actors",
        &[("id", &format!("eq.{}", actor_id))],
        // NULL IS A REAL ANSWER and means "has not rolled", which 011
        // keeps distinct from rolling zero. Clearing is how a DM undoes
        // a mistake without inventing a value.
        &json!({ "initiative": initiative }),
    )
    .map_err(|e| denied(e, "set an initiative"))
}

/// Hand the turn to whoever is next.
///
/// BEGINNING AND ADVANCING ARE THE SAME ACT, which is why there is one
/// command. Starting from nobody takes the top of the order and opens
/// round one; running off the end does the same and opens the next. The
/// walk is `initiative::next_turn` and the round arithmetic is the one
/// line it cannot do, because it does not know what round it is.
#[tauri::command]
pub fn advance_turn(state: State<AppState>, encounter_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let state_now = turn_order(state, encounter_id.clone())?;

    let round = state_now
        .get("encounter")
        .and_then(|e| e.get("round"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let Some(next) = state_now.get("up_next").filter(|v| !v.is_null()) else {
        return Err(
            "nobody can take a turn — roll for initiative first, or everyone is out".to_string(),
        );
    };
    let actor_id = next.get("actor_id").and_then(|v| v.as_str()).unwrap_or("");
    let new_round = next.get("new_round").and_then(|v| v.as_bool()).unwrap_or(false);

    supabase::rest_update(
        &token,
        "encounters",
        &[("id", &format!("eq.{}", encounter_id))],
        &json!({
            "turn_actor_id": actor_id,
            "round": if new_round { round + 1 } else { round },
        }),
    )
    .map_err(|e| denied(e, "run the turn order"))
}

/// Put the fight back before the first turn.
///
/// The round goes to zero, which 051 keeps distinct from round one, and
/// the turn pointer clears. INITIATIVES ARE LEFT ALONE - re-rolling is
/// a separate decision and losing everyone's number because the DM
/// wanted to restart the order would be the expensive kind of helpful.
#[tauri::command]
pub fn reset_order(state: State<AppState>, encounter_id: String) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_update(
        &token,
        "encounters",
        &[("id", &format!("eq.{}", encounter_id))],
        &json!({ "turn_actor_id": Value::Null, "round": 0 }),
    )
    .map_err(|e| denied(e, "run the turn order"))
}

/* ============================ READING ============================ */

fn as_text(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

/// DEX modifier per character, for every character in the roster, in
/// one read.
fn dex_by_character(
    token: &str,
    actors: &[Value],
) -> Result<std::collections::HashMap<String, i64>, String> {
    let ids: Vec<String> = actors
        .iter()
        .filter_map(|a| a.get("character_id").and_then(|v| v.as_str()))
        .map(str::to_string)
        .collect();
    let mut out = std::collections::HashMap::new();
    if ids.is_empty() {
        return Ok(out);
    }

    let rows = supabase::rest_get(
        token,
        "character_abilities",
        &[
            ("select", "character_id,score"),
            ("ability", "eq.dex"),
            ("character_id", &format!("in.({})", ids.join(","))),
        ],
    )?;
    for r in rows.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
        let Some(id) = r.get("character_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let score = r.get("score").and_then(|v| v.as_i64()).unwrap_or(10);
        // The same floor-toward-negative the sheet uses.
        out.insert(id.to_string(), (score - 10).div_euclid(2));
    }
    Ok(out)
}

fn dex_of_actor(token: &str, actor_id: &str) -> Result<i64, String> {
    let rows = supabase::rest_get(
        token,
        "encounter_actors",
        &[
            ("select", "id,character_id"),
            ("id", &format!("eq.{}", actor_id)),
        ],
    )?;
    let actor = rows
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| "no such creature in an encounter you can see".to_string())?;
    Ok(*dex_by_character(token, &[actor])?.values().next().unwrap_or(&0))
}
