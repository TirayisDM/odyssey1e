//! HOW BIG SOMETHING IS, and everything that follows from it.
//!
//! THE CAMPAIGN RUNS FROM TWO FEET TO TWENTY-FIVE. A rodent people at
//! 2', the Unt'gar at 4.5', the Felligar at 5.5', the Unt'garoth near
//! 8', the Jotun at 18' and the Imiear at nearly 25'. Size is not a
//! footnote in a world shaped like that - it is the difference between
//! two peoples in the same room, and it has to mean the same thing
//! everywhere it is read.
//!
//! IT DID NOT. The ladder was written out three times before this file
//! existed: `carry::size_multiplier` knew the capacities, `vitality`
//! knew the dice and the ordering, and `species::bump_size` carried its
//! own array of the six words. vitality.rs said so about itself -
//!
//! > a ladder in one file with a die table in another is two copies of
//! > one vocabulary waiting to disagree about whether "grg" exists
//!
//! - and then a third copy appeared anyway, because there was nowhere
//! for the second one to move to. This is that place. It is the same
//! lesson as `supabase::numeric`, which cost three bugs to learn: one
//! fact, one home, and nothing else holding an opinion.
//!
//! HEIGHT IS THE FACT; THE CATEGORY IS A CONSEQUENCE. A species states
//! how tall its people are, and `for_height` says which rung that lands
//! on. Storing the category alone would throw away the difference
//! between an 18' Jotun and a 25' Imiear, and that difference is real
//! even where the rules round it away.
//!
//! WHERE THE ROUNDING BITES, and it should be said plainly rather than
//! discovered: the Jotun and the Imiear are BOTH Huge, because 5e's
//! Huge runs from 16 to 32 feet. Seven feet of difference and not one
//! number between them changes. If that is wrong for this campaign the
//! fix is to make the continuous facts - reach, space, carrying - scale
//! off `height_ft` and keep the category only for the rules that need
//! discrete rungs, like grappling and squeezing. The table below is
//! built so that change is possible; it has not been made, because it
//! is a design decision rather than a defect.

/* ============================ THE LADDER ============================ */

/// One rung, and everything the rules hang on it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizeClass {
    /// The stored word. 010's vocabulary, unchanged.
    pub key: &'static str,
    /// For a screen.
    pub name: &'static str,
    /// ORDER AND NOTHING ELSE - never subtracted, scaled or printed.
    /// A gap between them would mean something, so there is none.
    pub rank: i64,
    /// How much floor this creature stands on, in feet.
    pub space_ft: f64,
    /// HOUSE RULE. 5e does NOT derive reach from size - a Large
    /// creature has 5 feet of reach unless its statblock says
    /// otherwise. In a world spanning 2 to 25 feet that is untenable,
    /// so reach scales here and STATUS.md records it among the house
    /// rules rather than letting it pass for 5e.
    pub reach_ft: i64,
    /// What carrying capacity is multiplied by.
    pub carry_multiplier: f64,
    /// The die a creature of this size rolls for hit points. The
    /// MONSTER rule - a player character's die comes from their class
    /// (055), which is the distinction vitality.rs draws at length.
    pub hit_die: i64,
    /// Feet, inclusive lower bound.
    pub min_height_ft: f64,
    /// Feet, exclusive upper bound. None on the top rung.
    pub max_height_ft: Option<f64>,
}

/// The six, smallest first. Index is rank.
pub const LADDER: [SizeClass; 6] = [
    SizeClass { key: "tiny", name: "Tiny",       rank: 0, space_ft:  2.5, reach_ft:  5,
                carry_multiplier: 0.5, hit_die:  4, min_height_ft:  0.0, max_height_ft: Some(2.0) },
    SizeClass { key: "sm",   name: "Small",      rank: 1, space_ft:  5.0, reach_ft:  5,
                carry_multiplier: 1.0, hit_die:  6, min_height_ft:  2.0, max_height_ft: Some(4.0) },
    SizeClass { key: "med",  name: "Medium",     rank: 2, space_ft:  5.0, reach_ft:  5,
                carry_multiplier: 1.0, hit_die:  8, min_height_ft:  4.0, max_height_ft: Some(8.0) },
    SizeClass { key: "lg",   name: "Large",      rank: 3, space_ft: 10.0, reach_ft: 10,
                carry_multiplier: 2.0, hit_die: 10, min_height_ft:  8.0, max_height_ft: Some(16.0) },
    SizeClass { key: "huge", name: "Huge",       rank: 4, space_ft: 15.0, reach_ft: 15,
                carry_multiplier: 4.0, hit_die: 12, min_height_ft: 16.0, max_height_ft: Some(32.0) },
    SizeClass { key: "grg",  name: "Gargantuan", rank: 5, space_ft: 20.0, reach_ft: 20,
                carry_multiplier: 8.0, hit_die: 20, min_height_ft: 32.0, max_height_ft: None },
];

