//! Casting a spell as an action in a fight.
//!
//! 110. 106 gave a cleric a list and 107 gave them slots to spend, and
//! neither gave them anything to DO on their turn. Luci stood in the
//! Tavern with initiative rolled, no weapons, and a prepared list the
//! encounter could not see.
//!
//! ---------------------------------------------------------------------
//! A SPELL IS AN ACTION LIKE A SWING IS
//! ---------------------------------------------------------------------
//!
//! The engine already knows what an attack costs and what it rolls -
//! `attack.rs` since 043, the action economy since 062. A spell has to
//! join that rather than grow a parallel one, which is why this answers
//! the same three questions an attack does: what does it cost, what do
//! you roll, and who is it aimed at.
//!
//! ---------------------------------------------------------------------
//! THE CASTING TIME DECIDES WHETHER IT CAN BE CAST AT ALL
//! ---------------------------------------------------------------------
//!
//! This falls straight out of the catalogue and is the most useful rule
//! here: Prayer of Healing takes ten minutes and CANNOT be cast in a
//! fight. Six seconds is a round and a ten-minute spell is a hundred of
//! them, so offering it on somebody's turn would be offering a thing
//! that cannot happen.
//!
//! The same data says Healing Word and Spiritual Weapon are bonus
//! actions, which is most of why a cleric picks them.
//!
//! ---------------------------------------------------------------------
//! OFFENSIVE, DEFENSIVE, NEUTRAL
//! ---------------------------------------------------------------------
//!
//! DERIVED FROM `cast_type` RATHER THAN STORED. 006's four words -
//! Attack, Save, Heal, Utility - already say it: an attack or a save is
//! aimed at somebody who does not want it, a heal is aimed at somebody
//! who does, and everything else is neither.
//!
//! IT IS A ROUGH EDGE AND WORTH NAMING. Zone of Truth is a Save and is
//! not an attack on anybody; Bane is a Save and plainly is. The
//! vocabulary cannot tell them apart, so the stance is a HINT for
//! sorting and colouring a list, never a rule that refuses anything.
//! When it needs to be exact it becomes a column.

use serde::{Deserialize, Serialize};

/// Which way a spell points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stance {
    Offensive,
    Defensive,
    Neutral,
}

impl Stance {
    pub fn as_str(self) -> &'static str {
        match self {
            Stance::Offensive => "offensive",
            Stance::Defensive => "defensive",
            Stance::Neutral => "neutral",
        }
    }
}

/// What a spell's `cast_type` says about which way it points.
pub fn stance(cast_type: &str) -> Stance {
    match cast_type {
        "Attack" | "Save" | "Auto" => Stance::Offensive,
        "Heal" => Stance::Defensive,
        _ => Stance::Neutral,
    }
}

/// Whether this spell's dice land on the target with no roll by either
/// side.
///
/// 183. THE FIFTH SHAPE, AND MAGIC MISSILE IS THE ONLY ONE. The four
/// words said how a spell resolves: `Attack` rolls to hit, `Save` lets
/// the target resist, `Heal` restores, `Utility` does nothing to hit
/// points at the moment of casting. Magic Missile is none of them - its
/// own text says "no attack roll and no save, which is why this is not
/// an Attack" - so it was filed `Utility`, and `cast_spell` has exactly
/// one route to a hit point: the `Heal` branch.
///
/// SO IT HAS NEVER DEALT DAMAGE. Dave cast it five times across four
/// rounds and nothing moved. 156 wrote the same sentence about healing
/// - "every route to `hp_events` ran through `resolved.attack` and a
/// heal is not an attack" - and fixed it for one cast type. This is
/// that fault one cast type over.
///
/// A FIFTH VALUE ON AN EXISTING AXIS, not a new column. `cast_type`
/// already answers "how does this resolve", and "it simply lands" is an
/// answer to that question rather than a new question.
///
/// WHY NOT INFER IT from dice with no save and no attack: nine Utility
/// spells carry dice and eight of them must NOT fire at cast time -
/// Bless's 1d4 is a bonus somebody adds to a roll, Glyph of Warding and
/// Forbiddance trigger later, Fire Shield answers a melee hit, Faithful
/// Hound and Arcane Hand strike on their own. Guessing from the shape
/// of the columns would damage somebody with Bless.
pub fn lands_automatically(cast_type: &str) -> bool {
    cast_type == "Auto"
}

