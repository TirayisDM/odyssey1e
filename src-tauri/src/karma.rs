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
//!     target = 50 + 2 * (audience - karma)      roll OVER it
//! ```
//!
//! Which means the whole 676-cell table is one number - the GAP - from
//! -25 to +25, and the diagonal is 50 everywhere along it. An even
//! match is a coin flip wherever on the chart it happens.
//!
//! THE NUMBER IS WHAT YOU HAVE TO BEAT, AND LOW IS GOOD. It was written
//! the other way first - karma raising a roll-under target - and the
//! first live performance is what caught it: Falon rolled 26 against a
//! printed 38 and the screen called it a success, when at that table it
//! is a miss. Dave's chart is a difficulty to clear, not an allowance
//! to stay inside.
//!
//! Mirroring the axes rather than only flipping the comparison is the
//! whole of the fix, and it has to be both. Keep `karma - audience` and
//! read it as roll-over and a master in a friendly room needs to beat
//! 100, which cannot be done - the better you get the worse you do. The
//! odds for any given pairing are identical either way; what changes is
//! which number is printed and which direction clears it.
//!
//! IT NEVER REACHES CERTAIN AT EITHER END. The best corner computes to
//! zero and is held at 01, so a master still fails on a natural 1; the
//! worst computes to 100 and is held at 99, so a fool still brings the
//! house down on a 00. That is the rule, not a rounding artefact - see
//! `target`.
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

/// The easiest and hardest numbers the chart will print.
///
/// NEITHER END IS CERTAIN, and these two constants are the whole of
/// that rule. Beating 1 fails only on a natural 1; beating 99 succeeds
/// only on a 00. Both corners compute past these - to 0 and to 100 -
/// and are held here.
pub const FLOOR: i64 = 1;
pub const CEILING: i64 = 99;

/// Two points of target per point of either axis.
const PER_STEP: i64 = 2;

/// The even-match result, and the whole diagonal of the chart.
const EVEN: i64 = 50;

/// The d100 number to roll OVER.
///
/// LOW IS GOOD: Karma pulls it down and a difficult room pushes it up,
/// which is the opposite of how this read until the first live
/// performance disagreed with it. See the module header.
///
/// CLAMPED AT BOTH ENDS AND NOT JUST THE HIGH ONE. A Karma of 25
/// against a Participating room computes to zero and is held at 1; the
/// clamp on the axes matters for inputs ABOVE 25, which `rating` can
/// produce, because a bard with expertise in both skills reaches the
/// mid thirties and the chart stops at 25.
pub fn target(karma: i64, audience: i64) -> i64 {
    let karma = karma.clamp(0, MAX);
    let audience = audience.clamp(0, MAX);
    (EVEN + PER_STEP * (audience - karma)).clamp(FLOOR, CEILING)
}

/// The target as the chart prints it: `01` through `99`.
///
/// NO `00` ANY MORE. A roll-under chart has to print its hundred as
/// `00` and then be careful that the two characters are not read as a
/// zero; a roll-over chart tops out at 99 and never says it. The
/// hundred still exists - it is the roll that beats 99 - but it is a
/// result now rather than a cell.
///
/// Kept as its own function anyway, because a cell is two characters
/// with a leading zero and a target is a number, and the screen wants
/// the first while every comparison wants the second.
pub fn printed(karma: i64, audience: i64) -> String {
    format!("{:02}", target(karma, audience))
}

/// Whether a roll succeeded, given the target.
///
/// STRICTLY OVER, not at-or-over, and the corners are why. The roller
/// yields 1-100 with 100 standing for the 00 on the dice. Beating the
/// floor of 1 then leaves exactly one failing roll and beating the
/// ceiling of 99 leaves exactly one succeeding roll, which is the
/// one-in-a-hundred at each end that the chart is built around. At or
/// over would make the best corner certain.
pub fn made_it(roll: i64, target: i64) -> bool {
    roll > target
}

