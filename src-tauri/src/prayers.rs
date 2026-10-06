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

use serde::{Deserialize, Serialize};

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

/* ======================== WHO IS CASTING ======================== */

/// Where a caster's list comes from, which is the real difference
/// between a cleric and a wizard.
///
/// 150. 101 to 107 built one shape and called it prayers, and the
/// naming hid a question: a cleric draws from the WHOLE cleric list
/// every day and prepares a subset of it, while a wizard may only
/// prepare what is written in a book they have been filling since
/// level one. Same slots, same preparing, different source - and the
/// source is where every other difference comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// The whole class list, every day. Cleric, druid, paladin.
    WholeList,
    /// Only what is in the book. The wizard, and only the wizard.
    Book,
}

/// A caster: which class, at what level, on what ability, off what.
///
/// RESOLVED FROM CLASS ROWS AND NOTHING ELSE, which is the whole of
/// 150's design. Six commands used to find a cleric class on the sheet
/// and refuse everybody else, so a Lich could not cast - a monster had
/// no classes at all. The answer was not a second mechanism for
/// creatures: it was giving a creature the class rows a player
/// character has, which is 022's rule that there is no monster branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Caster {
    pub class_key: String,
    /// What drives the slot table and the prepared count.
    pub level: i64,
    /// The ability code the DC and attack bonus are built on.
    pub ability: String,
    pub source: Source,
}

/// Which classes cast, on what, and from where.
///
/// A RULE RATHER THAN A COLUMN. `classes` carries hit dice, proficiency
/// and skill choices and has never said anything about spellcasting;
/// this is 5e's own table and it belongs where the rules live.
///
/// THE THREE THAT PREPARE, AND NOT THE ONES THAT KNOW. Bard, sorcerer
/// and warlock have a fixed KNOWN list and never prepare anything,
/// which is a third shape this module does not implement - so they are
/// absent rather than listed and quietly treated as clerics. Ranger is
/// absent for the same reason.
///
/// PALADIN IS ABSENT TOO, for a different one: it prepares from the
/// whole list like a cleric, but on a HALF-CASTER slot table, and
/// `slots_at` is the full table and says so in its own header.
pub fn casts(class_key: &str) -> Option<(&'static str, Source)> {
    match class_key {
        "cleric" => Some(("wis", Source::WholeList)),
        "druid" => Some(("wis", Source::WholeList)),
        "wizard" => Some(("int", Source::Book)),
        _ => None,
    }
}

/// The caster a sheet's class rows describe, if any.
///
/// THE HIGHEST CASTING CLASS WINS where there is more than one, which
/// is a simplification and worth naming: 5e multiclass spellcasting
/// adds the levels together on one shared slot table, and a Cleric 3 /
/// Wizard 3 has the slots of a 6th-level caster rather than of a 3rd.
/// 073 built multiclassing and this is the first rule that needs that
/// sum; computing it would mean deciding which ability the shared
/// slots cast on, which is not one answer. One class at a time is the
/// honest subset, and the day somebody plays that pair this is where
/// it gets fixed.
pub fn caster(classes: &[crate::multiclass::Taken]) -> Option<Caster> {
    classes
        .iter()
        .filter_map(|t| casts(&t.key).map(|(ability, source)| Caster {
            class_key: t.key.clone(),
            level: t.level,
            ability: ability.to_string(),
            source,
        }))
        .max_by_key(|c| c.level)
}