/// One rung by its stored word, or None for anything outside the six.
///
/// NONE IS A REAL ANSWER and callers differ on what to do with it: a
/// statblock written before 029 has no size and cannot have hit points
/// derived, which is reported; a carrying capacity treats an
/// unrecognised word as Medium, because refusing to say how much
/// somebody can lift over a typo helps nobody.
pub fn of(key: &str) -> Option<&'static SizeClass> {
    LADDER.iter().find(|s| s.key == key)
}

/// The rung a creature of this height stands on.
///
/// THE BOUNDARIES ARE REAL AND THEY BITE. 5e's Medium runs 4 to 8 feet
/// and Large starts AT 8, so an Unt'garoth at exactly 8' is Large and
/// one at 7'11" is Medium. Their own document says Large, which is why
/// a species STATES its category as well as its height: this function
/// checks that claim rather than replacing it.
pub fn for_height(feet: f64) -> &'static SizeClass {
    for s in LADDER.iter() {
        match s.max_height_ft {
            Some(max) if feet < max => return s,
            Some(_) => continue,
            None => return s,
        }
    }
    &LADDER[5]
}

/// Steps up the ladder, stopping at the top rather than wrapping.
///
/// Powerful Build is one step, and it moves CARRYING only - not reach,
/// not space, not what a container will admit. See species.rs.
pub fn bump(key: &str, steps: i64) -> &'static str {
    match of(key) {
        Some(s) => {
            let up = (s.rank + steps.max(0)).min(LADDER.len() as i64 - 1);
            LADDER[up as usize].key
        }
        // Untouched rather than defaulted: a typo should not silently
        // become Medium halfway through a calculation.
        None => "med",
    }
}

/* ======================= WHAT SIZE DECIDES ======================= */

/// Whether one creature can grapple or shove another.
///
/// 5e: the target may be at most ONE size larger than you. In a world
/// with 25-foot Imiear this is the rule that stops a 2-foot rodent
/// wrestling a giant, and it is worth having in code rather than in
/// somebody's memory.
#[allow(dead_code)] // THERE IS NO GRAPPLE ACTION YET. Kept because the rule
pub fn can_grapple(actor: &str, target: &str) -> bool {
    match (of(actor), of(target)) {
        (Some(a), Some(t)) => t.rank <= a.rank + 1,
        _ => true,
    }
}

/// The smallest space a creature can squeeze through: one rung down.
#[allow(dead_code)] // NO MAP, SO NOTHING TO SQUEEZE THROUGH. The rung-down
pub fn squeeze_into(key: &str) -> &'static str {
    match of(key) {
        Some(s) if s.rank > 0 => LADDER[(s.rank - 1) as usize].key,
        Some(s) => s.key,
        None => "med",
    }
}

/// Whether a mount of this size can carry a rider of that size.
///
/// 5e: a mount must be at least one size LARGER than its rider. A
/// two-foot rodent people riding a raven is the case this campaign
/// actually contains, and it passes - Tiny rider, Small bird.
#[allow(dead_code)] // NO MOUNTS YET, and this campaign has a two-foot
pub fn can_carry_rider(mount: &str, rider: &str) -> bool {
    match (of(mount), of(rider)) {
        (Some(m), Some(r)) => m.rank >= r.rank + 1,
        _ => false,
    }
}

