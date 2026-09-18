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
                "id,character_id,npc_key,label,ac_override,initiative",
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
    if !npc_keys.is_empty() {
        let quoted_keys: Vec<String> = npc_keys.iter().map(|k| quoted(k)).collect();
        let rows = supabase::rest_get(
            token,
            "npcs",
            &[
                ("select", "key,game_id,name,ac"),
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
            if scoped || !npc_ac.contains_key(&key) {
                npc_ac.insert(key, ac);
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

    let char_ac = if char_ids.is_empty() {
        HashMap::new()
    } else {
        batch_character_ac(token, &game_id, &char_ids)?
    };

    // --- assemble ---
    let mut out = Vec::new();

    for r in &actor_rows {
        let label = as_str(r, "label");
        let npc = as_opt_str(r, "npc_key").and_then(|k| npc_ac.get(&k).copied());
        let pc = as_opt_str(r, "character_id").and_then(|c| char_ac.get(&c).copied());

        match resolve_ac(r.get("ac_override").and_then(|x| x.as_i64()), npc, pc) {
            Some((value, source)) => out.push(Target {
                id: as_str(r, "id"),
                row: "actor",
                target_kind: "ac",
                label,
                value,
                source,
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
fn batch_character_ac(
    token: &str,
    game_id: &str,
    char_ids: &[String],
) -> Result<HashMap<String, i64>, String> {
    let list = char_ids.join(",");

    let chars = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "id,ac_mode,ac_override"),
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
            equipment::armor_class(
                dex.get(&id).copied().unwrap_or(0),
                &items,
                AcMode::parse(&as_str(r, "ac_mode")),
                r.get("ac_override").and_then(|x| x.as_i64()),
            ),
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
