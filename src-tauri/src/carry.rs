//! The limits of one body: what it can attune to, hold, and bear.
//!
//! THREE RULES THE SCHEMA COULD ALWAYS EXPRESS AND NOTHING ENFORCED.
//! Every fact these need has been in the catalogue since 008 - the
//! `attuned` flag, the `two` property, the `weight` column - and each
//! sat there with no rule reading it. That is the quiet kind of gap: it
//! does not fail, it simply permits something the rulebook forbids.
//!
//! WHY NOT equipment.rs. That file answers what a character can DO with
//! what they are holding - proficiency, modes, one suit of armour - and
//! it is 862 production lines against an 800 ceiling. These are about
//! what a BODY can bear at all, which is the same question asked of the
//! person rather than of the gear.
//!
//! WHAT IS ENFORCED AND WHAT IS ONLY REPORTED
//!   attunement  refused. Three is a hard cap in 5e and nothing in the
//!               game bends it.
//!   hands       refused. Two hands is two hands.
//!   encumbrance REPORTED, not refused - see `Burden`. The base rule
//!               and the variant disagree about what happens past the
//!               line, and choosing between them is a campaign's call
//!               rather than an engine's.

use serde::{Deserialize, Serialize};

use crate::equipment::Item;

/* ============================ ATTUNEMENT ============================ */

/// How many things one creature may be attuned to at once.
///
/// Three, and it is one of the few numbers in 5e with no exceptions
/// worth modelling - no class feature, no item and no spell raises it
/// in the core rules.
pub const ATTUNEMENT_LIMIT: usize = 3;

/// Whether one more may be attuned.
///
/// Takes the names rather than a count so the refusal can say WHAT is
/// in the way. "You are attuned to three things already" invites the
/// question this answers.
pub fn check_attunement(already: &[String], incoming: &str) -> Result<(), String> {
    if already.iter().any(|a| a == incoming) {
        // Re-attuning to something already attuned is not a fourth.
        return Ok(());
    }
    if already.len() < ATTUNEMENT_LIMIT {
        return Ok(());
    }
    Err(format!(
        "attuned to {} already, which is the limit - break one first: {}",
        ATTUNEMENT_LIMIT,
        already.join(", ")
    ))
}

/* ============================ HANDS ============================ */

/// How many hands this item occupies while it is in use.
///
/// Armour is worn, not held, so it costs nothing - which is why this
/// cannot simply count equipped rows. A shield is one hand and that is
/// the whole reason a greatsword and a shield cannot both be up.
pub fn hands_for(item: &Item) -> i64 {
    if crate::equipment::is_shield(item) {
        return 1;
    }
    if item.kind != "weapon" {
        return 0;
    }
    if item.properties.iter().any(|p| p == "two") {
        2
    } else {
        1
    }
}

/// Two hands, and what is in them.
pub const HANDS: i64 = 2;

/// Whether everything here could be held at once.
///
/// THE GENERAL RULE, not just the greatsword-and-shield case it was
/// asked for. A hands budget covers that one and three other things
/// nobody wrote down: two greatswords, three weapons, and a
/// two-handed weapon in the same grip as anything else.
///
/// It IS a tightening. 008 left `equipped` unconstrained on purpose,
/// noting that several weapons may be held at once - which was right
/// while nothing modelled hands, and a licence rather than a rule.
pub fn check_hands(equipped: &[&Item]) -> Result<(), String> {
    let used: i64 = equipped.iter().map(|i| hands_for(i)).sum();
    if used <= HANDS {
        return Ok(());
    }
    let held: Vec<String> = equipped
        .iter()
        .filter(|i| hands_for(i) > 0)
        .map(|i| format!("{} ({})", i.name, hands_for(i)))
        .collect();
    Err(format!(
        "that needs {} hands and you have {} - {}",
        used,
        HANDS,
        held.join(", ")
    ))
}

/* ============================ WEIGHT ============================ */

/// How much a creature of this size and strength can carry, in pounds.
///
/// Fifteen pounds per point of Strength, and SIZE MULTIPLIES IT: a
/// Large creature carries twice what a Medium one does, a Tiny one
/// half. That is 5e as written and it is the reason an ogre can carry a
/// portcullis that a halfling of the same Strength could not.
pub fn carry_capacity(str_score: i64, size: Option<&str>) -> f64 {
    let base = (str_score.max(0) as f64) * 15.0;
    base * size_multiplier(size)
}

