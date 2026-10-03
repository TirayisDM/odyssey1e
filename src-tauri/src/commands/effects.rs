//! Applying something, ending it, and listing what is running.
//!
//! 094. Plumbing. Every rule - whether a second one is admitted, what
//! is still true, when something ends - is effects.rs.

use serde_json::{json, Value};
use tauri::State;

use crate::effects::{self, Effect, Outcome, Stacking};
use crate::supabase::{self, AppState};

/// What is running, on everybody in the game.
///
/// THE WHOLE GAME IN ONE READ, because the screens that want this want
/// all of it: the initiative strip wants a mark beside each combatant
/// and the DM wants the party's state at a glance. One request beats
/// one per person.
#[tauri::command]
pub fn list_effects(state: State<AppState>, game_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let now = read_tick(&token, &game_id)?;
    let all = load_for_game(&token, &game_id)?;

    let live: Vec<Value> = effects::active(&all, now)
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "key": e.key,
                "name": e.name,
                "character_id": e.character_id,
                "magnitude": e.magnitude,
                "started_at": e.started_at,
                "expires_at": e.expires_at,
                "stacks": e.stacks.as_str(),
                "left": e.left(now),
                "said": e.said(now),
            })
        })
        .collect();

    Ok(json!({ "tick": now, "effects": live }))
}

/// Put something on somebody.
///
/// THE STACKING RULE DECIDES, and effects::admit is where it is
/// written. Three of the four outcomes are not an error - superseding,
/// keeping what is better, and adding are all "it worked" - so this
/// reports WHAT HAPPENED rather than just succeeding silently. A bard
/// whose weaker song was declined should be told that, not left
/// wondering.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn apply_effect(
    state: State<AppState>,
    game_id: String,
    character_id: String,
    key: String,
    name: String,
    ticks: Option<i64>,
    magnitude: Option<i64>,
    stacks: Option<String>,
    source_character_id: Option<String>,
    source_feature: Option<String>,
    note: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    let now = read_tick(&token, &game_id)?;
    let (outcome, said) = apply_inner(
        &token,
        &game_id,
        &character_id,
        &key,
        &name,
        ticks,
        magnitude,
        stacks.as_deref().unwrap_or("replace"),
        source_character_id.as_deref(),
        source_feature.as_deref(),
        note.as_deref(),
        &json!([]),
        now,
    )?;
    Ok(match outcome {
        Outcome::Keep(id) => json!({
            "outcome": "kept",
            "kept": id,
            "said": "what they already have is as good or better",
        }),
        Outcome::Supersede(ids) => json!({
            "outcome": "replaced", "superseded": ids, "said": said,
        }),
        _ => json!({ "outcome": "added", "superseded": [], "said": said }),
    })
}

/// The part the engine calls too - a bard's performance applies an
/// effect without going back out through a Tauri command.
///
/// RETURNS WHAT HAPPENED rather than just succeeding. Three of the four
/// outcomes are "it worked" and the fourth is a refusal, and a caller
/// applying one to a whole party needs to know which of them it
/// actually reached.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_inner(
    token: &str,
    game_id: &str,
    character_id: &str,
    key: &str,
    name: &str,
    ticks: Option<i64>,
    magnitude: Option<i64>,
    stacks: &str,
    source_character_id: Option<&str>,
    source_feature: Option<&str>,
    note: Option<&str>,
    // 114. WHAT IT DOES TO WHOEVER CARRIES IT, in grants.rs's
    // vocabulary. COPIED ONTO THE ROW rather than looked up later: a
    // DM retuning Bless next month must not silently change what is
    // already running on somebody - 001's snapshot rule.
    grants: &Value,
    now: i64,
) -> Result<(Outcome, String), String> {
    let rule = Stacking::parse(stacks);
    let character_id = character_id.to_string();
    let key = key.to_string();
    let name = name.to_string();

    let incoming = Effect {
        id: String::new(),
        key: key.trim().to_string(),
        name: name.trim().to_string(),
        character_id: character_id.clone(),
        magnitude,
        started_at: now,
        expires_at: effects::ends_at(now, ticks),
        stacks: rule,
    };
    if incoming.key.is_empty() || incoming.name.is_empty() {
        return Err("an effect needs a key and a name".to_string());
    }

    let existing = load_for_game(token, game_id)?;
    let outcome = effects::admit(&existing, &incoming, now);

    let superseded = match &outcome {
        Outcome::Refuse(why) => return Err(why.clone()),
        Outcome::Keep(_) => return Ok((outcome, String::new())),
        Outcome::Supersede(ids) => ids.clone(),
        Outcome::Add => Vec::new(),
    };

    // ENDED, NOT DELETED. A superseded effect was true until this
    // moment and the row says so - see the table comment.
    for id in &superseded {
        end_one(token, id, now)?;
    }

    supabase::rest_insert(
        token,
        "effects",
        &json!({
            "game_id": game_id,
            "character_id": character_id,
            "key": incoming.key,
            "name": incoming.name,
            "magnitude": magnitude,
            "source_character_id": source_character_id,
            "source_feature": source_feature,
            "started_at": now,
            "expires_at": incoming.expires_at,
            "stacks": rule.as_str(),
            "note": note,
            "grants": grants,
        }),
    )?;

    let said = match incoming.expires_at {
        Some(at) => format!("{} for {}", incoming.name, crate::clock::said(at - now)),
        None => format!("{} until it ends", incoming.name),
    };
    Ok((outcome, said))
}

