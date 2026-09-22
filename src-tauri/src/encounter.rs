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

use crate::character::Sheet;
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
///
/// TWO SOURCES, NOT THREE. There used to be a statblock source between
/// these, for monsters whose AC was read live off `npcs`. 022 removed
/// it: every actor is a character and carries its own armour class, a
/// statblock's arriving as ac_mode 'flat' with an override. So a
/// monster's AC resolves through the same `character_ac` a player's
/// does, and the difference is a setting rather than a branch.
pub fn resolve_ac(ac_override: Option<i64>, character_ac: Option<i64>) -> Option<(i64, String)> {
    if let Some(v) = ac_override {
        return Some((v, "override on this actor".to_string()));
    }
    if let Some(v) = character_ac {
        return Some((v, "the individual's own".to_string()));
    }
    // An actor whose character has vanished. Offering no target is
    // honest; inventing 10 would be a number someone rolls against.
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

    // NO STATBLOCK READ. Since 022 every actor IS a character and owns
    // its armour class and its maximum hit points, so there is nothing
    // to look up on `npcs` - a whole request per encounter gone, and
    // with it the hazard that editing a type moved a creature already
    // in play. An NPC's AC arrives as ac_mode 'flat' with an override
    // and is resolved by the same function a player's is.

    // --- player characters, computed in one batch ---
    let char_ids: Vec<String> = actor_rows
        .iter()
        // Every one of them, monsters included.
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
        let character_id = as_opt_str(r, "character_id");
        let stats = character_id.as_ref().and_then(|c| char_stats.get(c));
        let pc = stats.map(|s| s.ac);

        // An override on the actor wins, then the individual's own
        // maximum. Two goblins off one statblock still differ, because
        // 022 made them two characters rather than two views of one.
        let hp_max = r
            .get("hp_override")
            .and_then(|x| x.as_i64())
            .or_else(|| stats.and_then(|s| s.hp_max));

        // ONE KEY. Damage lands on the character since 023, for everyone,
        // so there is no longer a question of which id to look under.
        let spent = character_id
            .as_ref()
            .and_then(|c| deltas.get(c).copied())
            .unwrap_or(0);
        let hp_current = hp_max.map(|m| m + spent);

        // And the death saves live with the hit points, on the same row.
        let (successes, failures, flagged) = match stats {
            Some(st) => (st.successes, st.failures, st.dead),
            None => (0, 0, false),
        };

        // Unconscious, and therefore worth nothing to hit. HOUSE RULE -
        // see death.rs; 5e keeps the armour class and grants advantage.
        let condition = death::condition(hp_current.unwrap_or(1), successes, failures, flagged);

        match resolve_ac(r.get("ac_override").and_then(|x| x.as_i64()), pc) {
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
    /// What this instance is called — "Goblin 1". Carried because a roll
    /// has to snapshot WHOSE action it was, and for an NPC there is no
    /// character row for the 001 trigger to derive that name from: it
    /// falls through to 'Someone' and the log permanently cannot say
    /// which goblin was dying.
    pub label: String,
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
                "id,character_id,npc_key,label,hp_override,death_successes,death_failures,dead",
            ),
            ("id", &format!("eq.{}", actor_id)),
        ],
    )?;
    let r = match rows.as_array().and_then(|a| a.first()) {
        Some(r) => r.clone(),
        None => return Ok(None),
    };

    // ONE PATH SINCE 022. character_id is NOT NULL on every actor, so
    // this no longer asks whether it is looking at a person or a
    // monster - the individual owns its maximum, its death saves and
    // its wounds, whichever it is. The statblock branch that used to
    // sit here read `npcs` live, which is the hazard 022 removed.
    let character_id = as_opt_str(&r, "character_id");
    let mut hp_max = r.get("hp_override").and_then(|x| x.as_i64());
    let mut successes = 0;
    let mut failures = 0;
    let mut dead = false;

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
            // An override on this appearance still wins over the
            // individual's own maximum - the ogre that walks in already
            // wounded is a per-encounter fact.
            hp_max = hp_max.or_else(|| cr.get("hp_max").and_then(|x| x.as_i64()));
            successes = cr.get("death_successes").and_then(|x| x.as_i64()).unwrap_or(0);
            failures = cr.get("death_failures").and_then(|x| x.as_i64()).unwrap_or(0);
            dead = cr.get("dead").and_then(|x| x.as_bool()).unwrap_or(false);
        }
    }

    // Damage lands on the character since 023, for everyone.
    let chars: Vec<String> = character_id.clone().into_iter().collect();
    let deltas = load_hp_deltas(token, &[], &chars)?;
    let spent = character_id
        .as_ref()
        .and_then(|c| deltas.get(c).copied())
        .unwrap_or(0);

    Ok(Some(ActorVitals {
        character_id,
        label: as_opt_str(&r, "label").unwrap_or_default(),
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
        "objects",
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
        let (v, why) = resolve_ac(Some(19), Some(16)).unwrap();
        assert_eq!(v, 19);
        assert!(why.contains("override"), "unhelpful source: {}", why);
    }

    #[test]
    fn everyone_else_takes_their_own() {
        // Monster or person, one source since 022. A statblock's AC
        // arrives on the individual as a flat override and resolves
        // here exactly as a computed one does.
        let (v, why) = resolve_ac(None, Some(16)).unwrap();
        assert_eq!(v, 16);
        assert!(!why.trim().is_empty(), "a number arrived with no account of itself");
    }

    #[test]
    fn an_actor_with_no_source_yields_no_target() {
        // A character deleted out from under an actor. Better to offer
        // nothing than a default everybody hits.
        assert!(resolve_ac(None, None).is_none());
    }

    #[test]
    fn an_override_of_zero_is_still_an_override() {
        // Some(0) is a real AC, and must not fall through to the
        // individual's own the way a None would.
        let (v, _) = resolve_ac(Some(0), Some(15)).unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn every_source_says_where_the_number_came_from() {
        for case in [
            resolve_ac(Some(19), None),
            resolve_ac(None, Some(16)),
        ] {
            let (_, why) = case.unwrap();
            assert!(!why.trim().is_empty(), "a number arrived with no account of itself");
        }
    }
}

/* ======================== A MONSTER'S SHEET ======================== */

/// The sheet of whoever this actor is.
///
/// THIS USED TO BE NINETY LINES. It read a statblock, translated `intl`
/// back to "int", assembled six ability rows by hand, loaded a separate
/// NPC kit, and built a Sheet that looked like a character's without
/// being one. 022 deleted the reason for all of it: an actor points at a
/// character, and a monster's sheet is loaded by the function that loads
/// anyone's.
///
/// What is left is the lookup from an actor to its individual. Returns
/// None when the actor does not exist or is not visible.
pub fn load_actor_sheet(token: &str, actor_id: &str) -> Result<Option<Sheet>, String> {
    let rows = supabase::rest_get(
        token,
        "encounter_actors",
        &[
            ("select", "id,character_id"),
            ("id", &format!("eq.{}", actor_id)),
        ],
    )?;
    let character_id = match rows.as_array().and_then(|x| x.first()) {
        Some(r) => match as_opt_str(r, "character_id") {
            Some(c) => c,
            None => return Ok(None),
        },
        None => return Ok(None),
    };
    crate::character::load_sheet(token, &character_id).map(Some)
}