fn size_multiplier(size: Option<&str>) -> f64 {
    match size.unwrap_or("med") {
        "tiny" => 0.5,
        "sm" | "med" => 1.0,
        "lg" => 2.0,
        "huge" => 4.0,
        "grg" => 8.0,
        // An unrecognised size is treated as medium rather than
        // refused: this is a display number, and refusing to say how
        // much somebody can carry because their size is misspelled
        // helps nobody. `admits_size` is where a bad size is caught.
        _ => 1.0,
    }
}

/// How burdened a creature is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Burden {
    /// Carrying nothing that slows them.
    Free,
    /// Past five pounds per point of Strength. Variant rule: speed
    /// drops by 10.
    Encumbered,
    /// Past ten. Variant rule: speed drops by 20, and disadvantage on
    /// everything physical.
    Heavily,
    /// Past the carrying capacity itself. The BASE rule says this
    /// cannot happen - you simply cannot pick it up - and the variant
    /// has no tier beyond heavily encumbered, so both readings agree
    /// that something is wrong here.
    Overloaded,
}

impl Burden {
    pub fn as_str(self) -> &'static str {
        match self {
            Burden::Free => "unencumbered",
            Burden::Encumbered => "encumbered",
            Burden::Heavily => "heavily encumbered",
            Burden::Overloaded => "over capacity",
        }
    }
}

