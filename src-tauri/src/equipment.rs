//! Equipment: what a character is holding, and what that entitles them to.
//!
//! Split the way character.rs and narrative.rs are:
//!
//!   load_loadout()  talks to the network. Untestable without one.
//!   everything else is pure. Proficiency, attack modes, and the two
//!                   integrity rules are arithmetic and string matching,
//!                   so they get real tests against the seeded data.
//!
//! WHAT THIS DOES NOT DO
//!   It does not build an attack. No to-hit, no damage string, no crit
//!   threshold. Those need the live sheet and the technique table and
//!   belong in resolve_request, which is the next job. This module
//!   answers two narrower questions - what modes does this weapon offer,
//!   and is the holder proficient with it - and stops there. The
//!   AppSheet Attacks tab is the cautionary tale: it answered all of it
//!   at once, stored the answer, and went stale on every level-up.
//!
//! WHY EQUIPPED ONLY
//!   The loadout rides on the sheet, and the sheet is read on every
//!   roll. What a character is carrying in a backpack cannot change an
//!   attack, so it has no business in that read. A full inventory screen
//!   is a different query for a different page.
//!
//! THE TWO RULES THE SCHEMA CANNOT STATE
//!   One equipped armor, and every techniques.item_key resolving to a
//!   real item. Both are in here because both need to reach across
//!   tables in a way a partial unique index cannot - see 008's header.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::narrative::quoted;
use crate::supabase;

/* ============================ TYPES ============================ */

/// How a weapon is being used. The same three values as the
/// `techniques_mode_check` constraint, because a mode generated here has
/// to match a technique row stored there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Melee,
    Thrown,
    Ranged,
}

impl Mode {
    /// Not called until the attack key lands in resolve_request and has
    /// to match `techniques.mode`. Kept rather than deleted because it
    /// is the one place the enum and the check constraint are written
    /// down together, and the test below is what holds them equal.
    #[allow(dead_code)]
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Melee => "melee",
            Mode::Thrown => "thrown",
            Mode::Ranged => "ranged",
        }
    }
}

/// A catalogue row. Facts about a thing, never numbers derived from
/// whoever is holding it — see 008. Only the columns a rule reads are
/// carried; price, weight and art stay in the database until a screen
/// wants them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub key: String,
    pub name: String,
    pub kind: String,
    pub base_item: Option<String>,
    pub weapon_class: Option<String>,
    pub damage_number: Option<i64>,
    pub damage_denomination: Option<i64>,
    pub damage_types: Vec<String>,
    pub properties: Vec<String>,
    pub range_reach: Option<i64>,
    pub range_value: Option<i64>,
    pub range_long: Option<i64>,
    pub armor_category: Option<String>,
}

/// One thing a character has: the catalogue row plus their state for it,
/// with the derived answers already worked out.
///
/// Named for ownership rather than for being equipped, because the same
/// shape serves both reads — the sheet's loadout, which is equipped only,
/// and an inventory screen, which is everything.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owned {
    pub item: Item,
    pub quantity: i64,
    /// In hand or worn. Always true in a sheet loadout; meaningful in a
    /// full inventory read.
    pub equipped: bool,
    pub attuned: bool,
    /// TRI-STATE, straight off the column. See `is_proficient`.
    pub proficient_override: Option<bool>,
    pub uses_spent: i64,
    pub uses_max: Option<i64>,
    /// Derived, not stored. Kept on the struct so a UI and the attack
    /// resolver read the same answer rather than each deriving it.
    pub proficient: bool,
    /// Derived from weapon_class and `thr`. Empty for anything that is
    /// not a weapon.
    pub modes: Vec<Mode>,
}

/* ============================ RULES ============================ */

/// Split a weapon_class into the proficiency it is matched by and the
/// mode it is used in.
///
/// One function because the string encodes both facts and the check
/// constraint on the column admits exactly these four values: the
/// sim/mar PREFIX decides proficiency, the M/R SUFFIX decides melee or
/// ranged. Matching the four by name rather than slicing three
/// characters means a fifth value added later fails to resolve instead
/// of silently becoming "sim".
///
/// This is the asymmetry 008's header warns about. Armor is normalized
/// on the way into the database so it can be compared directly; weapons
/// are deliberately NOT, because the suffix carries a second fact that
/// normalizing away would destroy.
fn class_parts(weapon_class: &str) -> Option<(&'static str, Mode)> {
    match weapon_class {
        "simpleM" => Some(("sim", Mode::Melee)),
        "simpleR" => Some(("sim", Mode::Ranged)),
        "martialM" => Some(("mar", Mode::Melee)),
        "martialR" => Some(("mar", Mode::Ranged)),
        _ => None,
    }
}

