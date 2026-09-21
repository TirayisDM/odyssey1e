//! Dying, and what it takes to stop.
//!
//! Nobody in this system dies from a damage roll. Dropping to zero makes
//! a creature UNCONSCIOUS and dying; what settles it is three death
//! saves one way or the other. That is the 5e shape, and this campaign
//! extends it to NPCs, who by the book simply die at zero. A goblin here
//! bleeds out like anyone else.
//!
//! TWO HOUSE RULES, MARKED SO NOBODY LATER MISTAKES THEM FOR 5e.
//!
//! NPCs roll death saves at all. Deliberate, and the reason
//! `encounter_actors` carries the same two counters `characters` has
//! had since 010.
//!
//! AND AN UNCONSCIOUS CREATURE HAS AC 0. By the book it keeps its armour
//! class and attackers get advantage, with melee hits inside five feet
//! landing as crits. Flat zero is simpler, reads plainly at the table,
//! and is what was asked for. It is not rules as written.
//!
//! EVERYTHING ELSE IS 5e AND IS DELIBERATELY NOT INVENTED:
//!   a natural 20 on a death save regains one hit point
//!   a natural 1 counts as two failures
//!   ten or better succeeds, nine or less fails - no modifier applies
//!   three successes stabilise, three failures kill
//!   being hit while down is a failure, and two on a crit
//!   damage whose overflow meets the hit point maximum kills outright

/// What state a creature is in, derived rather than stored.
///
/// Deriving it means there is no status column to forget to update, and
/// no way for the counters and the word to disagree. Same reasoning as
/// the challenge watermark in 011.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Condition {
    /// Up and acting.
    Conscious,
    /// At zero and dying: unconscious, rolling death saves.
    Down,
    /// At zero, unconscious, no longer dying. Three successes.
    Stable,
    /// Three failures, or damage that overflowed the hit point maximum.
    Dead,
}

impl Condition {
    /// Whether the creature is on its feet. Everything that is not
    /// conscious is unconscious, including stable and dead.
    pub fn is_conscious(self) -> bool {
        matches!(self, Condition::Conscious)
    }
}

/// What state the numbers add up to.
///
/// `dead` is a flag rather than a count because two different things
/// produce it - three failures, and overflow damage - and only one of
/// them leaves evidence in the counters.
pub fn condition(hp_current: i64, successes: i64, failures: i64, dead: bool) -> Condition {
    if dead || failures >= 3 {
        return Condition::Dead;
    }
    if hp_current > 0 {
        // Healed back up. The counters are stale until something clears
        // them, so hit points win: a creature on its feet is conscious
        // whatever its death saves said a moment ago.
        return Condition::Conscious;
    }
    if successes >= 3 {
        Condition::Stable
    } else {
        Condition::Down
    }
}

/// What it takes to hit them now.
///
/// HOUSE RULE: zero while unconscious. See the module header - 5e keeps
/// the armour class and grants advantage instead.
pub fn effective_ac(base_ac: i64, condition: Condition) -> i64 {
    if condition.is_conscious() {
        base_ac
    } else {
        0
    }
}

/// Whether this creature rolls a death save at all.
///
/// Only something that is DYING does. Three states are refused and they
/// are refused for different reasons, so each says its own:
///
///   Conscious  is not making death saves, it is on its feet.
///   Stable     is three successes. Still unconscious, no longer dying,
///              and NOT rolling - this is the one the command missed.
///              Letting a stable creature roll again is not a harmless
///              extra die: a failure would start it dying a second time
///              from a state the rules say it has left.
///   Dead       is over. Nothing further to determine.
///
/// A rule rather than a guard in the command, because "may this thing
/// roll" is a question about the game and gets tested like one. The
/// command's job is to ask.
pub fn may_roll_death_save(condition: Condition) -> Result<(), String> {
    match condition {
        Condition::Down => Ok(()),
        Condition::Conscious => {
            Err("only something that is down rolls death saves".to_string())
        }
        Condition::Stable => {
            Err("it is stable - three successes, and no longer dying".to_string())
        }
        Condition::Dead => Err("it is already dead".to_string()),
    }
}

/// How a label reads once someone is off their feet.
///
/// "Goblin 1 - down" rather than "Goblin 1 - dead" while the death saves
/// are still running, because a creature that is rolling them is dying
/// and not yet dead. The word changes when the thing it describes does.
pub fn label_suffix(condition: Condition) -> Option<&'static str> {
    match condition {
        Condition::Conscious => None,
        Condition::Down => Some("down"),
        Condition::Stable => Some("stable"),
        Condition::Dead => Some("dead"),
    }
}

