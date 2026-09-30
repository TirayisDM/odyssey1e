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
            ("select", "id,name,status,round,turn_actor_id,game_id"),
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
                "id,label,character_id,npc_key,initiative,active,dead,enrolled_at,\n                 held_mode,held_after_id",
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

    // The game, because a people can be overridden per game and a
    // DEX modifier now resolves through one.
    let dex = dex_by_character(&token, &as_text(&enc, "game_id"), &actors)?;

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
            // 063. A mode the column does not recognise reads as no
            // hold at all rather than refusing the whole order - the
            // check constraint is what keeps a bad one out, and a
            // screen that empties because of one row is worse than a
            // creature standing in their rolled place.
            hold: a
                .get("held_mode")
                .and_then(|v| v.as_str())
                .and_then(initiative::HoldMode::parse)
                .map(|mode| initiative::Hold {
                    mode,
                    after: a
                        .get("held_after_id")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                }),
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
            // 063. The declaration itself, and - because the screen
            // should not have to work it out - whether it could be
            // honoured. A hold naming somebody who has gone is placed
            // at the end, and saying so is the difference between a
            // sort that looks broken and one that explains itself.
            row["held"] = json!(c.hold);
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

    // A HOLD LASTS ONE ROUND - 063. "End of round" only means anything
    // inside a round, so that is the life of the whole declaration.
    //
    // BEFORE the pointer moves, not after: if the wipe fails the turn
    // has not advanced, and a DM pressing the button again is harmless.
    // The other way round leaves the fight on round 2 with round 1's
    // declarations still standing, which is the state nobody could
    // explain.
    if new_round {
        release_all_holds(&token, &encounter_id)?;
    }

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

