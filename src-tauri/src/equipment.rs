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
    /// Armour only. For body armour this is the AC it sets; for a
    /// shield it is the bonus it adds. The two are not the same
    /// quantity and `armor_class` keeps them apart.
    pub base_ac: Option<i64>,
    /// The most DEX this armour lets through. None is no cap, which is
    /// light armour; Some(0) is heavy armour allowing none. The
    /// difference matters and NULL must not be read as zero.
    pub dex_cap: Option<i64>,
}

/// One thing a character has: the catalogue row plus their state for it,
/// with the derived answers already worked out.
///
/// Named for ownership rather than for being equipped, because the same
/// shape serves both reads — the sheet's loadout, which is equipped only,
/// and an inventory screen, which is everything.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owned {
    /// THE OBJECT'S OWN ID. Not the type's - this sword, which since 026
    /// is a row with an identity rather than a junction keyed by what
    /// kind of thing it is.
    pub id: String,
    /// What THIS one is called, when it has earned a name. None means
    /// call it by its type.
    pub name: Option<String>,
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
        .filter(|i| is_body_armor(i))
        .map(|i| i.name.as_str())
        .collect();

    if worn.len() > 1 {
        return Err(format!(
            "only one armor may be equipped at a time; found {}: {}",
            worn.len(),
            worn.join(", ")
        ));
    }

    // A SHIELD IS NOT A SECOND SUIT OF ARMOUR. Both carry kind 'armor'
    // because both are worn and both move AC, but mail AND a shield is
    // the ordinary case, not a conflict - counting them together
    // refused a legal loadout. Two separate limits, one each.
    let shields: Vec<&str> = equipped
        .iter()
        .filter(|i| is_shield(i))
        .map(|i| i.name.as_str())
        .collect();

    if shields.len() > 1 {
        return Err(format!(
            "only one shield may be equipped at a time; found {}: {}",
            shields.len(),
            shields.join(", ")
        ));
    }

    Ok(())
}

fn is_shield(item: &Item) -> bool {
    item.kind == "armor" && item.armor_category.as_deref() == Some("shl")
}

fn is_body_armor(item: &Item) -> bool {
    item.kind == "armor" && !is_shield(item)
}

/// How AC is arrived at. Mirrors `characters.ac_mode` and the export's
/// `attributes.ac.calc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcMode {
    /// Compute it from what is worn. The ordinary case.
    Default,
    /// Take the flat value and ignore the wardrobe - a monster, or an
    /// effect that sets AC outright.
    Flat,
}

impl AcMode {
    pub fn parse(s: &str) -> AcMode {
        match s.trim().to_lowercase().as_str() {
            "flat" => AcMode::Flat,
            // Anything else computes. An unrecognised mode computing is
            // a better failure than one returning a number nobody can
            // account for.
            _ => AcMode::Default,
        }
    }
}

/// What it takes to hit this character.
///
/// THE EXPORT'S `flat` FIELD IS NOT THE ANSWER. Rodnar's export reads
/// `{"calc": "default", "flat": 14}` and his AC is 15 - Scale Mail's 14
/// plus a DEX of +1 under a cap of 2. `flat` is consulted only when the
/// mode says so; reading it otherwise is wrong by one, permanently,
/// with nothing to show for it. 010's header has the whole story.
///
/// Body armour SETS the number, a shield ADDS to it, and both are kind
/// 'armor' - the difference is `armor_category = 'shl'`.
///
/// A `dex_cap` of None means no cap, which is light armour. Some(0) is
/// heavy armour admitting none. Reading None as zero would quietly cost
/// a rogue their entire modifier.
pub fn armor_class(dex_mod: i64, equipped: &[&Item], mode: AcMode, flat: Option<i64>) -> i64 {
    if mode == AcMode::Flat {
        // characters_flat_ac_has_a_value_check means a flat mode cannot
        // exist without its number, so the fallback is unreachable
        // through the app. Computing unarmoured beats panicking if some
        // other path ever reaches it.
        return flat.unwrap_or(10 + dex_mod);
    }

    let base = match equipped.iter().find(|i| is_body_armor(i)) {
        Some(armor) => {
            let allowed = match armor.dex_cap {
                None => dex_mod,
                Some(cap) => dex_mod.min(cap),
            };
            // An armour row with no base_ac is a data fault; 10 is the
            // unarmoured floor and keeps the number sane.
            armor.base_ac.unwrap_or(10) + allowed
        }
        None => 10 + dex_mod,
    };

    let shield: i64 = equipped
        .iter()
        .filter(|i| is_shield(i))
        .map(|i| i.base_ac.unwrap_or(0))
        .sum();

    base + shield
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
        base_ac: r.get("base_ac").and_then(|x| x.as_i64()),
        dex_cap: r.get("dex_cap").and_then(|x| x.as_i64()),
    }
}

