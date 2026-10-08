//! A creature as a file: what travels, and what to do when it arrives
//! somewhere that has never heard of half of it.
//!
//! 125. Templates belong to a game - 123 took that decision - so the way
//! one gets to another game, or into a backup, is a file. This module is
//! the shape of that file and the rule for reading one; the rows are
//! `commands/creatures.rs`'s business.
//!
//! ---------------------------------------------------------------------
//! KEYS TRAVEL, IDS DO NOT
//! ---------------------------------------------------------------------
//!
//! Nothing in here carries a uuid. A creature is a name, some numbers,
//! and a pile of KEYS into catalogues - `item_key`, `spell_key`,
//! `species_key`, `class_key`. That is what makes the file portable and
//! it is also the whole problem: the game receiving it may not have
//! them.
//!
//! SKIP WITH A WARNING, NOT REFUSE. A creature carrying one unknown
//! trinket is still worth having, and a file that will not open because
//! of a torch is a worse answer than a goblin with no torch. Every skip
//! is reported by name so nobody discovers it in a fight.
//!
//! THE EXCEPTION IS THE ENVELOPE ITSELF. A file that is not this format,
//! or is a version this build does not know, IS refused - there is
//! nothing to salvage from a shape we cannot read, and guessing would
//! import nonsense silently.
//!
//! ---------------------------------------------------------------------
//! WHAT A SKIP COSTS, SAID OUT LOUD
//! ---------------------------------------------------------------------
//!
//! The warnings are not all the same weight and the text says which:
//! losing a torch is a torch, losing a SPECIES costs ability bonuses,
//! a size and maybe a resistance. A DM reading the list should be able
//! to tell the difference without knowing the schema.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// What the file says it is. A reader checks this before anything else.
pub const FORMAT: &str = "odyssey1e.creature";

/// Bumped when the shape changes in a way an older reader would get
/// wrong. An older FILE stays readable - see `admits`.
pub const VERSION: i64 = 1;

/// 152. The three states `character_spells.state` allows. The column is
/// NOT NULL and defaults to 'prepared'; 'cantrip' is known rather than
/// prepared, and 'book' is held but not up today.
pub const STATES: [&str; 3] = ["cantrip", "prepared", "book"];

/// One thing that did not survive the journey.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warning {
    /// What kind of thing: "item", "spell", "skill", "class", "species".
    pub what: String,
    /// The key that was not found.
    pub key: String,
    /// What it costs, in words a DM can act on.
    pub note: String,
}

impl Warning {
    fn new(what: &str, key: &str, note: &str) -> Warning {
        Warning { what: what.into(), key: key.into(), note: note.into() }
    }
}

/// One thing a creature carries, and whatever is inside it.
///
/// NESTED, BECAUSE A BACKPACK IS NOT A FLAT LIST. 124 learned this the
/// hard way on the copy path: a flat kit leaves a backpack's contents
/// hanging off nobody.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct KitItem {
    pub item_key: String,
    #[serde(default = "one")]
    pub quantity: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<String>,
    #[serde(default)]
    pub attuned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proficient_override: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uses_spent: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uses_max: Option<i64>,
    /// 100's vocabulary, carried whole - an enchantment is part of the
    /// object rather than of the catalogue row it came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grants: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contents: Vec<KitItem>,
}

