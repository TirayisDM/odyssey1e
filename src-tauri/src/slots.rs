//! Where a thing is worn, and what each place will take.
//!
//! 084. `objects.equipped` was a boolean since 008 and said only THAT
//! something was in use, never where. A sheet cannot draw a right hand
//! and a left hand from a bare yes, and it cannot say that a greatsword
//! in one of them empties the other.
//!
//! ---------------------------------------------------------------------
//! THIS LADDER ORGANISES. IT DOES NOT INVENT RULES.
//! ---------------------------------------------------------------------
//!
//! The instruction was "just follow 5e rules here, otherwise we would be
//! drastically changing game flow", and it is the most important line in
//! this file. 5e has no slot system. What it has is a short list of
//! limits, and every one of them already existed in this codebase before
//! this module did:
//!
//!   two hands                 carry::check_hands, and a two-hander
//!                             takes both of them
//!   one suit of armour        equipment::check_one_armor
//!   three attuned items       carry::check_attunement
//!
//! The DMG adds one headwear, one cloak and one pair of each of
//! footwear, gloves and bracers - and pointedly does NOT limit rings.
//! Rings are bounded by attunement and by having fingers, which is why
//! `holds` is None for them here rather than one or two. Capping them
//! would be a house rule wearing 5e's clothes.
//!
//! THE ONE HOUSE RULE IS THE HIP, and it is Dave's, stated as his: six
//! places at the belt, a coin purse counting as one of them. 5e has
//! nothing to say about how much hangs off a character, so this is an
//! addition rather than a contradiction.
//!
//! ---------------------------------------------------------------------
//! WHAT DECIDES WHETHER A SLOT WILL TAKE A THING
//! ---------------------------------------------------------------------
//!
//! Three different kinds of answer, deliberately:
//!
//!   BY SIZE      the hip takes tiny and small - a sheath, a pouch, a
//!                potion, a wand, a rod, a dagger. One rule covers the
//!                whole list rather than a hardcoded set of keys that
//!                every new item would have to be added to.
//!   BY KIND      a backpack or a chest takes a container; the body
//!                takes armour that is not a shield.
//!   BY THE ITEM  rings, amulets and headwear say so themselves, in
//!                `items.worn_slot`. A ring is not recognisable from
//!                its size or its kind, and a Rust list of ring keys
//!                would need editing every time somebody writes one.
//!
//! A HAND TAKES ANYTHING. That is not laziness - a character can pick
//! up a chest, a lantern or somebody else's arm. What a hand enforces
//! is how MANY, which is `carry::hands_for`'s business and was already
//! written.

use crate::equipment::Item;

/// One place something can be worn or held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub key: &'static str,
    pub name: &'static str,
    /// How many things fit. None is unlimited - see the note on rings.
    pub holds: Option<i64>,
}

/// Every slot, in the order a sheet reads them: what is in your hands
/// first, then what is on your belt, then what you are carrying, then
/// what you are wearing.
pub const LADDER: &[Slot] = &[
    Slot { key: "right_hand", name: "Right hand", holds: Some(1) },
    Slot { key: "left_hand", name: "Left hand", holds: Some(1) },
    Slot { key: "hip", name: "Hip", holds: Some(HIP_PLACES) },
    Slot { key: "backpack", name: "Backpack", holds: Some(1) },
    Slot { key: "chest", name: "Chest", holds: Some(1) },
    Slot { key: "ring_right", name: "Rings, right hand", holds: None },
    Slot { key: "ring_left", name: "Rings, left hand", holds: None },
    Slot { key: "amulet", name: "Amulet", holds: Some(1) },
    Slot { key: "head", name: "Helm or hat", holds: Some(1) },
    Slot { key: "body", name: "Armour", holds: Some(1) },
];

/// How much hangs off a belt. Dave's rule, not the book's.
pub const HIP_PLACES: i64 = 6;

