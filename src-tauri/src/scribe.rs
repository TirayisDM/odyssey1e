//! Writing a spell down: what it costs, how long it takes, and who may.
//!
//! 170. 150 BUILT THE WIZARD AND LEFT THE BOOK EMPTY. `prayers::caster`
//! returns `Source::Book` for a wizard and `may_reach` already enforces
//! the rule that matters - a wizard prepares only from what somebody has
//! written down - but nothing has ever written anything down. 161's own
//! header says so: "nothing writes `book`, so a player wizard has an
//! empty book and no way to fill it". 161-167 then put 206 wizard spells
//! in the catalogue, which made the gap the whole feature.
//!
//! ---------------------------------------------------------------------
//! THE COST IS THE MATERIALS, NOT A NUMBER BESIDE THEM
//! ---------------------------------------------------------------------
//!
//! The book says copying a spell costs "2 hours and 50 gp per spell
//! level", and the 50 gp is explicitly the fine inks and the material
//! components burnt through while working the spell out. So this does
//! not charge 50 gp AND consume ink - it consumes the ink, and the ink
//! is what costs the money.
//!
//! TWO VIALS PER LEVEL, AND THIS MODULE DOES NOT KNOW WHAT A VIAL
//! COSTS. The ratio is the rule; the price is a row in `items`, which
//! is what lets 171 move the whole economy from gold to silver without
//! touching a line of Rust. A first-level spell is two vials whatever
//! those are worth that week.
//!
//! A wizard who already has ink does not pay twice, and a wizard in a
//! wilderness with a full purse and no ink cannot copy anything - which
//! is the point of making it a thing rather than a price.
//!
//! THE QUILL IS A TOOL AND IS NOT CONSUMED. It wears instead, through
//! 093's `uses_spent` / `uses_max`, so it is a thing to replace rather
//! than a thing to buy per spell.
//!
//! ---------------------------------------------------------------------
//! PER LEVEL AND PER SCHOOL
//! ---------------------------------------------------------------------
//!
//! Level drives it and school bends it. `school_pct` is a percentage so
//! the tuning lives in DATA - `scribe_schools` - rather than in a match
//! arm here that only Dave should be writing. 100 means the book's own
//! rate and is the default for every school until somebody says
//! otherwise; 150 would make necromancy half again as slow and costly.
//!
//! THE REAGENT IS DATA FOR THE SAME REASON and is not in this module at
//! all: which jar a conjurer empties is a question about the world, and
//! a rule that hardcoded it would be this file having opinions about
//! somebody else's setting.
//!
//! ---------------------------------------------------------------------
//! WHAT THIS DOES NOT DECIDE
//! ---------------------------------------------------------------------
//!
//! WHERE THE WRITING GOES. A wizard's book is `character_prayers` rows
//! in state 'book' today, which cannot describe a spellbook found in a
//! dungeon or taken off a corpse, because those rows belong to a
//! character. Scrolls have the same problem from the other end. That is
//! a schema decision and it is Dave's; this module is the arithmetic
//! either way, because what a copy costs does not depend on where the
//! answer is stored. See STATUS.
//!
//! SO NOTHING CALLS THIS YET, and `acquire.rs` is the precedent for
//! saying so in an attribute rather than rushing a caller: the rule is
//! settled and tested first, because the rule is the part that was
//! specified and the part worth getting right before a migration
//! hardens a shape around it.
#![allow(dead_code)]

/// What one scribing takes out of the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scribing {
    /// Hours at the desk. The clock takes ticks - multiply by
    /// `clock::HOUR` - but hours is what a person says out loud.
    pub hours: i64,
    /// Vials of fine ink, consumed.
    pub ink: i64,
}

impl Scribing {
    /// Ticks, for the one caller that has to move the clock.
    pub fn ticks(self) -> i64 {
        self.hours * crate::clock::HOUR
    }
}

/// A cantrip is level 0 and every per-level formula gives nothing for
/// it. Writing one down is still an afternoon's work and a pot of ink,
/// so the floor is one of each rather than a special case per rule.
const FLOOR: i64 = 1;

/// Two hours and two vials per spell level, bent by the school.
///
/// `school_pct` IS A PERCENTAGE AND 100 IS THE BOOK. It is applied to
/// the level-driven figure and then floored, so a cheap school never
/// makes a spell free and a cantrip is never nothing.
///
/// ROUNDED UP. Half a vial of ink is a vial you had to open, and half an
/// hour at the desk is an hour of the day gone. Rounding a cost down is
/// how a rule quietly becomes generous.
pub fn to_copy(level: i64, school_pct: i64) -> Scribing {
    let level = level.max(0);
    let pct = school_pct.max(1);
    Scribing {
        hours: bend(2 * level, pct),
        ink: bend(2 * level, pct),
    }
}