fn one() -> i64 {
    1
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AbilityScore {
    pub ability: String,
    pub score: i64,
    #[serde(default)]
    pub save_prof: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillProf {
    pub skill_key: String,
    pub prof: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClassLevel {
    pub class_key: String,
    pub level: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub class_key: String,
    pub feature_key: String,
    pub pick: i64,
    pub choice: String,
}

/// 152. A HELD SPELL CARRIES ITS STATE, which is one of three words and not
/// a yes or no. `character_spells.state` is 'cantrip', 'prepared' or
/// 'book': a cantrip is KNOWN rather than prepared, and a spell in the
/// book is neither. A boolean can hold two of those and silently turns
/// the third into something the rules have no word for.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HeldSpell {
    pub spell_key: String,
    /// Defaults to the column's own default, so a hand-written file may
    /// leave it out and mean the ordinary case.
    #[serde(default = "prepared")]
    pub state: String,
}

fn prepared() -> String {
    "prepared".into()
}

/// A creature, with nothing local in it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Creature {
    pub name: String,
    #[serde(default = "one")]
    pub level: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creature_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp_max: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ac_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ac_override: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prof_bonus: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub species_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_key: Option<String>,
    /// Where it came from, if it was ever imported from a statblock.
    /// PROVENANCE ONLY - losing it costs nothing but the breadcrumb.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npc_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weapon_profs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub armor_profs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_profs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abilities: Vec<AbilityScore>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<SkillProf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<ClassLevel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<Choice>,
    /// 176. WAS `prayers` AND STILL READS AS ONE. Every creature file
    /// written before the rename says "prayers", and `admits` promises
    /// an older file opens - so the alias is that promise, not a
    /// convenience. Exports write `spells` from now on.
    #[serde(default, alias = "prayers", skip_serializing_if = "Vec::is_empty")]
    pub spells: Vec<HeldSpell>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kit: Vec<KitItem>,
}

/// The file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub format: String,
    pub version: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exported_at: Option<String>,
    pub creature: Creature,
}

impl Envelope {
    pub fn wrap(creature: Creature, exported_at: Option<String>) -> Envelope {
        Envelope { format: FORMAT.into(), version: VERSION, exported_at, creature }
    }
}

/// What the receiving game actually has.
#[derive(Debug, Clone, Default)]
pub struct Known {
    pub items: HashSet<String>,
    pub spells: HashSet<String>,
    pub skills: HashSet<String>,
    pub classes: HashSet<String>,
    pub species: HashSet<String>,
    pub npcs: HashSet<String>,
}

/// Whether this build can read that file.
///
/// AN OLDER FILE IS FINE and a NEWER one is not. Every field added since
/// version 1 is optional, so a version-1 file parses into today's
/// structs with the new fields defaulted - that is what `serde(default)`
/// is doing on nearly everything here. A file from a LATER version may
/// rely on a field this build has never heard of, and reading it would
/// quietly drop whatever that was.
pub fn admits(format: &str, version: i64) -> Result<(), String> {
    if format != FORMAT {
        return Err(format!(
            "that is not a creature file - it says \"{}\" and should say \"{}\"",
            format, FORMAT
        ));
    }
    if version > VERSION {
        return Err(format!(
            "that file is version {} and this build reads up to {} - update first",
            version, VERSION
        ));
    }
    Ok(())
}

/// Drop everything the receiving game cannot resolve, and say what went.
///
/// THE CREATURE COMES BACK CHANGED rather than refused. A torch nobody
/// has is a torch, and a file that will not open because of one is a
/// worse answer than a goblin with no torch.
///
/// A CONTAINER TAKES ITS CONTENTS WITH IT. Losing a backpack and keeping
/// what was inside would put a loose torch in somebody's hands, which is
/// not what the file said. The warning says how many went with it.
pub fn vet(mut c: Creature, known: &Known) -> (Creature, Vec<Warning>) {
    let mut warn = Vec::new();

    if let Some(k) = c.species_key.clone() {
        if !known.species.contains(&k) {
            c.species_key = None;
            warn.push(Warning::new(
                "species", &k,
                "not in this game - the creature keeps its own numbers but loses \
                 any ability bonus, size or resistance its people gave it",
            ));
        }
    }

    if let Some(k) = c.class_key.clone() {
        if !known.classes.contains(&k) {
            c.class_key = None;
            warn.push(Warning::new("class", &k, "not in this game - dropped"));
        }
    }

    if let Some(k) = c.npc_key.clone() {
        if !known.npcs.contains(&k) {
            c.npc_key = None;
            warn.push(Warning::new(
                "statblock", &k,
                "not in this game - only the note of where it came from is lost",
            ));
        }
    }

    if let Some(t) = c.creature_type.clone() {
        if crate::creature::Kind::parse(&t).is_none() {
            c.creature_type = None;
            warn.push(Warning::new(
                "creature type", &t,
                "not one of 5e's fourteen - cleared, so nothing reads it wrongly",
            ));
        }
    }

    c.classes.retain(|x| {
        let ok = known.classes.contains(&x.class_key);
        if !ok {
            warn.push(Warning::new(
                "class",
                &x.class_key,
                "not in this game - those levels are gone, and so is anything                  chosen for them",
            ));
        }
        ok
    });

    let kept: HashSet<String> = c.classes.iter().map(|x| x.class_key.clone()).collect();
    c.choices.retain(|x| kept.contains(&x.class_key) || Some(&x.class_key) == c.class_key.as_ref());

    c.skills.retain(|x| {
        let ok = known.skills.contains(&x.skill_key);
        if !ok {
            warn.push(Warning::new("skill", &x.skill_key, "not in this game - the proficiency is dropped"));
        }
        ok
    });

    c.spells.retain_mut(|x| {
        let ok = known.spells.contains(&x.spell_key);
        if !ok {
            warn.push(Warning::new("spell", &x.spell_key, "not in this game - not prepared"));
        } else if !STATES.contains(&x.state.as_str()) {
            // 152. A STATE THE COLUMN WOULD REFUSE. Keeping the spell and
            // correcting the word beats dropping a spell the game does
            // have over one bad field, and the warning says what happened.
            warn.push(Warning::new(
                "spell",
                &x.spell_key,
                "not a spell state - taken as prepared",
            ));
            x.state = prepared();
        }
        ok
    });

    c.kit = vet_kit(c.kit, known, &mut warn);
    (c, warn)
}

