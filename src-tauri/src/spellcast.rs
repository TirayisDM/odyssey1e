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
        "Attack" | "Save" => Stance::Offensive,
        "Heal" => Stance::Defensive,
        _ => Stance::Neutral,
    }
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
    wis_mod: i64,
) -> Cast {
    let stance = stance(cast_type);
    let to_hit = (cast_type == "Attack").then(|| crate::prayers::attack_bonus(prof_bonus, wis_mod));
    let save_dc = save_ability
        .filter(|s| !s.is_empty())
        .map(|_| crate::prayers::save_dc(prof_bonus, wis_mod));

    let dice = dice.filter(|d| !d.is_empty()).map(|d| {
        if cast_type == "Heal" && wis_mod != 0 {
            format!("{}{}{}", d, if wis_mod > 0 { "+" } else { "-" }, wis_mod.abs())
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
