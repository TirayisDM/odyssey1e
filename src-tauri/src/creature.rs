//! What a thing IS, in 5e's sense.
//!
//! 122. Fifteen spells already in the catalogue name a creature type in
//! their text and not one of them can be checked, because nothing in
//! this schema records what anything is. Hold Person works only on a
//! humanoid. Cure Wounds does nothing for a construct or the undead.
//! Protection from Evil and Good names six types outright. Gentle
//! Repose stops a corpse becoming undead. All of it has been a DM call
//! for want of one column.
//!
//! ---------------------------------------------------------------------
//! A TYPE IS NOT A SPECIES AND NOT A ROLE
//! ---------------------------------------------------------------------
//!
//! `species` is what people you are - Unt'garoth, Ny'ook - and is this
//! world's own. `is_npc` is whether somebody is playing you. TYPE is
//! 5e's fourteen-way classification, and it is the one of the three
//! that spells are written against.
//!
//! THE FOURTEEN ARE CLOSED. 5e does not let a DM invent a fifteenth,
//! and neither does this: an unknown word is refused rather than
//! carried, because a creature typed `undeadd` would read correctly on
//! a screen and match nothing any spell ever asks for. That is this
//! codebase's named defect and the cheapest place to stop it is the
//! parse.
//!
//! NONE IS A REAL ANSWER. A character whose type nobody has stated is
//! not secretly a humanoid - it is unstated, and a rule that needs the
//! type says so rather than guessing. Every creature in the game had no
//! type at all until 122 and defaulting them all to humanoid would have
//! asserted something about this world that its designer had not said.

use serde::{Deserialize, Serialize};

/// 5e's fourteen, in the book's own order.
///
/// READ BY THE TESTS AND BY THE MIGRATION'S CHECK CONSTRAINT rather
/// than by running code: 122 writes the same fourteen into
/// `characters_creature_type_check` so a REST write that never passed
/// through Rust is refused too. This is the copy a human reads.
#[allow(dead_code, reason = "the canonical list; the DB constraint is its twin")]
pub const TYPES: &[&str] = &[
    "aberration", "beast", "celestial", "construct", "dragon", "elemental",
    "fey", "fiend", "giant", "humanoid", "monstrosity", "ooze", "plant", "undead",
];

/// The six that the "evil and good" spells are written against:
/// Protection from Evil and Good, Detect Evil and Good, and Dispel Evil
/// and Good all name exactly this set.
///
/// NAMED ONCE BECAUSE THREE SPELLS SHARE IT. Writing the list into each
/// spell's grant would be the same fact in three places, and the fourth
/// spell that wants it would make four.
#[allow(
    dead_code,
    reason = "waiting on the spell wiring: Protection from Evil and Good,               Detect Evil and Good and Dispel Evil and Good all name these six"
)]
pub const OUTSIDERS: &[&str] = &[
    "aberration", "celestial", "elemental", "fey", "fiend", "undead",
];

/// What the healing spells refuse. Cure Wounds, Healing Word, Mass Cure
/// Wounds, Heal and Spare the Dying all say the same thing: nothing for
/// a construct or the undead.
#[allow(
    dead_code,
    reason = "waiting on the spell wiring: Cure Wounds, Healing Word, Mass Cure               Wounds, Heal and Spare the Dying all refuse these two"
)]
pub const UNHEALABLE: &[&str] = &["construct", "undead"];

/// One of the fourteen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Aberration, Beast, Celestial, Construct, Dragon, Elemental,
    Fey, Fiend, Giant, Humanoid, Monstrosity, Ooze, Plant, Undead,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Aberration => "aberration",
            Kind::Beast => "beast",
            Kind::Celestial => "celestial",
            Kind::Construct => "construct",
            Kind::Dragon => "dragon",
            Kind::Elemental => "elemental",
            Kind::Fey => "fey",
            Kind::Fiend => "fiend",
            Kind::Giant => "giant",
            Kind::Humanoid => "humanoid",
            Kind::Monstrosity => "monstrosity",
            Kind::Ooze => "ooze",
            Kind::Plant => "plant",
            Kind::Undead => "undead",
        }
    }

    /// Read one off a stored value.
    ///
    /// CASE AND SURROUNDING SPACE ARE NOT A DIFFERENT ANSWER, because a
    /// DM types these. Anything that is not one of the fourteen is
    /// None - see the note at the top about `undeadd`.
    pub fn parse(s: &str) -> Option<Kind> {
        match s.trim().to_lowercase().as_str() {
            "aberration" => Some(Kind::Aberration),
            "beast" => Some(Kind::Beast),
            "celestial" => Some(Kind::Celestial),
            "construct" => Some(Kind::Construct),
            "dragon" => Some(Kind::Dragon),
            "elemental" => Some(Kind::Elemental),
            "fey" => Some(Kind::Fey),
            "fiend" => Some(Kind::Fiend),
            "giant" => Some(Kind::Giant),
            "humanoid" => Some(Kind::Humanoid),
            "monstrosity" => Some(Kind::Monstrosity),
            "ooze" => Some(Kind::Ooze),
            "plant" => Some(Kind::Plant),
            "undead" => Some(Kind::Undead),
            _ => None,
        }
    }

    /// Whether the "evil and good" spells reach this creature.
    #[allow(dead_code, reason = "waiting on the spell wiring - see OUTSIDERS")]
    pub fn is_outsider(self) -> bool {
        OUTSIDERS.contains(&self.as_str())
    }

    /// Whether healing does anything for it.
    #[allow(dead_code, reason = "waiting on the spell wiring - see UNHEALABLE")]
    pub fn can_be_healed(self) -> bool {
        !UNHEALABLE.contains(&self.as_str())
    }
}

