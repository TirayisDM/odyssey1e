//! How many times a feature can be used before it needs a rest.
//!
//! 092. `class_features` said WHAT a class gives and never HOW MUCH.
//! Action Surge is once, then twice from 17; Ki is the monk's level;
//! Bardic Inspiration is a Charisma modifier; Rage is a table. None of
//! that is a number, which is why this is an expression rather than a
//! column of integers.
//!
//! ---------------------------------------------------------------------
//! FOUR FORMS, AND THEY COVER NEARLY EVERYTHING
//! ---------------------------------------------------------------------
//!
//! ```text
//!   3            a flat number          Deflect Missiles, Second Wind
//!   level        the CLASS's level      Ki
//!   cha_mod      an ability modifier    Bardic Inspiration
//!   1@1,2@17     a table by level       Action Surge, Rage, Indomitable
//! ```
//!
//! THE LEVEL IS THE CLASS'S, NEVER THE CHARACTER'S. A Fighter 4 / Bard
//! 1 has one Action Surge because they are a Fighter 4, and reading
//! their total of 5 would be the same mistake `features::held` exists to
//! avoid. The caller passes the right one.
//!
//! A BAND TABLE READS DOWNWARD: `2@1,3@3,4@6` is two from level 1,
//! three from 3, four from 6. The highest band at or under the level
//! wins, so the order they are written in does not matter and a level
//! below every band is zero.
//!
//! AN ABILITY MODIFIER IS FLOORED AT ONE, which is 5e: a bard with
//! Charisma 10 still gets one Bardic Inspiration, because the book says
//! "a number of times equal to your Charisma modifier (a minimum of
//! once)". Nothing else here has a floor.
//!
//! WHAT IS NOT HERE: pools. Lay on Hands is five hit points per level
//! and a Sorcerer's font is a pile of points - both are a MAGNITUDE
//! rather than a count of uses, and spending three of one is not the
//! same shape as using a feature once. They leave `uses` NULL and wait
//! for a resource model that has not been asked for yet.

/// What an expression can read about the character asking.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    /// Levels in THE CLASS the feature belongs to, not the character's
    /// total - see the module header.
    pub level: i64,
    pub str_mod: i64,
    pub dex_mod: i64,
    pub con_mod: i64,
    pub int_mod: i64,
    pub wis_mod: i64,
    pub cha_mod: i64,
}

impl Context {
    /// A context with nothing but a level - every ability modifier
    /// zero.
    ///
    /// TEST SCAFFOLDING, and `#[cfg(test)]` rather than allowed dead
    /// code because that is what it is: the commands build a real
    /// context from a real sheet, and an all-zeroes one in the shipped
    /// library would be a trap for whoever reached for it next. The
    /// same call dice.rs made for SequenceRoller.
    #[cfg(test)]
    pub fn at_level(level: i64) -> Self {
        Context { level, str_mod: 0, dex_mod: 0, con_mod: 0, int_mod: 0, wis_mod: 0, cha_mod: 0 }
    }

    fn ability(&self, code: &str) -> Option<i64> {
        Some(match code {
            "str" => self.str_mod,
            "dex" => self.dex_mod,
            "con" => self.con_mod,
            "int" => self.int_mod,
            "wis" => self.wis_mod,
            "cha" => self.cha_mod,
            _ => return None,
        })
    }
}

/// How many uses this expression comes to.
///
/// NONE IS NOT ZERO. A feature with no `uses` is one with no limit to
/// track - Second Wind has a number and Evasion does not, and "unlimited"
/// and "none left" must not read the same on a sheet.
///
/// A MALFORMED EXPRESSION IS AN ERROR RATHER THAN A ZERO. The catalogue
/// is authored by hand, and a typo that quietly meant "no uses" would
/// take a feature away from everybody holding it with nothing on screen
/// to say why.
pub fn count(expr: Option<&str>, ctx: &Context) -> Result<Option<i64>, String> {
    let Some(raw) = expr.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };

    if raw.contains('@') {
        return bands(raw, ctx.level).map(Some);
    }
    if raw == "level" {
        return Ok(Some(ctx.level.max(0)));
    }
    if let Some(code) = raw.strip_suffix("_mod") {
        return match ctx.ability(code) {
            // 5e's own minimum, and the only floor in here.
            Some(m) => Ok(Some(m.max(1))),
            None => Err(format!("no ability called {}", code)),
        };
    }
    raw.parse::<i64>()
        .map(|n| Some(n.max(0)))
        .map_err(|_| format!("cannot read uses: {}", raw))
}