/// The modes this item can be attacked with, in the order a UI should
/// offer them: how it is classified first, then what its properties add.
///
/// `thr` grants a second mode rather than replacing the first. A light
/// hammer is a melee weapon that can also be thrown, and it carries
/// separate technique lists for each - 7 and 6 - which is the whole
/// reason 007 split mode out of the weapon name.
pub fn modes(item: &Item) -> Vec<Mode> {
    if item.kind != "weapon" {
        return Vec::new();
    }
    let base = match item.weapon_class.as_deref().and_then(class_parts) {
        Some((_, m)) => m,
        // A weapon with no class is a data fault, not a melee weapon.
        // Offering no mode is the honest answer; check_item_keys is
        // where a fault like this gets reported.
        None => return Vec::new(),
    };

    let mut out = vec![base];
    if item.properties.iter().any(|p| p == "thr") && base != Mode::Thrown {
        out.push(Mode::Thrown);
    }
    out
}

/// Is the holder proficient with this item?
///
/// AN EXPLICIT ANSWER WINS. `proficient_override` is tri-state and the
/// nullability is the point: TRUE or FALSE came from the source (the
/// Mace ships proficient: 1), NULL means nobody has said, so derive it.
/// Treating NULL as false would silently strip proficiency from every
/// unflagged weapon, which is a wrong to-hit with no error attached.
///
/// DERIVING IT, for a weapon, is two tests and 008's column comment
/// documents both: `weapon_profs` holds whole classes (`sim`, `mar`) and
/// may also hold a bare baseItem (`mace`, `lighthammer`) granting that
/// one weapon. STATUS.md mentions only the class test; the bare-baseItem
/// grant is in the column comment and is implemented here, because a
/// proficiency that exists in the data and not in the engine is the same
/// silent wrong answer in the other direction.
///
/// For armor it is one direct comparison, which is only possible because
/// 008 normalized the export's light/medium/heavy/shield to the prof
/// spelling on the way in.
pub fn is_proficient(
    item: &Item,
    proficient_override: Option<bool>,
    weapon_profs: &[String],
    armor_profs: &[String],
) -> bool {
    if let Some(explicit) = proficient_override {
        return explicit;
    }

    match item.kind.as_str() {
        "weapon" => {
            if let Some(base) = item.base_item.as_deref() {
                if weapon_profs.iter().any(|p| p == base) {
                    return true;
                }
            }
            match item.weapon_class.as_deref().and_then(class_parts) {
                Some((prefix, _)) => weapon_profs.iter().any(|p| p == prefix),
                None => false,
            }
        }
        "armor" => match item.armor_category.as_deref() {
            Some(cat) => armor_profs.iter().any(|p| p == cat),
            None => false,
        },
        // Nothing else grants or needs proficiency.
        _ => false,
    }
}

/// At most one armor equipped at a time.
///
/// `character_items.equipped` is deliberately unconstrained - several
/// weapons may be held at once - and the one-armor rule cannot live in a
/// partial index because deciding it needs `items.kind`, which is in
/// another table. 008's column comment says so outright: a schema that
/// cannot state a rule should say so rather than pretend. This is where
/// the rule actually lives, so it has to be called on the way IN, not
/// just noticed on the way out.
///
/// Takes items rather than `Owned` because the rule reads nothing
/// else: the caller that matters is the equip path, which is checking a
/// state that does not exist yet and has no derived fields to offer.
pub fn check_one_armor(equipped: &[&Item]) -> Result<(), String> {
    let worn: Vec<&str> = equipped
        .iter()
        .filter(|i| i.kind == "armor")
        .map(|i| i.name.as_str())
        .collect();

    if worn.len() > 1 {
        return Err(format!(
            "only one armor may be equipped at a time; found {}: {}",
            worn.len(),
            worn.join(", ")
        ));
    }
    Ok(())
}

