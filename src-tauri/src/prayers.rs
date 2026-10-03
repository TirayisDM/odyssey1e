//! What a cleric can prepare, and what they can spend it with.
//!
//! 106. THE CLERIC IS A PREPARED CASTER and that is the whole shape of
//! this: the list they can draw from is the ENTIRE cleric list, every
//! day, and what limits them is how many they may hold at once and how
//! many slots they have to spend.
//!
//! That is different from a bard or a sorcerer, who KNOW a small
//! number permanently. Nothing here would serve them, which is why the
//! module is named for the cleric rather than for casting.
//!
//! ---------------------------------------------------------------------
//! THREE NUMBERS, AND THEY COME FROM DIFFERENT PLACES
//! ---------------------------------------------------------------------
//!
//!   PREPARED   Wisdom modifier + cleric level, minimum 1. Changes
//!              when either does, which is why it is derived on every
//!              read rather than stored.
//!   CANTRIPS   3, then 4 at level 4 and 5 at level 10. Known rather
//!              than prepared - they are always there and never cost a
//!              slot.
//!   SLOTS      The full-caster table. A cleric 5 has 4/3/2, and the
//!              table is the only part of this that is simply data.
//!
//! THE LEVEL IS THE CLERIC'S, never the character's total. A Fighter 4
//! / Cleric 1 prepares as a cleric 1 - which is the same rule
//! `features::held` and `uses::Context` already follow, for the same
//! reason.
//!
//! WHAT IS NOT HERE: multiclass slots. A character with levels in two
//! casting classes has ONE pool of slots worked out from a combined
//! table, with half and third progressions for paladins and rangers.
//! That is a real rule and it needs more than one class to be worth
//! writing - `slots_at` takes a single level and says so.

/// How many spells a cleric may have prepared.
///
/// MINIMUM ONE, which is 5e's own floor: a cleric with Wisdom 10 at
/// level 1 still prepares something. Without it a dump-stat cleric
/// would prepare nothing at all.
pub fn prepared_max(cleric_level: i64, wis_mod: i64) -> i64 {
    (cleric_level.max(0) + wis_mod).max(1)
}

/// How many cantrips a cleric knows.
///
/// KNOWN, NOT PREPARED. They do not come out of the prepared count and
/// they never cost a slot - which is why they are a separate number
/// rather than a line in the same budget.
pub fn cantrips_known(cleric_level: i64) -> i64 {
    match cleric_level {
        l if l >= 10 => 5,
        l if l >= 4 => 4,
        l if l >= 1 => 3,
        _ => 0,
    }
}

/// Spell slots by cleric level: index 0 is 1st-level slots.
///
/// THE FULL-CASTER TABLE, which the cleric, bard, druid, sorcerer and
/// wizard all share. Written out rather than computed because it is
/// not a formula - the jumps at 11th and 13th are the book's own
/// shape, and a clever closed form would be a different table that
/// happened to agree for a while.
pub fn slots_at(cleric_level: i64) -> [i64; 9] {
    match cleric_level.clamp(0, 20) {
        0 => [0, 0, 0, 0, 0, 0, 0, 0, 0],
        1 => [2, 0, 0, 0, 0, 0, 0, 0, 0],
        2 => [3, 0, 0, 0, 0, 0, 0, 0, 0],
        3 => [4, 2, 0, 0, 0, 0, 0, 0, 0],
        4 => [4, 3, 0, 0, 0, 0, 0, 0, 0],
        5 => [4, 3, 2, 0, 0, 0, 0, 0, 0],
        6 => [4, 3, 3, 0, 0, 0, 0, 0, 0],
        7 => [4, 3, 3, 1, 0, 0, 0, 0, 0],
        8 => [4, 3, 3, 2, 0, 0, 0, 0, 0],
        9 => [4, 3, 3, 3, 1, 0, 0, 0, 0],
        10 => [4, 3, 3, 3, 2, 0, 0, 0, 0],
        11 | 12 => [4, 3, 3, 3, 2, 1, 0, 0, 0],
        13 | 14 => [4, 3, 3, 3, 2, 1, 1, 0, 0],
        15 | 16 => [4, 3, 3, 3, 2, 1, 1, 1, 0],
        17 => [4, 3, 3, 3, 2, 1, 1, 1, 1],
        18 => [4, 3, 3, 3, 3, 1, 1, 1, 1],
        19 => [4, 3, 3, 3, 3, 2, 1, 1, 1],
        _ => [4, 3, 3, 3, 3, 2, 2, 1, 1],
    }
}