/// `2@1,3@3,4@6` - the highest band at or under this level.
fn bands(raw: &str, level: i64) -> Result<i64, String> {
    let mut best: Option<(i64, i64)> = None;
    for part in raw.split(',') {
        let part = part.trim();
        let Some((n, at)) = part.split_once('@') else {
            return Err(format!("cannot read band: {}", part));
        };
        let n: i64 = n
            .trim()
            .parse()
            .map_err(|_| format!("cannot read band count: {}", part))?;
        let at: i64 = at
            .trim()
            .parse()
            .map_err(|_| format!("cannot read band level: {}", part))?;
        if at <= level && best.map(|(_, b)| at > b).unwrap_or(true) {
            best = Some((n, at));
        }
    }
    // BELOW EVERY BAND IS ZERO, not an error. A Fighter 9 reading
    // Indomitable's `1@9,2@13,3@17` at level 8 has none of it yet, and
    // that is an answer rather than a fault.
    Ok(best.map(|(n, _)| n).unwrap_or(0).max(0))
}

/// How many are left, given what has been spent.
///
/// NEVER BELOW ZERO. A feature whose maximum drops - a class level
/// lost, a Charisma drained - can leave somebody having spent more than
/// they now have, and "minus one left" helps nobody.
pub fn left(max: Option<i64>, spent: i64) -> Option<i64> {
    max.map(|m| (m - spent.max(0)).max(0))
}

