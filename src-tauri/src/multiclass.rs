//! A character who is more than one thing.
//!
//! 055 gave a character `class_key` and `level`: ONE class, and the
//! level column was both the class's level and the character's. For a
//! single-classed character those are the same number, which is why it
//! worked and why it could not be stretched. A Fighter 5 / Rogue 3 has
//! three different levels at once - 5, 3, and the 8 that decides their
//! proficiency bonus - and no single column holds that.
//!
//! 073 moves the fact to `character_classes`, one row per class, and
//! this is where the arithmetic over those rows lives. The ordering of
//! the slice is MEANINGFUL AND IS THE ORDER THEY WERE TAKEN - the
//! database sorts by `added_at` and every function here leans on it,
//! because 5e's hit point rule and "which did you start as" both do.
//!
//! THREE RULES THAT ARE EASY TO GET WRONG, and are wrong in a lot of
//! character sheets:
//!
//! - The proficiency bonus is from the TOTAL. A Fighter 5 / Rogue 3 has
//!   the +3 of a level 8, not the +3 of a level 5 and the +2 of a
//!   level 3.
//! - The first level's maximum hit die belongs to the STARTING class
//!   and is handed out once in a career, not once per class. Taking a
//!   second class at level 6 does not pay out another full die.
//! - Extra Attack DOES NOT STACK. A Fighter 5 / Ranger 5 has two
//!   attacks, not three - the features overlap rather than add, which
//!   is the one place "take the best" is the rule rather than a
//!   simplification.
//!
//! WHAT IS NOT HERE, deliberately: spell slots. Multiclass casting adds
//! levels on a separate table with half and third progressions, and
//! nothing in this engine casts anything yet. When it does it gets its
//! own file rather than a fourth rule in this one.

/// One class a character holds, with what it costs to look it up
/// already resolved. Built by the caller from `character_classes`
/// joined to the class catalogue.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Taken {
    pub key: String,
    pub level: i64,
    /// The class's hit die - 6, 8, 10 or 12. Carried on the row rather
    /// than looked up here, because a dangling `class_key` is a real
    /// state (055 cannot use a foreign key) and resolving it is the
    /// caller's problem, not this module's.
    ///
    /// ZERO MEANS THE CATALOGUE HAS NO SUCH CLASS. `class::load_taken`
    /// fills it that way rather than dropping the row, because a class
    /// a character genuinely holds should still appear on their sheet
    /// when the DM renames a catalogue entry out from under it. No
    /// hit-point arithmetic ever sees a zero: `rederive_hp_max` checks
    /// for one and refuses, the same refusal 055 has always given a
    /// dangling key. `hp` does not judge - see its own note.
    pub hit_die: i64,
}

/// The ceiling on a character's total level. 5e's, and the same bound
/// `set_level` has always enforced on the single-class column.
pub const MAX_LEVEL: i64 = 20;

/// What level this character is: the sum, not the largest.
pub fn total(taken: &[Taken]) -> i64 {
    taken.iter().map(|t| t.level).sum()
}

/// Which class leads - the one with the most levels in it.
///
/// A TIE GOES TO THE ONE TAKEN FIRST, which is why the order of the
/// slice matters. A Fighter 4 / Rogue 4 who started as a Fighter reads
/// as a Fighter, because that is the answer a player gives when asked.
/// `max_by_key` would return the LAST of equal keys, so this folds by
/// hand rather than quietly picking the newest.
pub fn primary(taken: &[Taken]) -> Option<&Taken> {
    taken.iter().fold(None, |best: Option<&Taken>, t| match best {
        Some(b) if b.level >= t.level => Some(b),
        _ => Some(t),
    })
}

/// Everything except the primary, in the order taken.
pub fn others(taken: &[Taken]) -> Vec<&Taken> {
    let lead = primary(taken).map(|p| p.key.as_str());
    taken
        .iter()
        .filter(|t| Some(t.key.as_str()) != lead)
        .collect()
}

