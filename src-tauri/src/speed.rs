//! 248. HOW FAR A CREATURE CAN GO.
//!
//! Dave, looking at a map where clicking a square moved a creature any
//! distance at all: *"don't characters have a movement - this is what
//! we should be reading"*.
//!
//! They do, and nothing read it. `species.speed` has said 25 / 30 / 40
//! since 057, and 059 wrote the contract down in as many words:
//!
//! > APPLIED IS FALSE, like every other movement trait in the schema,
//! > because there is still no movement system. **When one arrives,
//! > `speed` is the column it reads.**
//!
//! This is the module that reads it. **ONE PLACE**, because a speed
//! assembled in the command that needed it would be assembled again,
//! differently, by the next one - and a creature that moves six
//! squares on the map and five on its sheet is the two-places fault
//! with legs.
//!
//! ---------------------------------------------------------------------
//! WHAT GOES INTO IT, IN ORDER
//! ---------------------------------------------------------------------
//!
//! ```text
//!   natural    species.speed for a character, npcs.speed for a
//!              monster. In FEET, because that is what the books and
//!              the columns say.
//!   grants     `target: "speed"`. Longstrider is +10, a Monk's
//!              Unarmoured Movement is +10 and climbs with level.
//!   exhaustion level 2 halves it, level 5 stops it. Cumulative, so
//!              level 3 is still halved.
//!   conditions grappled, restrained and four others stop it dead.
//! ```
//!
//! **THE ORDER IS THE RULE.** Bonuses land on the natural speed, then
//! exhaustion halves whatever that came to, then a condition can zero
//! the lot. Longstrider on an exhausted creature is (30+10)/2 = 20,
//! not 30/2+10 = 25, and 5e words exhaustion as halving *your speed* -
//! the one you have, not the one you were born with.
//!
//! A zeroing condition wins over everything, which is why it is last.
//! Grappled says so out loud: *"Speed 0, and no bonus to speed."*
//!
//! ---------------------------------------------------------------------
//! AN UNKNOWN SPEED IS NOT THIRTY
//! ---------------------------------------------------------------------
//!
//! `natural` is an `Option` and `None` comes back as `None`. **All 134
//! monsters have no speed today** - `npcs.speed` is a new column and
//! nothing has filled it in - and a module that quietly answered 30
//! would put a number on a screen that nobody wrote and that would be
//! wrong for a zombie, a giant eagle and a wolf alike.
//!
//! This is 226's rule about a missing tile row, applied to a creature:
//! **absent and empty are different answers.** The caller says "no
//! speed recorded"; it does not say "30".

// 248. UNUSED UNTIL SOMETHING RESOLVES A SPEED, AND THAT IS ON PURPOSE.
//
// This module is complete and has 27 tests. Nothing calls it, because
// the two things that would both wanted a decision that was not mine:
//
//   THE SHEET needs conditions, which are `cond.`-prefixed rows in
//   `effects`, and live grants - neither is on `Sheet` today, so the
//   wiring is its own cycle rather than a line.
//
//   THE WALK READOUT needs the same two, and asking for them in
//   `walk_to` would mean resolving equipment there - which is the
//   two-places fault with a magic item attached. It must ask whatever
//   already resolves a sheet, not re-derive it.
//
// **RESTATED 2026-10-25, NOT RENEWED.** The first draft of this note
// gave a second reason: that `npcs.speed` was null for all 280
// creatures, so a readout would have said "no speed recorded" for
// every monster. 249 seeded 259 of them from the SRD, so **that
// reason is gone** and only the grants one remains. 233's rule is
// that a dated suppression gets its reason restated when the event it
// was waiting on happens, rather than the date quietly moved.
//
// THIS COMES OFF WHEN THE FIRST CALLER LANDS: a sheet that shows a
// resolved speed, which `walk_to` can then ask for.
#![allow(dead_code)]

use crate::conditions;
use crate::grants::{self, Grant};
use crate::grid;

/// Exhaustion halves movement from this level.
const HALVED_AT: i64 = 2;

/// Exhaustion stops it from this level.
const STOPPED_AT: i64 = 5;

/// How far a creature can go, in both units that matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Speed {
    /// Feet, the unit the books and the columns use.
    pub feet: i64,
    /// Squares, the unit the map uses. `grid` owns the conversion.
    pub squares: i64,
}

