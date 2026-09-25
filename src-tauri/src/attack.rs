//! Weapon attacks: what a swing is worth.
//!
//! The rules half of `WeaponsAttacks.js`, ported without the half that
//! made it a liability. The original computed to-hit and damage and then
//! WROTE THEM INTO A SPREADSHEET TAB, which is why its own header told
//! you to reseed after every level-up, every new weapon and every
//! ability change. Nothing here is stored. Every number is worked out
//! from the live sheet at the moment of the roll.
//!
//! Finesse is the argument that settles it: the ability a weapon uses
//! depends on which of STR and DEX is higher TODAY, so any stored answer
//! has a shelf life measured in stat changes.
//!
//! WHAT IS NOT HERE
//!   The damage roll itself. An attack is two rolls - to-hit, then
//!   damage - and what owns both of them does not exist yet. This module
//!   works out the damage FORMULA and hands it over; rolling it, and
//!   grouping the two, is the action problem named in STATUS.md.
//!
//! THE MAGICAL BONUS IS ABSENT ON PURPOSE. The original added
//! `s.magicalBonus` to both to-hit and damage. 008 did not port that
//! column because every weapon in the export has it null, and inventing
//! a column to hold zero would be ceremony. When a +1 sword arrives it
//! is one column and one addend in `to_hit`.

use crate::equipment::{Mode, Owned};

/// A house technique: a named attack with its own damage dice and its
/// own crit and fumble range. 006 seeded 31 of them; 007 keyed them to
/// an item and a mode.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Technique {
    pub key: String,
    pub name: String,
    /// What a player types. Lowercase, punctuation stripped.
    pub roll_name: String,
    pub item_key: String,
    pub mode: Mode,
    pub min_level: i64,
    /// Damage dice ONLY - "1d8", "3d6". The ability modifier is added
    /// at resolve time and must never be baked in here.
    pub dice: String,
    pub crit_min: i64,
    pub fumble_max: i64,
    /// What the table adjudicates. 043 calls it "prose the engine does
    /// not read", which was true of the engine and also of every
    /// screen - a hundred and fifty-nine of these were written and
    /// nothing rendered one. The engine still does not read it; the
    /// viewer does.
    pub special_text: Option<String>,
    /// Which scope this row won at: "global", "game" or "object". The
    /// screen says so, because "this sword's own move" and "the
    /// rulebook's" look identical once merged and a DM editing one
    /// should know which they are about to change.
    pub scope: String,
    /// WHOSE LIST THIS ENTRY IS IN - not where the definition came
    /// from, which is `scope`. The two differ constantly: the rulebook's
    /// Zwerchhau appears in a particular sword's list as scope "global"
    /// and object_id that sword.
    ///
    /// Set for everything the SHEET loads, because a technique that is
    /// not bound to an object cannot be told apart from the same
    /// technique on a second weapon of the same type - which is the
    /// collision that kept per-object moves out of the roll path until
    /// now. None only for a type-level read.
    pub object_id: Option<String>,
}

/// One row as it arrived, before the scopes are collapsed.
///
/// A SEPARATE TYPE FROM `Technique` because it carries two things the
/// merged answer must not: which scope it came from, and whether it is
/// a tombstone. Once `collapse_scopes` has run, a removed row is gone
/// rather than present-and-false - the absence is the answer.
#[derive(Debug, Clone)]
pub struct ScopedTechnique {
    pub technique: Technique,
    /// 0 global, 1 game, 2 object. Higher wins.
    pub rank: u8,
    pub removed: bool,
}

/// A resolved attack, with the evidence for every number in it.
///
/// `ability`, `proficient` and `proficiency_bonus` are carried rather
/// than folded away because a to-hit of +1 where another weapon gives +5
/// looks like a bug unless the card can say the character is not trained
/// with it. Same requirement the equipment panel already meets.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Attack {
    pub item_key: String,
    pub weapon_name: String,
    pub mode: Mode,
    /// "str" or "dex" - which one was used, after finesse.
    pub ability: String,
    pub ability_mod: i64,
    pub proficient: bool,
    /// The bonus APPLIED. Zero when not proficient, which is the whole
    /// difference between the Mace at +5 and the Heavy Crossbow at +1.
    pub proficiency_bonus: i64,
    pub to_hit: i64,
    /// Ready for the dice engine: "1d6+1".
    pub damage: String,
    pub crit_min: i64,
    pub fumble_max: i64,
    /// The technique used, if the request named one.
    pub technique: Option<String>,
}

/* ============================ RULES ============================ */

