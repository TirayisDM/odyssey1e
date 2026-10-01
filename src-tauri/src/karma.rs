//! Karma, and the percentile table it is read against.
//!
//! HOPPER resolves a performance on a 26 by 26 chart: the audience
//! along the top, the performer's Karma down the side, and a d100
//! target where they cross. 076 put both axes in the database. This is
//! the arithmetic between them.
//!
//! THE CHART IS A LINE WEARING A SQUARE'S FACE. Every step on either
//! axis moves the result by two points and they move it in opposite
//! directions, so a point of audience goodwill and a point of Karma
//! cancel exactly:
//!
//! ```text
//!     target = 50 + 2 * (karma - audience)
//! ```
//!
//! Which means the whole 676-cell table is one number - the GAP - from
//! -25 to +25, and the diagonal is 50 everywhere along it. An even
//! match is a coin flip wherever on the chart it happens.
//!
//! IT NEVER REACHES CERTAIN AT EITHER END. The worst corner computes to
//! zero and is printed 01; the best computes to 100 and is printed 00,
//! the percentile convention where 00 reads as a hundred. So there is
//! always a one-in-a-hundred chance of humiliating a master and always
//! one of a fool bringing the house down. That is the rule, not a
//! rounding artefact - see `target`.
//!
//! KARMA IS A PROPERTY OF THE CLASS. Bard Karma is Insight +
//! Performance; another class will sum different skills, and which
//! skills is `classes.karma_skills` rather than anything written here.
//! This module knows how to add modifiers, not whose modifiers to add.
//!
//! WHAT IS NOT HERE: the roll. `dice::percentile` rolls it and the
//! comparison is the caller's, because this is the part that has to be
//! the same every time it is asked.

/// The widest either axis goes. Both are 0-25 on the printed chart.
pub const MAX: i64 = 25;

/// The floor and ceiling on the printed target. 01 and 100, where the
/// chart prints the hundred as `00`.
pub const FLOOR: i64 = 1;
pub const CEILING: i64 = 100;

/// Two points of target per point of either axis.
const PER_STEP: i64 = 2;

/// The even-match result, and the whole diagonal of the chart.
const EVEN: i64 = 50;

/// The d100 target to roll at or under.
///
/// CLAMPED AT BOTH ENDS AND NOT JUST THE LOW ONE. A Karma of 25 against
/// a Participating room computes to exactly 100 and stays there; the
/// clamp matters for inputs ABOVE 25, which `rating` can produce,
/// because a bard with expertise in both skills reaches the mid
/// thirties and the chart stops at 25.
pub fn target(karma: i64, audience: i64) -> i64 {
    let karma = karma.clamp(0, MAX);
    let audience = audience.clamp(0, MAX);
    (EVEN + PER_STEP * (karma - audience)).clamp(FLOOR, CEILING)
}

/// The target as the chart prints it: `01` through `99`, and `00` for a
/// hundred.
///
/// A SEPARATE FUNCTION FROM `target`, because 100 and 0 are the same
/// two characters and only one of them is a number anything should
/// compare against. Everything that decides an outcome uses `target`;
/// this exists so a screen can show the cell a player would have found
/// with a finger on the printed chart.
pub fn printed(karma: i64, audience: i64) -> String {
    match target(karma, audience) {
        CEILING => "00".to_string(),
        n => format!("{:02}", n),
    }
}

/// Whether a roll succeeded, given the target.
///
/// AT OR UNDER, which is the percentile convention this chart is built
/// for: a higher target is a better chance. `dice::percentile` yields
/// 1-100 with 100 standing for the 00 on the dice, so a roll of exactly
/// 100 fails everything except a target of 100 - which is the one in a
/// hundred the ceiling is there to preserve.
pub fn made_it(roll: i64, target: i64) -> bool {
    roll <= target
}

/// How far over or under the target a roll landed.
///
/// THE MARGIN IS WHAT TIERS A SONG. A bard who needed 70 and rolled 12
/// has done something better than one who scraped in at 69, and the
/// draft quality reads off this rather than off the bare pass.
/// Positive is the amount to spare; negative is the amount missed by.
pub fn margin(roll: i64, target: i64) -> i64 {
    target - roll
}