/// The result of one death saving throw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveResult {
    pub successes: i64,
    pub failures: i64,
    /// A natural 20 brings them back up on one hit point. The counters
    /// are cleared in the same breath - a creature back on its feet is
    /// not two failures into dying.
    pub hp_regain: i64,
    pub clears_counters: bool,
}

/// A death saving throw, from the raw d20.
///
/// No modifier applies, which is why this takes a face rather than a
/// total. A creature with a feature that changes that is a later
/// problem, and it will change this function rather than its callers.
pub fn save(natural: i64) -> SaveResult {
    if natural >= 20 {
        return SaveResult {
            successes: 0,
            failures: 0,
            hp_regain: 1,
            clears_counters: true,
        };
    }
    if natural <= 1 {
        // A natural 1 is two failures, which is what makes three rounds
        // of bad luck genuinely lethal.
        return SaveResult {
            successes: 0,
            failures: 2,
            hp_regain: 0,
            clears_counters: false,
        };
    }
    if natural >= 10 {
        SaveResult { successes: 1, failures: 0, hp_regain: 0, clears_counters: false }
    } else {
        SaveResult { successes: 0, failures: 1, hp_regain: 0, clears_counters: false }
    }
}

/// Failures earned by being hit while already down.
///
/// The cost of letting a downed creature stay targetable: hitting one is
/// not free, and a crit is worth two.
pub fn failures_from_being_hit(crit: bool) -> i64 {
    if crit {
        2
    } else {
        1
    }
}