impl Speed {
    fn of(feet: i64) -> Speed {
        let feet = feet.max(0);
        Speed {
            feet,
            // 225 OWNS THIS AND NOTHING ELSE MAY. `grid::squares`
            // rounds DOWN, so a 25-foot Unt'gar gets five squares and
            // a 35-foot creature gets seven - there is no half square
            // to spend and pretending otherwise would put a creature
            // in the middle of one.
            squares: grid::squares(feet),
        }
    }
}

/// How far this creature can go, or `None` if nobody has said.
///
/// `natural` is feet. `conditions` are the keys on the creature;
/// `exhaustion` is its level, which is a number rather than a key
/// because 5e stacks it six deep.
pub fn resolve(
    natural: Option<i64>,
    conditions: &[String],
    exhaustion: i64,
    live: &[Grant],
) -> Option<Speed> {
    // Nobody has said. See the module note - this is not thirty.
    let natural = natural?;

    // A CONDITION THAT STOPS IT WINS OVER EVERYTHING, which is why it
    // is checked before the arithmetic rather than after: grappled
    // says "Speed 0, AND NO BONUS TO SPEED", so there is nothing for a
    // Longstrider to add to.
    if conditions::stops_movement(conditions).is_some() {
        return Some(Speed::of(0));
    }
    if exhaustion >= STOPPED_AT {
        return Some(Speed::of(0));
    }

    // Bonuses first, on the natural speed. `apply` takes the floor
    // before the adds, which is the rule 100 wrote and this must not
    // restate differently.
    let with_items = grants::apply(natural, live, TARGET);

    // Then exhaustion halves what that came to.
    //
    // ROUNDING DOWN, and 5e does not say. It rounds down everywhere
    // else it is explicit - half cover, half damage from a successful
    // save - and a speed rounded UP would hand an exhausted creature
    // the extra foot that gets it across the gap.
    let halved = match exhaustion >= HALVED_AT {
        true => with_items / 2,
        false => with_items,
    };

    Some(Speed::of(halved))
}