/// The dice this spell rolls when cast with a slot of `at_level`.
///
/// 185. THE SLOT YOU SPEND AND THE DICE YOU ROLL ARE TWO QUESTIONS.
/// `at_level` has picked the slot since 107; this is the other half.
/// `at_higher_dice` is what one extra level buys, as a formula, and
/// NULL means a bigger slot buys no extra dice - true for most of the
/// catalogue, where the rider promises targets or duration instead.
///
/// A CANTRIP IS NEVER UPCAST. Its dice climb with the CASTER's level,
/// not with a slot, and it costs no slot at all - so level 0 returns
/// the base whatever it is handed.
///
/// DOWNCASTING IS NOT A THING. A slot lower than the spell cannot cast
/// it, which `may_spend_slot` already refuses; if one arrives here it
/// scales by zero rather than subtracting dice.
pub fn upcast_dice(
    dice: Option<&str>,
    at_higher: Option<&str>,
    spell_level: i64,
    at_level: Option<i64>,
) -> Option<String> {
    let base = dice.map(str::trim).filter(|d| !d.is_empty())?;
    if spell_level == 0 {
        return Some(base.to_string());
    }
    let extra = match at_higher.map(str::trim).filter(|d| !d.is_empty()) {
        Some(e) => e,
        None => return Some(base.to_string()),
    };
    let steps = at_level.unwrap_or(spell_level) - spell_level;
    // A BAD FORMULA KEEPS THE BASE rather than failing the cast. The
    // column is ours and a typo in it should understate the spell, not
    // stop somebody casting in the middle of a fight.
    Some(crate::dice::add_formula(base, extra, steps).unwrap_or_else(|_| base.to_string()))
}

/// What an action costs on a turn, in 062's vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cost {
    Action,
    Bonus,
    Reaction,
    /// Longer than a turn. Not castable in a fight at all.
    TooLong,
}

impl Cost {
    pub fn as_str(self) -> &'static str {
        match self {
            Cost::Action => "action",
            Cost::Bonus => "bonus",
            Cost::Reaction => "reaction",
            Cost::TooLong => "too long",
        }
    }

    /// Whether this can happen on somebody's turn.
    pub fn in_a_fight(self) -> bool {
        self != Cost::TooLong
    }
}

/// What a casting time costs.
///
/// READ FROM THE TEXT the catalogue stores, because that is what 5e
/// prints and what 102 seeded. Anything that is not one of the three
/// turn-sized costs is too long - a minute, ten minutes, an hour - and
/// saying so is the point rather than a gap.
pub fn cost(casting_time: Option<&str>) -> Cost {
    let t = casting_time.unwrap_or("").trim().to_lowercase();
    if t.contains("bonus action") {
        Cost::Bonus
    } else if t.contains("reaction") {
        Cost::Reaction
    } else if t == "1 action" || t.starts_with("1 action") {
        Cost::Action
    } else {
        Cost::TooLong
    }
}

/// How long a spell hangs around.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lasts {
    /// Nothing lingers. Cure Wounds heals and is over; there is no
    /// effect to create, which is a different answer from one that
    /// lasts no time.
    Instant,
    /// This many ticks - see clock.rs.
    Ticks(i64),
    /// Until something ends it. "Until dispelled", and the handful the
    /// book marks Special.
    Indefinite,
}

/// Read the catalogue's duration text.
///
/// 112. TEXT, BECAUSE 5e'S OWN IS. "Up to 1 minute", "Until dispelled",
/// "Instantaneous" - a column of ticks would have had to decide what
/// Special means at seed time, and it does not mean a number.
///
/// "UP TO" IS CONCENTRATION'S WORDING and changes nothing here: a
/// concentrating caster can end it sooner, which is what ending an
/// effect early already does.
pub fn lasts(duration: Option<&str>) -> Lasts {
    let t = duration.unwrap_or("").trim().to_lowercase();
    if t.is_empty() || t.starts_with("instant") {
        return Lasts::Instant;
    }
    if t.starts_with("until") || t.starts_with("special") {
        return Lasts::Indefinite;
    }

    // The first number in the text, and the unit after it.
    let digits: String = t.chars().skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit()).collect();
    let Ok(n) = digits.parse::<i64>() else {
        // A duration nobody has written a rule for lingers rather than
        // vanishing - an effect a DM can see and end beats one that
        // silently never existed.
        return Lasts::Indefinite;
    };
    let per = if t.contains("round") {
        crate::clock::ROUND
    } else if t.contains("minute") {
        crate::clock::MINUTE
    } else if t.contains("hour") {
        crate::clock::HOUR
    } else if t.contains("day") {
        crate::clock::DAY
    } else {
        return Lasts::Indefinite;
    };
    Lasts::Ticks(n * per)
}