pub(crate) const ITEM_COLUMNS: &str = "key,game_id,name,kind,base_item,weapon_class,damage_number,\
damage_denomination,damage_types,properties,range_reach,range_value,range_long,armor_category,base_ac,dex_cap";

/// Global rows plus this game's overrides, collapsed so an override
/// replaces the global row sharing its key. Same two-pass shape as the
/// skill catalogue in character.rs, and for the same reason.
pub(crate) fn collapse_overrides(rows: &[Value]) -> Vec<Item> {
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
            // No spaces: PostgREST reads this verbatim.
            "id,name,item_key,quantity,equipped,attuned,proficient_override,uses_spent,uses_max"
                .to_string(),
        ),
        ("character_id", format!("eq.{}", character_id)),
        ("order", "acquired_at.asc".to_string()),
    ];
    if equipped_only {
        query.push(("equipped", "is.true".to_string()));
    }
    let query: Vec<(&str, &str)> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();

    let owned = supabase::rest_get(token, "objects", &query)?;
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
            id: as_str(r, "id"),
            name: as_opt_str(r, "name"),
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

/// What a STATBLOCK carries, in the same shape a character's loadout
/// takes, so everything downstream is unaware which it got.
///
/// Two differences from `load_loadout`, both stated in 019:
///
///   The kit belongs to the statblock, not the instance. Every goblin
///   off one row carries the same axe, the same way every goblin has the
///   same AC. An instance that differs is a different statblock.
///
///   NULL PROFICIENCY READS AS TRUE. On a character it means "derive
///   from training"; a monster has no training model, and a goblin is
///   proficient with the axe its statblock hands it. The override column
///   exists for the exception, and FALSE still means false.
/// NOT CALLED SINCE 022. Instantiation copies a statblock's kit inside
/// `instantiate_npc`, in SQL, so the whole thing lands or none of it
/// does. Kept rather than deleted because a statblock VIEWER still
/// wants exactly this - reading a type's kit without making one - and
/// that is the next thing the DM side is asking for.
#[allow(dead_code)]
pub fn load_npc_loadout(
    token: &str,
    npc_key: &str,
    game_id: &str,
    equipped_only: bool,
) -> Result<Vec<Owned>, String> {
    let mut query: Vec<(&str, String)> = vec![
        (
            "select",
            "npc_key,item_key,game_id,quantity,equipped,proficient_override".to_string(),
        ),
        ("npc_key", format!("eq.{}", quoted(npc_key).trim_matches('"'))),
        (
            "or",
            format!("(game_id.is.null,game_id.eq.{})", game_id),
        ),
    ];
    if equipped_only {
        query.push(("equipped", "is.true".to_string()));
    }
    let query: Vec<(&str, &str)> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();

    let carried = supabase::rest_get(token, "npc_items", &query)?;
    let carried = carried.as_array().cloned().unwrap_or_default();
    if carried.is_empty() {
        return Ok(Vec::new());
    }

    // A game-scoped kit row shadows the global one for the same item,
    // the same precedence every reference table here uses.
    let mut best: Vec<Value> = Vec::new();
    for r in &carried {
        let key = as_str(r, "item_key");
        let scoped = r.get("game_id").map(|g| !g.is_null()).unwrap_or(false);
        match best.iter_mut().find(|b| as_str(b, "item_key") == key) {
            Some(existing) => {
                let existing_scoped =
                    existing.get("game_id").map(|g| !g.is_null()).unwrap_or(false);
                if scoped && !existing_scoped {
                    *existing = r.clone();
                }
            }
            None => best.push(r.clone()),
        }
    }

    let keys: Vec<String> = best.iter().map(|r| quoted(&as_str(r, "item_key"))).collect();
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
    for r in &best {
        let key = as_str(r, "item_key");
        let item = match catalogue.iter().find(|i| i.key == key) {
            Some(i) => i.clone(),
            // Same fault check_item_keys reports, and the same answer:
            // a blank weapon is worse than an absent one.
            None => continue,
        };
        let proficient_override = r.get("proficient_override").and_then(|x| x.as_bool());
        out.push(Owned {
            // A KIT ENTRY IS NOT AN OBJECT. This is the type side - a
            // pattern of what the statblock carries - and nothing here
            // exists until instantiate_npc makes one.
            id: String::new(),
            name: None,
            proficient: proficient_override.unwrap_or(true),
            modes: modes(&item),
            quantity: r.get("quantity").and_then(|x| x.as_i64()).unwrap_or(1),
            equipped: r.get("equipped").and_then(|x| x.as_bool()).unwrap_or(true),
            attuned: false,
            proficient_override,
            uses_spent: 0,
            uses_max: None,
            item,
        });
    }
    Ok(out)
}