/// Which ability swings this weapon.
///
/// Ported from `abilityFor`. Finesse takes the better of STR and DEX;
/// otherwise the weapon's mode decides, and THROWN KEEPS STR - a thrown
/// hammer is a strength attack that happens to travel. Getting that
/// wrong is invisible on a character whose scores are close.
pub fn ability_for(properties: &[String], mode: Mode, str_mod: i64, dex_mod: i64) -> (&'static str, i64) {
    if properties.iter().any(|p| p == "fin") {
        return if dex_mod > str_mod {
            ("dex", dex_mod)
        } else {
            ("str", str_mod)
        };
    }
    match mode {
        Mode::Ranged => ("dex", dex_mod),
        Mode::Melee | Mode::Thrown => ("str", str_mod),
    }
}

/// Ability modifier plus the proficiency bonus, when it applies.
pub fn to_hit(ability_mod: i64, proficient: bool, proficiency_bonus: i64) -> i64 {
    ability_mod + if proficient { proficiency_bonus } else { 0 }
}

/// "1d6" and a modifier of 1 becomes "1d6+1"; a modifier of 0 becomes
/// "1d6" with no trailing sign, which is what the dice engine and the
/// original both render.
pub fn damage_formula(dice: &str, flat: i64) -> String {
    if flat == 0 {
        dice.to_string()
    } else if flat > 0 {
        format!("{}+{}", dice, flat)
    } else {
        format!("{}{}", dice, flat)
    }
}