/// How far over or under the target a roll landed.
///
/// THE MARGIN IS WHAT TIERS A SONG. A bard who had to beat 30 and
/// rolled 88 has done something better than one who scraped over at
/// 31, and the draft quality reads off this rather than off the bare
/// pass. Positive is the amount to spare; negative is the amount
/// missed by.
///
/// MIRRORED WITH THE CHART. This was `target - roll` while the chart
/// ran the other way, and leaving it would have reported every
/// triumph as a failure by the same number.
pub fn margin(roll: i64, target: i64) -> i64 {
    roll - target
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
    fn the_best_corner_is_one_and_not_zero() {
        // 50 + 2 * (0 - 25) computes to zero. The chart holds it at
        // 01, because a master is never quite safe: beating 1 still
        // fails on a natural 1.
        assert_eq!(target(MAX, 0), 1);
    }

    #[test]
    fn the_worst_corner_is_ninety_nine_and_not_a_hundred() {
        // And a fool is never quite doomed: beating 99 comes off on
        // the 00.
        assert_eq!(target(0, MAX), 99);
        assert_eq!(printed(0, MAX), "99");
    }

    #[test]
    fn ten_of_karma_takes_twenty_off_the_number() {
        // The old chart read 70 here and meant the same odds: beat 30
        // and 70 of the hundred faces clear it.
        assert_eq!(target(10, 0), 30);
    }

    #[test]
    fn ten_of_audience_puts_twenty_on() {
        assert_eq!(target(0, 10), 70);
    }

    /// THE DIRECTION, pinned on its own because reversing it is the
    /// bug this chart has already had once. Karma makes the number
    /// smaller; a harder room makes it bigger.
    #[test]
    fn karma_lowers_the_bar_and_the_room_raises_it() {
        assert!(target(10, 5) < target(5, 5), "more karma, lower number");
        assert!(target(5, 10) > target(5, 5), "worse room, higher number");
    }

    /* ---------- the shape of it ---------- */

    #[test]
    fn a_point_of_karma_is_worth_two_points_of_target() {
        // Downward now: each point of Karma takes two off the number
        // that has to be beaten.
        for k in 0..MAX {
            assert_eq!(target(k, 5) - target(k + 1, 5), 2, "at karma {}", k);
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
        assert_eq!(printed(MAX, 0), "01");
        assert_eq!(printed(24, 0), "02");
    }

    #[test]
    fn nothing_prints_double_zero_any_more() {
        // A roll-under chart had to print its hundred as 00 and then
        // take care nobody read it as nothing. This one tops out at
        // 99 and the hundred is a ROLL instead - the one that beats
        // the worst corner.
        for k in 0..=MAX {
            for a in 0..=MAX {
                assert_ne!(printed(k, a), "00", "karma {} vs audience {}", k, a);
            }
        }
    }

    /* ---------- the audience ladder as 076 seeded it ---------- */

    #[test]
    fn a_first_level_bard_in_an_ordinary_room() {
        // Karma 5 - CHA 16 and proficient Performance, nothing in
        // Insight - against Neutral at 8. Beat 56, which is the same
        // 44% the old chart meant by printing 44.
        assert_eq!(target(5, 8), 56);
    }

    #[test]
    fn the_career_arc_the_ladder_was_tuned_for() {
        // Participating 0, Neutral 8, Hostile 24. Every number is a
        // hundred minus what it used to be, so the odds the ladder
        // was tuned against have not moved.
        assert_eq!([target(5, 0), target(5, 8), target(5, 24)], [40, 56, 88]);
        assert_eq!([target(12, 0), target(12, 8), target(12, 24)], [26, 42, 74]);
        assert_eq!([target(22, 0), target(22, 8), target(22, 24)], [6, 22, 54]);
    }

    #[test]
    fn a_hostile_room_stays_hard_even_at_the_ceiling() {
        // Pinned Karma against the worst audience still has to beat
        // 48. A hostile crowd is never a formality.
        assert_eq!(target(MAX, 24), 48);
    }

    /// THE CASE THAT CAUGHT IT. Falon, Karma 2, playing to a Neutral
    /// room at 8, rolling 26. The screen called that a success and
    /// Dave said it was a miss.
    #[test]
    fn falons_twenty_six_is_a_miss() {
        let t = target(2, 8);
        assert_eq!(t, 62);
        assert!(!made_it(26, t));
        assert_eq!(margin(26, t), -36);
    }

    /* ---------- rolling against it ---------- */

    #[test]
    fn over_the_target_makes_it_and_equal_does_not() {
        assert!(!made_it(56, 56), "equal is a miss - strictly over");
        assert!(made_it(57, 56));
        assert!(!made_it(1, 56));
        assert!(made_it(100, 56));
    }

    #[test]
    fn neither_corner_is_certain() {
        // Beating the floor fails on exactly one face, and beating
        // the ceiling comes off on exactly one. That symmetry is the
        // reason FLOOR and CEILING are 1 and 99 rather than 0 and 100.
        assert!(!made_it(1, FLOOR), "a master still fumbles on a 1");
        assert!(made_it(2, FLOOR));
        assert!(made_it(100, CEILING), "a fool still lands the 00");
        assert!(!made_it(99, CEILING));
    }

    #[test]
    fn the_margin_says_how_well_and_not_just_whether() {
        assert_eq!(margin(88, 30), 58);
        assert_eq!(margin(31, 30), 1);
        assert_eq!(margin(20, 30), -10);
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