/// Which tier this weight falls in.
///
/// REPORTED RATHER THAN REFUSED, deliberately. The base rule forbids
/// carrying more than capacity outright; the variant lets you and slows
/// you down. They are different games, and an engine that picks one has
/// made a campaign's decision for it. What this does is say plainly
/// which side of each line somebody is, and leave the consequence to a
/// rule somebody chooses.
pub fn burden(carried: f64, str_score: i64, size: Option<&str>) -> Burden {
    let per_point = 15.0 * size_multiplier(size);
    let capacity = (str_score.max(0) as f64) * per_point;
    let light = (str_score.max(0) as f64) * 5.0 * size_multiplier(size);
    let heavy = (str_score.max(0) as f64) * 10.0 * size_multiplier(size);

    if carried > capacity {
        Burden::Overloaded
    } else if carried > heavy {
        Burden::Heavily
    } else if carried > light {
        Burden::Encumbered
    } else {
        Burden::Free
    }
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn item(key: &str, kind: &str, props: &[&str]) -> Item {
        Item {
            key: key.to_string(),
            name: key.to_string(),
            kind: kind.to_string(),
            base_item: None,
            weapon_class: if kind == "weapon" {
                Some("martialM".into())
            } else {
                None
            },
            damage_number: None,
            damage_denomination: None,
            damage_types: Vec::new(),
            properties: props.iter().map(|s| s.to_string()).collect(),
            range_reach: None,
            range_value: None,
            range_long: None,
            armor_category: if kind == "armor" { Some("hvy".into()) } else { None },
            base_ac: None,
            dex_cap: None,
            size: "med".into(),
            holds_size: None,
            weight: None,
            accepts: Vec::new(),
            capacity_slots: None,
        }
    }

    fn shield() -> Item {
        let mut s = item("shield", "armor", &[]);
        s.armor_category = Some("shl".into());
        s
    }

    /* ---------------- attunement ------------------------------------- */

    #[test]
    fn three_is_the_limit() {
        let three = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert!(check_attunement(&three, "d").is_err());
        assert!(check_attunement(&three[..2], "d").is_ok());
    }

    #[test]
    fn the_refusal_names_what_is_in_the_way() {
        let three = vec!["The Ember".to_string(), "b".to_string(), "c".to_string()];
        let e = check_attunement(&three, "d").unwrap_err();
        assert!(e.contains("The Ember"), "{}", e);
    }

    #[test]
    fn re_attuning_to_the_same_thing_is_not_a_fourth() {
        let three = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert!(check_attunement(&three, "b").is_ok());
    }

    /* ---------------- hands ------------------------------------------ */

    #[test]
    fn a_greatsword_and_a_shield_do_not_fit() {
        // The case this was asked for.
        let gs = item("greatsword", "weapon", &["two", "hvy"]);
        let e = check_hands(&[&gs, &shield()]).unwrap_err();
        assert!(e.contains("3 hands"), "{}", e);
    }

    #[test]
    fn a_longsword_and_a_shield_do() {
        let ls = item("longsword", "weapon", &["ver"]);
        assert!(check_hands(&[&ls, &shield()]).is_ok());
    }

    #[test]
    fn armour_is_worn_not_held() {
        // The reason this cannot just count equipped rows: a suit of
        // plate is equipped and occupies no hands.
        let plate = item("plate", "armor", &[]);
        let gs = item("greatsword", "weapon", &["two"]);
        assert_eq!(hands_for(&plate), 0);
        assert!(check_hands(&[&plate, &gs]).is_ok());
    }

    #[test]
    fn two_two_handers_is_four_hands() {
        let gs = item("greatsword", "weapon", &["two"]);
        let maul = item("maul", "weapon", &["two"]);
        assert!(check_hands(&[&gs, &maul]).is_err());
    }

    #[test]
    fn three_one_handers_is_one_too_many() {
        let a = item("mace", "weapon", &[]);
        let b = item("handaxe", "weapon", &["lgt"]);
        let c = item("dagger", "weapon", &["lgt", "fin"]);
        assert!(check_hands(&[&a, &b]).is_ok());
        assert!(check_hands(&[&a, &b, &c]).is_err());
    }

    #[test]
    fn nothing_in_hand_is_fine() {
        assert!(check_hands(&[]).is_ok());
    }

    /* ---------------- weight ----------------------------------------- */

    #[test]
    fn fifteen_pounds_a_point() {
        assert_eq!(carry_capacity(10, Some("med")), 150.0);
        assert_eq!(carry_capacity(16, Some("med")), 240.0);
    }

    #[test]
    fn size_multiplies_capacity() {
        // An ogre carries what a human of the same Strength cannot,
        // and a pixie carries half.
        assert_eq!(carry_capacity(10, Some("lg")), 300.0);
        assert_eq!(carry_capacity(10, Some("huge")), 600.0);
        assert_eq!(carry_capacity(10, Some("grg")), 1200.0);
        assert_eq!(carry_capacity(10, Some("tiny")), 75.0);
        // Small and medium are the same, which surprises people.
        assert_eq!(carry_capacity(10, Some("sm")), carry_capacity(10, Some("med")));
    }

    #[test]
    fn the_tiers_land_on_the_right_side_of_each_line() {
        // STR 10, medium: encumbered past 50, heavily past 100,
        // over capacity past 150.
        assert_eq!(burden(50.0, 10, Some("med")), Burden::Free);
        assert_eq!(burden(50.1, 10, Some("med")), Burden::Encumbered);
        assert_eq!(burden(100.0, 10, Some("med")), Burden::Encumbered);
        assert_eq!(burden(100.1, 10, Some("med")), Burden::Heavily);
        assert_eq!(burden(150.0, 10, Some("med")), Burden::Heavily);
        assert_eq!(burden(150.1, 10, Some("med")), Burden::Overloaded);
    }

    #[test]
    fn carrying_nothing_is_free_whatever_your_strength() {
        assert_eq!(burden(0.0, 1, Some("med")), Burden::Free);
        assert_eq!(burden(0.0, 20, Some("med")), Burden::Free);
    }

    #[test]
    fn a_strength_of_nothing_cannot_carry_anything() {
        // Guarded because the arithmetic would otherwise make every
        // weight "free" at capacity zero.
        assert_eq!(carry_capacity(0, Some("med")), 0.0);
        assert_eq!(burden(1.0, 0, Some("med")), Burden::Overloaded);
    }

    #[test]
    fn an_unknown_size_is_treated_as_medium() {
        // A display number should not refuse to exist over a typo.
        assert_eq!(carry_capacity(10, Some("enormous")), 150.0);
        assert_eq!(carry_capacity(10, None), 150.0);
    }
}