/// End one early.
#[tauri::command]
pub fn end_effect(
    state: State<AppState>,
    game_id: String,
    effect_id: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let now = read_tick(&token, &game_id)?;
    end_one(&token, &effect_id, now)?;
    Ok(json!({ "ended_at": now }))
}

/* ======================== THE WORKING PARTS ======================== */

/// Cut one short.
///
/// `ended_at` AND `expires_at` BOTH, because the two say different
/// things: when it was going to end, and when it actually did. Keeping
/// only the second would lose the fact that it was cut short at all.
pub(crate) fn end_one(token: &str, effect_id: &str, now: i64) -> Result<(), String> {
    supabase::rest_update(
        token,
        "effects",
        &[("id", &format!("eq.{}", effect_id))],
        &json!({ "ended_at": now, "expires_at": now }),
    )?;
    Ok(())
}

/// Every grant currently running on one character.
///
/// 114. WHAT THE ROLL PATH ASKS. Live effects only - an expired Bless
/// is a record and not a bonus - and the grants come off the row
/// rather than the catalogue, so what is running is what was true when
/// it was cast.
pub(crate) fn grants_on(
    token: &str,
    game_id: &str,
    character_id: &str,
) -> Result<Vec<crate::grants::Grant>, String> {
    let now = read_tick(token, game_id)?;
    let rows = supabase::rest_get(
        token,
        "effects",
        &[
            ("select", "name,grants,started_at,expires_at"),
            ("character_id", &format!("eq.{}", character_id)),
            ("ended_at", "is.null"),
        ],
    )?;
    let mut out = Vec::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let expires = r.get("expires_at").and_then(|v| v.as_i64());
        if let Some(at) = expires {
            if crate::clock::expired(now, at) {
                continue;
            }
        }
        let name = r.get("name").and_then(|v| v.as_str()).unwrap_or("an effect");
        if let Some(g) = r.get("grants") {
            out.extend(crate::grants::parse(g, name));
        }
    }
    Ok(out)
}

/// End whatever this caster is concentrating on.
///
/// 112. ONE AT A TIME, which is 5e and the rule a table forgets most
/// often. It is the CASTER's concentration, so this looks for what
/// THEY are holding rather than what is on any one target - Bless on
/// three allies is one concentration and ending it ends all three.
pub(crate) fn drop_concentration(
    token: &str,
    game_id: &str,
    caster_id: &str,
    now: i64,
) -> Result<Vec<String>, String> {
    let rows = supabase::rest_get(
        token,
        "effects",
        &[
            ("select", "id,name"),
            ("game_id", &format!("eq.{}", game_id)),
            ("source_character_id", &format!("eq.{}", caster_id)),
            ("source_feature", "eq.concentration"),
            ("ended_at", "is.null"),
        ],
    )?;
    let mut ended = Vec::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let Some(id) = r.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        end_one(token, id, now)?;
        if let Some(n) = r.get("name").and_then(|v| v.as_str()) {
            ended.push(n.to_string());
        }
    }
    Ok(ended)
}

/// Every effect in the game, expired ones included.
///
/// THE EXPIRED ONES ARE READ ON PURPOSE. `effects::admit` decides what
/// is in the way and needs to see everything to be sure nothing is;
/// filtering here would mean the rule could not tell "nothing there"
/// from "nothing shown".
fn load_for_game(token: &str, game_id: &str) -> Result<Vec<Effect>, String> {
    let rows = supabase::rest_get(
        token,
        "effects",
        &[
            (
                "select",
                "id,key,name,character_id,magnitude,started_at,expires_at,stacks",
            ),
            ("game_id", &format!("eq.{}", game_id)),
            ("ended_at", "is.null"),
        ],
    )?;
    Ok(rows
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|r| Effect {
            id: as_str(r, "id"),
            key: as_str(r, "key"),
            name: as_str(r, "name"),
            character_id: as_str(r, "character_id"),
            magnitude: r.get("magnitude").and_then(|v| v.as_i64()),
            started_at: r.get("started_at").and_then(|v| v.as_i64()).unwrap_or(0),
            expires_at: r.get("expires_at").and_then(|v| v.as_i64()),
            stacks: Stacking::parse(r.get("stacks").and_then(|v| v.as_str()).unwrap_or("")),
        })
        .collect())
}

pub(crate) fn read_tick(token: &str, game_id: &str) -> Result<i64, String> {
    let rows = supabase::rest_get(
        token,
        "games",
        &[("select", "tick"), ("id", &format!("eq.{}", game_id))],
    )?;
    rows.as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("tick"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "no such game".to_string())
}

fn as_str(r: &Value, key: &str) -> String {
    r.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}