/// A kit, recursively. A container that goes takes its contents.
fn vet_kit(kit: Vec<KitItem>, known: &Known, warn: &mut Vec<Warning>) -> Vec<KitItem> {
    let mut out = Vec::new();
    for mut it in kit {
        if !known.items.contains(&it.item_key) {
            let inside = count_kit(&it.contents);
            warn.push(Warning::new(
                "item",
                &it.item_key,
                &if inside == 0 {
                    "not in this game - dropped".to_string()
                } else {
                    format!(
                        "not in this game - dropped, and the {} thing(s) inside it went too",
                        inside
                    )
                },
            ));
            continue;
        }
        it.contents = vet_kit(it.contents, known, warn);
        out.push(it);
    }
    out
}

/// Everything in a kit, nesting included.
pub fn count_kit(kit: &[KitItem]) -> usize {
    kit.iter().map(|i| 1 + count_kit(&i.contents)).sum()
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn known_of(items: &[&str], spells: &[&str], skills: &[&str], classes: &[&str], species: &[&str]) -> Known {
        Known {
            items: items.iter().map(|s| s.to_string()).collect(),
            spells: spells.iter().map(|s| s.to_string()).collect(),
            skills: skills.iter().map(|s| s.to_string()).collect(),
            classes: classes.iter().map(|s| s.to_string()).collect(),
            species: species.iter().map(|s| s.to_string()).collect(),
            npcs: HashSet::new(),
        }
    }

    fn item(key: &str, contents: Vec<KitItem>) -> KitItem {
        KitItem { item_key: key.into(), quantity: 1, contents, ..Default::default() }
    }

    /* ---------- the envelope ---------- */

    #[test]
    fn a_file_that_is_not_one_of_ours_is_refused() {
        let e = admits("some.other.thing", 1).unwrap_err();
        assert!(e.contains("not a creature file"), "{}", e);
    }

    #[test]
    fn a_file_from_the_future_is_refused_rather_than_half_read() {
        let e = admits(FORMAT, VERSION + 1).unwrap_err();
        assert!(e.contains("update first"), "{}", e);
    }

    #[test]
    fn todays_version_is_admitted() {
        assert!(admits(FORMAT, VERSION).is_ok());
    }

    #[test]
    fn an_older_file_stays_readable() {
        // Every field added since is optional, which is what the
        // serde(default)s are for.
        assert!(admits(FORMAT, 1).is_ok());
    }

    #[test]
    fn a_creature_round_trips_through_json() {
        let c = Creature {
            name: "Cave Goblin".into(),
            level: 2,
            creature_type: Some("humanoid".into()),
            abilities: vec![AbilityScore { ability: "str".into(), score: 8, save_prof: false }],
            kit: vec![item("backpack", vec![item("torch", vec![])])],
            ..Default::default()
        };
        let text = serde_json::to_string(&Envelope::wrap(c.clone(), None)).unwrap();
        let back: Envelope = serde_json::from_str(&text).unwrap();
        assert_eq!(back.creature, c);
        assert_eq!(back.format, FORMAT);
    }

    #[test]
    fn nothing_local_is_in_the_file() {
        let text = serde_json::to_string(&Envelope::wrap(
            Creature { name: "x".into(), ..Default::default() },
            None,
        ))
        .unwrap();
        for local in ["game_id", "owner_uid", "entity_id", "\"id\""] {
            assert!(!text.contains(local), "{} leaked into the file: {}", local, text);
        }
    }

    /* ---------- skipping, and saying so ---------- */

    #[test]
    fn everything_known_survives_untouched() {
        let c = Creature {
            name: "Cave Goblin".into(),
            species_key: Some("goblin".into()),
            skills: vec![SkillProf { skill_key: "ste".into(), prof: 1.0 }],
            spells: vec![HeldSpell { spell_key: "sp_bless".into(), state: "prepared".into() }],
            kit: vec![item("torch", vec![])],
            ..Default::default()
        };
        let k = known_of(&["torch"], &["sp_bless"], &["ste"], &[], &["goblin"]);
        let (out, warn) = vet(c.clone(), &k);
        assert_eq!(out, c);
        assert!(warn.is_empty(), "{:?}", warn);
    }

    #[test]
    fn an_unknown_item_is_dropped_and_named() {
        let c = Creature {
            name: "x".into(),
            kit: vec![item("torch", vec![]), item("moonblade", vec![])],
            ..Default::default()
        };
        let (out, warn) = vet(c, &known_of(&["torch"], &[], &[], &[], &[]));
        assert_eq!(out.kit.len(), 1);
        assert_eq!(out.kit[0].item_key, "torch");
        assert_eq!(warn.len(), 1);
        assert_eq!(warn[0].key, "moonblade");
        assert_eq!(warn[0].what, "item");
    }

    /// LOSING A BACKPACK AND KEEPING THE TORCH would put a loose torch in
    /// somebody's hands, which is not what the file said.
    #[test]
    fn a_container_that_goes_takes_its_contents() {
        let c = Creature {
            name: "x".into(),
            kit: vec![item("chest", vec![item("torch", vec![]), item("rope", vec![])])],
            ..Default::default()
        };
        let (out, warn) = vet(c, &known_of(&["torch", "rope"], &[], &[], &[], &[]));
        assert!(out.kit.is_empty());
        assert_eq!(warn.len(), 1);
        assert!(warn[0].note.contains("2 thing(s) inside it"), "{}", warn[0].note);
    }

    #[test]
    fn a_known_container_keeps_what_it_can_and_drops_what_it_cannot() {
        let c = Creature {
            name: "x".into(),
            kit: vec![item("backpack", vec![item("torch", vec![]), item("moonblade", vec![])])],
            ..Default::default()
        };
        let (out, warn) = vet(c, &known_of(&["backpack", "torch"], &[], &[], &[], &[]));
        assert_eq!(out.kit.len(), 1);
        assert_eq!(out.kit[0].contents.len(), 1);
        assert_eq!(out.kit[0].contents[0].item_key, "torch");
        assert_eq!(warn.len(), 1);
        assert_eq!(warn[0].key, "moonblade");
    }

    #[test]
    fn nesting_is_counted_all_the_way_down() {
        let kit = vec![item("a", vec![item("b", vec![item("c", vec![])])])];
        assert_eq!(count_kit(&kit), 3);
    }

    #[test]
    fn a_lost_species_says_what_it_costs() {
        let c = Creature { name: "x".into(), species_key: Some("untgaroth".into()), ..Default::default() };
        let (out, warn) = vet(c, &known_of(&[], &[], &[], &[], &[]));
        assert_eq!(out.species_key, None);
        assert_eq!(warn[0].what, "species");
        assert!(warn[0].note.contains("ability bonus"), "{}", warn[0].note);
    }

    #[test]
    fn a_lost_statblock_says_it_costs_only_the_breadcrumb() {
        let c = Creature { name: "x".into(), npc_key: Some("goblin".into()), ..Default::default() };
        let (out, warn) = vet(c, &known_of(&[], &[], &[], &[], &[]));
        assert_eq!(out.npc_key, None);
        assert!(warn[0].note.contains("where it came from"), "{}", warn[0].note);
    }

    #[test]
    fn an_unknown_spell_is_not_prepared() {
        let c = Creature {
            name: "x".into(),
            spells: vec![
                HeldSpell { spell_key: "sp_bless".into(), state: "cantrip".into() },
                HeldSpell { spell_key: "sp_invented".into(), state: "prepared".into() },
            ],
            ..Default::default()
        };
        let (out, warn) = vet(c, &known_of(&[], &["sp_bless"], &[], &[], &[]));
        assert_eq!(out.spells.len(), 1);
        assert_eq!(warn.len(), 1);
        assert_eq!(warn[0].key, "sp_invented");
    }

    /// 152. A cantrip is not a prepared spell and must not arrive as one.
    /// This is the round trip a boolean could not make.
    #[test]
    fn a_cantrip_survives_the_journey_as_a_cantrip() {
        let c = Creature {
            name: "x".into(),
            spells: vec![
                HeldSpell { spell_key: "sp_light".into(), state: "cantrip".into() },
                HeldSpell { spell_key: "sp_bless".into(), state: "book".into() },
            ],
            ..Default::default()
        };
        let (out, warn) = vet(c, &known_of(&[], &["sp_light", "sp_bless"], &[], &[], &[]));
        assert!(warn.is_empty(), "{:?}", warn);
        assert_eq!(out.spells[0].state, "cantrip");
        assert_eq!(out.spells[1].state, "book");
    }

    /// A word the column would refuse costs the state, not the spell.
    #[test]
    fn a_state_the_column_would_refuse_is_taken_as_prepared() {
        let c = Creature {
            name: "x".into(),
            spells: vec![HeldSpell { spell_key: "sp_bless".into(), state: "memorised".into() }],
            ..Default::default()
        };
        let (out, warn) = vet(c, &known_of(&[], &["sp_bless"], &[], &[], &[]));
        assert_eq!(out.spells.len(), 1, "the spell is kept");
        assert_eq!(out.spells[0].state, "prepared");
        assert_eq!(warn.len(), 1);
        assert!(warn[0].note.contains("prepared"), "{}", warn[0].note);
    }

    /// A file that leaves the field out means the ordinary case, which is
    /// the column's own default.
    #[test]
    fn a_held_spell_with_no_state_is_prepared() {
        let p: HeldSpell = serde_json::from_str(r#"{"spell_key":"sp_bless"}"#).unwrap();
        assert_eq!(p.state, "prepared");
    }

    #[test]
    fn a_creature_type_that_is_not_one_of_the_fourteen_is_cleared() {
        let c = Creature { name: "x".into(), creature_type: Some("undeadd".into()), ..Default::default() };
        let (out, warn) = vet(c, &known_of(&[], &[], &[], &[], &[]));
        assert_eq!(out.creature_type, None);
        assert_eq!(warn[0].what, "creature type");
    }

    #[test]
    fn a_choice_whose_class_went_goes_with_it() {
        // A subclass pick for a class this game does not have is a
        // dangling decision nothing will ever read.
        let c = Creature {
            name: "x".into(),
            classes: vec![ClassLevel { class_key: "bard".into(), level: 3 }],
            choices: vec![Choice {
                class_key: "bard".into(),
                feature_key: "subclass".into(),
                pick: 1,
                choice: "lore".into(),
            }],
            ..Default::default()
        };
        let (out, warn) = vet(c, &known_of(&[], &[], &[], &[], &[]));
        assert!(out.classes.is_empty());
        assert!(out.choices.is_empty(), "a choice outlived its class");
        // AND IT SAYS WHICH CLASS, by name, like every other warning.
        assert_eq!(warn.len(), 1);
        assert_eq!(warn[0].key, "bard");
        assert!(warn[0].note.contains("chosen for them"), "{}", warn[0].note);
    }

    #[test]
    fn the_creature_still_arrives_when_everything_is_unknown() {
        // The whole point of skipping rather than refusing.
        let c = Creature {
            name: "Stranger".into(),
            level: 5,
            hp_max: Some(40),
            species_key: Some("nope".into()),
            skills: vec![SkillProf { skill_key: "nope".into(), prof: 1.0 }],
            kit: vec![item("nope", vec![])],
            ..Default::default()
        };
        let (out, warn) = vet(c, &Known::default());
        assert_eq!(out.name, "Stranger");
        assert_eq!(out.level, 5);
        assert_eq!(out.hp_max, Some(40));
        assert_eq!(warn.len(), 3);
    }
}