/// Whether a creature this size struggles with a Heavy weapon.
///
/// 5e: Small creatures have disadvantage with Heavy weapons. Tiny is
/// not addressed by the book at all - there are no Tiny player
/// characters in it - so this extends the rule down rather than leaving
/// a two-foot creature swinging a greataxe unremarked.
#[allow(dead_code)] // NOTHING ASKS YET. The attack path does not
pub fn heavy_weapon_is_awkward(key: &str) -> bool {
    matches!(of(key), Some(s) if s.rank <= 1)
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ladder_is_ordered_and_its_index_is_its_rank() {
        for (i, s) in LADDER.iter().enumerate() {
            assert_eq!(s.rank, i as i64, "{} sits wrong", s.key);
        }
    }

    // Every rung's ceiling must be the next one's floor, or a height
    // between them would belong to nobody.
    #[test]
    fn the_height_bands_meet_without_a_gap_or_an_overlap() {
        for pair in LADDER.windows(2) {
            assert_eq!(
                pair[0].max_height_ft,
                Some(pair[1].min_height_ft),
                "{} and {} do not meet",
                pair[0].key,
                pair[1].key
            );
        }
        assert_eq!(LADDER[5].max_height_ft, None, "gargantuan has no ceiling");
    }

    /* ------------------- this campaign's peoples ------------------- */

    #[test]
    fn every_species_in_the_campaign_lands_where_it_should() {
        // The heights Dave stated, and the rung each one falls on.
        assert_eq!(for_height(2.0).key, "sm", "the rodent people, 2 ft");
        assert_eq!(for_height(4.5).key, "med", "Unt'gar");
        assert_eq!(for_height(5.5).key, "med", "Felligar");
        assert_eq!(for_height(8.0).key, "lg", "Unt'garoth");
        assert_eq!(for_height(18.0).key, "huge", "Jotun");
        assert_eq!(for_height(25.0).key, "huge", "Imiear");
    }

    // THE COLLISION, asserted so it cannot be forgotten. Seven feet
    // apart and mechanically identical. If this ever stops being
    // acceptable, this test is where the decision gets recorded.
    #[test]
    fn a_jotun_and_an_imiear_are_the_same_size_and_that_is_a_choice() {
        let jotun = for_height(18.0);
        let imiear = for_height(25.0);
        assert_eq!(jotun.key, imiear.key);
        assert_eq!(jotun.carry_multiplier, imiear.carry_multiplier);
        assert_eq!(jotun.reach_ft, imiear.reach_ft);
    }

    // An Unt'garoth at exactly 8 feet is Large; a hair under is Medium.
    #[test]
    fn the_boundary_is_inclusive_below_and_exclusive_above() {
        assert_eq!(for_height(7.99).key, "med");
        assert_eq!(for_height(8.0).key, "lg");
        assert_eq!(for_height(15.99).key, "lg");
        assert_eq!(for_height(16.0).key, "huge");
    }

    #[test]
    fn something_enormous_is_gargantuan_rather_than_nothing() {
        assert_eq!(for_height(400.0).key, "grg");
    }

    /* --------------------------- bumping --------------------------- */

    #[test]
    fn powerful_build_is_one_rung_and_the_top_holds() {
        assert_eq!(bump("lg", 1), "huge");
        assert_eq!(bump("med", 0), "med");
        assert_eq!(bump("grg", 1), "grg");
        assert_eq!(bump("huge", 5), "grg");
    }

    /* ------------------------ what it decides ------------------------ */

    #[test]
    fn you_may_grapple_something_one_size_larger_and_no_further() {
        assert!(can_grapple("med", "lg"), "a person may grab an ogre");
        assert!(!can_grapple("med", "huge"), "but not a Jotun");
        assert!(can_grapple("huge", "med"), "downward is always fine");
        // The rodent and the giant, which is the case worth naming.
        assert!(!can_grapple("sm", "huge"));
        assert!(can_grapple("lg", "huge"), "an Unt'garoth may try a Jotun");
    }

    #[test]
    fn a_creature_squeezes_one_rung_down() {
        assert_eq!(squeeze_into("lg"), "med");
        assert_eq!(squeeze_into("huge"), "lg");
        assert_eq!(squeeze_into("tiny"), "tiny", "nothing smaller to become");
    }

    // The raven and its rider, which this campaign actually contains.
    #[test]
    fn a_mount_must_be_larger_than_who_rides_it() {
        assert!(can_carry_rider("med", "sm"), "a two-foot rider on a bird");
        assert!(!can_carry_rider("med", "med"), "same size will not do");
        assert!(!can_carry_rider("sm", "med"));
        assert!(can_carry_rider("huge", "lg"), "an Unt'garoth on a Jotun's mount");
    }

    #[test]
    fn the_small_peoples_find_heavy_weapons_awkward() {
        assert!(heavy_weapon_is_awkward("sm"));
        assert!(heavy_weapon_is_awkward("tiny"));
        assert!(!heavy_weapon_is_awkward("med"));
        assert!(!heavy_weapon_is_awkward("lg"));
    }

    /* ----------------- the copies this file replaced ----------------- */

    // The numbers carry.rs held before this file existed. If these ever
    // drift, capacity has silently changed for every creature.
    #[test]
    fn the_carry_multipliers_are_the_ones_carry_rs_used_to_hold() {
        assert_eq!(of("tiny").unwrap().carry_multiplier, 0.5);
        assert_eq!(of("sm").unwrap().carry_multiplier, 1.0);
        assert_eq!(of("med").unwrap().carry_multiplier, 1.0);
        assert_eq!(of("lg").unwrap().carry_multiplier, 2.0);
        assert_eq!(of("huge").unwrap().carry_multiplier, 4.0);
        assert_eq!(of("grg").unwrap().carry_multiplier, 8.0);
    }

    // And the dice vitality.rs held, checked against the book there.
    #[test]
    fn the_hit_dice_are_the_ones_vitality_used_to_hold() {
        assert_eq!(of("tiny").unwrap().hit_die, 4);
        assert_eq!(of("sm").unwrap().hit_die, 6);
        assert_eq!(of("med").unwrap().hit_die, 8);
        assert_eq!(of("lg").unwrap().hit_die, 10);
        assert_eq!(of("huge").unwrap().hit_die, 12);
        assert_eq!(of("grg").unwrap().hit_die, 20);
    }

    #[test]
    fn a_word_outside_the_six_is_none() {
        assert!(of("enormous").is_none());
        assert!(of("").is_none());
        assert!(of("MED").is_none(), "the vocabulary is lowercase");
    }
}