/// The four armour proficiency codes, and nothing else is one.
///
/// A CLOSED SET, which is what makes it checkable. 008 normalizes the
/// export's light/medium/heavy/shield into these and says the two
/// vocabularies are reconciled there and nowhere else - so `light` in
/// this column is not a synonym, it is a value that matches no armour
/// and silently grants nothing.
pub const ARMOR_PROF_CODES: [&str; 4] = ["lgt", "med", "hvy", "shl"];

/// Parse a typed armour proficiency list, refusing anything outside the
/// vocabulary.
///
/// The refusal is the whole point. A wrong code here does not fail, it
/// under-grants - the creature simply turns out not to be proficient
/// with armour it should be wearing, which reads as the AC being wrong
/// rather than as the input being wrong.
pub fn parse_armor_profs(raw: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for part in raw.split(|c| c == ',' || c == ' ') {
        let t = part.trim().to_lowercase();
        if t.is_empty() {
            continue;
        }
        if !ARMOR_PROF_CODES.contains(&t.as_str()) {
            return Err(format!(
                "'{}' is not an armour proficiency - use {}",
                t,
                ARMOR_PROF_CODES.join(", ")
            ));
        }
        if !out.contains(&t) {
            out.push(t);
        }
    }
    Ok(out)
}

/// Parse a typed weapon proficiency list.
///
/// NOT a closed set: 008 allows `sim`, `mar`, or a bare baseItem
/// granting one weapon, so `longsword` is a legal entry and this cannot
/// reject an unknown token without rejecting that.
///
/// What it CAN catch is the near miss. `simple`, `martial` and
/// `simpleM` all look like an answer and all match nothing -
/// `is_proficient` compares against the sim/mar prefix of a weapon's
/// class, so `simpleM` in this column is a baseItem named simpleM. Those
/// four spellings are the ones a person actually types, and letting them
/// through is how a statblock ends up trained in nothing at all.
pub fn parse_weapon_profs(raw: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for part in raw.split(|c| c == ',' || c == ' ') {
        let t = part.trim().to_lowercase();
        if t.is_empty() {
            continue;
        }
        let meant = match t.as_str() {
            "simple" | "simplem" | "simpler" => Some("sim"),
            "martial" | "martialm" | "martialr" => Some("mar"),
            _ => None,
        };
        if let Some(code) = meant {
            return Err(format!("'{}' matches no weapon - did you mean '{}'?", t, code));
        }
        if !out.contains(&t) {
            out.push(t);
        }
    }
    Ok(out)
}

