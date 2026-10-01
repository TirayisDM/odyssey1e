//! Rolling a character up. 066.
//!
//! SEVEN SCORES FOR SIX SLOTS, each 3d6 with every 1 rerolled. The
//! spare is the point: a player assigns six and discards one, so the
//! worst roll of the seven need not be lived with.
//!
//! IN ITS OWN FILE because `dice.rs` is already past the 800-line
//! ceiling ARCHITECTURE.md sets, and because this is a different
//! subject. `dice.rs` resolves a formula somebody typed; this is
//! character generation, which will grow - starting coin, starting kit,
//! a point-buy alternative - and none of that belongs beside the
//! formula parser.
//!
//! IT TAKES A `Roller` rather than reaching for the RNG, which is what
//! makes the rule testable: a fixed sequence in, an exact spread out.
//! The same reason dice.rs injects one.
//!
//! ---------------------------------------------------------------------
//! WHAT "REROLL 1s" WAS TAKEN TO MEAN
//! ---------------------------------------------------------------------
//!
//! A 1 is rerolled, and if the reroll is also a 1 it is rerolled again.
//! No 1 survives. Each die is therefore uniform over 2..=6, a score
//! runs 6 to 18 with a mean of 12, and a 3 is impossible.
//!
//! The other reading - reroll each 1 ONCE and keep whatever comes back -
//! leaves 1s on the table and gives a mean near 11.75. Both are in use
//! at real tables. This one is the commoner phrasing of the house rule
//! and the kinder of the two, which is usually the point of adopting
//! it. If it is the wrong one, `REROLL_BELOW` and the loop in
//! `score_with` are the whole change.

use crate::dice::Roller;

/* ============================ THE RULE ============================ */

/// How many scores are rolled.
pub const ROLLED: usize = 7;

/// How many are kept. The difference is the spare a player discards.
pub const KEPT: usize = 6;

/// Dice per score, and their faces.
const DICE: usize = 3;
const FACES: u32 = 6;

/// Any die at or below this is rolled again. One, by the house rule.
const REROLL_BELOW: i64 = 2;

/// A ceiling on rerolls per die.
///
/// NOT FOR THE RNG, which clears 1 with probability 1 - it is for a
/// `Roller` that misbehaves. `SequenceRoller` runs out and returns the
/// same value forever, and a test that fed it nothing but 1s would
/// otherwise hang the suite rather than fail it. Twenty is far past
/// anything real dice will need: the odds of twenty 1s running are one
/// in 3.6 x 10^15.
const MAX_REROLLS: usize = 20;

/// One ability score: three six-sided dice, every 1 rerolled.
pub fn score_with<R: Roller>(r: &mut R) -> i64 {
    let mut total = 0;
    for _ in 0..DICE {
        let mut die = r.roll(FACES);
        let mut tries = 0;
        while die < REROLL_BELOW && tries < MAX_REROLLS {
            die = r.roll(FACES);
            tries += 1;
        }
        total += die;
    }
    total
}

/// Seven of them, in the order they were rolled.
///
/// NOT SORTED. The order dice came up in is what a player watched
/// happen, and sorting it would quietly throw that away - the screen
/// can present them however it likes.
pub fn spread_with<R: Roller>(r: &mut R) -> Vec<i64> {
    (0..ROLLED).map(|_| score_with(r)).collect()
}

/// Seven scores off real dice.
pub fn spread() -> Vec<i64> {
    spread_with(&mut crate::dice::RandomRoller)
}

/* ========================== WHAT THEY CHOSE ========================== */

/// The six ability codes, in sheet order.
pub const CODES: [&str; 6] = ["str", "dex", "con", "int", "wis", "cha"];