/// Whether a stored value is one of the fourteen.
///
/// FOR THE WRITE PATH, which is where a typo is cheap to refuse and
/// expensive to discover later. The database carries the same list as a
/// CHECK constraint - two copies on purpose, the same arrangement
/// `character_slots` has had since 107, because the constraint also
/// catches a write that never went through this code.
#[allow(
    dead_code,
    reason = "for the write path, which arrives with the Creatures tab; the CHECK               constraint from 122 guards the database until then"
)]
pub fn known(s: &str) -> bool {
    Kind::parse(s).is_some()
}

/// What a creature is, given what its own row says and what its people
/// say.
///
/// THE CHARACTER'S OWN WORD WINS, and is almost always absent. A people
/// has a type - every Ny'ook is the same thing - so the answer normally
/// comes from the species and the character column exists for the
/// exception: one cursed Unt'garoth who is now undead.
///
/// THE SAME SHAPE `carry_size` USES, which takes the species' answer
/// unless the character states its own. One rule, written once.
pub fn of(own: Option<&str>, species: Option<&str>) -> Option<Kind> {
    own.and_then(Kind::parse).or_else(|| species.and_then(Kind::parse))
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_fourteen_are_here_and_round_trip() {
        assert_eq!(TYPES.len(), 14);
        for t in TYPES {
            let k = Kind::parse(t).unwrap_or_else(|| panic!("{} did not parse", t));
            assert_eq!(k.as_str(), *t, "{} did not round trip", t);
        }
    }

    #[test]
    fn a_dm_types_these_so_case_and_space_are_forgiven() {
        assert_eq!(Kind::parse("  Undead "), Some(Kind::Undead));
        assert_eq!(Kind::parse("HUMANOID"), Some(Kind::Humanoid));
    }

    /// THE SILENT NO-OP THIS EXISTS TO STOP. A creature typed `undeadd`
    /// reads correctly on a screen and matches nothing any spell asks.
    #[test]
    fn a_fifteenth_type_is_refused() {
        for s in ["undeadd", "humanoids", "outsider", "npc", "", "   "] {
            assert_eq!(Kind::parse(s), None, "{} should not parse", s);
            assert!(!known(s));
        }
    }

    /* ---------- the groupings the catalogue already names ---------- */

    #[test]
    fn the_evil_and_good_spells_name_exactly_six() {
        assert_eq!(OUTSIDERS.len(), 6);
        for t in OUTSIDERS {
            assert!(Kind::parse(t).unwrap().is_outsider(), "{}", t);
        }
        // A beast is not something Protection from Evil and Good wards.
        assert!(!Kind::Beast.is_outsider());
        assert!(!Kind::Humanoid.is_outsider());
        assert!(!Kind::Construct.is_outsider(), "a construct is not an outsider in 5e");
    }

    #[test]
    fn healing_does_nothing_for_a_construct_or_the_undead() {
        assert!(!Kind::Construct.can_be_healed());
        assert!(!Kind::Undead.can_be_healed());
        assert!(Kind::Humanoid.can_be_healed());
        assert!(Kind::Beast.can_be_healed());
        // An elemental is not alive in the ordinary sense and 5e heals
        // it anyway - the exclusion is exactly two types, not a vibe.
        assert!(Kind::Elemental.can_be_healed());
    }

    /* ---------- whose word decides ---------- */

    #[test]
    fn a_people_answers_for_its_members() {
        assert_eq!(of(None, Some("giant")), Some(Kind::Giant));
    }

    #[test]
    fn a_creature_may_contradict_its_people() {
        // One cursed Unt'garoth who is now undead.
        assert_eq!(of(Some("undead"), Some("giant")), Some(Kind::Undead));
    }

    #[test]
    fn unstated_is_unstated_and_not_secretly_humanoid() {
        // Every creature in the game had no type until 122, and
        // defaulting them would assert something about this world that
        // its designer had not said.
        assert_eq!(of(None, None), None);
    }

    #[test]
    fn a_nonsense_override_falls_back_rather_than_erasing_the_people() {
        // The column is constrained, so this should be unreachable -
        // but if a bad value ever lands, the species still answers.
        assert_eq!(of(Some("undeadd"), Some("humanoid")), Some(Kind::Humanoid));
    }
}