/// The highest spell level this cleric can cast at all.
pub fn top_slot(cleric_level: i64) -> i64 {
    slots_at(cleric_level)
        .iter()
        .rposition(|n| *n > 0)
        .map(|i| i as i64 + 1)
        .unwrap_or(0)
}

/// The save DC for this cleric's spells: 8 + proficiency + Wisdom.
///
/// A PROPERTY OF THE CASTER, which is exactly what 101 cleared the
/// baked 15 out of the catalogue for.
pub fn save_dc(prof_bonus: i64, wis_mod: i64) -> i64 {
    8 + prof_bonus + wis_mod
}

/// The attack bonus for a spell that makes an attack roll.
pub fn attack_bonus(prof_bonus: i64, wis_mod: i64) -> i64 {
    prof_bonus + wis_mod
}

/// Whether a spell may be prepared, given what is already prepared.
///
/// FOUR REFUSALS AND THEY ARE DIFFERENT MISTAKES: a cantrip is not
/// prepared at all, a spell above your slots cannot be held, the list
/// is full, and it is already there. Each says which.
pub fn may_prepare(
    spell_level: i64,
    cleric_level: i64,
    wis_mod: i64,
    already: &[String],
    key: &str,
) -> Result<(), String> {
    if spell_level == 0 {
        return Err("a cantrip is known rather than prepared".to_string());
    }
    if already.iter().any(|k| k == key) {
        return Err("already prepared".to_string());
    }
    let top = top_slot(cleric_level);
    if spell_level > top {
        return Err(match top {
            0 => "they have no spell slots yet".to_string(),
            t => format!("that is a level {} spell and they cast up to {}", spell_level, t),
        });
    }
    let max = prepared_max(cleric_level, wis_mod);
    if already.len() as i64 >= max {
        return Err(format!(
            "that is {} prepared and they may hold {} - put one down first",
            already.len(),
            max
        ));
    }
    Ok(())
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    /* ---------- how many they hold ---------- */

    #[test]
    fn prepared_is_wisdom_plus_cleric_level() {
        assert_eq!(prepared_max(1, 3), 4);
        assert_eq!(prepared_max(5, 3), 8);
        assert_eq!(prepared_max(20, 5), 25);
    }

    #[test]
    fn and_never_fewer_than_one() {
        // 5e's own floor. Without it a cleric with Wisdom 8 at level 1
        // would prepare nothing at all.
        assert_eq!(prepared_max(1, -1), 1);
        assert_eq!(prepared_max(1, -5), 1);
    }

    /* ---------- cantrips ---------- */

    #[test]
    fn cantrips_are_three_then_four_then_five() {
        assert_eq!(cantrips_known(1), 3);
        assert_eq!(cantrips_known(3), 3);
        assert_eq!(cantrips_known(4), 4);
        assert_eq!(cantrips_known(9), 4);
        assert_eq!(cantrips_known(10), 5);
        assert_eq!(cantrips_known(20), 5);
    }

    #[test]
    fn nobody_at_level_zero_knows_any() {
        assert_eq!(cantrips_known(0), 0);
    }

    /* ---------- slots ---------- */

    #[test]
    fn a_first_level_cleric_has_two_firsts() {
        assert_eq!(slots_at(1), [2, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn a_fifth_level_cleric_has_four_three_two() {
        assert_eq!(slots_at(5)[0..3], [4, 3, 2]);
        assert_eq!(top_slot(5), 3);
    }

    #[test]
    fn the_table_tops_out_at_twenty() {
        assert_eq!(slots_at(20), [4, 3, 3, 3, 3, 2, 2, 1, 1]);
        assert_eq!(top_slot(20), 9);
        // Past the end of the table is still the end of the table.
        assert_eq!(slots_at(25), slots_at(20));
    }

    #[test]
    fn a_ninth_level_slot_arrives_at_seventeen() {
        assert_eq!(top_slot(16), 8);
        assert_eq!(top_slot(17), 9);
    }

    #[test]
    fn nobody_at_level_zero_has_a_slot() {
        assert_eq!(slots_at(0), [0; 9]);
        assert_eq!(top_slot(0), 0);
    }

    #[test]
    fn every_level_has_as_many_slots_as_the_one_before() {
        // The table only ever grows, which is worth pinning because it
        // was typed out by hand.
        for l in 1..=20 {
            let now = slots_at(l);
            let before = slots_at(l - 1);
            for i in 0..9 {
                assert!(
                    now[i] >= before[i],
                    "level {} lost a level-{} slot",
                    l,
                    i + 1
                );
            }
        }
    }

    /* ---------- the caster's own numbers ---------- */

    #[test]
    fn the_dc_is_eight_plus_proficiency_plus_wisdom() {
        assert_eq!(save_dc(2, 3), 13);
        assert_eq!(save_dc(4, 5), 17);
    }

    #[test]
    fn the_attack_bonus_is_the_same_without_the_eight() {
        assert_eq!(attack_bonus(2, 3), 5);
        assert_eq!(save_dc(2, 3) - attack_bonus(2, 3), 8);
    }

    /* ---------- what may be prepared ---------- */

    fn nothing() -> Vec<String> {
        Vec::new()
    }

    #[test]
    fn a_spell_within_reach_may_be_prepared() {
        assert!(may_prepare(1, 1, 3, &nothing(), "sp_bless").is_ok());
        assert!(may_prepare(3, 5, 3, &nothing(), "sp_revivify").is_ok());
    }

    #[test]
    fn a_cantrip_is_known_rather_than_prepared() {
        let err = may_prepare(0, 5, 3, &nothing(), "sp_guidance").unwrap_err();
        assert!(err.contains("known"), "unhelpful: {}", err);
    }

    #[test]
    fn a_spell_above_their_slots_is_refused_with_the_reason() {
        // A cleric 5 casts up to 3rd.
        let err = may_prepare(4, 5, 3, &nothing(), "sp_deathward").unwrap_err();
        assert!(err.contains("level 4") && err.contains("up to 3"), "unhelpful: {}", err);
    }

    #[test]
    fn a_cleric_with_no_slots_yet_is_told_that_instead() {
        let err = may_prepare(1, 0, 3, &nothing(), "sp_bless").unwrap_err();
        assert!(err.contains("no spell slots"), "unhelpful: {}", err);
    }

    #[test]
    fn a_full_list_says_how_full() {
        // Cleric 1 with Wisdom +3 holds four.
        let held: Vec<String> = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        let err = may_prepare(1, 1, 3, &held, "sp_bless").unwrap_err();
        assert!(err.contains('4'), "unhelpful: {}", err);
    }

    #[test]
    fn preparing_the_same_one_twice_is_refused_before_the_count() {
        // Which matters: a full list holding this very spell should say
        // "already prepared", not "put one down first".
        let held: Vec<String> = vec!["sp_bless".into()];
        let err = may_prepare(1, 1, -5, &held, "sp_bless").unwrap_err();
        assert!(err.contains("already"), "unhelpful: {}", err);
    }

    #[test]
    fn a_cleric_level_is_not_a_character_level() {
        // A Fighter 4 / Cleric 1 prepares as a cleric 1 - the same rule
        // features::held and uses::Context already follow. Passing 5
        // here would give them a 3rd-level slot they have not earned.
        assert_eq!(top_slot(1), 1);
        assert_eq!(prepared_max(1, 3), 4);
    }
}