/// Every distinct technique item_key that resolves to no item.
///
/// 007 mints the key vocabulary and 008 has to match it exactly, with no
/// FK between them to notice when they stop agreeing - by design, since
/// a partial unique index cannot back a foreign key and an FK on the
/// surrogate id would bind a technique to either the global item or one
/// campaign's override. The cost of that design is this function.
///
/// Sorted and deduplicated so the answer is stable enough to assert on.
pub fn unresolved_item_keys(technique_keys: &[String], items: &[Item]) -> Vec<String> {
    let mut missing: Vec<String> = technique_keys
        .iter()
        .filter(|k| !items.iter().any(|i| &i.key == *k))
        .cloned()
        .collect();
    missing.sort();
    missing.dedup();
    missing
}

/// Every technique attached to a mode its own weapon cannot be used in,
/// as `item_key/mode`.
///
/// The companion fault to an unresolved key, and the stronger one. A key
/// typo makes a technique point at nothing; a mode mismatch makes it
/// point at a real weapon in a way that never comes up - the technique
/// exists, resolves, and is simply never offered. 007 split mode out of
/// the weapon name precisely because the light hammer carries two lists,
/// so the pair is the thing that has to agree, not the key alone.
///
/// Nothing in the schema can check this either: mode lives on techniques
/// and the properties that generate it live on items, with no FK between
/// them by design.
pub fn unreachable_technique_modes(pairs: &[(String, String)], items: &[Item]) -> Vec<String> {
    let mut bad: Vec<String> = pairs
        .iter()
        .filter(|(key, mode)| match items.iter().find(|i| &i.key == key) {
            // An unresolved key is the other function's fault to report,
            // not this one's. Reporting it twice helps nobody.
            None => false,
            Some(item) => !modes(item).iter().any(|m| m.as_str() == mode),
        })
        .map(|(key, mode)| format!("{}/{}", key, mode))
        .collect();
    bad.sort();
    bad.dedup();
    bad
}

/* ============================ LOADING ============================ */