/// Clear every hold in this fight.
///
/// Used when the round turns over, and by `reset_order` - putting a
/// fight back before its first turn has to put the order back too, or
/// a reset would keep declarations made about a round that no longer
/// exists.
fn release_all_holds(token: &str, encounter_id: &str) -> Result<(), String> {
    supabase::rest_update(
        token,
        "encounter_actors",
        &[
            ("encounter_id", &format!("eq.{}", encounter_id)),
            // Only the rows that have one, so a fight where nobody
            // held is not rewritten every round.
            ("held_mode", "not.is.null"),
        ],
        &json!({ "held_mode": Value::Null, "held_after_id": Value::Null }),
    )
    .map(|_| ())
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
    // 063. A reset puts the fight before its first turn, so the
    // declarations made about a round that no longer exists go with it.
    release_all_holds(&token, &encounter_id)?;
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

/// DEX modifier per character, for every character in the roster.
///
/// THE EFFECTIVE SCORE, NOT THE STORED ONE. This read `eq.dex` off the
/// row and subtracted ten, which was the whole answer until 056: a
/// people's bonus is deliberately never written into
/// `character_abilities`, so the row holds what somebody rolled and
/// the character has something else. Initiative was therefore rolled
/// on the wrong modifier for anybody whose people raises DEX. Neither
/// seeded people does, which is why nobody saw it - and why it is
/// worth fixing before the third species arrives rather than after.
fn dex_by_character(
    token: &str,
    game_id: &str,
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

    let eff = crate::character::load_effective(token, game_id, &ids)?;
    for id in ids {
        let m = eff.modifier(&id, "dex");
        out.insert(id, m);
    }
    Ok(out)
}

/// One creature's DEX, for its own roll.
///
/// THE GAME TRAVELS WITH THE ACTOR, embedded rather than fetched
/// separately: a people can be overridden per game, so resolving a
/// score needs to know whose game this is, and `roll_for` is handed an
/// actor id and nothing else.
fn dex_of_actor(token: &str, actor_id: &str) -> Result<i64, String> {
    let rows = supabase::rest_get(
        token,
        "encounter_actors",
        &[
            // THE CHARACTER'S GAME, NOT THE ENCOUNTER'S - and they are
            // the same game, so this is about which embed PostgREST
            // will accept. `encounters(game_id)` is refused: 051 added
            // `encounters.turn_actor_id` pointing back at this table,
            // so there are now TWO relationships between the pair and
            // PostgREST will not guess which one is meant. It says so
            // plainly, which is the good version of this failure.
            //
            // A character belongs to exactly one game and an actor to
            // exactly one character, so this answers the same question
            // through the one unambiguous path.
            ("select", "id,character_id,characters(game_id)"),
            ("id", &format!("eq.{}", actor_id)),
        ],
    )?;
    let actor = rows
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| "no such creature in an encounter you can see".to_string())?;
    let game_id = actor
        .get("characters")
        .and_then(|c| c.get("game_id"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    Ok(*dex_by_character(token, &game_id, &[actor])?
        .values()
        .next()
        .unwrap_or(&0))
}

/* ======================== THE OTHER THREE SLOTS ======================== */

/// The slots a tick can spend. Not `attack` and not `action`: those are
/// spent by DOING something - swinging, rolling a check - and a tick
/// that could mark them would be a second way to say what the log
/// already says.
const TICKABLE: [&str; 3] = ["bonus", "reaction", "free"];

fn tickable(cost: &str) -> Result<&str, String> {
    TICKABLE
        .iter()
        .find(|c| **c == cost)
        .copied()
        .ok_or_else(|| format!("{} is not a slot you can tick", cost))
}

/// What a spent slot is called in the log.
fn slot_label(cost: &str) -> &'static str {
    match cost {
        "bonus" => "Bonus action",
        "reaction" => "Reaction",
        _ => "Free interaction",
    }
}

/// The encounter, the round, and this actor's character - everything
/// both halves of the toggle need.
fn slot_context(
    token: &str,
    encounter_id: &str,
    actor_id: &str,
) -> Result<(String, i64, Option<String>), String> {
    let enc = supabase::rest_get(
        token,
        "encounters",
        &[
            ("select", "id,game_id,round"),
            ("id", &format!("eq.{}", encounter_id)),
        ],
    )?;
    let enc = enc
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| "that encounter is not visible to you".to_string())?;

    let rows = supabase::rest_get(
        token,
        "encounter_actors",
        &[
            ("select", "id,character_id"),
            ("id", &format!("eq.{}", actor_id)),
            ("encounter_id", &format!("eq.{}", encounter_id)),
        ],
    )?;
    let row = rows
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| "that creature is not in this fight".to_string())?;

    Ok((
        as_text(&enc, "game_id"),
        enc.get("round").and_then(|v| v.as_i64()).unwrap_or(0),
        row.get("character_id")
            .and_then(|v| v.as_str())
            .map(String::from),
    ))
}

/// Mark a bonus action, reaction or free interaction as spent. 062.
///
/// IT WRITES AN ACTION WITH NO DICE. That was the decision worth
/// making: the alternative was a per-turn state table, which would
/// have been a second account of the same round. An action already has
/// its round stamped, already records whose turn it was taken in,
/// already shows in the log and already deletes cleanly - which is
/// exactly what un-ticking a box has to do.
///
/// NOTHING IS REFUSED, including ticking twice. 051 decided the order
/// informs and never refuses and a slot is the same: a DM granting a
/// second bonus action is an ordinary Tuesday, and the count simply
/// reads as over budget. What the app owes them is to SAY so.
#[tauri::command]
pub fn spend_slot(
    state: State<AppState>,
    encounter_id: String,
    actor_id: String,
    cost: String,
) -> Result<Value, String> {
    let session = state
        .current()?
        .ok_or_else(|| "not signed in".to_string())?;
    let token = &session.access_token;
    let cost = tickable(cost.trim())?;
    let (game_id, _round, character_id) = slot_context(token, &encounter_id, &actor_id)?;

    // THE KEY IS THE COST WORD and the cost is left for the trigger,
    // which 062 taught to read one from the other. Passing both would
    // be two things to keep in step.
    supabase::rest_insert(
        token,
        "actions",
        &json!({
            "game_id": game_id,
            "character_id": character_id,
            "encounter_id": encounter_id,
            "owner_uid": session.user_id,
            "actor_id": actor_id,
            "request": cost,
            "key": cost,
            "label": slot_label(cost),
            "status": "resolved",
        }),
    )
}