/// The same classes, with the one that leads moved to the front.
///
/// THE ORDER A SHEET READS IN, and it is computed here rather than on
/// the screen for the reason every rule in this file is: the screen
/// would be a second copy, and a second copy of "which class leads" is
/// how a roster and a sheet come to name a character differently.
///
/// THE REST KEEP THE ORDER THEY WERE TAKEN IN, so the additional-class
/// rows do not reshuffle when a level moves.
///
/// NOT THE ORDER FOR HIT POINTS. `hp` needs the STARTING class first
/// and this puts the BIGGEST first, which are different questions with
/// the same shape - a Fighter 1 / Wizard 5 leads as a Wizard and still
/// paid a d10 for level one. Pass `hp` the rows as they were loaded.
pub fn lead_first(taken: &[Taken]) -> Vec<Taken> {
    let Some(lead) = primary(taken) else {
        return Vec::new();
    };
    let mut out = vec![lead.clone()];
    out.extend(others(taken).into_iter().cloned());
    out
}

/// Maximum hit points across every class held.
///
/// THE FULL DIE IS PAID ONCE, by the starting class - `taken[0]`, which
/// is the earliest `added_at`. Every other level in the character's
/// career, in whatever class, is the fixed value of THAT class's die:
/// a Fighter 5 / Wizard 3 gets three d6 levels, not three d10 ones.
///
/// CONSTITUTION APPLIES PER LEVEL, including the first, exactly as
/// `vitality::pc_hp` has it. A test pins the two against each other for
/// the single-class case, because a character who takes a second class
/// and then abandons it must land back on the number they started with.
///
/// A HIT DIE OF ZERO IS ARITHMETIC, NOT A JUDGEMENT. It yields one
/// point per level and is nonsense as hit points - which is why the
/// caller refuses before it gets here rather than why this function
/// returns an error. See `Taken::hit_die`.
pub fn hp(taken: &[Taken], con_mod: i64) -> i64 {
    let Some(first) = taken.first() else {
        return 1;
    };
    if total(taken) < 1 {
        return 1;
    }
    // Level one: the starting class's whole die.
    let mut hp = first.hit_die + con_mod;
    for (i, t) in taken.iter().enumerate() {
        // The starting class has already been paid for its first.
        let after = if i == 0 { t.level - 1 } else { t.level };
        hp += after * (t.hit_die / 2 + 1 + con_mod);
    }
    hp.max(1)
}

/// How many swings the Attack action buys, given what each class grants
/// on its own.
///
/// THE BEST, NOT THE SUM. 5e says it outright: a character who gains
/// Extra Attack from more than one class does not add them together. A
/// Fighter 5 / Ranger 5 swings twice. Adding would give three, and is
/// the single most common multiclass arithmetic error there is.
///
/// ONE WHEN THERE IS NOTHING - a character with no class still gets the
/// one attack everybody gets.
pub fn attacks(per_class: &[i64]) -> i64 {
    per_class.iter().copied().max().unwrap_or(1).max(1)
}