/// The two slots that are hands, which several rules care about
/// together.
pub const HANDS: [&str; 2] = ["right_hand", "left_hand"];

/// A slot by key.
pub fn of(key: &str) -> Option<&'static Slot> {
    LADDER.iter().find(|s| s.key == key)
}

/// Whether this is one of the hands.
pub fn is_hand(key: &str) -> bool {
    HANDS.contains(&key)
}

/// Whether a slot will take this item at all.
///
/// SIZE, KIND OR THE ITEM'S OWN SAY-SO - see the module header for why
/// each slot uses the one it does. An unknown slot admits nothing,
/// which is how a typo in a key fails loudly rather than placing a
/// sword somewhere that does not exist.
pub fn admits(slot: &str, item: &Item) -> bool {
    match slot {
        // A hand takes anything you HOLD. How many is carry::hands_for.
        //
        // NOT SOMETHING THAT IS WORN. A ring in the right hand was
        // counted against the one thing a hand holds, so putting one on
        // reported "Right hand holds 1 and that is 2: Battleaxe, Ring"
        // - an error about the axe, for a ring, naming a slot the
        // player was not thinking about. Anything with a `worn_slot`
        // has a place of its own and belongs in it; refusing it here
        // means the message says so and points at that place.
        "right_hand" | "left_hand" => worn_as(item).is_none(),
        // Tiny and small, which is the sheaths, pouches, potions,
        // wands, rods and daggers of the instruction, without naming
        // any of them.
        "hip" => matches!(item.size.as_str(), "tiny" | "sm"),
        "backpack" | "chest" => item.kind == "container",
        "ring_right" | "ring_left" => worn_as(item) == Some("ring"),
        "amulet" => worn_as(item) == Some("amulet"),
        "head" => worn_as(item) == Some("head"),
        // Armour, and not a shield - a shield is held, which is why
        // `is_shield` has been a separate question since 008.
        "body" => item.kind == "armor" && !crate::equipment::is_shield(item),
        _ => false,
    }
}

/// What an item says it is worn as, if anything.
fn worn_as(item: &Item) -> Option<&str> {
    item.worn_slot.as_deref().filter(|s| !s.is_empty())
}

/// Where a thing ACTUALLY goes when somebody picks `slot` for it.
///
/// 090. "Ring does not go in the Right hand" is a true sentence and a
/// useless one. A player putting a ring on picks the hand they mean,
/// because that is how a hand works - and the ladder's answer was to
/// refuse them and name a slot further down the list they had not
/// looked at.
///
/// So a worn thing dropped into a hand is routed to the place it is
/// worn, and a ring takes the SIDE from the hand that was picked. The
/// rule is not loosened: nothing ends up somewhere `admits` would
/// reject, and a ring still never occupies a hand.
///
/// ONLY FROM A HAND. Picking "Armour" for a helm is a mistake worth
/// reporting rather than quietly correcting - the hands are the one
/// place somebody reaches for by reflex, which is why they are the one
/// place this forgives.
pub fn route(slot: &str, item: &Item) -> String {
    let Some(worn) = worn_as(item) else {
        return slot.to_string();
    };
    if !is_hand(slot) {
        return slot.to_string();
    }
    match worn {
        "ring" if slot == "left_hand" => "ring_left".to_string(),
        "ring" => "ring_right".to_string(),
        // `amulet` and `head` are slot keys already.
        other => other.to_string(),
    }
}

/// One thing in one place.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed<'a> {
    pub slot: String,
    pub item: &'a Item,
}

