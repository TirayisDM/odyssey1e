//! Encounters: the things a roll can be aimed at.
//!
//! Split the way the other modules are - `load_targets` talks to the
//! network, everything else is arithmetic with tests.
//!
//! WHAT A TARGET IS
//!
//! 009 settled the interface before this module existed. A roll takes
//! three things: a value, a kind (`ac` or `dc`), and a label. An actor
//! supplies an AC, a challenge supplies a DC, and the roll path cannot
//! tell them apart. "Target or attempt" is a distinction the UI draws
//! for the player, not one the engine makes.
//!
//! WHY AC IS RESOLVED HERE AND NOT STORED
//!
//! An actor row holds identity and overrides, never a copy of AC - see
//! 011. So the AC has to be worked out per actor at read time:
//!
//!   an override on the actor      wins outright
//!   an NPC                        takes its statblock's ac
//!   a player character            is COMPUTED from what they are
//!                                 wearing, by equipment::armor_class
//!
//! That last one is why this module batches. Computing one character's
//! AC needs their abilities and their equipped items; doing it per
//! actor would be three requests each and a party of five would cost
//! fifteen. Instead every character in the encounter is gathered once -
//! abilities in one request, owned rows in one, the item catalogue in
//! one - and the AC is computed in memory. Six requests for a target
//! list, whatever the party size.

use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::death::{self, Condition};
use crate::equipment::{self, AcMode, Item};
use crate::narrative::quoted;
use crate::supabase;

/// One thing a player can aim at.
#[derive(Debug, Clone, Serialize)]
pub struct Target {
    /// The actor or challenge row this came from. A roll stores the
    /// label and value as its own record, but keeps this so "has the
    /// lock been picked?" is answerable without trusting a snapshot.
    pub id: String,
    /// `actor` or `challenge`. What the UI groups by.
    pub row: &'static str,
    /// `ac` or `dc`. What the roll needs, and the only distinction the
    /// resolver makes.
    pub target_kind: &'static str,
    pub label: String,
    pub value: i64,
    /// Where the number came from, in words, so a DM can see why a
    /// goblin is 15 and Rodnar is 16 without opening the database.
    pub source: String,
    /// The character behind this actor, when there is one.
    ///
    /// A CHARACTER'S HIT POINTS BELONG TO THE CHARACTER, not to one
    /// appearance in one encounter, so damage to a player is recorded
    /// against this and follows them between fights. An NPC instance
    /// has no character and its damage is recorded against the actor
    /// row, which is what lets two goblins off one statblock bleed
    /// separately.
    pub character_id: Option<String>,
    /// Hit points now, and at full. None for a challenge, and for an
    /// actor whose source never had a maximum set.
    pub hp_current: Option<i64>,
    pub hp_max: Option<i64>,
    /// conscious, down, stable or dead. Derived from hit points and the
    /// two death save counters rather than stored - see death.rs. None
    /// for a challenge, which cannot be knocked out.
    pub condition: Option<Condition>,
    /// Death saves so far, for a card that wants to show the tally.
    pub death_successes: i64,
    pub death_failures: i64,
}

/* ============================ RULES ============================ */

/// Which AC applies to an actor, and why.
///
/// Returns the number and the account of it. Kept together because a
/// number with no provenance is the thing this codebase keeps refusing
/// to ship - see the equipment panel, and `rolls.reason`.
pub fn resolve_ac(
    ac_override: Option<i64>,
    npc_ac: Option<i64>,
    character_ac: Option<i64>,
) -> Option<(i64, String)> {
    if let Some(v) = ac_override {
        return Some((v, "override on this actor".to_string()));
    }
    if let Some(v) = npc_ac {
        return Some((v, "statblock".to_string()));
    }
    if let Some(v) = character_ac {
        return Some((v, "computed from equipment".to_string()));
    }
    // An actor whose source has vanished - a statblock deleted out from
    // under it. Offering no target is honest; inventing 10 would be a
    // number someone rolls against.
    None
}

/* ============================ LOADING ============================ */