/// Whether this character has room to be `level` in `key`.
///
/// CHECKED AGAINST THE TOTAL AFTER THE CHANGE, with the class's own
/// current levels taken back out first - otherwise raising a Fighter
/// from 5 to 6 would be read as adding six more levels to a character
/// who already has five of them.
pub fn room_for(taken: &[Taken], key: &str, level: i64) -> Result<(), String> {
    if level < 1 {
        return Err("a class is at least level 1 - remove it instead".to_string());
    }
    if level > MAX_LEVEL {
        return Err(format!("a class goes no higher than {}", MAX_LEVEL));
    }
    let without: i64 = taken
        .iter()
        .filter(|t| t.key != key)
        .map(|t| t.level)
        .sum();
    let after = without + level;
    if after > MAX_LEVEL {
        return Err(format!(
            "that would make them level {} - {} is the ceiling",
            after, MAX_LEVEL
        ));
    }
    Ok(())
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn t(key: &str, level: i64, die: i64) -> Taken {
        Taken { key: key.into(), level, hit_die: die }
    }

    /* ---------- the total ---------- */

    #[test]
    fn a_character_level_is_the_sum_and_not_the_largest() {
        let who = [t("fighter", 5, 10), t("rogue", 3, 8)];
        assert_eq!(total(&who), 8);
    }

    #[test]
    fn one_class_is_its_own_total() {
        assert_eq!(total(&[t("fighter", 4, 10)]), 4);
    }

    #[test]
    fn nobody_at_all_is_level_nothing() {
        assert_eq!(total(&[]), 0);
    }

    /* ---------- who leads ---------- */

    #[test]
    fn the_primary_is_the_one_with_the_most_levels() {
        let who = [t("rogue", 3, 8), t("fighter", 5, 10)];
        assert_eq!(primary(&who).map(|p| p.key.as_str()), Some("fighter"));
    }

    #[test]
    fn a_tie_goes_to_the_one_taken_first() {
        // Started as a Fighter, picked up Rogue later, both at 4. Ask a
        // player what they are and they say Fighter.
        let who = [t("fighter", 4, 10), t("rogue", 4, 8)];
        assert_eq!(primary(&who).map(|p| p.key.as_str()), Some("fighter"));
    }

    #[test]
    fn the_tie_break_is_not_the_newest() {
        // The bug `max_by_key` would have written: equal keys return
        // the LAST, so a tie would silently promote whatever was added
        // most recently.
        let who = [t("rogue", 4, 8), t("fighter", 4, 10)];
        assert_eq!(primary(&who).map(|p| p.key.as_str()), Some("rogue"));
    }

    #[test]
    fn nobody_leads_an_empty_list() {
        assert!(primary(&[]).is_none());
    }

    #[test]
    fn the_others_are_everything_but_the_lead() {
        let who = [t("fighter", 5, 10), t("rogue", 3, 8), t("cleric", 1, 8)];
        let rest: Vec<&str> = others(&who).iter().map(|t| t.key.as_str()).collect();
        assert_eq!(rest, vec!["rogue", "cleric"]);
    }

    #[test]
    fn a_single_class_has_no_others() {
        assert!(others(&[t("fighter", 5, 10)]).is_empty());
    }

    #[test]
    fn the_lead_comes_first_and_the_rest_keep_their_order() {
        let who = [t("rogue", 1, 8), t("fighter", 5, 10), t("cleric", 2, 8)];
        let shown: Vec<String> = lead_first(&who).into_iter().map(|t| t.key).collect();
        assert_eq!(shown, ["fighter", "rogue", "cleric"]);
    }

    #[test]
    fn one_class_leads_itself() {
        let who = [t("fighter", 5, 10)];
        assert_eq!(lead_first(&who), who.to_vec());
    }

    #[test]
    fn nobody_leads_nothing() {
        assert!(lead_first(&[]).is_empty());
    }

    #[test]
    fn the_display_order_is_not_the_hit_point_order() {
        // A Fighter 1 / Wizard 5 READS as a Wizard and still paid a
        // d10 for level one, because they started as the Fighter. The
        // two orders are different questions and `hp` must be given the
        // rows as loaded.
        let as_taken = [t("fighter", 1, 10), t("wizard", 5, 6)];
        assert_eq!(lead_first(&as_taken)[0].key, "wizard");
        // 10 + 5*4 = 30, not the 6 + ... a wizard-first career gives.
        assert_eq!(hp(&as_taken, 0), 30);
    }

    /* ---------- hit points ---------- */

    #[test]
    fn one_class_agrees_with_the_single_class_rule() {
        // THE PIN. 061's `pc_hp` is the rule for a character with one
        // class and must stay the rule - a character who takes a second
        // class and drops it again has to land back on the number they
        // started with.
        for die in [6, 8, 10, 12] {
            for level in 1..=20 {
                for con in -1..=4 {
                    assert_eq!(
                        hp(&[t("x", level, die)], con),
                        crate::vitality::pc_hp(die, level, con),
                        "die {} level {} con {}",
                        die,
                        level,
                        con
                    );
                }
            }
        }
    }

    #[test]
    fn the_full_die_is_paid_once_in_a_career() {
        // Fighter 5 / Rogue 3, Constitution +2.
        //   level 1 as a Fighter: 10 + 2        = 12
        //   four more Fighter levels: 4 * (6+2) = 32
        //   three Rogue levels:       3 * (5+2) = 21
        //                                         65
        let who = [t("fighter", 5, 10), t("rogue", 3, 8)];
        assert_eq!(hp(&who, 2), 65);
    }

    #[test]
    fn the_second_class_does_not_pay_out_another_maximum() {
        // The error this guards: handing the full die to every class.
        // That would give the Rogue above 8 + 2 for its first level
        // instead of 5 + 2, three hit points the character never had.
        let who = [t("fighter", 5, 10), t("rogue", 3, 8)];
        let wrong = crate::vitality::pc_hp(10, 5, 2) + crate::vitality::pc_hp(8, 3, 2);
        assert!(hp(&who, 2) < wrong);
    }

    #[test]
    fn which_class_was_started_changes_the_answer() {
        // Same two classes, different history. The starting class is
        // paid its WHOLE die for level one where every other level of
        // it is only die/2+1, so starting as the Fighter is worth
        // (10 - 6) - (6 - 4) = 2 hit points for the rest of a career.
        //
        // Fighter first: 10 + 2*6 + 3*4 = 34
        // Wizard first:   6 + 2*4 + 3*6 = 32
        let fighter_first = [t("fighter", 3, 10), t("wizard", 3, 6)];
        let wizard_first = [t("wizard", 3, 6), t("fighter", 3, 10)];
        assert_eq!(hp(&fighter_first, 0), 34);
        assert_eq!(hp(&wizard_first, 0), 32);
    }

    #[test]
    fn three_classes_add_up() {
        // Fighter 1 / Rogue 1 / Cleric 1, Constitution 0.
        //   10 + 5 + 5 = 20
        let who = [t("fighter", 1, 10), t("rogue", 1, 8), t("cleric", 1, 8)];
        assert_eq!(hp(&who, 0), 20);
    }

    #[test]
    fn a_punishing_constitution_still_leaves_them_alive() {
        let who = [t("wizard", 1, 6), t("rogue", 1, 8)];
        assert!(hp(&who, -5) >= 1);
    }

    #[test]
    fn a_class_the_catalogue_cannot_find_is_arithmetic_not_a_crash() {
        // Pinned so the sentinel is not a surprise. `rederive_hp_max`
        // refuses before this is reached; what it would do if it did
        // not is written down rather than left to be discovered.
        // A whole die of nothing for level one, then die/2+1 = 1 for
        // each of the other two.
        assert_eq!(hp(&[t("ghost", 3, 0)], 0), 2);
    }

    #[test]
    fn nothing_taken_is_one_hit_point_rather_than_a_panic() {
        assert_eq!(hp(&[], 3), 1);
    }

    /* ---------- extra attack ---------- */

    #[test]
    fn extra_attack_does_not_stack_across_classes() {
        // Fighter 5 and Ranger 5 each grant a second attack. Two, not
        // three - 5e says so outright and adding is the usual mistake.
        assert_eq!(attacks(&[2, 2]), 2);
    }

    #[test]
    fn the_best_class_decides() {
        // Fighter 11 grants three; the Rogue beside it grants one.
        assert_eq!(attacks(&[3, 1]), 3);
    }

    #[test]
    fn no_class_still_swings_once() {
        assert_eq!(attacks(&[]), 1);
        assert_eq!(attacks(&[0]), 1);
    }

    /* ---------- the ceiling ---------- */

    #[test]
    fn twenty_levels_is_the_whole_of_it() {
        let who = [t("fighter", 15, 10)];
        assert!(room_for(&who, "rogue", 5).is_ok());
        assert!(room_for(&who, "rogue", 6).is_err());
    }

    #[test]
    fn raising_a_class_counts_its_own_levels_once() {
        // The arithmetic error this exists to stop: a Fighter 5 raised
        // to 6 is one more level, not six more. Without taking the
        // class's own levels back out first, a Fighter 15 could not be
        // raised to 16.
        let who = [t("fighter", 15, 10)];
        assert!(room_for(&who, "fighter", 16).is_ok());
        assert!(room_for(&who, "fighter", 20).is_ok());
    }

    #[test]
    fn a_class_below_first_level_is_a_removal_not_a_level() {
        assert!(room_for(&[], "fighter", 0).is_err());
        assert!(room_for(&[], "fighter", -1).is_err());
    }

    #[test]
    fn no_single_class_goes_past_twenty_either() {
        assert!(room_for(&[], "fighter", 21).is_err());
        assert!(room_for(&[], "fighter", 20).is_ok());
    }

    #[test]
    fn the_message_says_what_the_total_would_have_been() {
        let who = [t("fighter", 15, 10)];
        let err = room_for(&who, "rogue", 9).unwrap_err();
        assert!(err.contains("24"), "unhelpful message: {}", err);
    }
}