/// Whether there is one to spend.
pub fn may_spend(max: Option<i64>, spent: i64) -> Result<(), String> {
    match left(max, spent) {
        // Unlimited: nothing to count, so nothing to refuse.
        None => Ok(()),
        Some(0) => Err("none left until they rest".to_string()),
        Some(_) => Ok(()),
    }
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(level: i64) -> Context {
        Context::at_level(level)
    }

    fn bard(level: i64, cha: i64) -> Context {
        Context { cha_mod: cha, ..Context::at_level(level) }
    }

    /* ---------- nothing to count ---------- */

    #[test]
    fn no_expression_is_no_limit_rather_than_none_left() {
        // Evasion has no number. "Unlimited" and "spent" must not read
        // the same on a sheet.
        assert_eq!(count(None, &ctx(7)), Ok(None));
        assert_eq!(count(Some(""), &ctx(7)), Ok(None));
        assert_eq!(count(Some("   "), &ctx(7)), Ok(None));
    }

    /* ---------- a flat number ---------- */

    #[test]
    fn a_number_is_a_number() {
        assert_eq!(count(Some("1"), &ctx(1)), Ok(Some(1)));
        assert_eq!(count(Some("3"), &ctx(20)), Ok(Some(3)));
    }

    #[test]
    fn whitespace_does_not_change_an_answer() {
        assert_eq!(count(Some(" 2 "), &ctx(1)), Ok(Some(2)));
    }

    /* ---------- the class's level ---------- */

    #[test]
    fn ki_is_the_monks_level() {
        assert_eq!(count(Some("level"), &ctx(5)), Ok(Some(5)));
        assert_eq!(count(Some("level"), &ctx(20)), Ok(Some(20)));
    }

    /* ---------- an ability modifier ---------- */

    #[test]
    fn bardic_inspiration_is_a_charisma_modifier() {
        assert_eq!(count(Some("cha_mod"), &bard(5, 4)), Ok(Some(4)));
    }

    #[test]
    fn and_never_fewer_than_one() {
        // 5e: "a minimum of once". A bard with Charisma 10 still sings.
        assert_eq!(count(Some("cha_mod"), &bard(1, 0)), Ok(Some(1)));
        assert_eq!(count(Some("cha_mod"), &bard(1, -2)), Ok(Some(1)));
    }

    #[test]
    fn every_ability_can_be_asked_for() {
        let c = Context {
            level: 1, str_mod: 1, dex_mod: 2, con_mod: 3,
            int_mod: 4, wis_mod: 5, cha_mod: 6,
        };
        assert_eq!(count(Some("str_mod"), &c), Ok(Some(1)));
        assert_eq!(count(Some("con_mod"), &c), Ok(Some(3)));
        assert_eq!(count(Some("wis_mod"), &c), Ok(Some(5)));
    }

    #[test]
    fn an_ability_nobody_has_heard_of_is_an_error() {
        assert!(count(Some("luck_mod"), &ctx(1)).is_err());
    }

    /* ---------- bands ---------- */

    #[test]
    fn action_surge_is_one_then_two() {
        let e = Some("1@1,2@17");
        assert_eq!(count(e, &ctx(2)), Ok(Some(1)));
        assert_eq!(count(e, &ctx(16)), Ok(Some(1)));
        assert_eq!(count(e, &ctx(17)), Ok(Some(2)));
        assert_eq!(count(e, &ctx(20)), Ok(Some(2)));
    }

    #[test]
    fn rage_climbs_the_whole_way() {
        let e = Some("2@1,3@3,4@6,5@12,6@17");
        assert_eq!(count(e, &ctx(1)), Ok(Some(2)));
        assert_eq!(count(e, &ctx(3)), Ok(Some(3)));
        assert_eq!(count(e, &ctx(11)), Ok(Some(4)));
        assert_eq!(count(e, &ctx(12)), Ok(Some(5)));
        assert_eq!(count(e, &ctx(20)), Ok(Some(6)));
    }

    #[test]
    fn the_order_bands_are_written_in_does_not_matter() {
        assert_eq!(count(Some("2@17,1@1"), &ctx(17)), Ok(Some(2)));
        assert_eq!(count(Some("2@17,1@1"), &ctx(5)), Ok(Some(1)));
    }

    #[test]
    fn below_every_band_is_none_yet_rather_than_an_error() {
        // A Fighter 8 reading Indomitable's 1@9 has not got it yet.
        assert_eq!(count(Some("1@9,2@13,3@17"), &ctx(8)), Ok(Some(0)));
    }

    #[test]
    fn spaces_around_a_band_are_fine() {
        assert_eq!(count(Some("1@1, 2@17"), &ctx(17)), Ok(Some(2)));
    }

    /* ---------- a typo is loud ---------- */

    #[test]
    fn a_malformed_expression_is_an_error_not_a_zero() {
        // The catalogue is hand-authored. A typo that quietly meant "no
        // uses" would take a feature off everybody holding it with
        // nothing on screen to say why.
        assert!(count(Some("levl"), &ctx(5)).is_err());
        assert!(count(Some("1@"), &ctx(5)).is_err());
        assert!(count(Some("@17"), &ctx(5)).is_err());
        assert!(count(Some("two"), &ctx(5)).is_err());
    }

    /* ---------- spending ---------- */

    #[test]
    fn what_is_left_is_what_is_left() {
        assert_eq!(left(Some(3), 0), Some(3));
        assert_eq!(left(Some(3), 2), Some(1));
        assert_eq!(left(Some(3), 3), Some(0));
    }

    #[test]
    fn never_below_zero_however_it_got_there() {
        // A class level lost, a Charisma drained: somebody can have
        // spent more than they now have, and "minus one" helps nobody.
        assert_eq!(left(Some(1), 4), Some(0));
    }

    #[test]
    fn unlimited_stays_unlimited() {
        assert_eq!(left(None, 99), None);
        assert!(may_spend(None, 99).is_ok());
    }

    #[test]
    fn spending_the_last_one_is_allowed_and_the_next_is_not() {
        assert!(may_spend(Some(1), 0).is_ok());
        assert!(may_spend(Some(1), 1).is_err());
    }

    #[test]
    fn the_refusal_says_what_fixes_it() {
        let err = may_spend(Some(2), 2).unwrap_err();
        assert!(err.contains("rest"), "unhelpful: {}", err);
    }
}