fn as_str(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn as_opt_str(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(str::to_string)
        .filter(|s| !s.trim().is_empty())
}

/// Every active target in an encounter, actors and challenges together.
pub fn load_targets(token: &str, encounter_id: &str) -> Result<Vec<Target>, String> {
    let encs = supabase::rest_get(
        token,
        "encounters",
        &[
            ("select", "id,game_id,name,status"),
            ("id", &format!("eq.{}", encounter_id)),
        ],
    )?;
    let enc = encs
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| "encounter not found, or not visible to you".to_string())?;
    let game_id = as_str(enc, "game_id");

    let actor_rows = supabase::rest_get(
        token,
        "encounter_actors",
        &[
            (
                "select",
                "id,character_id,npc_key,label,ac_override,hp_override,initiative,\
                 death_successes,death_failures,dead",
            ),
            ("encounter_id", &format!("eq.{}", encounter_id)),
            ("active", "is.true"),
            ("order", "enrolled_at.asc"),
        ],
    )?;
    let actor_rows = actor_rows.as_array().cloned().unwrap_or_default();

    let challenge_rows = supabase::rest_get(
        token,
        "encounter_challenges",
        &[
            ("select", "id,label,dc,skill_key"),
            ("encounter_id", &format!("eq.{}", encounter_id)),
            ("active", "is.true"),
            ("order", "created_at.asc"),
        ],
    )?;

    // --- statblocks, one request for every distinct key ---
    let npc_keys: Vec<String> = actor_rows
        .iter()
        .filter_map(|r| as_opt_str(r, "npc_key"))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let mut npc_ac: HashMap<String, i64> = HashMap::new();
    let mut npc_hp: HashMap<String, i64> = HashMap::new();
    if !npc_keys.is_empty() {
        let quoted_keys: Vec<String> = npc_keys.iter().map(|k| quoted(k)).collect();
        let rows = supabase::rest_get(
            token,
            "npcs",
            &[
                ("select", "key,game_id,name,ac,hp_max"),
                ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
                ("key", &format!("in.({})", quoted_keys.join(","))),
            ],
        )?;
        // A game-scoped statblock shadows the global one sharing its key,
        // same precedence as items and narrative lines.
        for r in rows.as_array().unwrap_or(&Vec::new()) {
            let key = as_str(r, "key");
            let scoped = as_opt_str(r, "game_id").is_some();
            let ac = r.get("ac").and_then(|x| x.as_i64()).unwrap_or(10);
            let hp = r.get("hp_max").and_then(|x| x.as_i64()).unwrap_or(1);
            if scoped || !npc_ac.contains_key(&key) {
                npc_ac.insert(key.clone(), ac);
                npc_hp.insert(key, hp);
            }
        }
    }

    // --- player characters, computed in one batch ---
    let char_ids: Vec<String> = actor_rows
        .iter()
        .filter_map(|r| as_opt_str(r, "character_id"))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let char_stats = if char_ids.is_empty() {
        HashMap::new()
    } else {
        batch_character_stats(token, &game_id, &char_ids)?
    };

    // Every hit point ever lost or regained by anyone in this encounter,
    // in one request. Current HP is the maximum plus the sum of the
    // deltas - see 013. A subject with no events is at full health,
    // which is the right answer and needs no row to say so.
    let actor_ids: Vec<String> = actor_rows.iter().map(|r| as_str(r, "id")).collect();
    let deltas = load_hp_deltas(token, &actor_ids, &char_ids)?;

    // --- assemble ---
    let mut out = Vec::new();

    for r in &actor_rows {
        let label = as_str(r, "label");
        let npc_key = as_opt_str(r, "npc_key");
        let character_id = as_opt_str(r, "character_id");
        let npc = npc_key.as_ref().and_then(|k| npc_ac.get(k).copied());
        let pc = character_id.as_ref().and_then(|c| char_stats.get(c).map(|s| s.ac));

        // An override on the actor wins, then the source's own maximum.
        let hp_max = r
            .get("hp_override")
            .and_then(|x| x.as_i64())
            .or_else(|| npc_key.as_ref().and_then(|k| npc_hp.get(k).copied()))
            .or_else(|| {
                character_id
                    .as_ref()
                    .and_then(|c| char_stats.get(c).and_then(|s| s.hp_max))
            });

        // Keyed the way the damage is recorded: a character by their
        // character_id wherever they appear, an NPC by this instance.
        let spent = match &character_id {
            Some(c) => deltas.get(c).copied().unwrap_or(0),
            None => deltas.get(&as_str(r, "id")).copied().unwrap_or(0),
        };
        let hp_current = hp_max.map(|m| m + spent);

        // The death saves live where the hit points do: a character
        // keeps theirs on their own row wherever they appear, an NPC
        // instance keeps its own so two goblins die separately.
        let (successes, failures, flagged) = match &character_id {
            Some(c) => match char_stats.get(c) {
                Some(st) => (st.successes, st.failures, st.dead),
                None => (0, 0, false),
            },
            None => (
                r.get("death_successes").and_then(|x| x.as_i64()).unwrap_or(0),
                r.get("death_failures").and_then(|x| x.as_i64()).unwrap_or(0),
                r.get("dead").and_then(|x| x.as_bool()).unwrap_or(false),
            ),
        };

        // Unconscious, and therefore worth nothing to hit. HOUSE RULE -
        // see death.rs; 5e keeps the armour class and grants advantage.
        let condition = death::condition(hp_current.unwrap_or(1), successes, failures, flagged);

        match resolve_ac(r.get("ac_override").and_then(|x| x.as_i64()), npc, pc) {
            Some((value, source)) => out.push(Target {
                id: as_str(r, "id"),
                row: "actor",
                target_kind: "ac",
                label,
                value: death::effective_ac(value, condition),
                source: match death::label_suffix(condition) {
                    Some(word) => format!("{} · {}, so AC 0", source, word),
                    None => source,
                },
                character_id,
                hp_current,
                hp_max,
                condition: Some(condition),
                death_successes: successes,
                death_failures: failures,
            }),
            // Skipped rather than shown at a made-up number. An actor
            // with no resolvable AC is a data fault, and a target nobody
            // can roll against is better than one everybody hits.
            None => continue,
        }
    }

    for r in challenge_rows.as_array().unwrap_or(&Vec::new()) {
        let skill = as_opt_str(r, "skill_key");
        out.push(Target {
            id: as_str(r, "id"),
            row: "challenge",
            target_kind: "dc",
            label: as_str(r, "label"),
            value: r.get("dc").and_then(|x| x.as_i64()).unwrap_or(10),
            source: match skill {
                Some(k) => format!("DM-set, suggests {}", k),
                None => "DM-set".to_string(),
            },
            // A lock has no hit points and no character behind it.
            character_id: None,
            hp_current: None,
            hp_max: None,
            condition: None,
            death_successes: 0,
            death_failures: 0,
        });
    }

    Ok(out)
}