/// Did that kill outright?
///
/// 5e: damage that drops a creature to zero AND has enough left over to
/// meet its hit point maximum kills instantly, with no saves to make.
/// `overflow` is how far below zero the total went.
///
/// Goblin 1 is the worked example: seven hit points, fourteen taken,
/// seven of overflow. Exactly lethal.
pub fn massive_damage_kills(overflow: i64, hp_max: i64) -> bool {
    overflow >= hp_max && hp_max > 0
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    /* ---------------- who may roll ----------------------------------- */

    #[test]
    fn only_the_dying_roll_death_saves() {
        assert!(may_roll_death_save(Condition::Down).is_ok());
        assert!(may_roll_death_save(Condition::Conscious).is_err());
        assert!(may_roll_death_save(Condition::Stable).is_err());
        assert!(may_roll_death_save(Condition::Dead).is_err());
    }

    #[test]
    fn stable_is_refused_and_says_why() {
        // The case the command missed. Stable is neither conscious nor
        // dead, so a guard that only checks those two lets a creature
        // that has finished dying start again.
        let e = may_roll_death_save(Condition::Stable).unwrap_err();
        assert!(e.contains("stable"), "{}", e);
        assert!(e.contains("no longer dying"), "{}", e);
    }

    #[test]
    fn each_refusal_says_its_own_reason() {
        // Three different situations; three different sentences. "Cannot
        // roll" for all of them would leave the DM guessing which.
        let c = may_roll_death_save(Condition::Conscious).unwrap_err();
        let s = may_roll_death_save(Condition::Stable).unwrap_err();
        let d = may_roll_death_save(Condition::Dead).unwrap_err();
        assert_ne!(c, s);
        assert_ne!(s, d);
        assert_ne!(c, d);
    }

    #[test]
    fn three_successes_then_a_failure_cannot_happen() {
        // The sequence the missing guard allowed: stabilise, then roll
        // again and fail. The rule refuses at the point of rolling, so
        // the failure never gets a chance to be recorded.
        let after_three_successes = condition(0, 3, 0, false);
        assert_eq!(after_three_successes, Condition::Stable);
        assert!(may_roll_death_save(after_three_successes).is_err());
    }

    /* ---------------- the condition ---------------------------------- */

    #[test]
    fn above_zero_is_conscious() {
        assert_eq!(condition(7, 0, 0, false), Condition::Conscious);
        assert_eq!(condition(1, 0, 0, false), Condition::Conscious);
    }

    #[test]
    fn zero_is_down_not_dead() {
        // The whole point: dropping does not kill, it starts the dying.
        assert_eq!(condition(0, 0, 0, false), Condition::Down);
        assert_eq!(condition(-7, 0, 0, false), Condition::Down);
    }

    #[test]
    fn three_successes_stabilise_and_three_failures_kill() {
        assert_eq!(condition(0, 3, 0, false), Condition::Stable);
        assert_eq!(condition(0, 0, 3, false), Condition::Dead);
    }

    #[test]
    fn failures_beat_successes_when_both_are_full() {
        // Not reachable through the rules, but a row can hold anything
        // and dead has to win over stable rather than the order of the
        // checks deciding it.
        assert_eq!(condition(0, 3, 3, false), Condition::Dead);
    }

    #[test]
    fn healing_above_zero_makes_them_conscious_whatever_the_counters_say() {
        assert_eq!(condition(4, 0, 2, false), Condition::Conscious);
    }

    #[test]
    fn the_dead_flag_wins_over_everything() {
        // Overflow damage kills with no failures recorded, so the flag
        // has to outrank hit points and counters both.
        assert_eq!(condition(0, 3, 0, true), Condition::Dead);
        assert_eq!(condition(9, 0, 0, true), Condition::Dead);
    }

    /* ---------------- armour class ----------------------------------- */

    #[test]
    fn an_unconscious_creature_has_no_armour_class() {
        assert_eq!(effective_ac(15, Condition::Conscious), 15);
        assert_eq!(effective_ac(15, Condition::Down), 0);
        assert_eq!(effective_ac(15, Condition::Stable), 0);
        assert_eq!(effective_ac(15, Condition::Dead), 0);
    }

    /* ---------------- the save --------------------------------------- */

    #[test]
    fn ten_or_better_succeeds_and_nine_or_less_fails() {
        assert_eq!(save(10).successes, 1);
        assert_eq!(save(19).successes, 1);
        assert_eq!(save(9).failures, 1);
        assert_eq!(save(2).failures, 1);
    }

    #[test]
    fn a_natural_one_is_two_failures() {
        assert_eq!(save(1).failures, 2);
        assert_eq!(save(1).successes, 0);
    }

    #[test]
    fn a_natural_twenty_brings_them_back_on_one_hit_point() {
        let r = save(20);
        assert_eq!(r.hp_regain, 1);
        assert!(r.clears_counters);
        // Not a success — they are up, which is better than a tally.
        assert_eq!(r.successes, 0);
        assert_eq!(r.failures, 0);
    }

    #[test]
    fn nothing_but_a_twenty_clears_the_counters() {
        for n in 1..=19 {
            assert!(!save(n).clears_counters, "{} cleared them", n);
        }
    }

    #[test]
    fn three_saves_of_nine_are_lethal_and_three_of_ten_are_not() {
        let mut f = 0;
        for _ in 0..3 {
            f += save(9).failures;
        }
        assert_eq!(condition(0, 0, f, false), Condition::Dead);

        let mut s = 0;
        for _ in 0..3 {
            s += save(10).successes;
        }
        assert_eq!(condition(0, s, 0, false), Condition::Stable);
    }

    #[test]
    fn two_natural_ones_are_enough() {
        let f = save(1).failures + save(1).failures;
        assert_eq!(f, 4);
        assert_eq!(condition(0, 0, f, false), Condition::Dead);
    }

    /* ---------------- being hit while down --------------------------- */

    #[test]
    fn hitting_someone_who_is_down_costs_them() {
        assert_eq!(failures_from_being_hit(false), 1);
        assert_eq!(failures_from_being_hit(true), 2);
    }

    /* ---------------- overflow --------------------------------------- */

    #[test]
    fn goblin_one_is_the_worked_example() {
        // Seven hit points, fourteen damage, seven of overflow. Exactly
        // lethal, and the reason this rule is in rather than skipped.
        assert!(massive_damage_kills(7, 7));
    }

    #[test]
    fn one_short_of_the_maximum_only_drops_them() {
        assert!(!massive_damage_kills(6, 7));
    }

    #[test]
    fn a_clean_drop_to_zero_is_not_massive_damage() {
        assert!(!massive_damage_kills(0, 7));
    }

    #[test]
    fn a_creature_with_no_maximum_cannot_be_killed_by_overflow() {
        // hp_max unset reads as zero, and `overflow >= 0` would then be
        // true for everything. Guarded, because that would silently kill
        // every actor whose statblock is incomplete.
        assert!(!massive_damage_kills(0, 0));
        assert!(!massive_damage_kills(50, 0));
    }
}