/// The grant target that touches a speed.
///
/// 248. `grants.rs` carried no speed term at all, so Longstrider,
/// Boots of Striding and a Monk's Unarmoured Movement had nowhere to
/// land. Adding it cost no migration: `target` is free text against a
/// documented vocabulary, so this is one word and one line in
/// `known_target`.
///
/// **DOUBLING IS STILL NOT EXPRESSIBLE**, and that is the vocabulary's
/// deliberate shape rather than an oversight here. `add` is cumulative
/// and `set` is a floor; there is no multiply, because a multiply
/// would make the order of two items matter and 100 refused that.
/// Haste and Boots of Speed double a speed, and today they have to be
/// written as an `add` of whatever that creature's speed is - which
/// works and is ugly, and wants its own decision rather than a
/// `Mode::Multiply` smuggled in here.
pub const TARGET: &str = "speed";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grants::Mode;

    /// Condition keys AS THEY ARE STORED.
    ///
    /// 248. The first draft of these tests passed bare words and
    /// three of them failed, which was the right failure: conditions
    /// are stored `cond.grappled`, `of_key` strips that prefix, and a
    /// bare "grappled" is not a condition at all. Had `resolve`
    /// matched on bare words instead, every one of these would have
    /// passed and a grappled creature would have run at full speed in
    /// the app.
    fn keys(ks: &[&str]) -> Vec<String> {
        ks.iter()
            .map(|s| format!("{}{}", conditions::PREFIX, s))
            .collect()
    }

    fn grant(target: &str, mode: Mode, value: i64) -> Grant {
        Grant {
            target: target.to_string(),
            mode,
            value,
            dice: None,
            needs_attunement: false,
            source: "a test".to_string(),
        }
    }

    fn plain(feet: i64) -> Option<Speed> {
        resolve(Some(feet), &[], 0, &[])
    }

    /* ---------------- the natural speed ---------------- */

    #[test]
    fn thirty_feet_is_six_squares() {
        let s = plain(30).unwrap();
        assert_eq!(s.feet, 30);
        assert_eq!(s.squares, 6);
    }

    #[test]
    fn the_untgar_twenty_five_is_five_squares() {
        // 059's whole point: 25 against a 30-foot norm is a real
        // disadvantage, and it is one square.
        assert_eq!(plain(25).unwrap().squares, 5);
    }

    #[test]
    fn a_speed_that_does_not_divide_rounds_down() {
        // 35 feet is seven squares; 32 is six and the odd two feet are
        // not a third of a square anybody can stand in.
        assert_eq!(plain(35).unwrap().squares, 7);
        assert_eq!(plain(32).unwrap().squares, 6);
    }

    #[test]
    fn nobody_has_said_is_not_thirty() {
        // THE POINT OF THE Option. All 134 monsters are here today.
        assert_eq!(resolve(None, &[], 0, &[]), None);
    }

    #[test]
    fn a_speed_of_zero_is_a_real_answer() {
        // Distinct from None: a shrieker has a speed and it is 0.
        let s = resolve(Some(0), &[], 0, &[]).unwrap();
        assert_eq!(s.feet, 0);
        assert_eq!(s.squares, 0);
    }

    /* ---------------- grants ---------------- */

    #[test]
    fn longstrider_adds_ten_feet() {
        let g = [grant(TARGET, Mode::Add, 10)];
        let s = resolve(Some(30), &[], 0, &g).unwrap();
        assert_eq!(s.feet, 40);
        assert_eq!(s.squares, 8);
    }

    #[test]
    fn two_bonuses_stack_because_add_is_cumulative() {
        let g = [grant(TARGET, Mode::Add, 10), grant(TARGET, Mode::Add, 5)];
        assert_eq!(resolve(Some(30), &[], 0, &g).unwrap().feet, 45);
    }

    #[test]
    fn a_set_is_a_floor_and_does_not_slow_anybody_down() {
        // 100's rule, restated nowhere: a set BELOW what you have does
        // nothing. Boots that make your speed 30 do not hobble a
        // 40-foot Unt'garoth.
        let g = [grant(TARGET, Mode::Set, 30)];
        assert_eq!(resolve(Some(40), &[], 0, &g).unwrap().feet, 40);
        assert_eq!(resolve(Some(25), &[], 0, &g).unwrap().feet, 30);
    }

    #[test]
    fn a_grant_on_something_else_does_not_touch_speed() {
        let g = [grant("ac", Mode::Add, 2), grant("str", Mode::Add, 4)];
        assert_eq!(resolve(Some(30), &[], 0, &g).unwrap().feet, 30);
    }

    #[test]
    fn a_curse_is_an_add_with_a_negative_value() {
        let g = [grant(TARGET, Mode::Add, -10)];
        assert_eq!(resolve(Some(30), &[], 0, &g).unwrap().feet, 20);
    }

    #[test]
    fn a_speed_cannot_go_below_zero() {
        let g = [grant(TARGET, Mode::Add, -50)];
        let s = resolve(Some(30), &[], 0, &g).unwrap();
        assert_eq!(s.feet, 0, "a creature walks backwards nowhere");
        assert_eq!(s.squares, 0);
    }

    /* ---------------- exhaustion ---------------- */

    #[test]
    fn exhaustion_one_changes_nothing() {
        assert_eq!(resolve(Some(30), &[], 1, &[]).unwrap().feet, 30);
    }

    #[test]
    fn exhaustion_two_halves_it() {
        let s = resolve(Some(30), &[], 2, &[]).unwrap();
        assert_eq!(s.feet, 15);
        assert_eq!(s.squares, 3);
    }

    #[test]
    fn exhaustion_stays_halved_at_three_and_four() {
        // Cumulative, not a new effect per level.
        assert_eq!(resolve(Some(30), &[], 3, &[]).unwrap().feet, 15);
        assert_eq!(resolve(Some(30), &[], 4, &[]).unwrap().feet, 15);
    }

    #[test]
    fn exhaustion_five_stops_it() {
        assert_eq!(resolve(Some(30), &[], 5, &[]).unwrap().feet, 0);
        assert_eq!(resolve(Some(30), &[], 6, &[]).unwrap().feet, 0);
    }

    #[test]
    fn halving_rounds_down() {
        // 25 halves to 12, not 13. Rounding up would hand an exhausted
        // creature the foot that gets it across the gap.
        assert_eq!(resolve(Some(25), &[], 2, &[]).unwrap().feet, 12);
        assert_eq!(resolve(Some(25), &[], 2, &[]).unwrap().squares, 2);
    }

    #[test]
    fn a_bonus_is_halved_with_everything_else() {
        // THE ORDER IS THE RULE: (30 + 10) / 2 = 20, not 30/2 + 10.
        // 5e halves YOUR speed, which is the one you have.
        let g = [grant(TARGET, Mode::Add, 10)];
        assert_eq!(resolve(Some(30), &[], 2, &g).unwrap().feet, 20);
    }

    /* ---------------- conditions ---------------- */

    #[test]
    fn grappled_is_speed_zero() {
        assert_eq!(resolve(Some(30), &keys(&["grappled"]), 0, &[]).unwrap().feet, 0);
    }

    #[test]
    fn grappled_beats_a_bonus_because_it_says_so() {
        // "Speed 0, AND NO BONUS TO SPEED." A Longstrider on a
        // grappled creature is 0, not 10.
        let g = [grant(TARGET, Mode::Add, 10)];
        assert_eq!(resolve(Some(30), &keys(&["grappled"]), 0, &g).unwrap().feet, 0);
    }

    #[test]
    fn the_other_five_stop_it_too() {
        for k in ["restrained", "paralysed", "petrified", "stunned", "unconscious"] {
            assert_eq!(
                resolve(Some(30), &keys(&[k]), 0, &[]).unwrap().feet,
                0,
                "{} should stop a creature",
                k
            );
        }
    }

    #[test]
    fn incapacitated_alone_does_not_stop_movement() {
        // It takes actions and reactions and says nothing about speed.
        // `cannot_act` and `stops_movement` are different questions.
        assert_eq!(
            resolve(Some(30), &keys(&["incapacitated"]), 0, &[]).unwrap().feet,
            30
        );
    }

    #[test]
    fn prone_does_not_reduce_speed() {
        // Prone makes movement COST more - crawling - and standing up
        // cost half. Both are rules about a square's price, which is
        // `stride`'s business, not about how many squares there are.
        assert_eq!(resolve(Some(30), &keys(&["prone"]), 0, &[]).unwrap().feet, 30);
    }

    #[test]
    fn a_condition_that_does_nothing_to_speed_is_ignored() {
        let c = keys(&["blinded", "charmed", "frightened", "poisoned"]);
        assert_eq!(resolve(Some(30), &c, 0, &[]).unwrap().feet, 30);
    }

    #[test]
    fn a_bare_key_is_not_a_condition() {
        // THE STORAGE CONTRACT, ASSERTED. A caller that hands over
        // "grappled" instead of "cond.grappled" is passing something
        // that is not a condition, and this records that it has no
        // effect rather than a partial one. The failure mode it
        // guards is a creature that should be held fast walking off.
        let bare = vec!["grappled".to_string()];
        assert_eq!(resolve(Some(30), &bare, 0, &[]).unwrap().feet, 30);
    }

    #[test]
    fn an_unknown_condition_key_is_ignored_rather_than_feared() {
        // `of_key` returns None and the creature walks. A typo in a
        // condition must not silently freeze somebody.
        assert_eq!(resolve(Some(30), &keys(&["sleepy"]), 0, &[]).unwrap().feet, 30);
    }

    #[test]
    fn nobody_has_said_stays_unknown_whatever_is_wrong_with_them() {
        // A monster with no recorded speed that is also grappled is
        // still a monster with no recorded speed. Answering 0 here
        // would look like a rule and be an absence.
        assert_eq!(resolve(None, &keys(&["grappled"]), 6, &[]), None);
    }

    /* ---------------- all of it at once ---------------- */

    #[test]
    fn an_exhausted_unt_gar_with_boots() {
        // 25 natural, +10 boots, exhaustion 2: (25 + 10) / 2 = 17,
        // which is three squares with two feet left over that buy
        // nothing.
        let g = [grant(TARGET, Mode::Add, 10)];
        let s = resolve(Some(25), &keys(&["poisoned"]), 2, &g).unwrap();
        assert_eq!(s.feet, 17);
        assert_eq!(s.squares, 3);
    }
}