/// Whether a set of picks is a complete, legal assignment.
///
/// THE RULE A SCREEN MUST NOT BE THE ONLY ONE ENFORCING. Six codes,
/// each exactly once, each holding a score in range. A dropdown that
/// removes what is taken makes a duplicate impossible by construction -
/// and a dropdown is not where a rule should live, because the next
/// screen will not have one.
///
/// IT DOES NOT CHECK THE SCORE CAME FROM THE SPREAD. The DM may say a
/// number, and 051's rule holds here as everywhere: the engine informs
/// and does not referee the table.
pub fn complete(picks: &[(String, i64)]) -> Result<(), String> {
    if picks.len() != KEPT {
        return Err(format!("six abilities, not {}", picks.len()));
    }
    for (code, score) in picks {
        if !CODES.contains(&code.as_str()) {
            return Err(format!("{} is not an ability", code));
        }
        if !(1..=30).contains(score) {
            return Err(format!("{} of {} is outside 1-30", score, code));
        }
    }
    for code in CODES {
        if picks.iter().filter(|(c, _)| c == code).count() != 1 {
            return Err(format!("{} is not set exactly once", code));
        }
    }
    Ok(())
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::SequenceRoller;

    #[test]
    fn three_dice_with_no_ones_is_their_sum() {
        let mut r = SequenceRoller::new(&[4, 5, 6]);
        assert_eq!(score_with(&mut r), 15);
        assert!(r.exhausted(), "it rolled exactly three dice");
    }

    // The house rule, and the whole reason this is not just 3d6.
    #[test]
    fn a_one_is_rolled_again() {
        //          die1: 1 -> 4     die2: 3     die3: 1 -> 1 -> 6
        let mut r = SequenceRoller::new(&[1, 4, 3, 1, 1, 6]);
        assert_eq!(score_with(&mut r), 4 + 3 + 6);
        assert!(r.exhausted());
    }

    #[test]
    fn a_score_can_never_be_below_six_nor_above_eighteen() {
        for _ in 0..2000 {
            let n = score_with(&mut crate::dice::RandomRoller);
            assert!((6..=18).contains(&n), "{} is outside the band", n);
        }
    }

    // A 3 needs three 1s, and no 1 survives.
    #[test]
    fn three_four_and_five_are_impossible() {
        for _ in 0..2000 {
            let n = score_with(&mut crate::dice::RandomRoller);
            assert!(n != 3 && n != 4 && n != 5, "{} should be unreachable", n);
        }
    }

    // A Roller that never clears a 1 must fail the test, not hang it.
    #[test]
    fn a_roller_that_only_gives_ones_still_terminates() {
        let mut r = SequenceRoller::new(&[1]);
        let n = score_with(&mut r);
        assert_eq!(n, 3, "three dice that never cleared, capped and taken as they lie");
    }

    #[test]
    fn a_spread_is_seven_long() {
        let s = spread();
        assert_eq!(s.len(), ROLLED);
        assert_eq!(ROLLED - KEPT, 1, "one spare, which is the point of seven");
    }

    #[test]
    fn a_spread_keeps_the_order_the_dice_came_in() {
        // 7 scores x 3 dice, ascending, so a sorted answer would differ.
        let vals: Vec<i64> = (0..21).map(|i| 2 + (i % 5)).collect();
        let mut r = SequenceRoller::new(&vals);
        let got = spread_with(&mut r);
        let mut sorted = got.clone();
        sorted.sort();
        assert_ne!(got, sorted, "the roll order is not the sorted order");
    }

    /* ------------------------ the assignment ------------------------ */

    fn full() -> Vec<(String, i64)> {
        CODES.iter().map(|c| (c.to_string(), 12)).collect()
    }

    #[test]
    fn six_distinct_abilities_in_range_is_complete() {
        assert!(complete(&full()).is_ok());
    }

    #[test]
    fn five_is_not_enough() {
        let mut p = full();
        p.pop();
        assert!(complete(&p).unwrap_err().contains("six abilities"));
    }

    // The screen's dropdown makes this impossible; the rule says so
    // anyway, because the next screen will not have a dropdown.
    #[test]
    fn the_same_ability_twice_is_refused() {
        let mut p = full();
        p[5] = ("str".into(), 9);
        let e = complete(&p).unwrap_err();
        assert!(e.contains("cha") || e.contains("str"), "{}", e);
    }

    #[test]
    fn a_code_nobody_recognises_is_refused() {
        let mut p = full();
        p[0] = ("luck".into(), 12);
        assert!(complete(&p).unwrap_err().contains("luck"));
    }

    #[test]
    fn a_score_outside_the_sheet_range_is_refused() {
        let mut p = full();
        p[0] = ("str".into(), 31);
        assert!(complete(&p).unwrap_err().contains("31"));
        p[0] = ("str".into(), 0);
        assert!(complete(&p).unwrap_err().contains("0"));
    }

    // A DM may hand out a number the dice never made - 051's rule.
    #[test]
    fn a_score_that_did_not_come_from_the_spread_is_allowed() {
        let mut p = full();
        p[0] = ("str".into(), 20);
        assert!(complete(&p).is_ok());
    }
}