/// Every enrolled character's AC, in three requests rather than three
/// per character.
///
/// The arithmetic is `equipment::armor_class`, the same function the
/// sheet uses, so a character's AC cannot read one number on their own
/// sheet and another in the DM's target list.
/// One actor's live state, read fresh at the moment it decides
/// something.
///
/// The frontend already has all of this from the target list, and it is
/// NOT trusted for it. Whether a hit kills outright, and whether it
/// costs a death save, are rules with a body on the end of them; they
/// get read from the database rather than taken from a dropdown that
/// may be seconds stale.
#[derive(Debug, Clone)]
pub struct ActorVitals {
    pub character_id: Option<String>,
    pub hp_max: Option<i64>,
    pub hp_current: i64,
    pub successes: i64,
    pub failures: i64,
    pub dead: bool,
}

/// Read one actor's hit points and dying state.
pub fn load_actor_vitals(token: &str, actor_id: &str) -> Result<Option<ActorVitals>, String> {
    let rows = supabase::rest_get(
        token,
        "encounter_actors",
        &[
            (
                "select",
                "id,character_id,npc_key,hp_override,death_successes,death_failures,dead",
            ),
            ("id", &format!("eq.{}", actor_id)),
        ],
    )?;
    let r = match rows.as_array().and_then(|a| a.first()) {
        Some(r) => r.clone(),
        None => return Ok(None),
    };

    let character_id = as_opt_str(&r, "character_id");
    let mut hp_max = r.get("hp_override").and_then(|x| x.as_i64());
    let mut successes = r.get("death_successes").and_then(|x| x.as_i64()).unwrap_or(0);
    let mut failures = r.get("death_failures").and_then(|x| x.as_i64()).unwrap_or(0);
    let mut dead = r.get("dead").and_then(|x| x.as_bool()).unwrap_or(false);

    // A character keeps their maximum and their death saves on their own
    // row, wherever they happen to be standing. An NPC instance keeps
    // both here, which is what lets two goblins die separately.
    if let Some(c) = &character_id {
        let chars = supabase::rest_get(
            token,
            "characters",
            &[
                ("select", "hp_max,death_successes,death_failures,dead"),
                ("id", &format!("eq.{}", c)),
            ],
        )?;
        if let Some(cr) = chars.as_array().and_then(|a| a.first()) {
            hp_max = hp_max.or_else(|| cr.get("hp_max").and_then(|x| x.as_i64()));
            successes = cr.get("death_successes").and_then(|x| x.as_i64()).unwrap_or(0);
            failures = cr.get("death_failures").and_then(|x| x.as_i64()).unwrap_or(0);
            dead = cr.get("dead").and_then(|x| x.as_bool()).unwrap_or(false);
        }
    } else if let Some(k) = as_opt_str(&r, "npc_key") {
        let npcs = supabase::rest_get(
            token,
            "npcs",
            &[
                ("select", "key,game_id,hp_max"),
                ("key", &format!("eq.{}", k)),
            ],
        )?;
        if hp_max.is_none() {
            // A game override shadows the global statblock; taking the
            // last row read is enough when there is at most one of each.
            for nr in npcs.as_array().unwrap_or(&Vec::new()) {
                hp_max = nr.get("hp_max").and_then(|x| x.as_i64()).or(hp_max);
            }
        }
    }

    let key = character_id.clone().unwrap_or_else(|| actor_id.to_string());
    let deltas = load_hp_deltas(
        token,
        &[actor_id.to_string()],
        &character_id.clone().into_iter().collect::<Vec<_>>(),
    )?;
    let spent = deltas.get(&key).copied().unwrap_or(0);

    Ok(Some(ActorVitals {
        character_id,
        hp_max,
        hp_current: hp_max.unwrap_or(0) + spent,
        successes,
        failures,
        dead,
    }))
}