/// Everything a turn needs to know about casting one spell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cast {
    pub key: String,
    pub name: String,
    /// The spell's own level. 0 is a cantrip and costs no slot.
    pub level: i64,
    pub stance: Stance,
    pub cost: Cost,
    /// The to-hit bonus, for a spell that makes an attack roll.
    pub to_hit: Option<i64>,
    /// The DC the target rolls against, for a spell that forces a save.
    pub save_dc: Option<i64>,
    /// Which save - "dex", "wis". None where there is none.
    pub save_ability: Option<String>,
    /// What gets rolled for effect, with the caster's modifier already
    /// in it where 5e adds one.
    pub dice: Option<String>,
    /// True when it needs a slot - every spell but a cantrip.
    pub needs_slot: bool,
}

/// Work out what casting this spell would be.
///
/// THE MODIFIER GOES INTO THE DICE FOR A HEAL AND NOT FOR DAMAGE, which
/// is 5e and catches people out: Cure Wounds is 1d8 + your modifier and
/// Sacred Flame is 1d8 flat. The rule is that healing adds it and damage
/// does not, barring a few spells that say otherwise.
#[allow(clippy::too_many_arguments)]
pub fn cast(
    key: &str,
    name: &str,
    level: i64,
    cast_type: &str,
    casting_time: Option<&str>,
    save_ability: Option<&str>,
    dice: Option<&str>,
    prof_bonus: i64,
    ability_mod: i64,
) -> Cast {
    let stance = stance(cast_type);
    let to_hit = (cast_type == "Attack").then(|| crate::casting::attack_bonus(prof_bonus, ability_mod));
    let save_dc = save_ability
        .filter(|s| !s.is_empty())
        .map(|_| crate::casting::save_dc(prof_bonus, ability_mod));

    let dice = dice.filter(|d| !d.is_empty()).map(|d| {
        if cast_type == "Heal" && ability_mod != 0 {
            format!("{}{}{}", d, if ability_mod > 0 { "+" } else { "-" }, ability_mod.abs())
        } else {
            d.to_string()
        }
    });

    Cast {
        key: key.to_string(),
        name: name.to_string(),
        level,
        stance,
        cost: cost(casting_time),
        to_hit,
        save_dc,
        save_ability: save_ability.filter(|s| !s.is_empty()).map(str::to_string),
        dice,
        needs_slot: level > 0,
    }
}

/// One spell a character can cast right now, as the sheet carries it.
///
/// 154. CATALOGUE FACTS AND NOTHING DERIVED. The to-hit, the DC and the
/// damage all come out of `cast()`, which needs the caster's numbers;
/// holding a computed to-hit on the sheet would be a second place for
/// the same arithmetic to live and drift.
///
/// WHAT IS ON IT IS WHAT `cast()` TAKES, so the sheet can hand one
/// straight to the same function `cast_spell` calls and get the same
/// answer. Two callers, one rule.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Known {
    pub key: String,
    pub name: String,
    /// What the roll box matches on. Usually the name; it is a separate
    /// column because 006 let a spell be asked for by a shorter word.
    pub roll_name: String,
    pub level: i64,
    pub cast_type: String,
    pub casting_time: Option<String>,
    pub save_ability: Option<String>,
    pub dice: Option<String>,
    /// Only to tell a touch spell from one thrown across the room.
    pub range: Option<String>,
}

/// The spell a typed request is asking for, if it is asking for one.
///
/// BY NAME OR BY ROLL NAME, trimmed and case-folded, which is how every
/// other branch of `resolve_request` matches. Nothing fuzzier: a request
/// that half-matches two spells should find neither rather than pick.
pub fn find<'a>(known: &'a [Known], request: &str) -> Option<&'a Known> {
    let want = request.trim().to_lowercase();
    if want.is_empty() {
        return None;
    }
    known.iter().find(|k| {
        k.roll_name.trim().to_lowercase() == want || k.name.trim().to_lowercase() == want
    })
}

