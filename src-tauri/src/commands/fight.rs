//! What a player sees of the fight they are in.
//!
//! 203. Plumbing. Every rule - the band, who gets numbers, which
//! condition stops a turn - is `reveal.rs` and `conditions.rs`.
//!
//! THREE THINGS THIS DELIBERATELY DOES NOT DO ITSELF:
//!
//! - **It does not sort the fight.** `initiative::ordered_actors` does,
//!   which is the same call the DM's card makes. Two sorts that agree
//!   today are two sorts to keep in agreement, and the player's strip
//!   showing a different order from the DM's screen is the worst
//!   version of this feature failing.
//! - **It does not read hit points its own way.** `load_targets` is
//!   already the batched reader for an encounter's vitals - three
//!   requests rather than three per creature - and it is what the
//!   attack path aims with, so the numbers behind a band are the same
//!   numbers a swing resolves against.
//! - **It does not decide which effects are still running.**
//!   `effects::active` does, against the game's own tick.

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use tauri::State;

use crate::effects;
use crate::encounter;
use crate::reveal::{self, Facts};
use crate::supabase::{self, AppState};

use super::effects as effects_cmd;
use super::initiative;
use super::Ctx;

/// The fight strip for the signed-in player, or nothing when no fight
/// is running.
///
/// RETURNS `null` FOR "NO FIGHT", not an empty strip. Dave's call on
/// placement: the panel is absent while nothing is happening rather
/// than sitting there saying so, which on a phone is the difference
/// between a sheet you can read and a sheet you scroll to.
///
/// ONE ACTIVE ENCOUNTER PER GAME is what this assumes, and 011 does not
/// constrain it - `status` is a plain column and two rows could be
/// 'active' at once. The newest wins here rather than refusing, because
/// a player whose screen goes blank because the DM left an old fight
/// open has no way to understand that and no way to fix it.
#[tauri::command]
pub fn fight_strip(state: State<AppState>, game_id: String) -> Result<Value, String> {
    let me = Ctx::of(&state)?;
    let token = &me.token;

    let encs = supabase::rest_get(
        token,
        "encounters",
        &[
            ("select", "id,name,status,round,turn_actor_id"),
            ("game_id", &format!("eq.{}", game_id)),
            ("status", "eq.active"),
            ("order", "created_at.desc"),
            ("limit", "1"),
        ],
    )?;
    let enc = match encs.as_array().and_then(|a| a.first()) {
        Some(e) => e.clone(),
        None => return Ok(Value::Null),
    };
    let enc_id = enc
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    // The order, from the one place that decides it.
    let ordered = initiative::ordered_actors(token, &enc_id)?;
    let rows = ordered
        .get("order")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if rows.is_empty() {
        return Ok(Value::Null);
    }

    // Vitals, batched, from the reader the attack path already uses.
    let vitals: HashMap<String, encounter::Target> = encounter::load_targets(token, &enc_id)?
        .into_iter()
        .filter(|t| t.row == "actor")
        .map(|t| (t.id.clone(), t))
        .collect();

    // WHICH CHARACTERS ARE MINE. `owner_uid` holds a PROFILE since 197,
    // and `Ctx` carries the profile rather than the sign-in for exactly
    // this comparison - handing it `user_id` would work today and stop
    // working the day somebody links a second sign-in.
    let mine = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "id"),
            ("game_id", &format!("eq.{}", game_id)),
            ("owner_uid", &format!("eq.{}", me.profile_id)),
        ],
    )?;
    let owned: Vec<String> = mine
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|r| r.get("id").and_then(|v| v.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // Live conditions, per character, through the expiry rule.
    let now = effects_cmd::read_tick(token, &game_id)?;
    let all = effects_cmd::load_for_game(token, &game_id)?;
    let mut running: HashMap<String, Vec<String>> = HashMap::new();
    for e in effects::active(&all, now) {
        // The key goes in AS STORED. `reveal` and `conditions` both want
        // the `cond.` prefix - see the note on `reveal::Facts`.
        running
            .entry(e.character_id.clone())
            .or_default()
            .push(e.key.clone());
    }

    let current = enc.get("turn_actor_id").and_then(|v| v.as_str());
    let mut lines: Vec<Value> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for r in &rows {
        let actor_id = r
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if !seen.insert(actor_id.clone()) {
            continue;
        }
        let t = vitals.get(&actor_id);
        let character_id = t
            .and_then(|t| t.character_id.clone())
            .or_else(|| r.get("character_id").and_then(|v| v.as_str()).map(String::from));

        let facts = Facts {
            actor_id: actor_id.clone(),
            // The actor's label, which 018 guarantees is a name and not
            // 'Someone'. Falling back to the target's label keeps a
            // creature named if one read has it and the other does not.
            name: r
                .get("label")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(String::from)
                .or_else(|| t.map(|t| t.label.clone()))
                .unwrap_or_else(|| "Someone".to_string()),
            character_id: character_id.clone(),
            hp_current: t.and_then(|t| t.hp_current).unwrap_or(0),
            hp_max: t.and_then(|t| t.hp_max),
            conditions: character_id
                .as_deref()
                .and_then(|c| running.get(c).cloned())
                .unwrap_or_default(),
            is_current: Some(actor_id.as_str()) == current,
            takes_turns: r
                .get("takes_turns")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            // 013's floor means hp_current never reads negative, so a
            // creature at zero is the only derived 'down' - and the
            // death condition is the explicit answer beside it.
            dead: t
                .and_then(|t| t.condition)
                .is_some_and(|c| c == crate::death::Condition::Dead),
        };

        let line = reveal::reveal(&owned, &facts);
        lines.push(json!({
            "actor_id": line.actor_id,
            "name": line.name,
            "yours": line.yours,
            "hp_current": line.hp.map(|(now, _)| now),
            "hp_max": line.hp.map(|(_, max)| max),
            "band": line.band,
            "conditions": line.conditions,
            "cannot_act": line.cannot_act,
            "is_current": line.is_current,
            "takes_turns": line.takes_turns,
        }));
    }

    Ok(json!({
        "encounter": {
            "id": enc_id,
            "name": enc.get("name").cloned().unwrap_or(Value::Null),
            "round": enc.get("round").cloned().unwrap_or(json!(0)),
        },
        "order": lines,
    }))
}