/// Making a scroll of a spell already in the book.
///
/// NOT IN THE BOOK'S RULES AT ALL, which is worth saying plainly: the
/// SRD prices scrolls as treasure and leaves writing one to the DM.
/// This is a house rate and the numbers are the first honest guess:
/// TWICE the copying time and twice the ink, plus the blank it is
/// written on.
///
/// WHY TWICE. Copying into a book is transcription - the spell is
/// already understood and the book is a reference. A scroll has to carry
/// the whole working on its own, for somebody who may not understand it,
/// which is the harder job and the one worth charging for. If it should
/// be days rather than hours, that is a number in this function and a
/// line in STATUS, not a redesign.
pub fn to_scroll(level: i64, school_pct: i64) -> Scribing {
    let base = to_copy(level, school_pct);
    Scribing {
        hours: base.hours * 2,
        ink: base.ink * 2,
    }
}

/// The level-driven figure bent by a percentage, rounded up, floored.
fn bend(base: i64, pct: i64) -> i64 {
    // Integer ceiling: (a + b - 1) / b, with the multiply first so the
    // percentage is not lost to truncation before it is applied.
    let scaled = (base * pct + 99) / 100;
    scaled.max(FLOOR)
}

/// How much room one spell takes in a book.
///
/// 171. A BOOK HOLDS SPELL LEVELS, NOT SPELLS. Dave's Tome holds fifty,
/// which is five ninth-level spells or fifty cantrips - a unit that
/// makes a big spell expensive to carry as well as to write, which
/// counting spells never did.
///
/// A CANTRIP TAKES ONE. Level 0 would be free and a book would hold
/// infinitely many of them; a page is a page. Same floor `to_copy`
/// applies to the time and the ink.
pub fn pages(level: i64) -> i64 {
    level.max(1)
}

/// How much room one spell takes on one particular thing.
///
/// 175. A SCROLL IS A ONE-PAGE BOOK, which is Dave's phrase and the
/// whole rule. It holds `spell_levels = 1` and a spell written on it
/// takes that one page WHATEVER ITS LEVEL - charging a 3rd-level spell
/// three pages against a one-page scroll would make every scroll a
/// cantrip scroll, which is not what a scroll is for.
///
/// IN A BOOK THE LEVEL IS THE COST, unchanged: that is what makes a
/// Tome hold five ninth-level spells or fifty cantrips.
pub fn pages_on(level: i64, scroll: bool) -> i64 {
    if scroll {
        1
    } else {
        pages(level)
    }
}

/// Whether a spell of this level still fits.
///
/// `used` IS THE SUM OF `pages()` OVER WHAT IS ALREADY WRITTEN, which
/// the caller counts because only it knows what is in the book. The
/// rule is the arithmetic and the refusal.
///
/// IT SAYS WHAT IS LEFT. "No room" sends somebody to count pages by
/// hand; "needs 3, 2 left of 10" is a sentence they can act on - buy a
/// bigger book, or write a smaller spell.
pub fn fits(capacity: i64, used: i64, level: i64) -> Result<(), String> {
    fits_on(capacity, used, level, false)
}

/// The same question for one particular thing, which is the one the
/// plumbing asks: 175 makes a spell on a scroll cost one page whatever
/// its level, so the room it needs depends on what it is being written
/// on and not only on the spell.
pub fn fits_on(capacity: i64, used: i64, level: i64, scroll: bool) -> Result<(), String> {
    let want = pages_on(level, scroll);
    let left = capacity - used;
    if want <= left {
        return Ok(());
    }
    Err(format!(
        "that needs {} level{} of room and there {} {} left of {}",
        want,
        if want == 1 { "" } else { "s" },
        if left == 1 { "is" } else { "are" },
        left.max(0),
        capacity
    ))
}