/// Un-tick it: delete this round's markers of that kind for that
/// creature.
///
/// DELETES EVERY ONE OF THEM, not the newest. A box that is off should
/// be off - if two got written because somebody clicked twice, leaving
/// one behind would make the tick lie about the round.
///
/// SCOPED TO THE ROUND, so un-ticking in round 3 cannot reach into
/// round 2 and rewrite what happened there. A marker with no round -
/// one written before the order started - is left alone for the same
/// reason 054 refuses to count it.
#[tauri::command]
pub fn clear_slot(
    state: State<AppState>,
    encounter_id: String,
    actor_id: String,
    cost: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let cost = tickable(cost.trim())?;
    let (_game_id, round, _character_id) = slot_context(&token, &encounter_id, &actor_id)?;

    supabase::rest_delete(
        &token,
        "actions",
        &[
            ("encounter_id", &format!("eq.{}", encounter_id)),
            ("actor_id", &format!("eq.{}", actor_id)),
            ("cost", &format!("eq.{}", cost)),
            ("round", &format!("eq.{}", round)),
        ],
    )?;
    Ok(json!({ "cleared": cost, "round": round }))
}

/* ========================== HOLDING A PLACE ========================== */

/// Declare where this creature will act instead of on their roll. 063.
///
/// THREE DECLARATIONS AND NO TRIGGER. "After the next one", "after that
/// character", "at the end of the round". A condition in prose - when
/// the goblin steps into the doorway - is a sentence for a DM, and a
/// column nothing reads would be decoration.
///
/// THE ROLL IS UNTOUCHED. 011 made `initiative` the record of what
/// somebody rolled and this lays a declaration over it; releasing puts
/// them back with nothing to restore, because nothing was overwritten.
///
/// NOT REFUSED WHEN IT MAKES NO SENSE. Naming a creature who has
/// already acted is legal and means acting sooner than the dice said,
/// which a DM may well want. Naming one who has left the fight
/// resolves to the end of the round. 051 decided the order informs and
/// never refuses; the only thing 063 does refuse is holding for
/// yourself, which is a check constraint because it is not a
/// declaration anybody could mean.
#[tauri::command]
pub fn hold_turn(
    state: State<AppState>,
    actor_id: String,
    mode: String,
    after_id: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    let mode = initiative::HoldMode::parse(mode.trim())
        .ok_or_else(|| format!("{} is not a way to hold", mode))?;

    let after = after_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    // The pair has to agree, and saying so here gives a sentence where
    // the check constraint would give "violates constraint".
    let after = match mode {
        initiative::HoldMode::AfterActor => Some(
            after
                .ok_or_else(|| "holding after somebody needs somebody to name".to_string())?,
        ),
        _ => None,
    };
    if after == Some(actor_id.as_str()) {
        return Err("a creature cannot wait for themselves".to_string());
    }

    supabase::rest_update(
        &token,
        "encounter_actors",
        &[("id", &format!("eq.{}", actor_id))],
        &json!({
            "held_mode": match mode {
                initiative::HoldMode::AfterNext => "after_next",
                initiative::HoldMode::AfterActor => "after_actor",
                initiative::HoldMode::EndOfRound => "end_of_round",
            },
            "held_after_id": after.map(Value::from).unwrap_or(Value::Null),
        }),
    )
    .map_err(|e| denied(e, "change the turn order"))
}

/// Put one creature back on their rolled initiative.
#[tauri::command]
pub fn release_hold(state: State<AppState>, actor_id: String) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_update(
        &token,
        "encounter_actors",
        &[("id", &format!("eq.{}", actor_id))],
        &json!({ "held_mode": Value::Null, "held_after_id": Value::Null }),
    )
    .map_err(|e| denied(e, "change the turn order"))
}
