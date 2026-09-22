//! Hit dice, and the hit points that come out of them.
//!
//! LEVEL IS HIT DICE. That is the decision this file exists to carry: a
//! goblin is 2d6, so a goblin is level 2. The Monster Manual has always
//! written it that way - `Hit Points 7 (2d6)` - and treating the number
//! in brackets as a level rather than as trivia is what lets one rule
//! serve a monster and a character.
//!
//! WHAT THAT BUYS. `npcs.hp_max` was a magic number: 7, because the
//! book says 7. Nothing could check it, nothing could move it, and a DM
//! who wanted a tougher goblin had to invent a second statblock. Level
//! times a die is a number that can be recomputed, which is what makes
//! "set level" a button rather than a rewrite.
//!
//! THE DIE COMES FROM SIZE, which is why the book never states it
//! separately - a Small creature rolls d6 whatever it is. The six sizes
//! are already in the schema with the vocabulary 010 chose, so nothing
//! new had to be stored to know this.
//!
//! VERIFIED AGAINST THE BOOK, not derived from first principles and
//! hoped over. The tests below check five monsters spanning d6 to d20,
//! and they are worth more than most tests here because they can be
//! falsified by anyone holding the Monster Manual.
//!
//! WHAT THIS IS NOT. A player character's hit points are not this.
//! 5e maxes a PC's first hit die and rolls the rest, and the die comes
//! from class rather than size - so `average_hp` is the MONSTER rule,
//! and a PC's maximum stays stated. The shapes are close enough that
//! one function will cover both when rests arrive and a PC needs a
//! pool; they are not close enough to pretend today.

/* ============================ RULES ============================ */

/// The hit die of a creature of this size.
///
/// The 010 vocabulary, and None for anything outside it - including
/// NULL, which is a real case: a statblock written through the DM panel
/// before 029 has no size, and a creature whose die is unknown cannot
/// have its hit points derived. That is reported rather than guessed,
/// because guessing d8 would silently give every sizeless statblock a
/// medium creature's hit points.
pub fn hit_die(size: Option<&str>) -> Option<i64> {
    match size? {
        "tiny" => Some(4),
        "sm" => Some(6),
        "med" => Some(8),
        "lg" => Some(10),
        "huge" => Some(12),
        "grg" => Some(20),
        _ => None,
    }
}

/// The average of one die, doubled, so the arithmetic stays in integers.
///
/// A d6 averages 3.5 and there is no honest way to hold that in an i64.
/// Doubling every term and halving once at the end keeps the rounding
/// in one place - and it has to be one place, because the book rounds
/// the TOTAL down rather than each die. An ogre is 7d10: 38.5 becomes
/// 38, not seven lots of 5.5 rounded to 35.
fn twice_average(die: i64) -> i64 {
    die + 1
}

/// A monster's hit points at this level.
///
/// `floor(level * (die + 1) / 2) + level * con_mod`, which is exactly
/// what the Monster Manual prints - see the tests.
///
/// CONSTITUTION APPLIES PER DIE, not once. That is the part people get
/// wrong from memory: the ogre's +21 is +3 across seven dice, and a
/// creature that levels up gains its modifier again each time.
///
/// FLOORED AT ONE. A d4 creature with a punishing Constitution can
/// compute to zero or less, and a living thing with no hit points is
/// not a creature, it is a corpse. One is the floor 5e uses everywhere
/// else for the same reason.
pub fn average_hp(level: i64, die: i64, con_mod: i64) -> i64 {
    if level < 1 {
        return 1;
    }
    let from_dice = (level * twice_average(die)) / 2;
    (from_dice + level * con_mod).max(1)
}