/// Whether this caster may write this spell into their book at all.
///
/// 170. FOUR REFUSALS, AND EVERY ONE OF THEM IS A SENTENCE A PLAYER CAN
/// ACT ON. "You cannot" with no reason is the thing this app keeps
/// refusing to do.
///
/// A BOOK CASTER ONLY. A cleric does not copy spells - they are granted
/// the whole list every day, which is `Source::WholeList` and the reason
/// that enum exists. Refusing here rather than silently succeeding keeps
/// a Copy button off a cleric's sheet by accident.
///
/// ON THE CASTER'S OWN LIST. Being a spell is not enough; being a WIZARD
/// spell is the test, and `spells.classes` is the list - the same check
/// `may_reach` makes for a cleric, from the other side.
///
/// NO HIGHER THAN THEY CAN CAST. A 3rd-level wizard cannot copy a
/// 9th-level spell out of a captured book and sit on it: the book's rule
/// is that you must be able to prepare it. `top_slot` is the highest
/// slot level they have, which `prayers::slots_at` already answers.
///
/// AND NOT TWICE. A spell already written is already written, and the
/// second copy would be a second row saying the same thing - 013's rule
/// about a log recording what changed, applied to a book.
pub fn may_copy(
    source: crate::prayers::Source,
    spell_classes: &[String],
    class_key: &str,
    spell_level: i64,
    top_slot: i64,
    already_written: bool,
) -> Result<(), String> {
    if source != crate::prayers::Source::Book {
        return Err("only a wizard copies spells into a book".to_string());
    }
    if !spell_classes.iter().any(|c| c == class_key) {
        return Err(format!("that is not a {} spell", class_key));
    }
    if already_written {
        return Err("that is already in the book".to_string());
    }
    if spell_level > top_slot {
        return Err(format!(
            "a level {} spell needs a level {} slot to prepare, and they have nothing above {}",
            spell_level, spell_level, top_slot
        ));
    }
    Ok(())
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prayers::Source;

    /* ---------------------- what it costs ---------------------- */

    /// Two hours and two vials a level.
    ///
    /// 171. AND NOT A WORD ABOUT WHAT A VIAL COSTS. The price is a row
    /// in `items` - 5 sp in this campaign, 25 gp in the catalogue 170
    /// shipped with - and a test that asserted the gold would have had
    /// to be rewritten when the economy moved. The ratio is the rule.
    #[test]
    fn two_hours_and_two_vials_a_level() {
        for (level, hours, ink) in [(1, 2, 2), (3, 6, 6), (5, 10, 10), (9, 18, 18)] {
            let s = to_copy(level, 100);
            assert_eq!(s.hours, hours, "level {}", level);
            assert_eq!(s.ink, ink, "level {}", level);
        }
    }

    #[test]
    fn a_cantrip_is_not_free() {
        let s = to_copy(0, 100);
        assert_eq!(s.hours, 1);
        assert_eq!(s.ink, 1);
    }

    #[test]
    fn a_school_bends_it_and_cannot_zero_it() {
        // Half again as hard.
        assert_eq!(to_copy(2, 150).hours, 6);
        // Generous, but never free: the floor holds.
        assert_eq!(to_copy(1, 1).hours, 1);
        assert_eq!(to_copy(1, 1).ink, 1);
        // A percentage of zero or less is nonsense and is treated as 1
        // rather than dividing the world by nothing.
        assert_eq!(to_copy(5, 0).hours, 1);
        assert_eq!(to_copy(5, -40).hours, 1);
    }

    /// Rounding a cost DOWN is how a rule quietly becomes generous.
    #[test]
    fn it_rounds_up() {
        // 2 hours at 125% is 2.5, which is three hours of the day.
        assert_eq!(to_copy(1, 125).hours, 3);
        // 6 at 110% is 6.6.
        assert_eq!(to_copy(3, 110).hours, 7);
    }

    #[test]
    fn a_scroll_is_twice_the_work() {
        let book = to_copy(3, 100);
        let scroll = to_scroll(3, 100);
        assert_eq!(scroll.hours, book.hours * 2);
        assert_eq!(scroll.ink, book.ink * 2);
    }

    #[test]
    fn hours_become_ticks_for_the_clock() {
        assert_eq!(to_copy(1, 100).ticks(), 2 * crate::clock::HOUR);
    }

    /* ---------------------- how much room ---------------------- */

    #[test]
    fn a_spell_takes_its_level_in_room() {
        assert_eq!(pages(3), 3);
        assert_eq!(pages(9), 9);
    }

    /// Level 0 would be free and a book would hold infinitely many.
    #[test]
    fn a_cantrip_takes_a_page_like_everything_else() {
        assert_eq!(pages(0), 1);
    }

    /// 175. A scroll holds one spell whatever its level - the page cost
    /// is the scroll, not the level. Charging by level against a
    /// one-page scroll would make every scroll a cantrip scroll.
    #[test]
    fn a_spell_on_a_scroll_takes_the_scroll() {
        for level in 0..=9 {
            assert_eq!(pages_on(level, true), 1, "level {}", level);
        }
        // and a one-page scroll holds exactly one of them
        assert!(fits(1, 0, 9).is_err(), "nine pages do not fit a one-page book");
        assert!(fits(1, 0, pages_on(9, true) - pages_on(9, true)).is_ok());
    }

    #[test]
    fn in_a_book_the_level_is_still_the_cost() {
        assert_eq!(pages_on(3, false), 3);
        assert_eq!(pages_on(0, false), 1);
    }

    /// The second spell has nowhere to go.
    #[test]
    fn a_scroll_takes_one_spell_and_no_more() {
        let used = pages_on(3, true); // the first one, written
        assert!(fits(1, used, 1).is_err());
    }

    /// Dave's Tome holds fifty levels: five ninth-level spells, or
    /// fifty cantrips, and the unit is what makes those different.
    #[test]
    fn a_tome_is_five_ninths_or_fifty_cantrips() {
        let tome = 50;
        assert!(fits(tome, 9 * 5 - 9, 9).is_ok(), "the fifth ninth fits");
        assert!(fits(tome, 9 * 5, 9).is_err(), "the sixth does not");
        assert!(fits(tome, 49, 0).is_ok(), "the fiftieth cantrip fits");
        assert!(fits(tome, 50, 0).is_err(), "the fifty-first does not");
    }

    #[test]
    fn an_adventure_book_runs_out_fast() {
        // 10 levels: three 3rd-level spells and a cantrip fill it.
        assert!(fits(10, 9, 0).is_ok());
        assert!(fits(10, 10, 0).is_err());
        // And a 3rd-level spell will not go into the last two.
        assert!(fits(10, 8, 3).is_err());
    }

    /// "No room" sends somebody to count pages by hand.
    #[test]
    fn the_refusal_says_what_is_left() {
        let e = fits(10, 8, 3).unwrap_err();
        assert!(e.contains("needs 3 levels"), "{}", e);
        assert!(e.contains("2 left of 10"), "{}", e);
    }

    /// A book somehow over its limit must not report a negative.
    #[test]
    fn an_overfull_book_says_zero_left() {
        let e = fits(10, 14, 1).unwrap_err();
        assert!(e.contains("0 left of 10"), "{}", e);
    }

    /* ---------------------- who may ---------------------- */

    fn wizard_list() -> Vec<String> {
        vec!["sorcerer".to_string(), "wizard".to_string()]
    }

    #[test]
    fn a_wizard_may_copy_a_wizard_spell_they_can_cast() {
        assert!(may_copy(Source::Book, &wizard_list(), "wizard", 3, 5, false).is_ok());
    }

    #[test]
    fn a_cleric_does_not_copy_anything() {
        let e = may_copy(Source::WholeList, &wizard_list(), "cleric", 1, 9, false).unwrap_err();
        assert!(e.contains("only a wizard"), "{}", e);
    }

    #[test]
    fn a_spell_off_the_list_is_refused_by_name() {
        let cleric_only = vec!["cleric".to_string()];
        let e = may_copy(Source::Book, &cleric_only, "wizard", 1, 9, false).unwrap_err();
        assert!(e.contains("not a wizard spell"), "{}", e);
    }

    /// The captured-spellbook case: a third-level wizard cannot bank a
    /// ninth-level spell against the day they can cast it.
    #[test]
    fn nothing_above_what_they_could_prepare() {
        let e = may_copy(Source::Book, &wizard_list(), "wizard", 9, 2, false).unwrap_err();
        assert!(e.contains("level 9"), "{}", e);
        assert!(e.contains("nothing above 2"), "{}", e);
    }

    #[test]
    fn exactly_at_the_top_slot_is_allowed() {
        assert!(may_copy(Source::Book, &wizard_list(), "wizard", 5, 5, false).is_ok());
    }

    #[test]
    fn a_spell_already_written_is_not_written_again() {
        let e = may_copy(Source::Book, &wizard_list(), "wizard", 1, 9, true).unwrap_err();
        assert!(e.contains("already in the book"), "{}", e);
    }

    /// A cantrip is level 0, so it clears any slot a wizard has - and a
    /// wizard with no slots at all is still level 1 and still writes.
    #[test]
    fn a_cantrip_needs_no_slot() {
        assert!(may_copy(Source::Book, &wizard_list(), "wizard", 0, 0, false).is_ok());
    }
}