/// Normalize a typed request for matching: lowercase, collapse runs of
/// whitespace, drop apostrophes.
///
/// `techniques.roll_name` is stored already stripped ("stones judgment"),
/// so a player typing "Stone's Judgment" has to arrive at the same
/// string or the technique is simply never found.
pub fn normalize(s: &str) -> String {
    let lowered = s.trim().to_lowercase().replace('\'', "");
    lowered.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The weapon name a player would type for a given mode.
///
/// Thrown is suffixed because one weapon offers two attacks with
/// different technique lists - the distinction 007 pulled out of the
/// display name and into a column.
pub fn weapon_request_name(name: &str, mode: Mode) -> String {
    match mode {
        Mode::Thrown => format!("{} (thrown)", normalize(name)),
        _ => normalize(name),
    }
}

/// Resolve a typed request into an attack, if it names one.
///
/// A TECHNIQUE IS TRIED FIRST. "heavy smash" names a technique, which
/// carries its own weapon, mode, dice and thresholds; "mace of the deep
/// song" names the weapon itself and gets the plain attack. Weapons are
/// matched second so a technique can never be shadowed by a weapon
/// whose name happens to contain it.
///
/// Returns None for anything that is not an attack, which the caller
/// treats as "carry on and try the skills".
pub fn resolve(
    request: &str,
    loadout: &[Owned],
    techniques: &[Technique],
    level: i64,
    proficiency_bonus: i64,
    str_mod: i64,
    dex_mod: i64,
) -> Option<Attack> {
    let want = normalize(request);

    // --- a technique, if the player named one ---
    if let Some(t) = techniques.iter().find(|t| normalize(&t.roll_name) == want) {
        // Gated by level. A technique that has not been earned is not
        // an error and not a fallback - it simply is not available, and
        // saying so beats silently rolling the plain weapon instead.
        if t.min_level > level {
            return None;
        }
        // BOUND TO ONE OBJECT WHEN IT SAYS SO. A move the sheet
        // loaded names the weapon it belongs to, which is what lets two
        // greatswords carry different versions of it. A type-level
        // technique - anything not loaded through expand_for_objects -
        // still matches any equipped weapon of its kind.
        let owned = loadout.iter().find(|o| {
            o.modes.contains(&t.mode)
                && match t.object_id.as_deref() {
                    Some(id) => o.id == id,
                    None => o.item.key == t.item_key,
                }
        })?;

        let (ability, ability_mod) =
            ability_for(&owned.item.properties, t.mode, str_mod, dex_mod);

        return Some(Attack {
            item_key: owned.item.key.clone(),
            weapon_name: owned.item.name.clone(),
            mode: t.mode,
            ability: ability.to_string(),
            ability_mod,
            proficient: owned.proficient,
            proficiency_bonus: if owned.proficient { proficiency_bonus } else { 0 },
            to_hit: to_hit(ability_mod, owned.proficient, proficiency_bonus),
            damage: damage_formula(&t.dice, ability_mod),
            crit_min: t.crit_min,
            fumble_max: t.fumble_max,
            technique: Some(t.name.clone()),
        });
    }

    // --- otherwise a plain weapon attack, in one of its modes ---
    for owned in loadout {
        if owned.item.kind != "weapon" {
            continue;
        }
        for mode in &owned.modes {
            if weapon_request_name(&owned.item.name, *mode) != want {
                continue;
            }
            let (ability, ability_mod) =
                ability_for(&owned.item.properties, *mode, str_mod, dex_mod);

            // A weapon with no dice is a data fault. Offering no attack
            // beats offering one that rolls nothing.
            let n = owned.item.damage_number?;
            let d = owned.item.damage_denomination?;

            return Some(Attack {
                item_key: owned.item.key.clone(),
                weapon_name: owned.item.name.clone(),
                mode: *mode,
                ability: ability.to_string(),
                ability_mod,
                proficient: owned.proficient,
                proficiency_bonus: if owned.proficient { proficiency_bonus } else { 0 },
                to_hit: to_hit(ability_mod, owned.proficient, proficiency_bonus),
                damage: damage_formula(&format!("{}d{}", n, d), ability_mod),
                // Standard thresholds. Only a technique widens them.
                crit_min: 20,
                fumble_max: 1,
                technique: None,
            });
        }
    }

    None
}

/// What the card says: "Mace of the Deep Song (Melee)", or the technique
/// name with its weapon underneath it.
pub fn label(a: &Attack) -> String {
    let mode = match a.mode {
        Mode::Melee => "Melee",
        Mode::Thrown => "Thrown",
        Mode::Ranged => "Ranged",
    };
    match &a.technique {
        Some(t) => format!("{} · {} ({})", t, a.weapon_name, mode),
        None => format!("{} ({})", a.weapon_name, mode),
    }
}

/// Does this swing roll damage?
///
/// No target means nobody can say whether it connected, so the damage is
/// rolled and the table decides - which is exactly what the old system
/// did, because it never knew an AC and posted attack and damage
/// together regardless.
///
/// With a target the dice have already decided, so a miss rolls nothing.
/// That is the difference knowing the AC buys: a missed swing stops
/// producing a damage figure nobody should read.
pub fn rolls_damage(success: Option<bool>) -> bool {
    success.unwrap_or(true)
}

/* ============================ LOADING ============================ */

fn mode_from(s: &str) -> Option<Mode> {
    match s.trim().to_lowercase().as_str() {
        "melee" => Some(Mode::Melee),
        "thrown" => Some(Mode::Thrown),
        "ranged" => Some(Mode::Ranged),
        // A fifth value added to the check constraint without being
        // added here is dropped rather than guessed at. check_item_keys
        // is where a mismatch of this shape gets reported.
        _ => None,
    }
}

/// Every technique available to the weapons a character is holding.
///
/// EQUIPPED ONLY, for the same reason the loadout is: a technique for a
/// weapon in the backpack cannot be used, and the sheet is read on every
/// roll. Character1 holding the Mace pulls ten rows, not thirty-one.
///
/// An empty key list means no request at all - an empty `in.()` is not
/// valid PostgREST, so the guard is load-bearing rather than tidiness.
/// Same trap `load_loadout` documents.
/// Collapse the three scopes into what this object actually has.
///
/// MOST SPECIFIC WINS, per key: an object row beats a game row beats a
/// global one. That is 001's nullable-tenancy precedence with a third
/// level, and the same rule `collapse_overrides` applies to items.
///
/// A TOMBSTONE DELETES THE KEY RATHER THAN WINNING IT. 050 added
/// `removed` because deleting an object's row means "stop overriding"
/// and restores the inherited move - there is no row to delete to say
/// this sword does NOT have what its type has. So the flag wins the
/// key and then removes it, and the caller sees an absence rather than
/// a row it has to know to skip.
///
/// AN OBJECT ROW WITH NO ANCESTOR IS AN ADDITION, and nothing special
/// happens for it: it wins its key because nothing else claims it.
/// That is how a named sword carries a move nothing else has.
///
/// ORDER IS BY LEVEL THEN NAME, so a list reads as a progression rather
/// than in whatever order three scopes happened to arrive.
pub fn collapse_scopes(rows: &[ScopedTechnique]) -> Vec<Technique> {
    let mut best: Vec<&ScopedTechnique> = Vec::new();
    for r in rows {
        match best.iter().position(|b| b.technique.key == r.technique.key) {
            Some(i) => {
                if r.rank >= best[i].rank {
                    best[i] = r;
                }
            }
            None => best.push(r),
        }
    }

    let mut out: Vec<Technique> = best
        .into_iter()
        .filter(|b| !b.removed)
        .map(|b| b.technique.clone())
        .collect();
    out.sort_by(|a, b| {
        a.min_level
            .cmp(&b.min_level)
            .then_with(|| a.name.cmp(&b.name))
    });
    out
}

/// One row, whichever scope it came from.
///
/// Extracted so the two loaders below cannot disagree about what a
/// technique row means - which is the failure this repo keeps meeting
/// whenever one rule is written twice.
fn scoped_from_row(r: &serde_json::Value) -> Option<ScopedTechnique> {
    let mode = mode_from(r.get("mode").and_then(|x| x.as_str())?)?;
    let has_game = r.get("game_id").and_then(|x| x.as_str()).is_some();
    let has_object = r.get("object_id").and_then(|x| x.as_str()).is_some();
    let (rank, scope) = match (has_object, has_game) {
        (true, _) => (2u8, "object"),
        (false, true) => (1, "game"),
        (false, false) => (0, "global"),
    };
    Some(ScopedTechnique {
        rank,
        removed: r.get("removed").and_then(|x| x.as_bool()).unwrap_or(false),
        technique: Technique {
            key: r.get("key").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            name: r.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            roll_name: r.get("roll_name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            item_key: r.get("item_key").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            mode,
            min_level: r.get("min_level").and_then(|x| x.as_i64()).unwrap_or(1),
            dice: r.get("dice").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            crit_min: r.get("crit_min").and_then(|x| x.as_i64()).unwrap_or(20),
            fumble_max: r.get("fumble_max").and_then(|x| x.as_i64()).unwrap_or(1),
            special_text: r
                .get("special_text")
                .and_then(|x| x.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string),
            scope: scope.to_string(),
            // Filled by whoever expands these into a list; a raw row
            // knows which object it BELONGS to, not whose list it is
            // about to appear in.
            object_id: r.get("object_id").and_then(|x| x.as_str()).map(str::to_string),
        },
    })
}

const TECHNIQUE_COLUMNS: &str = "key,game_id,object_id,removed,name,roll_name,item_key,mode,\
     min_level,dice,crit_min,fumble_max,special_text";

/// The techniques a TYPE offers, for the sheet.
///
/// TYPE SCOPES ONLY - global and game, never an object's own. The
/// sheet's technique list is flat and keyed by technique, not by the
/// object holding it, so two greatswords with different versions of one
/// move would collide on the key and one would silently win. 050 made
/// per-object moves possible and this is the loader that deliberately
/// does not read them; `load_for_object` does.
///
/// THAT IS A STATED GAP, NOT A SILENT ONE. A per-object move is visible
/// and editable in the object viewer and is NOT yet rollable from the
/// sheet. Closing it means `Sheet.techniques` carrying which object
/// each move came from, which is a change to the attack path rather
/// than to this query.
/// Every equipped weapon's own list of moves, each one bound to the
/// object it belongs to.
///
/// THE COLLISION THIS EXISTS TO REMOVE. A flat list keyed by technique
/// cannot hold two greatswords with different versions of one move -
/// one silently wins, which is the failure this repo keeps meeting. So
/// nothing here is keyed by technique: the list is expanded PER OBJECT,
/// and two greatswords contribute two Zwerchhaus that differ in nothing
/// but `object_id` if neither is overridden, and in their dice if one
/// is.
///
/// A TOMBSTONE NEEDS NO REPRESENTATION, which is the other reason to
/// expand rather than merge. A sword that struck out Zwerchhau simply
/// has no Zwerchhau in its own list - there is no type-level entry left
/// over for the resolver to find and wrongly apply. A merged list would
/// have needed a suppression rule beside it, which is a second
/// mechanism for an absence that 050 already represents.
///
/// `weapons` is (object id, item key) for the EQUIPPED weapons, which
/// is what a loadout is.
pub fn expand_for_objects(
    rows: &[ScopedTechnique],
    weapons: &[(String, String)],
) -> Vec<Technique> {
    let mut out: Vec<Technique> = Vec::new();
    for (object_id, item_key) in weapons {
        // This object's own rows and its type's, and no other object's.
        let mine: Vec<ScopedTechnique> = rows
            .iter()
            .filter(|r| {
                r.technique.item_key == *item_key
                    && match r.technique.object_id.as_deref() {
                        Some(id) => id == object_id,
                        None => true,
                    }
            })
            .cloned()
            .collect();

        for mut t in collapse_scopes(&mine) {
            // WHOSE LIST, not where it came from. `scope` keeps the
            // second answer.
            t.object_id = Some(object_id.clone());
            out.push(t);
        }
    }
    out
}

/// Every equipped weapon's moves, for the sheet.
///
/// Replaces the type-scoped read that could not see an object's own -
/// see `expand_for_objects` for why a flat list could not hold them.
pub fn load_for_loadout(
    token: &str,
    game_id: &str,
    weapons: &[(String, String)],
) -> Result<Vec<Technique>, String> {
    if weapons.is_empty() {
        return Ok(Vec::new());
    }
    let keys: Vec<String> = weapons
        .iter()
        .map(|(_, k)| crate::narrative::quoted(k))
        .collect();
    let ids: Vec<String> = weapons.iter().map(|(id, _)| id.clone()).collect();

    let rows = crate::supabase::rest_get(
        token,
        "techniques",
        &[
            ("select", TECHNIQUE_COLUMNS),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            // The types' rows plus these objects' own, and no other
            // object's - which the second half is what excludes.
            (
                "or",
                &format!("(object_id.is.null,object_id.in.({}))", ids.join(",")),
            ),
            ("item_key", &format!("in.({})", keys.join(","))),
        ],
    )?;

    let scoped: Vec<ScopedTechnique> = rows
        .as_array()
        .map(|a| a.as_slice())
        .unwrap_or(&[])
        .iter()
        .filter_map(scoped_from_row)
        .collect();
    Ok(expand_for_objects(&scoped, weapons))
}

/// Everything ONE object can do: its type's moves, with its own on top.
///
/// All three scopes, collapsed by `collapse_scopes` - an object row
/// beats a game row beats a global one, and a tombstone removes the key
/// rather than winning it.
pub fn load_for_object(
    token: &str,
    game_id: &str,
    object_id: &str,
    item_key: &str,
) -> Result<Vec<Technique>, String> {
    let rows = crate::supabase::rest_get(
        token,
        "techniques",
        &[
            ("select", TECHNIQUE_COLUMNS),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            // The type's rows, plus this object's own - and no other
            // object's, which is what the second half excludes.
            (
                "or",
                &format!("(object_id.is.null,object_id.eq.{})", object_id),
            ),
            ("item_key", &format!("eq.{}", item_key)),
        ],
    )?;

    let scoped: Vec<ScopedTechnique> = rows
        .as_array()
        .map(|a| a.as_slice())
        .unwrap_or(&[])
        .iter()
        .filter_map(scoped_from_row)
        .collect();
    // The same expansion the sheet uses, for one object - so both
    // screens agree that object_id means "whose list this is in".
    Ok(expand_for_objects(
        &scoped,
        &[(object_id.to_string(), item_key.to_string())],
    ))
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equipment::Item;

    /// Rodnar as the Attacks tab knew him: STR +2, DEX +1, PB +3,
    /// trained in `sim` only. Those four numbers produce the four rows
    /// the old seeder wrote, which is what makes this a parity fixture
    /// rather than a fixture I chose.
    const STR: i64 = 2;
    const DEX: i64 = 1;
    const PB: i64 = 3;

    fn weapon(key: &str, name: &str, class: &str, n: i64, d: i64, props: &[&str]) -> Item {
        Item {
            key: key.into(),
            name: name.into(),
            kind: "weapon".into(),
            base_item: None,
            weapon_class: Some(class.into()),
            damage_number: Some(n),
            damage_denomination: Some(d),
            damage_types: vec![],
            properties: props.iter().map(|s| s.to_string()).collect(),
            range_reach: None,
            range_value: None,
            range_long: None,
            armor_category: None,
            base_ac: None,
            dex_cap: None,
            // Not what these fixtures are about. Written out
            // rather than defaulted, because a fixture that is
            // faithful to the seed makes a failure mean the RULE
            // changed.
            size: "med".into(),
            holds_size: None,
            weight: None,
            accepts: vec![],
            capacity_slots: None,
        }
    }

    fn owned(item: Item, proficient: bool) -> Owned {
        let modes = crate::equipment::modes(&item);
        Owned {
            id: String::new(),
            name: None,
            item,
            quantity: 1,
            equipped: true,
            attuned: false,
            proficient_override: None,
            uses_spent: 0,
            uses_max: None,
            proficient,
            modes,
        }
    }

    fn party() -> Vec<Owned> {
        vec![
            owned(weapon("mace_of_the_deep_song", "Mace of the Deep Song", "simpleM", 1, 6, &[]), true),
            owned(weapon("light_hammer", "Light Hammer", "simpleM", 1, 4, &["lgt", "thr"]), true),
            owned(weapon("heavy_crossbow", "Heavy Crossbow", "martialR", 1, 10, &["amm", "hvy", "lod", "two"]), false),
        ]
    }

    fn heavy_smash() -> Technique {
        Technique {
            key: "smash".into(),
            name: "Heavy Smash".into(),
            roll_name: "heavy smash".into(),
            item_key: "mace_of_the_deep_song".into(),
            mode: Mode::Melee,
            min_level: 1,
            dice: "1d8".into(),
            crit_min: 20,
            fumble_max: 2,
            // Not what these fixtures are about.
            special_text: None,
            scope: "global".into(),
            object_id: None,
        }
    }

    fn deepsong_echo() -> Technique {
        Technique {
            key: "echo".into(),
            name: "Deepsong Echo".into(),
            roll_name: "deepsong echo".into(),
            item_key: "mace_of_the_deep_song".into(),
            mode: Mode::Melee,
            min_level: 6,
            dice: "1d14".into(),
            crit_min: 18,
            fumble_max: 1,
            special_text: None,
            scope: "global".into(),
            object_id: None,
        }
    }

    /* ---------------- parity with the Attacks tab ------------------- */

    #[test]
    fn the_mace_matches_the_seeded_row() {
        let a = resolve("mace of the deep song", &party(), &[], 5, PB, STR, DEX).unwrap();
        assert_eq!(a.to_hit, 5); // the tab's "+5"
        assert_eq!(a.damage, "1d6+2"); // the tab's "1d6+2"
        assert_eq!(a.ability, "str");
        assert!(a.proficient);
    }

    #[test]
    fn the_light_hammer_matches_and_is_proficient_by_derivation() {
        let a = resolve("light hammer", &party(), &[], 5, PB, STR, DEX).unwrap();
        assert_eq!(a.to_hit, 5);
        assert_eq!(a.damage, "1d4+2");
    }

    #[test]
    fn the_crossbow_is_dex_and_unproficient() {
        // The row that proves training moves the number: +1, not +4.
        let a = resolve("heavy crossbow", &party(), &[], 5, PB, STR, DEX).unwrap();
        assert_eq!(a.ability, "dex");
        assert_eq!(a.to_hit, 1);
        assert_eq!(a.proficiency_bonus, 0);
        assert!(!a.proficient);
        assert_eq!(a.damage, "1d10+1");
    }

    #[test]
    fn thrown_is_a_separate_request_and_keeps_strength() {
        // The tab's fourth row. A thrown hammer is a STR attack that
        // happens to travel, which is the easy thing to get wrong.
        let a = resolve("light hammer (thrown)", &party(), &[], 5, PB, STR, DEX).unwrap();
        assert_eq!(a.mode, Mode::Thrown);
        assert_eq!(a.ability, "str");
        assert_eq!(a.to_hit, 5);
        assert_eq!(a.damage, "1d4+2");
    }

    /* ---------------- ability selection ----------------------------- */

    #[test]
    fn finesse_takes_the_better_of_the_two() {
        let props = vec!["fin".to_string()];
        assert_eq!(ability_for(&props, Mode::Melee, 2, 5), ("dex", 5));
        assert_eq!(ability_for(&props, Mode::Melee, 5, 2), ("str", 5));
    }

    #[test]
    fn finesse_ties_go_to_strength() {
        // Not arbitrary: the original tests dex > str, so equal scores
        // fall through to str. Same tie, same winner.
        let props = vec!["fin".to_string()];
        assert_eq!(ability_for(&props, Mode::Melee, 3, 3), ("str", 3));
    }

    #[test]
    fn a_negative_modifier_still_applies() {
        let a = resolve("mace of the deep song", &party(), &[], 5, PB, -1, DEX).unwrap();
        assert_eq!(a.to_hit, 2);
        assert_eq!(a.damage, "1d6-1");
    }

    #[test]
    fn a_zero_modifier_leaves_the_formula_bare() {
        let a = resolve("mace of the deep song", &party(), &[], 5, PB, 0, DEX).unwrap();
        assert_eq!(a.damage, "1d6");
    }

    /* ---------------- techniques ------------------------------------ */

    #[test]
    fn a_technique_replaces_the_dice_and_the_thresholds() {
        let t = vec![heavy_smash()];
        let a = resolve("heavy smash", &party(), &t, 5, PB, STR, DEX).unwrap();
        assert_eq!(a.damage, "1d8+2"); // the technique's dice, the character's modifier
        assert_eq!(a.fumble_max, 2);
        assert_eq!(a.to_hit, 5); // to-hit is unchanged by a technique
        assert_eq!(a.technique.as_deref(), Some("Heavy Smash"));
    }

    #[test]
    fn a_technique_below_its_level_is_not_available() {
        let t = vec![deepsong_echo()]; // min_level 6
        assert!(resolve("deepsong echo", &party(), &t, 5, PB, STR, DEX).is_none());
        let a = resolve("deepsong echo", &party(), &t, 6, PB, STR, DEX).unwrap();
        assert_eq!(a.crit_min, 18);
        assert_eq!(a.damage, "1d14+2");
    }

    #[test]
    fn a_technique_whose_weapon_is_not_equipped_is_not_available() {
        let t = vec![heavy_smash()];
        let no_mace: Vec<Owned> = party().into_iter().filter(|o| o.item.key != "mace_of_the_deep_song").collect();
        assert!(resolve("heavy smash", &no_mace, &t, 5, PB, STR, DEX).is_none());
    }

    #[test]
    fn a_plain_weapon_uses_standard_thresholds() {
        let a = resolve("mace of the deep song", &party(), &[heavy_smash()], 5, PB, STR, DEX).unwrap();
        assert_eq!(a.crit_min, 20);
        assert_eq!(a.fumble_max, 1);
        assert!(a.technique.is_none());
    }

    /* ---------------- whether damage is rolled ---------------------- */

    #[test]
    fn a_hit_rolls_damage_and_a_miss_does_not() {
        assert!(rolls_damage(Some(true)));
        assert!(!rolls_damage(Some(false)));
    }

    #[test]
    fn no_target_rolls_damage_because_nobody_can_say_otherwise() {
        // The old system's behaviour, and the honest one: with no AC to
        // check against, the damage is rolled and the table decides.
        assert!(rolls_damage(None));
    }

    /* ---------------- matching -------------------------------------- */

    #[test]
    fn capitals_punctuation_and_spacing_do_not_matter() {
        for typed in [
            "Mace of the Deep Song",
            "  mace   of the deep song ",
            "MACE OF THE DEEP SONG",
        ] {
            assert!(resolve(typed, &party(), &[], 5, PB, STR, DEX).is_some(), "failed on {:?}", typed);
        }
    }

    #[test]
    fn an_apostrophe_survives_normalisation() {
        // techniques.roll_name is stored stripped - "stones judgment" -
        // so a player typing it properly has to land on the same string.
        assert_eq!(normalize("Stone's Judgment"), "stones judgment");
    }

    #[test]
    fn a_skill_request_is_not_an_attack() {
        // The branch has to decline cleanly so resolve_request carries
        // on to the skills.
        assert!(resolve("insight", &party(), &[], 5, PB, STR, DEX).is_none());
        assert!(resolve("wis save", &party(), &[], 5, PB, STR, DEX).is_none());
        assert!(resolve("", &party(), &[], 5, PB, STR, DEX).is_none());
    }

    #[test]
    fn the_label_names_the_mode_and_the_technique() {
        let plain = resolve("light hammer (thrown)", &party(), &[], 5, PB, STR, DEX).unwrap();
        assert_eq!(label(&plain), "Light Hammer (Thrown)");
        let tech = resolve("heavy smash", &party(), &[heavy_smash()], 5, PB, STR, DEX).unwrap();
        assert_eq!(label(&tech), "Heavy Smash · Mace of the Deep Song (Melee)");
    }
    /* ---------- three scopes, 050 ---------- */

    fn scoped(key: &str, rank: u8, dice: &str, level: i64, removed: bool) -> ScopedTechnique {
        ScopedTechnique {
            rank,
            removed,
            technique: Technique {
                key: key.into(),
                name: key.into(),
                roll_name: key.into(),
                item_key: "greatsword".into(),
                mode: Mode::Melee,
                min_level: level,
                dice: dice.into(),
                crit_min: 20,
                fumble_max: 1,
                special_text: None,
                scope: match rank { 2 => "object", 1 => "game", _ => "global" }.into(),
                // The scope fixture is about precedence, not about
                // whose list the answer lands in - expand_for_objects
                // fills that, and has its own tests below.
                object_id: if rank == 2 { Some("sword-a".into()) } else { None },
            },
        }
    }

    #[test]
    fn the_type_is_what_an_ordinary_object_has() {
        let out = collapse_scopes(&[scoped("zwerchhau", 0, "2d6", 3, false)]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].dice, "2d6");
        assert_eq!(out[0].scope, "global");
    }

    // MOST SPECIFIC WINS, and the order rows arrive in must not matter -
    // PostgREST returns them however the index feels like.
    #[test]
    fn an_object_beats_a_game_beats_the_global() {
        let g = scoped("zwerchhau", 0, "2d6", 3, false);
        let c = scoped("zwerchhau", 1, "2d8", 3, false);
        let o = scoped("zwerchhau", 2, "2d10", 3, false);

        for order in [
            vec![g.clone(), c.clone(), o.clone()],
            vec![o.clone(), c.clone(), g.clone()],
            vec![c.clone(), o.clone(), g.clone()],
        ] {
            let out = collapse_scopes(&order);
            assert_eq!(out.len(), 1, "one key, one answer");
            assert_eq!(out[0].dice, "2d10");
            assert_eq!(out[0].scope, "object");
        }
    }

    // THE CASE 050 EXISTS FOR. Deleting the object's row restores the
    // inherited move, so the absence needs its own representation.
    #[test]
    fn a_tombstone_removes_the_key_rather_than_winning_it() {
        let out = collapse_scopes(&[
            scoped("zwerchhau", 0, "2d6", 3, false),
            scoped("zwerchhau", 2, "2d6", 3, true),
        ]);
        assert!(out.is_empty(), "the sword does not have that move");
    }

    #[test]
    fn a_tombstone_takes_only_its_own_key() {
        let out = collapse_scopes(&[
            scoped("zwerchhau", 0, "2d6", 3, false),
            scoped("zwerchhau", 2, "2d6", 3, true),
            scoped("descending_cut", 0, "2d6", 1, false),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].key, "descending_cut");
    }

    // A NAMED SWORD WITH A MOVE NOTHING ELSE HAS. No ancestor, so it
    // wins its key by being the only claim on it.
    #[test]
    fn an_object_move_with_no_ancestor_is_an_addition() {
        let out = collapse_scopes(&[
            scoped("descending_cut", 0, "2d6", 1, false),
            scoped("emberfall", 2, "3d8", 5, false),
        ]);
        assert_eq!(out.len(), 2);
        assert!(out.iter().any(|t| t.key == "emberfall" && t.scope == "object"));
    }

    // Level then name, so a list reads as a progression rather than in
    // whatever order three scopes happened to arrive.
    #[test]
    fn the_order_is_the_progression() {
        let out = collapse_scopes(&[
            scoped("c", 0, "1d6", 5, false),
            scoped("a", 0, "1d6", 1, false),
            scoped("b", 0, "1d6", 3, false),
        ]);
        assert_eq!(
            out.iter().map(|t| t.key.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
    }

    #[test]
    fn nothing_in_nothing_out() {
        assert!(collapse_scopes(&[]).is_empty());
    }

    /* ---------- two swords of one kind ---------- */

    fn owned_by(key: &str, rank: u8, dice: &str, object: Option<&str>) -> ScopedTechnique {
        let mut st = scoped(key, rank, dice, 1, false);
        st.technique.object_id = object.map(str::to_string);
        st
    }

    fn two_greatswords() -> Vec<(String, String)> {
        vec![
            ("sword-a".into(), "greatsword".into()),
            ("sword-b".into(), "greatsword".into()),
        ]
    }

    // THE COLLISION THE WHOLE CHANGE EXISTS TO REMOVE. One type-level
    // move, two swords: the list holds it twice, bound to each, so
    // neither can silently win the other's.
    #[test]
    fn one_type_move_becomes_one_entry_per_object() {
        let out = expand_for_objects(
            &[owned_by("zwerchhau", 0, "2d6", None)],
            &two_greatswords(),
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].object_id.as_deref(), Some("sword-a"));
        assert_eq!(out[1].object_id.as_deref(), Some("sword-b"));
        assert!(out.iter().all(|t| t.dice == "2d6"));
    }

    // One sword enchanted, the other not. Both keep the move; only one
    // has the better dice.
    #[test]
    fn an_override_reaches_only_its_own_sword() {
        let out = expand_for_objects(
            &[
                owned_by("zwerchhau", 0, "2d6", None),
                owned_by("zwerchhau", 2, "2d10", Some("sword-a")),
            ],
            &two_greatswords(),
        );
        let a = out.iter().find(|t| t.object_id.as_deref() == Some("sword-a")).unwrap();
        let b = out.iter().find(|t| t.object_id.as_deref() == Some("sword-b")).unwrap();
        assert_eq!(a.dice, "2d10");
        assert_eq!(a.scope, "object");
        assert_eq!(b.dice, "2d6");
        assert_eq!(b.scope, "global");
    }

    // A TOMBSTONE NEEDS NO SUPPRESSION RULE, which is the other reason
    // to expand rather than merge: the struck sword simply has no entry,
    // and there is no type-level leftover for the resolver to find.
    #[test]
    fn a_struck_move_is_absent_from_that_sword_only() {
        let mut gone = owned_by("zwerchhau", 2, "2d6", Some("sword-a"));
        gone.removed = true;
        let out = expand_for_objects(
            &[owned_by("zwerchhau", 0, "2d6", None), gone],
            &two_greatswords(),
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].object_id.as_deref(), Some("sword-b"));
    }

    // An addition belongs to its sword and to no other, even when the
    // other is the same kind of weapon.
    #[test]
    fn an_addition_does_not_spread_to_the_other_sword() {
        let out = expand_for_objects(
            &[owned_by("emberfall", 2, "3d8", Some("sword-a"))],
            &two_greatswords(),
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].key, "emberfall");
        assert_eq!(out[0].object_id.as_deref(), Some("sword-a"));
    }

    // A row for a weapon nobody is holding contributes nothing.
    #[test]
    fn another_weapons_moves_are_not_borrowed() {
        let out = expand_for_objects(
            &[{
                let mut t = owned_by("cleave", 0, "1d12", None);
                t.technique.item_key = "greataxe".into();
                t
            }],
            &two_greatswords(),
        );
        assert!(out.is_empty());
    }

    #[test]
    fn nothing_equipped_is_nothing_to_roll() {
        assert!(expand_for_objects(&[owned_by("zwerchhau", 0, "2d6", None)], &[]).is_empty());
    }

}