fn as_str(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn as_opt_str(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn as_strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn item_from_row(r: &Value) -> Item {
    Item {
        key: as_str(r, "key"),
        name: as_str(r, "name"),
        kind: as_str(r, "kind"),
        base_item: as_opt_str(r, "base_item"),
        weapon_class: as_opt_str(r, "weapon_class"),
        damage_number: r.get("damage_number").and_then(|x| x.as_i64()),
        damage_denomination: r.get("damage_denomination").and_then(|x| x.as_i64()),
        damage_types: as_strings(r, "damage_types"),
        properties: as_strings(r, "properties"),
        range_reach: r.get("range_reach").and_then(|x| x.as_i64()),
        range_value: r.get("range_value").and_then(|x| x.as_i64()),
        range_long: r.get("range_long").and_then(|x| x.as_i64()),
        armor_category: as_opt_str(r, "armor_category"),
    }
}

const ITEM_COLUMNS: &str = "key,game_id,name,kind,base_item,weapon_class,damage_number,\
damage_denomination,damage_types,properties,range_reach,range_value,range_long,armor_category";

/// Global rows plus this game's overrides, collapsed so an override
/// replaces the global row sharing its key. Same two-pass shape as the
/// skill catalogue in character.rs, and for the same reason.
fn collapse_overrides(rows: &[Value]) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    for pass in [true, false] {
        for r in rows {
            let is_global = r.get("game_id").map(|g| g.is_null()).unwrap_or(true);
            if is_global != pass {
                continue;
            }
            let item = item_from_row(r);
            match out.iter_mut().find(|i| i.key == item.key) {
                Some(existing) => *existing = item,
                None => out.push(item),
            }
        }
    }
    out
}

/// What this character has, with proficiency and modes already derived.
///
/// `equipped_only` is the difference between the two callers. The sheet
/// passes true, because what is in a backpack cannot change an attack and
/// the sheet is read on every roll. An inventory screen passes false and
/// pays for the whole list, which is the different query for a different
/// page the module header promises.
///
/// Two requests, not one embedded join: an embedded join that a policy
/// trims looks like missing data instead of a permission problem. Same
/// reasoning as load_sheet's four.
pub fn load_loadout(
    token: &str,
    character_id: &str,
    game_id: &str,
    weapon_profs: &[String],
    armor_profs: &[String],
    equipped_only: bool,
) -> Result<Vec<Owned>, String> {
    let mut query: Vec<(&str, String)> = vec![
        (
            "select",
            "item_key,quantity,equipped,attuned,proficient_override,uses_spent,uses_max"
                .to_string(),
        ),
        ("character_id", format!("eq.{}", character_id)),
        ("order", "acquired_at.asc".to_string()),
    ];
    if equipped_only {
        query.push(("equipped", "is.true".to_string()));
    }
    let query: Vec<(&str, &str)> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();

    let owned = supabase::rest_get(token, "character_items", &query)?;
    let owned = owned.as_array().cloned().unwrap_or_default();
    // Nothing owned means no second request. An empty in.() list is not
    // valid PostgREST, so this guard is load-bearing, not tidiness.
    if owned.is_empty() {
        return Ok(Vec::new());
    }

    let keys: Vec<String> = owned
        .iter()
        .map(|r| quoted(&as_str(r, "item_key")))
        .collect();

    let item_rows = supabase::rest_get(
        token,
        "items",
        &[
            ("select", ITEM_COLUMNS),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("key", &format!("in.({})", keys.join(","))),
        ],
    )?;
    let catalogue = collapse_overrides(item_rows.as_array().unwrap_or(&Vec::new()));

    let mut out = Vec::new();
    for r in &owned {
        let key = as_str(r, "item_key");
        // An owned key with no catalogue row is the same fault
        // check_item_keys reports. Skip it rather than inventing an
        // item: a blank weapon on a sheet is worse than an absent one.
        let item = match catalogue.iter().find(|i| i.key == key) {
            Some(i) => i.clone(),
            None => continue,
        };

        let proficient_override = r.get("proficient_override").and_then(|x| x.as_bool());
        out.push(Owned {
            proficient: is_proficient(&item, proficient_override, weapon_profs, armor_profs),
            modes: modes(&item),
            quantity: r.get("quantity").and_then(|x| x.as_i64()).unwrap_or(1),
            equipped: r.get("equipped").and_then(|x| x.as_bool()).unwrap_or(false),
            attuned: r.get("attuned").and_then(|x| x.as_bool()).unwrap_or(false),
            proficient_override,
            uses_spent: r.get("uses_spent").and_then(|x| x.as_i64()).unwrap_or(0),
            uses_max: r.get("uses_max").and_then(|x| x.as_i64()),
            item,
        });
    }

    Ok(out)
}

/// One catalogue row by key, with this game's override applied, or None
/// if no such item is visible. Used by the equip path, which has to know
/// what kind of thing it is being asked to equip before it can tell
/// whether the one-armor rule applies.
pub fn load_item(token: &str, game_id: &str, key: &str) -> Result<Option<Item>, String> {
    let rows = supabase::rest_get(
        token,
        "items",
        &[
            ("select", ITEM_COLUMNS),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("key", &format!("in.({})", quoted(key))),
        ],
    )?;
    Ok(collapse_overrides(rows.as_array().unwrap_or(&Vec::new()))
        .into_iter()
        .next())
}

/// Both integrity faults the schema cannot catch, as a flat list of
/// descriptions. Empty is the passing answer.
///
/// The network half of `unresolved_item_keys` and
/// `unreachable_technique_modes`. Reads the full item columns rather
/// than just the key, because deciding reachability needs weapon_class
/// and properties.
pub fn check_item_keys(token: &str, game_id: &str) -> Result<Vec<String>, String> {
    let tech = supabase::rest_get(
        token,
        "techniques",
        &[
            ("select", "item_key,mode"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
        ],
    )?;
    let pairs: Vec<(String, String)> = tech
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|r| (as_str(r, "item_key"), as_str(r, "mode")))
                .collect()
        })
        .unwrap_or_default();

    let item_rows = supabase::rest_get(
        token,
        "items",
        &[
            ("select", ITEM_COLUMNS),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
        ],
    )?;
    let items = collapse_overrides(item_rows.as_array().unwrap_or(&Vec::new()));

    let keys: Vec<String> = pairs.iter().map(|(k, _)| k.clone()).collect();
    let mut faults: Vec<String> = unresolved_item_keys(&keys, &items)
        .into_iter()
        .map(|k| format!("unresolved item_key: {}", k))
        .collect();
    faults.extend(
        unreachable_technique_modes(&pairs, &items)
            .into_iter()
            .map(|p| format!("unreachable mode: {}", p)),
    );
    Ok(faults)
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    /// The four seeded rows the rules actually turn on, with the values
    /// 008 gives them. Keeping the fixture faithful to the seed means a
    /// test failing here means the RULE changed, not the fixture.
    fn mace() -> Item {
        Item {
            key: "mace_of_the_deep_song".into(),
            name: "Mace of the Deep Song".into(),
            kind: "weapon".into(),
            base_item: Some("mace".into()),
            weapon_class: Some("simpleM".into()),
            damage_number: Some(1),
            damage_denomination: Some(6),
            damage_types: vec!["bludgeoning".into()],
            properties: vec![],
            range_reach: Some(8),
            range_value: None,
            range_long: None,
            armor_category: None,
        }
    }

    fn light_hammer() -> Item {
        Item {
            key: "light_hammer".into(),
            name: "Light Hammer".into(),
            kind: "weapon".into(),
            base_item: Some("lighthammer".into()),
            weapon_class: Some("simpleM".into()),
            damage_number: Some(1),
            damage_denomination: Some(4),
            damage_types: vec!["bludgeoning".into()],
            properties: vec!["lgt".into(), "thr".into()],
            range_reach: None,
            range_value: Some(20),
            range_long: Some(60),
            armor_category: None,
        }
    }

    fn heavy_crossbow() -> Item {
        Item {
            key: "heavy_crossbow".into(),
            name: "Heavy Crossbow".into(),
            kind: "weapon".into(),
            base_item: Some("heavycrossbow".into()),
            weapon_class: Some("martialR".into()),
            damage_number: Some(1),
            damage_denomination: Some(10),
            damage_types: vec!["piercing".into()],
            properties: vec!["amm".into(), "hvy".into(), "lod".into(), "two".into()],
            range_reach: None,
            range_value: Some(100),
            range_long: Some(400),
            armor_category: None,
        }
    }

    fn scale_mail() -> Item {
        Item {
            key: "scale_mail".into(),
            name: "Scale Mail".into(),
            kind: "armor".into(),
            base_item: Some("scalemail".into()),
            weapon_class: None,
            damage_number: None,
            damage_denomination: None,
            damage_types: vec![],
            properties: vec!["stealthDisadvantage".into()],
            range_reach: None,
            range_value: None,
            range_long: None,
            armor_category: Some("med".into()),
        }
    }

    fn rations() -> Item {
        Item {
            key: "rations".into(),
            name: "Rations".into(),
            kind: "consumable".into(),
            base_item: None,
            weapon_class: None,
            damage_number: None,
            damage_denomination: None,
            damage_types: vec![],
            properties: vec![],
            range_reach: None,
            range_value: None,
            range_long: None,
            armor_category: None,
        }
    }

    /// Rodnar's, from the 008 backfill: simple weapons only, no martial.
    fn weapon_profs() -> Vec<String> {
        vec!["sim".into()]
    }
    fn armor_profs() -> Vec<String> {
        vec!["lgt".into(), "med".into(), "shl".into()]
    }

    /* ---------------- modes ---------------- */

    #[test]
    fn a_plain_melee_weapon_offers_one_mode() {
        assert_eq!(modes(&mace()), vec![Mode::Melee]);
    }

    #[test]
    fn thrown_adds_a_mode_rather_than_replacing_it() {
        // The reason 007 exists: one weapon, two technique lists, 7 and 6.
        assert_eq!(modes(&light_hammer()), vec![Mode::Melee, Mode::Thrown]);
    }

    #[test]
    fn the_class_suffix_decides_melee_or_ranged() {
        assert_eq!(modes(&heavy_crossbow()), vec![Mode::Ranged]);
    }

    #[test]
    fn nothing_that_is_not_a_weapon_has_modes() {
        assert!(modes(&scale_mail()).is_empty());
        assert!(modes(&rations()).is_empty());
    }

    #[test]
    fn a_weapon_with_no_class_offers_no_mode_rather_than_guessing() {
        let mut broken = mace();
        broken.weapon_class = None;
        assert!(modes(&broken).is_empty());

        let mut unknown = mace();
        unknown.weapon_class = Some("exoticM".into());
        assert!(modes(&unknown).is_empty());
    }

    #[test]
    fn mode_strings_match_the_techniques_check_constraint() {
        assert_eq!(Mode::Melee.as_str(), "melee");
        assert_eq!(Mode::Thrown.as_str(), "thrown");
        assert_eq!(Mode::Ranged.as_str(), "ranged");
    }

    /* ---------------- proficiency ---------------- */

    #[test]
    fn an_explicit_override_wins_over_everything() {
        // The Mace ships proficient: 1 and would also derive true. The
        // case that matters is the override saying NO to something the
        // derivation would allow.
        assert!(is_proficient(&mace(), Some(true), &weapon_profs(), &armor_profs()));
        assert!(!is_proficient(&light_hammer(), Some(false), &weapon_profs(), &armor_profs()));
        // And saying YES to something it would not.
        assert!(is_proficient(&heavy_crossbow(), Some(true), &weapon_profs(), &armor_profs()));
    }

    #[test]
    fn null_derives_rather_than_meaning_false() {
        // The whole reason the column is nullable. The Light Hammer is
        // not flagged in the source; it earns proficiency by matching sim.
        assert!(is_proficient(&light_hammer(), None, &weapon_profs(), &armor_profs()));
    }

    #[test]
    fn a_martial_weapon_is_not_proficient_on_simple_training() {
        // STATUS.md states this outcome directly: Rodnar holds only sim,
        // which is why his Heavy Crossbow to-hit is DEX alone.
        assert!(!is_proficient(&heavy_crossbow(), None, &weapon_profs(), &armor_profs()));
    }

    #[test]
    fn a_bare_base_item_grants_that_one_weapon() {
        // Documented on characters.weapon_profs: sim and mar are whole
        // classes, a bare baseItem grants one weapon. Someone trained on
        // heavycrossbow alone is proficient with it and still not martial.
        let profs = vec!["heavycrossbow".to_string()];
        assert!(is_proficient(&heavy_crossbow(), None, &profs, &armor_profs()));

        let other = Item { key: "other".into(), ..heavy_crossbow() };
        assert!(!is_proficient(
            &Item { base_item: Some("greatsword".into()), ..other },
            None,
            &profs,
            &armor_profs()
        ));
    }

    #[test]
    fn armor_matches_its_category_directly() {
        // Only possible because 008 normalized light/medium/heavy/shield
        // to lgt/med/hvy/shl on the way in. Unnormalized, 'medium' would
        // miss 'med' and every armor check would quietly read false.
        assert!(is_proficient(&scale_mail(), None, &weapon_profs(), &armor_profs()));

        let untrained = vec!["lgt".to_string()];
        assert!(!is_proficient(&scale_mail(), None, &weapon_profs(), &untrained));
    }

    #[test]
    fn the_unnormalized_spelling_would_have_failed() {
        // Guards the trap itself: if anything ever writes the export's
        // spelling into armor_category, this is what it looks like.
        let mut foundry_spelling = scale_mail();
        foundry_spelling.armor_category = Some("medium".into());
        assert!(!is_proficient(&foundry_spelling, None, &weapon_profs(), &armor_profs()));
    }

    #[test]
    fn ordinary_gear_is_neither_proficient_nor_armed() {
        assert!(!is_proficient(&rations(), None, &weapon_profs(), &armor_profs()));
    }

    /* ---------------- one equipped armor ---------------- */

    #[test]
    fn one_armor_and_many_weapons_is_fine() {
        // Several weapons equipped at once is explicitly allowed - that
        // is why the column has no constraint on it.
        let (m, h, x, s, r) = (mace(), light_hammer(), heavy_crossbow(), scale_mail(), rations());
        assert!(check_one_armor(&[&m, &h, &x, &s, &r]).is_ok());
    }

    #[test]
    fn no_armor_at_all_is_fine() {
        let m = mace();
        assert!(check_one_armor(&[&m]).is_ok());
        assert!(check_one_armor(&[]).is_ok());
    }

    #[test]
    fn two_equipped_armors_are_refused_and_both_are_named() {
        let first = scale_mail();
        let second = Item {
            key: "chain_mail".into(),
            name: "Chain Mail".into(),
            armor_category: Some("hvy".into()),
            ..scale_mail()
        };

        let err = check_one_armor(&[&first, &second]).unwrap_err();
        assert!(err.contains("Scale Mail"), "{}", err);
        assert!(err.contains("Chain Mail"), "{}", err);
    }

    /* ---------------- key integrity ---------------- */

    #[test]
    fn the_seeded_technique_keys_all_resolve() {
        // The three 007 mints, against the three 008 seeds.
        let keys = vec![
            "mace_of_the_deep_song".to_string(),
            "light_hammer".to_string(),
            "heavy_crossbow".to_string(),
        ];
        let items = vec![mace(), light_hammer(), heavy_crossbow(), scale_mail()];
        assert!(unresolved_item_keys(&keys, &items).is_empty());
    }

    #[test]
    fn a_typo_is_reported_once_and_sorted() {
        // What this function exists for: no FK will ever catch this, so
        // the repeated key must surface exactly once, not thirty-one times.
        let keys = vec![
            "light_hammer".to_string(),
            "mace_of_the_deepsong".to_string(),
            "mace_of_the_deepsong".to_string(),
            "abacus".to_string(),
        ];
        let items = vec![mace(), light_hammer()];
        assert_eq!(
            unresolved_item_keys(&keys, &items),
            vec!["abacus".to_string(), "mace_of_the_deepsong".to_string()]
        );
    }

    #[test]
    fn the_seeded_technique_modes_are_all_reachable() {
        // The live pairs, with their real counts: 8 + 7 + 6 + 10 = 31.
        // The light hammer carrying both melee and thrown is the whole
        // reason 007 split mode out of the weapon name.
        let pairs = vec![
            ("heavy_crossbow".to_string(), "ranged".to_string()),
            ("light_hammer".to_string(), "melee".to_string()),
            ("light_hammer".to_string(), "thrown".to_string()),
            ("mace_of_the_deep_song".to_string(), "melee".to_string()),
        ];
        let items = vec![mace(), light_hammer(), heavy_crossbow()];
        assert!(unreachable_technique_modes(&pairs, &items).is_empty());
    }

    #[test]
    fn a_mode_the_weapon_cannot_be_used_in_is_reported() {
        // Drop `thr` from the light hammer and its six thrown techniques
        // become unreachable - resolving perfectly, offered never. This
        // is the failure no FK and no constraint would notice.
        let thrown_removed = Item { properties: vec!["lgt".into()], ..light_hammer() };
        let pairs = vec![
            ("light_hammer".to_string(), "melee".to_string()),
            ("light_hammer".to_string(), "thrown".to_string()),
        ];
        assert_eq!(
            unreachable_technique_modes(&pairs, &[thrown_removed]),
            vec!["light_hammer/thrown".to_string()]
        );
    }

    #[test]
    fn an_unresolved_key_is_not_also_reported_as_an_unreachable_mode() {
        // One fault, one message. The key check already owns this row.
        let pairs = vec![("no_such_item".to_string(), "melee".to_string())];
        assert!(unreachable_technique_modes(&pairs, &[mace()]).is_empty());
        assert_eq!(
            unresolved_item_keys(&["no_such_item".to_string()], &[mace()]),
            vec!["no_such_item".to_string()]
        );
    }

    /* ---------------- override collapse ---------------- */

    #[test]
    fn a_game_override_replaces_the_global_item_sharing_its_key() {
        let rows = vec![
            serde_json::json!({
                "key": "mace_of_the_deep_song", "game_id": null,
                "name": "Mace of the Deep Song", "kind": "weapon",
                "weapon_class": "simpleM", "damage_denomination": 6
            }),
            serde_json::json!({
                "key": "mace_of_the_deep_song", "game_id": "g1",
                "name": "Mace of the Deep Song (this table)", "kind": "weapon",
                "weapon_class": "simpleM", "damage_denomination": 8
            }),
        ];
        let items = collapse_overrides(&rows);
        assert_eq!(items.len(), 1, "the override must replace, not sit beside");
        assert_eq!(items[0].damage_denomination, Some(8));
    }

    #[test]
    fn the_override_wins_whichever_order_the_rows_arrive_in() {
        let global = serde_json::json!({
            "key": "lamp", "game_id": null, "name": "Lamp", "kind": "equipment"
        });
        let override_row = serde_json::json!({
            "key": "lamp", "game_id": "g1", "name": "Everburning Lamp", "kind": "equipment"
        });

        let a = collapse_overrides(&[global.clone(), override_row.clone()]);
        let b = collapse_overrides(&[override_row, global]);
        assert_eq!(a[0].name, "Everburning Lamp");
        assert_eq!(b[0].name, "Everburning Lamp");
    }
}