/// Whether this caster may reach for this spell at all, before any
/// question of how many they can hold.
///
/// 150. THE DIFFERENCE BETWEEN A CLERIC AND A WIZARD, in one function.
/// A cleric reaches for anything on the cleric list, every day, and
/// `spells.classes` is the list. A WIZARD REACHES ONLY INTO THE BOOK -
/// being a wizard spell is not enough, somebody has to have written it
/// down - so the test is what this caster already holds, not what the
/// catalogue says.
///
/// `held` IS THE STATE THIS CASTER HAS THE SPELL IN, or None for one
/// they have never held. For a book caster that is the whole question:
/// `book` means written down and not prepared today, which is exactly
/// the state a wizard prepares FROM.
pub fn may_reach(
    caster: &Caster,
    spell_classes: &[String],
    held: Option<&str>,
) -> Result<(), String> {
    match caster.source {
        Source::WholeList => match spell_classes.iter().any(|c| *c == caster.class_key) {
            true => Ok(()),
            false => Err(format!("that is not a {} spell", caster.class_key)),
        },
        Source::Book => match held {
            Some("book") | Some("prepared") => Ok(()),
            _ => Err("that is not in their book - a wizard prepares from what they have written down".to_string()),
        },
    }
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

/// How many slots of each level are left, given what has been spent.
///
/// 107. NEVER BELOW ZERO. A level lost, or a DM editing a class row,
/// can leave somebody having spent more than they now have, and
/// "minus one slot" helps nobody.
pub fn slots_left(cleric_level: i64, spent: &[i64; 9]) -> [i64; 9] {
    let have = slots_at(cleric_level);
    let mut left = [0; 9];
    for i in 0..9 {
        left[i] = (have[i] - spent[i].max(0)).max(0);
    }
    left
}

/// Whether a slot of this level can be spent.
///
/// A SLOT IS A SLOT. Nothing here asks what spell it is for, because a
/// 3rd-level slot can carry a 1st-level spell and often should -
/// tying expenditure to the spell would make upcasting
/// unrepresentable.
pub fn may_spend_slot(cleric_level: i64, spent: &[i64; 9], level: i64) -> Result<(), String> {
    if !(1..=9).contains(&level) {
        return Err("a spell slot is level 1 to 9".to_string());
    }
    let i = (level - 1) as usize;
    if slots_at(cleric_level)[i] == 0 {
        return Err(format!("they have no level {} slots", level));
    }
    if slots_left(cleric_level, spent)[i] == 0 {
        return Err(format!("no level {} slots left until they rest", level));
    }
    Ok(())
}

/// What a rest gives back, for a cleric.
///
/// EVERYTHING ON A LONG REST AND NOTHING ON A SHORT ONE. That is the
/// cleric's rule and not everybody's - a warlock's Pact Magic comes
/// back on a short rest - which is why this takes the class rather
/// than being a property of the slot.
pub fn slots_restored(class_key: &str, long: bool) -> bool {
    match class_key {
        "warlock" => true,
        _ => long,
    }
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

    /* ---------------- who is casting (150) -------------------------- */

    fn taken(key: &str, level: i64) -> crate::multiclass::Taken {
        // The hit die is carried on the row and is nothing to do with
        // casting; d8 keeps the fixture honest without pretending it
        // matters here.
        crate::multiclass::Taken { key: key.to_string(), level, hit_die: 8 }
    }

    #[test]
    fn a_cleric_casts_on_wisdom_from_the_whole_list() {
        let c = caster(&[taken("cleric", 5)]).unwrap();
        assert_eq!(c.ability, "wis");
        assert_eq!(c.level, 5);
        assert_eq!(c.source, Source::WholeList);
    }

    #[test]
    fn a_wizard_casts_on_intelligence_from_a_book() {
        let c = caster(&[taken("wizard", 18)]).unwrap();
        assert_eq!(c.ability, "int");
        assert_eq!(c.source, Source::Book);
    }

    #[test]
    fn a_fighter_is_not_a_caster() {
        assert!(caster(&[taken("fighter", 20)]).is_none());
        assert!(caster(&[]).is_none(), "and neither is a creature with no classes");
    }

    #[test]
    fn the_classes_that_know_rather_than_prepare_are_absent() {
        // Bard, sorcerer and warlock have a fixed known list and never
        // prepare. Treating them as clerics would hand them the whole
        // cleric list to prepare from, which is a worse answer than
        // "not built yet".
        for k in ["bard", "sorcerer", "warlock", "ranger", "paladin"] {
            assert!(casts(k).is_none(), "{} should not be built yet", k);
        }
    }

    #[test]
    fn the_highest_casting_class_wins() {
        // A simplification, and the test says so: 5e adds multiclass
        // caster levels on one shared table, and this takes the larger.
        let c = caster(&[taken("cleric", 3), taken("wizard", 5)]).unwrap();
        assert_eq!(c.class_key, "wizard");
        assert_eq!(c.level, 5);
    }

    #[test]
    fn a_non_casting_class_beside_a_casting_one_is_ignored() {
        let c = caster(&[taken("fighter", 11), taken("cleric", 2)]).unwrap();
        assert_eq!(c.class_key, "cleric");
        assert_eq!(c.level, 2);
    }

    /* ---------------- what a caster may reach for ------------------- */

    fn cleric5() -> Caster {
        Caster { class_key: "cleric".into(), level: 5, ability: "wis".into(), source: Source::WholeList }
    }
    fn wizard9() -> Caster {
        Caster { class_key: "wizard".into(), level: 9, ability: "int".into(), source: Source::Book }
    }

    #[test]
    fn a_cleric_reaches_for_anything_on_the_cleric_list() {
        let on = vec!["cleric".to_string(), "paladin".to_string()];
        assert!(may_reach(&cleric5(), &on, None).is_ok(), "never held it, and that is fine");
    }

    #[test]
    fn a_cleric_is_refused_a_spell_off_their_list() {
        let on = vec!["wizard".to_string()];
        let e = may_reach(&cleric5(), &on, None).unwrap_err();
        assert!(e.contains("not a cleric spell"), "{}", e);
    }

    #[test]
    fn a_wizard_is_refused_a_wizard_spell_that_is_not_in_the_book() {
        // THE WHOLE DIFFERENCE. Being a wizard spell is not enough -
        // somebody has to have written it down.
        let on = vec!["wizard".to_string()];
        let e = may_reach(&wizard9(), &on, None).unwrap_err();
        assert!(e.contains("not in their book"), "{}", e);
    }

    #[test]
    fn a_wizard_prepares_from_the_book() {
        let on = vec!["wizard".to_string()];
        assert!(may_reach(&wizard9(), &on, Some("book")).is_ok());
        assert!(may_reach(&wizard9(), &on, Some("prepared")).is_ok(), "already up is not a refusal here");
    }

    #[test]
    fn a_wizards_book_beats_the_catalogue_either_way() {
        // A spell written into the book is castable whatever the
        // catalogue says it is - a wizard who copied something odd has
        // it, and the book is the authority for a book caster.
        assert!(may_reach(&wizard9(), &[], Some("book")).is_ok());
    }

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

    /* ---------- spending a slot ---------- */

    #[test]
    fn what_is_left_is_what_is_left() {
        // Cleric 5 has 4/3/2.
        let spent = [1, 0, 2, 0, 0, 0, 0, 0, 0];
        assert_eq!(slots_left(5, &spent)[0..3], [3, 3, 0]);
    }

    #[test]
    fn never_below_zero_however_it_got_there() {
        let spent = [9, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(slots_left(5, &spent)[0], 0);
    }

    #[test]
    fn a_slot_they_have_may_be_spent() {
        assert!(may_spend_slot(5, &[0; 9], 1).is_ok());
        assert!(may_spend_slot(5, &[0; 9], 3).is_ok());
    }

    #[test]
    fn a_level_they_cannot_reach_says_so() {
        let err = may_spend_slot(5, &[0; 9], 4).unwrap_err();
        assert!(err.contains("no level 4"), "unhelpful: {}", err);
    }

    #[test]
    fn an_empty_level_says_to_rest() {
        let spent = [0, 0, 2, 0, 0, 0, 0, 0, 0];
        let err = may_spend_slot(5, &spent, 3).unwrap_err();
        assert!(err.contains("rest"), "unhelpful: {}", err);
    }

    #[test]
    fn a_slot_outside_one_to_nine_is_not_a_slot() {
        assert!(may_spend_slot(20, &[0; 9], 0).is_err());
        assert!(may_spend_slot(20, &[0; 9], 10).is_err());
    }

    #[test]
    fn a_cleric_gets_them_back_on_a_long_rest_and_not_a_short_one() {
        assert!(slots_restored("cleric", true));
        assert!(!slots_restored("cleric", false));
    }

    #[test]
    fn a_warlock_gets_them_back_on_either() {
        // Pact Magic, and the reason this takes a class at all.
        assert!(slots_restored("warlock", true));
        assert!(slots_restored("warlock", false));
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