/// A character's numbers, gathered once for the whole encounter.
struct CharStats {
    ac: i64,
    hp_max: Option<i64>,
    successes: i64,
    failures: i64,
    dead: bool,
}

/// Every hit point delta recorded against anyone in the encounter,
/// summed per subject.
///
/// One request for both kinds of subject. The key is the actor id for an
/// NPC instance and the character id for a player, matching how the
/// events are written - see Target::character_id.
fn load_hp_deltas(
    token: &str,
    actor_ids: &[String],
    char_ids: &[String],
) -> Result<HashMap<String, i64>, String> {
    if actor_ids.is_empty() && char_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut clauses: Vec<String> = Vec::new();
    if !actor_ids.is_empty() {
        clauses.push(format!("actor_id.in.({})", actor_ids.join(",")));
    }
    if !char_ids.is_empty() {
        clauses.push(format!("character_id.in.({})", char_ids.join(",")));
    }

    let rows = supabase::rest_get(
        token,
        "hp_events",
        &[
            ("select", "character_id,actor_id,delta"),
            ("or", &format!("({})", clauses.join(","))),
        ],
    )?;

    let mut out: HashMap<String, i64> = HashMap::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let key = as_opt_str(r, "character_id").or_else(|| as_opt_str(r, "actor_id"));
        if let Some(k) = key {
            *out.entry(k).or_insert(0) += r.get("delta").and_then(|x| x.as_i64()).unwrap_or(0);
        }
    }
    Ok(out)
}