/// The proficiency bonus a creature of this level derives.
///
/// The same `floor((level - 1) / 4) + 2` a character uses, stated here
/// because a monster's level now MOVES and something has to recompute
/// it when it does.
///
/// A MONSTER'S PRINTED BONUS COMES FROM CHALLENGE RATING, NOT HIT DICE,
/// and those diverge: an ogre is 7 hit dice and CR 2, so the book gives
/// it +2 where this gives +3. That is why `npcs.prof_bonus` is a stated
/// override and why 022 made it nullable - a statblock copied from the
/// book keeps the book's answer. This is what a creature whose level a
/// DM has moved by hand derives instead, because at that point it is no
/// longer the monster the book rated.
pub fn prof_bonus_for(level: i64) -> i64 {
    ((level.max(1) - 1) / 4) + 2
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    /// Each of these is a printed Monster Manual line, and the point of
    /// them is that they can be checked against the book rather than
    /// against this file. (name, level/HD, size, CON score, printed HP)
    const FROM_THE_BOOK: [(&str, i64, &str, i64, i64); 5] = [
        ("goblin", 2, "sm", 10, 7),      // 7 (2d6)
        ("orc", 2, "med", 16, 15),       // 15 (2d8 + 6)
        ("bugbear", 5, "med", 13, 27),   // 27 (5d8 + 5)
        ("ogre", 7, "lg", 16, 59),       // 59 (7d10 + 21)
        ("tarrasque", 33, "grg", 30, 676), // 676 (33d20 + 330)
    ];

    fn con_mod(score: i64) -> i64 {
        (score - 10).div_euclid(2)
    }

    #[test]
    fn the_monster_manual_agrees() {
        for (name, level, size, con, printed) in FROM_THE_BOOK {
            let die = hit_die(Some(size)).expect("a real size");
            let got = average_hp(level, die, con_mod(con));
            assert_eq!(got, printed, "{} should be {} hp, got {}", name, printed, got);
        }
    }

    #[test]
    fn the_total_rounds_down_not_each_die() {
        // An ogre is 7d10. 7 x 5.5 is 38.5, which the book prints as
        // 38 + 21 = 59. Rounding each die to 5 would give 35 + 21 = 56
        // and rounding each up would give 42 + 21 = 63. Both are wrong
        // and both are the mistake this arithmetic is arranged to avoid.
        assert_eq!(average_hp(7, 10, 3), 59);
    }

    #[test]
    fn constitution_applies_once_per_die() {
        // The thing people get wrong from memory. Seven dice, +3 each.
        assert_eq!(average_hp(7, 10, 3) - average_hp(7, 10, 0), 21);
    }

    #[test]
    fn a_negative_constitution_subtracts_per_die_too() {
        // Kobold: 5 (2d6 - 2), CON 9. Also from the book.
        assert_eq!(average_hp(2, 6, -1), 5);
    }

    #[test]
    fn nothing_living_drops_below_one() {
        // A tiny creature with a ruinous Constitution computes to zero
        // or less. It is still alive.
        assert_eq!(average_hp(1, 4, -5), 1);
        assert_eq!(average_hp(2, 4, -9), 1);
    }

    #[test]
    fn level_zero_is_not_a_creature() {
        assert_eq!(average_hp(0, 6, 0), 1);
        assert_eq!(average_hp(-3, 6, 0), 1);
    }

    #[test]
    fn every_size_has_a_die_and_nothing_else_does() {
        for (size, die) in [
            ("tiny", 4), ("sm", 6), ("med", 8),
            ("lg", 10), ("huge", 12), ("grg", 20),
        ] {
            assert_eq!(hit_die(Some(size)), Some(die));
        }
        // A statblock written before 029 has no size, and a die that
        // cannot be known must not be invented.
        assert_eq!(hit_die(None), None);
        assert_eq!(hit_die(Some("large")), None);
        assert_eq!(hit_die(Some("")), None);
    }

    #[test]
    fn proficiency_moves_every_four_levels() {
        assert_eq!(prof_bonus_for(1), 2);
        assert_eq!(prof_bonus_for(2), 2);  // the goblin
        assert_eq!(prof_bonus_for(4), 2);
        assert_eq!(prof_bonus_for(5), 3);
        assert_eq!(prof_bonus_for(17), 6);
        // Never below the floor, whatever it is handed.
        assert_eq!(prof_bonus_for(0), 2);
    }

    #[test]
    fn hit_dice_and_challenge_rating_disagree_and_that_is_expected() {
        // The ogre is the standing example: 7 hit dice, CR 2. The book
        // prints +2; level derives +3. A statblock keeps the book's
        // answer by stating it - see prof_bonus_for.
        assert_eq!(prof_bonus_for(7), 3);
    }
}