/// Whether a whole loadout stands up.
///
/// EVERY SLOT AT ONCE, because the interesting failures are between
/// slots rather than inside one: a greatsword in the right hand and a
/// shield in the left is two legal placements and an illegal pair.
///
/// THE HANDS ARE CHECKED BY carry::check_hands, which already knew the
/// rule before slots existed and still owns it. What this adds is the
/// placement half - that the right hand holds one thing - and then
/// hands the pair to the budget that was already written.
pub fn check(placed: &[Placed]) -> Result<(), String> {
    for slot in LADDER {
        let here: Vec<&Placed> = placed.iter().filter(|p| p.slot == slot.key).collect();
        if let Some(limit) = slot.holds {
            if here.len() as i64 > limit {
                return Err(match slot.key {
                    "hip" => format!(
                        "that is {} things at the belt and there is room for {}",
                        here.len(),
                        limit
                    ),
                    _ => format!(
                        "{} holds {} and that is {}: {}",
                        slot.name,
                        limit,
                        here.len(),
                        here.iter().map(|p| p.item.name.as_str()).collect::<Vec<_>>().join(", ")
                    ),
                });
            }
        }
        for p in &here {
            if !admits(slot.key, p.item) {
                return Err(format!("{} does not go in the {}", p.item.name, slot.name));
            }
        }
    }

    // THE TWO-HANDED RULE, which is why the hands cannot be checked one
    // at a time. A greatsword needs both, so the other hand has to be
    // empty - and saying so is `check_hands` summing what is held.
    let in_hand: Vec<&Item> = placed
        .iter()
        .filter(|p| is_hand(&p.slot))
        .map(|p| p.item)
        .collect();
    crate::carry::check_hands(&in_hand)?;

    Ok(())
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, kind: &str, size: &str) -> Item {
        let mut i = crate::equipment::blank(name, kind);
        i.size = size.to_string();
        i
    }

    fn weapon(name: &str, props: &[&str]) -> Item {
        let mut i = item(name, "weapon", "med");
        i.properties = props.iter().map(|p| p.to_string()).collect();
        i
    }

    fn worn(name: &str, slot: &str) -> Item {
        let mut i = item(name, "equipment", "tiny");
        i.worn_slot = Some(slot.to_string());
        i
    }

    fn placed<'a>(pairs: &[(&str, &'a Item)]) -> Vec<Placed<'a>> {
        pairs.iter().map(|(s, i)| Placed { slot: s.to_string(), item: i }).collect()
    }

    /* ---------- the ladder ---------- */

    #[test]
    fn the_order_is_hands_belt_carried_worn() {
        let keys: Vec<&str> = LADDER.iter().map(|s| s.key).collect();
        assert_eq!(
            keys,
            vec!["right_hand", "left_hand", "hip", "backpack", "chest",
                 "ring_right", "ring_left", "amulet", "head", "body"]
        );
    }

    #[test]
    fn a_slot_can_be_found_by_key_and_an_invented_one_cannot() {
        assert_eq!(of("hip").map(|s| s.name), Some("Hip"));
        assert!(of("pocket").is_none());
    }

    /* ---------- what goes where ---------- */

    #[test]
    fn a_hand_takes_anything_you_hold() {
        // Not laziness: a character can pick up a chest, a lantern or
        // a suit of armour. How MANY is the hands budget.
        assert!(admits("right_hand", &item("Treasure Chest", "container", "lg")));
        assert!(admits("left_hand", &weapon("Greatsword", &["two"])));
    }

    #[test]
    fn a_ring_does_not_take_up_a_hand() {
        // The bug, by name. A ring in the right hand was counted
        // against the one thing a hand holds, so putting one on read
        // "Right hand holds 1 and that is 2: Battleaxe, Ring" - an
        // error about an axe, for a ring, naming a slot nobody was
        // thinking about. Rings have fingers of their own.
        let ring = worn("Ring of Protection", "ring");
        assert!(!admits("right_hand", &ring));
        assert!(!admits("left_hand", &ring));
        assert!(admits("ring_right", &ring));
    }

    /* ---------- routing a worn thing out of a hand ---------- */

    #[test]
    fn a_ring_picked_for_a_hand_goes_on_that_hands_fingers() {
        // Dave's second report. "Ring does not go in the Right hand"
        // was true and useless: a player putting a ring on picks the
        // hand they mean.
        let ring = worn("Ring of Protection", "ring");
        assert_eq!(route("right_hand", &ring), "ring_right");
        assert_eq!(route("left_hand", &ring), "ring_left");
    }

    #[test]
    fn routing_lands_somewhere_the_rule_allows() {
        // The guarantee that makes this forgiveness and not a hole.
        let ring = worn("Ring of Protection", "ring");
        for hand in ["right_hand", "left_hand"] {
            assert!(admits(&route(hand, &ring), &ring));
        }
    }

    #[test]
    fn an_amulet_or_helm_in_a_hand_goes_where_it_is_worn() {
        let amulet = worn("Amulet of Health", "amulet");
        let helm = worn("Helm", "head");
        assert_eq!(route("right_hand", &amulet), "amulet");
        assert_eq!(route("left_hand", &helm), "head");
    }

    #[test]
    fn a_held_thing_is_never_rerouted() {
        let sword = weapon("Longsword", &[]);
        assert_eq!(route("right_hand", &sword), "right_hand");
        assert_eq!(route("hip", &sword), "hip");
    }

    #[test]
    fn only_a_hand_forgives() {
        // Picking Armour for a helm is a mistake worth reporting, not
        // one worth quietly correcting.
        let helm = worn("Helm", "head");
        assert_eq!(route("body", &helm), "body");
        assert!(!admits("body", &helm), "and it is still refused");
    }

    #[test]
    fn nor_does_an_amulet_or_a_helm() {
        assert!(!admits("right_hand", &worn("Amulet of Health", "amulet")));
        assert!(!admits("left_hand", &worn("Helm", "head")));
    }

    #[test]
    fn a_hand_full_of_rings_still_leaves_the_hand_free() {
        // The whole point: ten rings and a greatsword is a legal
        // character, because the rings are not in the hands at all.
        let ring = worn("Ring of Protection", "ring");
        let great = weapon("Greatsword", &["two"]);
        let mut p = placed(&[("right_hand", &great)]);
        for _ in 0..10 {
            p.push(Placed { slot: "ring_left".into(), item: &ring });
        }
        assert!(check(&p).is_ok());
    }

    #[test]
    fn the_hip_takes_tiny_and_small_and_nothing_larger() {
        assert!(admits("hip", &item("Dagger", "weapon", "tiny")));
        assert!(admits("hip", &item("Sheath", "container", "sm")));
        assert!(admits("hip", &item("Potion", "consumable", "tiny")));
        assert!(!admits("hip", &item("Greatsword", "weapon", "lg")));
        assert!(!admits("hip", &item("Backpack", "container", "med")));
    }

    #[test]
    fn a_ring_says_it_is_a_ring_rather_than_being_guessed_at() {
        let ring = worn("Ring of Protection", "ring");
        assert!(admits("ring_left", &ring));
        assert!(admits("ring_right", &ring));
        // A tiny piece of equipment that has not said so is not a ring.
        assert!(!admits("ring_left", &item("Tinderbox", "equipment", "tiny")));
    }

    #[test]
    fn an_amulet_and_a_helm_keep_to_their_own_slots() {
        let amulet = worn("Amulet of Health", "amulet");
        let helm = worn("Helm", "head");
        assert!(admits("amulet", &amulet));
        assert!(!admits("head", &amulet));
        assert!(admits("head", &helm));
        assert!(!admits("amulet", &helm));
    }

    #[test]
    fn armour_goes_on_the_body_and_a_shield_does_not() {
        let mut plate = item("Plate Armor", "armor", "lg");
        plate.armor_category = Some("hvy".into());
        assert!(admits("body", &plate));

        let mut shield = item("Shield", "armor", "med");
        shield.armor_category = Some("shl".into());
        assert!(!admits("body", &shield), "a shield is held, not worn");
        assert!(admits("left_hand", &shield));
    }

    #[test]
    fn a_slot_nobody_has_heard_of_takes_nothing() {
        assert!(!admits("pocket", &item("Dagger", "weapon", "tiny")));
    }

    /* ---------- the whole loadout ---------- */

    #[test]
    fn a_sword_and_a_shield_is_fine() {
        let sword = weapon("Longsword", &[]);
        let mut shield = item("Shield", "armor", "med");
        shield.armor_category = Some("shl".into());
        let p = placed(&[("right_hand", &sword), ("left_hand", &shield)]);
        assert!(check(&p).is_ok());
    }

    #[test]
    fn a_two_hander_wants_the_other_hand_empty() {
        let great = weapon("Greatsword", &["two"]);
        assert!(check(&placed(&[("right_hand", &great)])).is_ok());

        let mut shield = item("Shield", "armor", "med");
        shield.armor_category = Some("shl".into());
        let both = placed(&[("right_hand", &great), ("left_hand", &shield)]);
        assert!(check(&both).is_err(), "a greatsword and a shield is three hands");
    }

    #[test]
    fn one_hand_holds_one_thing() {
        let a = weapon("Dagger", &["lgt"]);
        let b = weapon("Shortsword", &["lgt"]);
        let p = placed(&[("right_hand", &a), ("right_hand", &b)]);
        let err = check(&p).unwrap_err();
        assert!(err.contains("Right hand"), "unhelpful: {}", err);
    }

    #[test]
    fn two_light_weapons_one_in_each_hand_is_fine() {
        let a = weapon("Dagger", &["lgt"]);
        let b = weapon("Shortsword", &["lgt"]);
        assert!(check(&placed(&[("right_hand", &a), ("left_hand", &b)])).is_ok());
    }

    #[test]
    fn six_things_hang_off_a_belt_and_a_seventh_does_not() {
        let pouch = item("Hip Pouch", "container", "sm");
        let mut p: Vec<Placed> = Vec::new();
        for _ in 0..HIP_PLACES {
            p.push(Placed { slot: "hip".into(), item: &pouch });
        }
        assert!(check(&p).is_ok());

        p.push(Placed { slot: "hip".into(), item: &pouch });
        let err = check(&p).unwrap_err();
        assert!(err.contains("belt"), "unhelpful: {}", err);
    }

    #[test]
    fn a_coin_purse_is_one_of_the_six() {
        // Stated in the instruction, and it falls out of the rule
        // rather than needing a case: a purse is small, so it takes a
        // place like anything else small.
        let purse = item("Coin Purse", "container", "tiny");
        assert!(admits("hip", &purse));
    }

    #[test]
    fn rings_do_not_run_out() {
        // 5e caps attunement at three and says nothing about ring
        // count - see the module header. Capping them here would be a
        // house rule wearing the book's clothes.
        let ring = worn("Ring of Protection", "ring");
        let p: Vec<Placed> = (0..8)
            .map(|_| Placed { slot: "ring_left".into(), item: &ring })
            .collect();
        assert!(check(&p).is_ok());
    }

    #[test]
    fn one_suit_of_armour() {
        let mut plate = item("Plate Armor", "armor", "lg");
        plate.armor_category = Some("hvy".into());
        let mut leather = item("Leather Armor", "armor", "med");
        leather.armor_category = Some("lgt".into());
        let p = placed(&[("body", &plate), ("body", &leather)]);
        assert!(check(&p).is_err());
    }

    #[test]
    fn one_hat() {
        let helm = worn("Helm", "head");
        let hat = worn("Wide Hat", "head");
        assert!(check(&placed(&[("head", &helm), ("head", &hat)])).is_err());
    }

    #[test]
    fn a_thing_in_the_wrong_place_says_which_place() {
        let great = weapon("Greatsword", &["two"]);
        let err = check(&placed(&[("hip", &great)])).unwrap_err();
        assert!(err.contains("Greatsword") && err.contains("Hip"), "unhelpful: {}", err);
    }

    #[test]
    fn an_empty_loadout_is_a_fine_loadout() {
        assert!(check(&[]).is_ok());
    }
}