/// A spell attack, in the shape the attack path already understands.
///
/// 154. THE WHOLE POINT, and the reason this returns an `attack::Attack`
/// rather than something new. `resolve_request` hands that struct to the
/// roll path and everything downstream reads it: `resolution::admits`
/// lets the roll be aimed at an AC, the crit range decides the verdict,
/// `a.damage` is rolled and doubled on a crit, `a.damage_types` and
/// `a.magical` are what the target resists, and the hit point event
/// falls out at the end. A spell attack that filled in this struct gets
/// all of that for nothing; one that invented its own path would be a
/// second damage pipeline beside `attack.rs`, which is the thing this
/// module's header says it exists to avoid.
///
/// NONE FOR EVERYTHING THAT IS NOT AN ATTACK. A Save spell is the
/// target's roll and not the caster's, and a Utility spell rolls no d20
/// at all - neither is a thing to put in the roll box, and saying so
/// here keeps the branch in `resolve_request` to one line.
///
/// `magical` IS TRUE AND IT MATTERS. 143 made "resistant to bludgeoning,
/// piercing and slashing from nonmagical attacks" expressible, and that
/// is the commonest resistance in 5e. A spectral mace is magical by
/// definition, so a werewolf does not halve it.
///
/// `damage_types` IS EMPTY AND THAT IS A REAL GAP. `spells` has no
/// damage type column - Inflict Wounds is necrotic and Guiding Bolt is
/// radiant, and neither can say so - so a target's resistance simply
/// does not apply. The Attack struct's own comment calls an empty list
/// "the safe way to be ignorant", and that is what this is: understated
/// rather than wrong. The fix is a column, not a rule.
///
/// PROFICIENT, ALWAYS. There is no such thing as casting a spell you
/// know without proficiency - the bonus is baked into every spell attack
/// in 5e - so this is true rather than derived from a weapon list a
/// spell was never on.
pub fn attack_for(
    k: &Known,
    prof_bonus: i64,
    abil_mod: i64,
    ability: &str,
) -> Option<crate::attack::Attack> {
    let c = cast(
        &k.key,
        &k.name,
        k.level,
        &k.cast_type,
        k.casting_time.as_deref(),
        k.save_ability.as_deref(),
        k.dice.as_deref(),
        prof_bonus,
        abil_mod,
    );
    let to_hit = c.to_hit?;
    // A DAMAGE DIE IS REQUIRED, because the roll path rolls `a.damage`
    // the moment the swing lands and an empty formula fails the whole
    // roll rather than just the damage. An Attack spell with no dice is
    // a catalogue row that cannot be rolled; declining sends it to the
    // ordinary fallback, where the dice engine says so plainly.
    let damage = c.dice.filter(|d| !d.trim().is_empty())?;

    Some(crate::attack::Attack {
        item_key: k.key.clone(),
        weapon_name: k.name.clone(),
        // DESCRIPTIVE ONLY. Nothing downstream reads the mode of a spell
        // attack, and the catalogue cannot settle it anyway: Spiritual
        // Weapon reaches 60 feet and still makes a MELEE spell attack.
        // Touch is the one case the data does answer.
        mode: match k.range.as_deref().map(str::trim) {
            Some(r) if r.eq_ignore_ascii_case("touch") || r.eq_ignore_ascii_case("self") => {
                crate::equipment::Mode::Melee
            }
            _ => crate::equipment::Mode::Ranged,
        },
        ability: ability.to_string(),
        ability_mod: abil_mod,
        proficient: true,
        proficiency_bonus: prof_bonus,
        to_hit,
        magic: 0,
        magical: true,
        damage,
        crit_min: 20,
        fumble_max: 1,
        technique: None,
        damage_types: Vec::new(),
    })
}