/// Every enrolled character's AC and hit point maximum, in three
/// requests rather than three per character.
fn batch_character_stats(
    token: &str,
    game_id: &str,
    char_ids: &[String],
) -> Result<HashMap<String, CharStats>, String> {
    let list = char_ids.join(",");

    let chars = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "id,ac_mode,ac_override,hp_max,death_successes,death_failures,dead"),
            ("id", &format!("in.({})", list)),
        ],
    )?;

    let abil = supabase::rest_get(
        token,
        "character_abilities",
        &[
            ("select", "character_id,ability,score"),
            ("character_id", &format!("in.({})", list)),
            ("ability", "eq.dex"),
        ],
    )?;
    let mut dex: HashMap<String, i64> = HashMap::new();
    for r in abil.as_array().unwrap_or(&Vec::new()) {
        let score = r.get("score").and_then(|x| x.as_i64()).unwrap_or(10);
        dex.insert(as_str(r, "character_id"), (score - 10).div_euclid(2));
    }

    // Equipped rows for everyone at once, then the catalogue for every
    // key they mention. Two requests, not two per character.
    let owned = supabase::rest_get(
        token,
        "character_items",
        &[
            ("select", "character_id,item_key"),
            ("character_id", &format!("in.({})", list)),
            ("equipped", "is.true"),
        ],
    )?;
    let owned = owned.as_array().cloned().unwrap_or_default();

    let keys: Vec<String> = owned
        .iter()
        .map(|r| as_str(r, "item_key"))
        .collect::<HashSet<_>>()
        .into_iter()
        .map(|k| quoted(&k))
        .collect();

    let mut catalogue: HashMap<String, Item> = HashMap::new();
    if !keys.is_empty() {
        let rows = supabase::rest_get(
            token,
            "items",
            &[
                ("select", equipment::ITEM_COLUMNS),
                ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
                ("key", &format!("in.({})", keys.join(","))),
            ],
        )?;
        for item in equipment::collapse_overrides(rows.as_array().unwrap_or(&Vec::new())) {
            catalogue.insert(item.key.clone(), item);
        }
    }

    let mut worn: HashMap<String, Vec<Item>> = HashMap::new();
    for r in &owned {
        if let Some(item) = catalogue.get(&as_str(r, "item_key")) {
            worn.entry(as_str(r, "character_id"))
                .or_default()
                .push(item.clone());
        }
    }

    let mut out = HashMap::new();
    for r in chars.as_array().unwrap_or(&Vec::new()) {
        let id = as_str(r, "id");
        let items: Vec<&Item> = worn.get(&id).map(|v| v.iter().collect()).unwrap_or_default();
        out.insert(
            id.clone(),
            CharStats {
                ac: equipment::armor_class(
                    dex.get(&id).copied().unwrap_or(0),
                    &items,
                    AcMode::parse(&as_str(r, "ac_mode")),
                    r.get("ac_override").and_then(|x| x.as_i64()),
                ),
                hp_max: r.get("hp_max").and_then(|x| x.as_i64()),
                successes: r.get("death_successes").and_then(|x| x.as_i64()).unwrap_or(0),
                failures: r.get("death_failures").and_then(|x| x.as_i64()).unwrap_or(0),
                dead: r.get("dead").and_then(|x| x.as_bool()).unwrap_or(false),
            },
        );
    }
    Ok(out)
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_override_beats_everything() {
        let (v, why) = resolve_ac(Some(19), Some(15), Some(16)).unwrap();
        assert_eq!(v, 19);
        assert!(why.contains("override"), "unhelpful source: {}", why);
    }

    #[test]
    fn an_npc_takes_its_statblock() {
        let (v, why) = resolve_ac(None, Some(15), None).unwrap();
        assert_eq!(v, 15);
        assert!(why.contains("statblock"), "unhelpful source: {}", why);
    }

    #[test]
    fn a_character_takes_the_computed_number() {
        let (v, why) = resolve_ac(None, None, Some(16)).unwrap();
        assert_eq!(v, 16);
        assert!(why.contains("computed"), "unhelpful source: {}", why);
    }

    #[test]
    fn an_actor_with_no_source_yields_no_target() {
        // A statblock deleted out from under an actor. Better to offer
        // nothing than a default everybody hits.
        assert!(resolve_ac(None, None, None).is_none());
    }

    #[test]
    fn an_override_of_zero_is_still_an_override() {
        // Some(0) is a real AC, and must not fall through to the
        // statblock the way a None would.
        let (v, _) = resolve_ac(Some(0), Some(15), None).unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn every_source_says_where_the_number_came_from() {
        for case in [
            resolve_ac(Some(19), None, None),
            resolve_ac(None, Some(15), None),
            resolve_ac(None, None, Some(16)),
        ] {
            let (_, why) = case.unwrap();
            assert!(!why.trim().is_empty(), "a number arrived with no account of itself");
        }
    }
}