/// Every item this campaign can offer, global rows plus its own
/// overrides.
///
/// The same collapse `load_loadout` does on the way to a sheet, without
/// the loadout - an add-item picker needs the whole shelf, and the one
/// thing it must not do is offer both longswords when a campaign has
/// redefined one.
pub fn load_catalogue(token: &str, game_id: &str) -> Result<Vec<Item>, String> {
    let rows = supabase::rest_get(
        token,
        "items",
        &[
            ("select", ITEM_COLUMNS),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("order", "name.asc"),
        ],
    )?;
    Ok(collapse_overrides(rows.as_array().unwrap_or(&Vec::new())))
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
            base_ac: None,
            dex_cap: None,
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
            base_ac: None,
            dex_cap: None,
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
            base_ac: None,
            dex_cap: None,
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
            base_ac: Some(14),
            dex_cap: Some(2),
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
            base_ac: None,
            dex_cap: None,
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
            base_ac: Some(18),
            dex_cap: Some(0),
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

    /* ---------------- armour class ---------------------------------- */

    fn shield() -> Item {
        // Foundry stores a shield's BONUS in the same armor.value slot
        // body armour uses for its base. Same column, different meaning.
        Item {
            key: "shield".into(),
            name: "Shield".into(),
            kind: "armor".into(),
            base_item: Some("shield".into()),
            weapon_class: None,
            damage_number: None,
            damage_denomination: None,
            damage_types: vec![],
            properties: vec![],
            range_reach: None,
            range_value: None,
            range_long: None,
            armor_category: Some("shl".into()),
            base_ac: Some(2),
            dex_cap: None,
        }
    }

    fn leather() -> Item {
        // Light armour: no DEX cap at all, which is NOT a cap of zero.
        Item {
            key: "leather_armor".into(),
            name: "Leather Armor".into(),
            kind: "armor".into(),
            base_item: Some("leather".into()),
            weapon_class: None,
            damage_number: None,
            damage_denomination: None,
            damage_types: vec![],
            properties: vec![],
            range_reach: None,
            range_value: None,
            range_long: None,
            armor_category: Some("lgt".into()),
            base_ac: Some(11),
            dex_cap: None,
        }
    }

    fn plate() -> Item {
        Item {
            key: "plate_armor".into(),
            name: "Plate Armor".into(),
            kind: "armor".into(),
            base_item: Some("plate".into()),
            weapon_class: None,
            damage_number: None,
            damage_denomination: None,
            damage_types: vec![],
            properties: vec![],
            range_reach: None,
            range_value: None,
            range_long: None,
            armor_category: Some("hvy".into()),
            base_ac: Some(18),
            dex_cap: Some(0),
        }
    }

    #[test]
    fn rodnar_is_fifteen_not_the_fourteen_in_the_export() {
        // THE test. His export says {"calc":"default","flat":14} and the
        // spreadsheet has an AC_Flat column of 14. His AC is 15: Scale
        // Mail's 14 plus a DEX of +1 under a cap of 2. If this ever
        // returns 14, the flat field has been read when it should not
        // have been, and every attack on him is wrong by one.
        let mail = scale_mail();
        let ac = armor_class(1, &[&mail], AcMode::Default, Some(14));
        assert_eq!(ac, 15);
    }

    #[test]
    fn the_flat_field_is_ignored_unless_the_mode_says_otherwise() {
        let mail = scale_mail();
        assert_eq!(armor_class(1, &[&mail], AcMode::Default, Some(99)), 15);
        assert_eq!(armor_class(1, &[&mail], AcMode::Flat, Some(99)), 99);
    }

    #[test]
    fn unarmoured_is_ten_plus_dex() {
        assert_eq!(armor_class(3, &[], AcMode::Default, None), 13);
        assert_eq!(armor_class(0, &[], AcMode::Default, None), 10);
    }

    #[test]
    fn armour_codes_outside_the_vocabulary_are_refused() {
        // The exact typo 008 warns about: the export's own spelling,
        // which matches no armour and grants nothing.
        assert!(parse_armor_profs("light, shield").is_err());
        assert_eq!(
            parse_armor_profs("lgt, shl"),
            Ok(vec!["lgt".to_string(), "shl".to_string()])
        );
    }

    #[test]
    fn armour_parsing_is_forgiving_about_everything_but_the_code() {
        assert_eq!(
            parse_armor_profs("  LGT ,, med   hvy "),
            Ok(vec!["lgt".to_string(), "med".to_string(), "hvy".to_string()])
        );
        assert_eq!(parse_armor_profs("   "), Ok(Vec::new()));
        // Said twice is still said once.
        assert_eq!(parse_armor_profs("shl shl"), Ok(vec!["shl".to_string()]));
    }

    #[test]
    fn a_bare_base_item_is_a_legal_weapon_proficiency() {
        // 008 allows one weapon by name, so this cannot be a closed set.
        assert_eq!(
            parse_weapon_profs("sim, longsword"),
            Ok(vec!["sim".to_string(), "longsword".to_string()])
        );
    }

    #[test]
    fn the_near_misses_are_caught_by_name() {
        // Each of these looks like an answer and matches nothing:
        // is_proficient compares against the sim/mar prefix, so
        // "simpleM" here is a baseItem called simpleM.
        for typo in ["simple", "martial", "simpleM", "martialR"] {
            assert!(
                parse_weapon_profs(typo).is_err(),
                "{} should have been caught",
                typo
            );
        }
    }

    #[test]
    fn a_dex_cap_limits_but_does_not_replace() {
        let mail = scale_mail(); // base 14, cap 2
        assert_eq!(armor_class(0, &[&mail], AcMode::Default, None), 14);
        assert_eq!(armor_class(2, &[&mail], AcMode::Default, None), 16);
        assert_eq!(armor_class(5, &[&mail], AcMode::Default, None), 16);
    }

    #[test]
    fn no_cap_is_not_a_cap_of_zero() {
        // Light armour lets the whole modifier through. Reading NULL as
        // zero would quietly cost a rogue four points of AC.
        let l = leather();
        assert_eq!(armor_class(4, &[&l], AcMode::Default, None), 15);
        let p = plate();
        assert_eq!(armor_class(4, &[&p], AcMode::Default, None), 18);
    }

    #[test]
    fn a_dex_penalty_still_applies_under_a_cap() {
        // min(-1, 2) is -1. A cap is a ceiling, not a floor.
        let mail = scale_mail();
        assert_eq!(armor_class(-1, &[&mail], AcMode::Default, None), 13);
    }

    #[test]
    fn a_shield_adds_to_armour_rather_than_replacing_it() {
        let mail = scale_mail();
        let sh = shield();
        assert_eq!(armor_class(1, &[&mail, &sh], AcMode::Default, None), 17);
    }

    #[test]
    fn a_shield_alone_adds_to_the_unarmoured_floor() {
        let sh = shield();
        assert_eq!(armor_class(2, &[&sh], AcMode::Default, None), 14);
    }

    #[test]
    fn weapons_and_gear_do_not_move_ac() {
        let mace = mace();
        let r = rations();
        let mail = scale_mail();
        assert_eq!(armor_class(1, &[&mace, &r, &mail], AcMode::Default, None), 15);
    }

    /* ---------------- a shield is not a second armour ---------------- */

    #[test]
    fn armour_and_a_shield_together_are_legal() {
        // The bug this fixes: both carry kind 'armor', so counting them
        // together refused the most ordinary loadout in the game.
        let mail = scale_mail();
        let sh = shield();
        assert!(check_one_armor(&[&mail, &sh]).is_ok());
    }

    #[test]
    fn two_shields_are_refused_and_both_are_named() {
        let a = shield();
        let mut b = shield();
        b.key = "shield_2".into();
        b.name = "Buckler".into();
        let e = check_one_armor(&[&a, &b]).unwrap_err();
        assert!(e.contains("shield"), "wrong error: {}", e);
        assert!(e.contains("Buckler"), "second shield not named: {}", e);
    }

    #[test]
    fn two_body_armours_are_still_refused_with_a_shield_present() {
        let mail = scale_mail();
        let p = plate();
        let sh = shield();
        assert!(check_one_armor(&[&mail, &p, &sh]).is_err());
    }

    #[test]
    fn ac_mode_parses_the_column_vocabulary() {
        assert_eq!(AcMode::parse("flat"), AcMode::Flat);
        assert_eq!(AcMode::parse("default"), AcMode::Default);
        assert_eq!(AcMode::parse(" FLAT "), AcMode::Flat);
        // Anything unrecognised computes rather than inventing a number.
        assert_eq!(AcMode::parse("natural"), AcMode::Default);
    }
}