/// How much of a save spell's dice actually land.
///
/// 158. THREE ANSWERS AND NOT TWO. `spells.on_save` says what a creature
/// that MAKES the save takes - half the dice or none of them - and NULL
/// says the dice are not save damage at all. Bane's 1d4 is the penalty
/// it hangs on somebody, Geas's 5d10 is a daily toll for disobeying, and
/// Bestow Curse's 1d8 belongs to one of four options chosen later. A
/// rule that damaged on every failed save would hit people with all
/// three.
///
/// `None` MEANS NOTHING HAPPENS TO HIT POINTS, which is different from
/// `Some(0)` - the caller writes no event at all rather than a row
/// saying a spell did nothing. 013's log records what changed.
///
/// HALVING ROUNDS DOWN, which is 5e everywhere it halves.
pub fn save_damage(rolled: i64, saved: bool, on_save: Option<&str>) -> Option<i64> {
    let rule = on_save.map(str::trim).filter(|s| !s.is_empty())?;
    if !saved {
        // A failure takes the lot, whichever rule it is.
        return Some(rolled.max(0));
    }
    match rule {
        "half" => Some((rolled / 2).max(0)),
        "none" => Some(0),
        // A word nobody has taught this function. Nothing rather than a
        // guess, for the reason 158 gives: invented damage is invisible
        // and absent damage is not.
        _ => None,
    }
}