/// A character's Karma: the sum of the skill modifiers their class
/// names, clamped to the chart.
///
/// MODIFIERS, NOT SCORES. The caller resolves each named skill to its
/// modifier - ability modifier plus proficiency, doubled for expertise
/// - and hands the list over. Doing the resolution here would mean this
/// module knowing what an ability is, which is character.rs's job and
/// already written there.
///
/// CLAMPED AT 25 BECAUSE THE CHART STOPS THERE. A bard with expertise
/// in Insight and Performance passes 25 well before level 20 and sits
/// pinned at the top of the table for the rest of their career. That is
/// the intent and not an overflow: the ceiling is the reward.
///
/// FLOORED AT ZERO, because the chart has no negative axis. A bard with
/// two punishing abilities and no training is as bad as the table can
/// describe, which is still a 36% chance in a neutral room.
pub fn rating(modifiers: &[i64]) -> i64 {
    modifiers.iter().sum::<i64>().clamp(0, MAX)
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    /* ---------- the five anchors off the printed chart ---------- */

    #[test]
    fn an_even_match_is_a_coin_flip() {
        assert_eq!(target(0, 0), 50);
    }

    #[test]
    fn the_whole_diagonal_is_fifty() {
        // The chart's defining property: only the GAP matters, so
        // every equal pairing anywhere on it reads the same.
        for n in 0..=MAX {
            assert_eq!(target(n, n), 50, "karma {} vs audience {}", n, n);
        }
    }

    #[test]
    fn the_worst_corner_is_one_and_not_zero() {
        // 50 + 2 * (0 - 25) computes to zero. The chart prints 01,
        // because a master is never quite safe and a fool is never
        // quite doomed.
        assert_eq!(target(0, MAX), 1);
    }

    #[test]
    fn the_best_corner_is_a_hundred_printed_as_double_zero() {
        assert_eq!(target(MAX, 0), 100);
        assert_eq!(printed(MAX, 0), "00");
    }

    #[test]
    fn ten_against_nothing_is_seventy() {
        assert_eq!(target(10, 0), 70);
    }

    #[test]
    fn nothing_against_ten_is_thirty() {
        assert_eq!(target(0, 10), 30);
    }

    /* ---------- the shape of it ---------- */

    #[test]
    fn a_point_of_karma_is_worth_two_points_of_target() {
        for k in 0..MAX {
            assert_eq!(target(k + 1, 5) - target(k, 5), 2, "at karma {}", k);
        }
    }

    #[test]
    fn a_point_of_audience_cancels_a_point_of_karma() {
        // The reason the table is really one-dimensional.
        for gap in 0..20 {
            assert_eq!(target(gap, 0), target(gap + 5, 5));
        }
    }

    #[test]
    fn past_the_edges_the_chart_simply_stops() {
        // A bard with expertise in both skills walks off the side of
        // the printed table. They do not get a better than best.
        assert_eq!(target(40, 0), target(MAX, 0));
        assert_eq!(target(10, 40), target(10, MAX));
        assert_eq!(target(-5, 0), target(0, 0));
    }

    /* ---------- how it prints ---------- */

    #[test]
    fn single_digits_carry_a_leading_zero() {
        assert_eq!(printed(0, MAX), "01");
    }

    #[test]
    fn ninety_nine_is_not_a_hundred() {
        // Nothing on the chart computes to 99 - every cell is even
        // except the clamped corner - so this pins that `printed` is
        // not quietly rounding anything.
        assert_eq!(printed(24, 0), "98");
        assert_eq!(printed(25, 0), "00");
    }

    /* ---------- the audience ladder as 076 seeded it ---------- */

    #[test]
    fn a_first_level_bard_in_an_ordinary_room() {
        // Karma 5 - CHA 16 and proficient Performance, nothing in
        // Insight - against Neutral at 8.
        assert_eq!(target(5, 8), 44);
    }

    #[test]
    fn the_career_arc_the_ladder_was_tuned_for() {
        // Participating 0, Neutral 8, Hostile 24.
        assert_eq!([target(5, 0), target(5, 8), target(5, 24)], [60, 44, 12]);
        assert_eq!([target(12, 0), target(12, 8), target(12, 24)], [74, 58, 26]);
        assert_eq!([target(22, 0), target(22, 8), target(22, 24)], [94, 78, 46]);
    }

    #[test]
    fn a_hostile_room_stays_hard_even_at_the_ceiling() {
        // Pinned Karma against the worst audience is still barely
        // better than even. A hostile crowd is never a formality.
        assert_eq!(target(MAX, 24), 52);
    }

    /* ---------- rolling against it ---------- */

    #[test]
    fn at_or_under_the_target_makes_it() {
        assert!(made_it(44, 44));
        assert!(made_it(1, 44));
        assert!(!made_it(45, 44));
    }

    #[test]
    fn a_hundred_fails_everything_but_a_hundred() {
        // The percentile 00. It is the one roll the best corner of the
        // chart still survives, and the reason the ceiling is 100
        // rather than 99.
        assert!(!made_it(100, 99));
        assert!(made_it(100, 100));
    }

    #[test]
    fn the_margin_says_how_well_and_not_just_whether() {
        assert_eq!(margin(12, 70), 58);
        assert_eq!(margin(69, 70), 1);
        assert_eq!(margin(80, 70), -10);
    }

    /* ---------- karma itself ---------- */

    #[test]
    fn karma_is_the_sum_of_what_the_class_names() {
        // Bard: Insight +3, Performance +5.
        assert_eq!(rating(&[3, 5]), 8);
    }

    #[test]
    fn no_skills_named_is_no_karma() {
        // Every class but the bard, today.
        assert_eq!(rating(&[]), 0);
    }

    #[test]
    fn expertise_counts_and_then_the_chart_runs_out() {
        // Level 20, CHA 20, WIS 20, expertise in both: 11 + 11 with
        // ordinary proficiency, 17 + 17 with it doubled. The chart
        // stops at 25 and the bard stays there.
        assert_eq!(rating(&[11, 11]), 22);
        assert_eq!(rating(&[17, 17]), MAX);
    }

    #[test]
    fn a_punishing_pair_of_abilities_is_floored_rather_than_negative() {
        assert_eq!(rating(&[-2, -3]), 0);
    }
}