/// What the card says: "Sacred Flame — DEX save DC 14, 1d8".
pub fn label(c: &Cast) -> String {
    let mut bits: Vec<String> = Vec::new();
    if let Some(n) = c.to_hit {
        bits.push(format!("{}{} to hit", if n >= 0 { "+" } else { "" }, n));
    }
    if let (Some(dc), Some(ab)) = (c.save_dc, c.save_ability.as_deref()) {
        bits.push(format!("{} save DC {}", ab.to_uppercase(), dc));
    }
    if let Some(d) = &c.dice {
        bits.push(d.clone());
    }
    if bits.is_empty() {
        c.name.clone()
    } else {
        format!("{} \u{2014} {}", c.name, bits.join(", "))
    }
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    /* ---------- 185. what a bigger slot rolls ---------- */

    #[test]
    fn fireball_at_fifth_rolls_ten_d6() {
        assert_eq!(
            upcast_dice(Some("8d6"), Some("1d6"), 3, Some(5)).as_deref(),
            Some("10d6")
        );
    }

    #[test]
    fn magic_missile_gains_whole_darts() {
        assert_eq!(
            upcast_dice(Some("3d4+3"), Some("1d4+1"), 1, Some(3)).as_deref(),
            Some("5d4+5")
        );
    }

    #[test]
    fn at_its_own_level_the_base_comes_back() {
        assert_eq!(
            upcast_dice(Some("8d6"), Some("1d6"), 3, Some(3)).as_deref(),
            Some("8d6")
        );
        assert_eq!(
            upcast_dice(Some("8d6"), Some("1d6"), 3, None).as_deref(),
            Some("8d6")
        );
    }

    #[test]
    fn no_column_means_no_extra_dice() {
        // Most of the catalogue. A bigger slot still gets SPENT - that
        // is `at_level`'s job - and the dice do not move.
        assert_eq!(
            upcast_dice(Some("4d8"), None, 2, Some(5)).as_deref(),
            Some("4d8")
        );
    }

    #[test]
    fn a_cantrip_is_never_upcast() {
        // Its dice climb with the CASTER, not with a slot, and it
        // spends no slot to be raised by.
        assert_eq!(
            upcast_dice(Some("1d10"), Some("1d10"), 0, Some(5)).as_deref(),
            Some("1d10")
        );
    }

    #[test]
    fn a_lower_slot_does_not_subtract_dice() {
        // `may_spend_slot` refuses this before it gets here; if it ever
        // arrives, understating is safe and negative dice are not.
        assert_eq!(
            upcast_dice(Some("8d6"), Some("1d6"), 3, Some(1)).as_deref(),
            Some("8d6")
        );
    }

    #[test]
    fn a_broken_column_understates_rather_than_failing() {
        // The column is ours; a typo should cost damage, not stop a
        // cast in the middle of a fight.
        assert_eq!(
            upcast_dice(Some("8d6"), Some("banana"), 3, Some(5)).as_deref(),
            Some("8d6")
        );
    }

    #[test]
    fn no_dice_at_all_stays_none() {
        assert_eq!(upcast_dice(None, Some("1d6"), 3, Some(5)), None);
        assert_eq!(upcast_dice(Some("  "), Some("1d6"), 3, Some(5)), None);
    }

    #[test]
    fn every_shape_the_catalogue_actually_holds_scales_cleanly() {
        // 186. THE SWEEP'S SAFETY NET. `at_higher_dice` is data, and a
        // test cannot read the database - but it CAN pin the distinct
        // (dice, at_higher_dice) shapes the sweep produced, so a future
        // row in a shape nothing has seen is a shape somebody has to
        // think about rather than one that silently understates.
        //
        // Twenty pairs across 32 spells, taken straight out of the
        // catalogue after 186 ran.
        let pairs = [
            ("3d10", "1d10"), ("4d10", "1d10"),
            ("1d4", "1d4"), ("4d4", "1d4"), ("3d4+3", "1d4+1"),
            ("2d6", "1d6"), ("3d6", "1d6"), ("4d6", "1d6"),
            ("8d6", "1d6"), ("10d6", "1d6"), ("12d6", "1d6"),
            ("1d8", "1d8"), ("2d8", "1d8"), ("3d8", "1d8"),
            ("5d8", "1d8"), ("8d8", "1d8"),
            ("8d6", "2d6"), ("10d6", "2d6"), ("4d8", "2d8"),
            ("10d6+40", "3d6"),
        ];
        for (base, per) in pairs {
            for up in 1..=8 {
                let got = crate::dice::add_formula(base, per, up)
                    .unwrap_or_else(|e| panic!("{} + {}x{} failed: {}", base, up, per, e));
                // IT MUST STILL ROLL. A formula that scales into
                // something the dice engine cannot parse would fail at
                // the moment somebody casts, which is the worst place.
                crate::dice::roll_formula(&got)
                    .unwrap_or_else(|e| panic!("{} did not roll: {}", got, e));
            }
        }
    }

    #[test]
    fn the_two_that_carry_a_flat_part_keep_it_proportional() {
        // Magic Missile's dart and Disintegrate's +40 are the only flat
        // parts in the catalogue, and they behave differently ON
        // PURPOSE: a dart brings its +1 along, the +40 does not grow.
        assert_eq!(
            upcast_dice(Some("3d4+3"), Some("1d4+1"), 1, Some(4)).as_deref(),
            Some("6d4+6")
        );
        assert_eq!(
            upcast_dice(Some("10d6+40"), Some("3d6"), 6, Some(8)).as_deref(),
            Some("16d6+40")
        );
    }

    /* ---------- 183. a spell that simply lands ---------- */

    #[test]
    fn only_auto_lands_by_itself() {
        assert!(lands_automatically("Auto"));
    }

    #[test]
    fn the_other_four_do_not() {
        // EACH FOR ITS OWN REASON, which is why this is a list and not
        // a negation. Attack rolls to hit, Save lets the target resist,
        // Heal already has its own branch, and Utility is the pile this
        // was wrongly in - eight of the nine Utility spells carrying
        // dice must NOT fire at cast time.
        for other in ["Attack", "Save", "Heal", "Utility"] {
            assert!(!lands_automatically(other), "{} should not land by itself", other);
        }
    }

    #[test]
    fn an_unknown_cast_type_does_not_land() {
        // A typo or a word nobody taught this must not deal damage.
        // Inventing a hit is worse than refusing one - the same reason
        // `save_damage` returns None for a rule it does not know.
        for odd in ["", "auto", "AUTO", "Damage", "Automatic"] {
            assert!(!lands_automatically(odd), "{:?} should not land", odd);
        }
    }

    #[test]
    fn landing_by_itself_is_offensive() {
        // It deals damage, so it reads as an attack on the panel rather
        // than as a neutral utility - which is what it looked like for
        // as long as it was filed Utility.
        assert_eq!(stance("Auto"), Stance::Offensive);
    }

    /* ------------- 158. what a successful save is worth ------------- */

    #[test]
    fn a_failed_save_takes_the_lot_whichever_rule_it_is() {
        assert_eq!(save_damage(8, false, Some("half")), Some(8));
        assert_eq!(save_damage(8, false, Some("none")), Some(8));
    }

    #[test]
    fn half_rounds_down() {
        assert_eq!(save_damage(9, true, Some("half")), Some(4));
        assert_eq!(save_damage(8, true, Some("half")), Some(4));
        assert_eq!(save_damage(1, true, Some("half")), Some(0));
    }

    /// Sacred Flame is the cantrip that gives no mercy for a success.
    #[test]
    fn none_means_none() {
        assert_eq!(save_damage(8, true, Some("none")), Some(0));
    }

    /// The trap 158 exists for. Bane's 1d4 is a penalty, Geas's 5d10 is
    /// a daily toll, Bestow Curse's 1d8 is conditional and later - none
    /// of them is damage for failing the save.
    #[test]
    fn dice_that_are_not_save_damage_do_nothing() {
        assert_eq!(save_damage(4, false, None), None);
        assert_eq!(save_damage(50, false, None), None);
        assert_eq!(save_damage(4, false, Some("")), None);
        assert_eq!(save_damage(4, false, Some("   ")), None);
    }

    /// A word nobody taught it is nothing, not a guess.
    #[test]
    fn an_unknown_rule_deals_nothing() {
        assert_eq!(save_damage(8, true, Some("quarter")), None);
    }

    fn sacred_flame() -> Cast {
        cast("sp_sacredflame", "Sacred Flame", 0, "Save", Some("1 action"),
             Some("dex"), Some("1d8"), 3, 3)
    }

    fn cure_wounds() -> Cast {
        cast("sp_curewounds", "Cure Wounds", 1, "Heal", Some("1 action"),
             None, Some("1d8"), 3, 3)
    }

    fn guiding_bolt() -> Cast {
        cast("sp_guidingbolt", "Guiding Bolt", 1, "Attack", Some("1 action"),
             None, Some("4d6"), 3, 3)
    }

    /* ---------- what it costs ---------- */

    #[test]
    fn an_action_is_an_action() {
        assert_eq!(cost(Some("1 action")), Cost::Action);
    }

    #[test]
    fn healing_word_is_a_bonus_action() {
        // Which is most of why a cleric prepares it.
        assert_eq!(cost(Some("1 bonus action")), Cost::Bonus);
    }

    #[test]
    fn a_reaction_is_a_reaction() {
        assert_eq!(cost(Some("1 reaction, which you take when...")), Cost::Reaction);
    }

    #[test]
    fn prayer_of_healing_cannot_be_cast_in_a_fight() {
        // Ten minutes is a hundred rounds. Offering it on somebody's
        // turn would be offering a thing that cannot happen.
        assert_eq!(cost(Some("10 minutes")), Cost::TooLong);
        assert!(!cost(Some("10 minutes")).in_a_fight());
    }

    #[test]
    fn so_do_the_rituals_and_the_hour_long_ones() {
        assert_eq!(cost(Some("1 minute")), Cost::TooLong);
        assert_eq!(cost(Some("1 hour")), Cost::TooLong);
        assert_eq!(cost(Some("24 hours")), Cost::TooLong);
    }

    #[test]
    fn an_unknown_casting_time_is_too_long_rather_than_an_action() {
        // The safe way round: a spell the catalogue has not described
        // should not be offered as a free action in a fight.
        assert_eq!(cost(None), Cost::TooLong);
        assert_eq!(cost(Some("")), Cost::TooLong);
    }

    /* ---------- which way it points ---------- */

    #[test]
    fn an_attack_or_a_save_points_outward() {
        assert_eq!(stance("Attack"), Stance::Offensive);
        assert_eq!(stance("Save"), Stance::Offensive);
    }

    #[test]
    fn a_heal_points_inward() {
        assert_eq!(stance("Heal"), Stance::Defensive);
    }

    #[test]
    fn everything_else_is_neither() {
        assert_eq!(stance("Utility"), Stance::Neutral);
        assert_eq!(stance(""), Stance::Neutral);
    }

    /* ---------- what it rolls ---------- */

    #[test]
    fn an_attack_spell_carries_a_to_hit() {
        let c = guiding_bolt();
        assert_eq!(c.to_hit, Some(6), "proficiency 3 plus Wisdom 3");
        assert_eq!(c.save_dc, None);
    }

    #[test]
    fn a_save_spell_carries_a_dc_and_no_to_hit() {
        let c = sacred_flame();
        assert_eq!(c.save_dc, Some(14), "8 plus proficiency 3 plus Wisdom 3");
        assert_eq!(c.save_ability.as_deref(), Some("dex"));
        assert_eq!(c.to_hit, None);
    }

    #[test]
    fn a_heal_adds_the_modifier_to_the_dice() {
        // 5e, and it catches people out.
        assert_eq!(cure_wounds().dice.as_deref(), Some("1d8+3"));
    }

    #[test]
    fn and_damage_does_not() {
        assert_eq!(sacred_flame().dice.as_deref(), Some("1d8"));
        assert_eq!(guiding_bolt().dice.as_deref(), Some("4d6"));
    }

    #[test]
    fn a_negative_modifier_subtracts_rather_than_reading_as_a_plus() {
        let c = cast("x", "X", 1, "Heal", Some("1 action"), None, Some("1d8"), 2, -1);
        assert_eq!(c.dice.as_deref(), Some("1d8-1"));
    }

    #[test]
    fn a_modifier_of_zero_is_left_off_entirely() {
        let c = cast("x", "X", 1, "Heal", Some("1 action"), None, Some("1d8"), 2, 0);
        assert_eq!(c.dice.as_deref(), Some("1d8"), "1d8+0 is noise");
    }

    #[test]
    fn a_spell_with_no_dice_has_none() {
        let c = cast("x", "Bless", 1, "Utility", Some("1 action"), None, None, 3, 3);
        assert_eq!(c.dice, None);
    }

    /* ---------- slots ---------- */

    #[test]
    fn a_cantrip_costs_no_slot() {
        assert!(!sacred_flame().needs_slot);
    }

    #[test]
    fn everything_else_does() {
        assert!(cure_wounds().needs_slot);
        assert!(guiding_bolt().needs_slot);
    }

    /* ---------- how long it lasts ---------- */

    #[test]
    fn instantaneous_leaves_nothing_behind() {
        // Different from lasting no time: there is no effect to make.
        assert_eq!(lasts(Some("Instantaneous")), Lasts::Instant);
        assert_eq!(lasts(None), Lasts::Instant);
        assert_eq!(lasts(Some("")), Lasts::Instant);
    }

    #[test]
    fn bless_lasts_a_minute() {
        // "Up to" is concentration's wording and changes nothing - ten
        // rounds either way, and a caster may drop it sooner.
        assert_eq!(lasts(Some("Up to 1 minute")), Lasts::Ticks(crate::clock::MINUTE));
    }

    #[test]
    fn spirit_guardians_lasts_ten() {
        assert_eq!(lasts(Some("Up to 10 minutes")), Lasts::Ticks(100));
    }

    #[test]
    fn the_longer_ones_read_too() {
        assert_eq!(lasts(Some("1 hour")), Lasts::Ticks(crate::clock::HOUR));
        assert_eq!(lasts(Some("8 hours")), Lasts::Ticks(8 * crate::clock::HOUR));
        assert_eq!(lasts(Some("24 hours")), Lasts::Ticks(crate::clock::DAY));
        assert_eq!(lasts(Some("10 days")), Lasts::Ticks(10 * crate::clock::DAY));
        assert_eq!(lasts(Some("1 round")), Lasts::Ticks(1));
    }

    #[test]
    fn until_dispelled_has_no_end() {
        assert_eq!(lasts(Some("Until dispelled")), Lasts::Indefinite);
        assert_eq!(lasts(Some("Until dispelled or triggered")), Lasts::Indefinite);
        assert_eq!(lasts(Some("Special")), Lasts::Indefinite);
    }

    #[test]
    fn something_unreadable_lingers_rather_than_vanishing() {
        // An effect a DM can see and end beats one that silently never
        // existed.
        assert_eq!(lasts(Some("as long as the song holds")), Lasts::Indefinite);
        assert_eq!(lasts(Some("3 fortnights")), Lasts::Indefinite);
    }

    /* ---------- the card ---------- */

    #[test]
    fn the_label_says_what_it_takes_to_land() {
        assert_eq!(label(&sacred_flame()), "Sacred Flame \u{2014} DEX save DC 14, 1d8");
        assert_eq!(label(&guiding_bolt()), "Guiding Bolt \u{2014} +6 to hit, 4d6");
        assert_eq!(label(&cure_wounds()), "Cure Wounds \u{2014} 1d8+3");
    }

    #[test]
    fn a_spell_with_nothing_to_roll_is_just_its_name() {
        let c = cast("x", "Bless", 1, "Utility", Some("1 action"), None, None, 3, 3);
        assert_eq!(label(&c), "Bless");
    }
}
