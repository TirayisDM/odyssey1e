//! The conditions, as a closed list.
//!
//! THE RULES ONLY. Reading and writing the rows is
//! `commands/effects.rs`; a condition IS an effect - 094 built that
//! table and its own comment names `poisoned` as an example key and
//! NULL expiry as "a condition a DM is tracking by hand". Nothing new
//! is stored here. This says which keys are conditions, what they are
//! called, and what each one does.
//!
//! ---------------------------------------------------------------------
//! WHY THIS IS NOT A GRANT
//! ---------------------------------------------------------------------
//!
//! 100's grants are one vocabulary for things that change A NUMBER -
//! `ac`, `attack`, `save`, `resist.fire` - and `grants::parse` drops
//! any row with neither a value nor dice, because a grant with no
//! number is not a grant.
//!
//! A CONDITION IS NOT A NUMBER. "Blinded" is: you cannot see, you fail
//! any check that needs sight, attacks against you have ADVANTAGE and
//! yours have DISADVANTAGE. Advantage is not +4 - it is a second d20,
//! and 5e is careful never to price it. Squeezing that into a grant
//! would mean inventing a number the book refuses to give, which is
//! 158's and 183's lesson in a third place.
//!
//! So a condition is an effect with a condition KEY, and what it does
//! is text a DM reads. The engine knows WHICH conditions exist and
//! WHEN they end; it does not pretend to apply advantage.
//!
//! ---------------------------------------------------------------------
//! THE KEY IS NAMESPACED, AND THAT IS LOAD-BEARING
//! ---------------------------------------------------------------------
//!
//! Effect keys are spell keys today - `sp_holdperson`, `sp_bless`. A
//! condition shares the table, so `cond.` tells them apart in one
//! comparison and without a column. It also means Hold Person can put
//! BOTH on somebody: the spell, which is what is concentrating and what
//! ends when concentration drops, and the condition, which is what the
//! target is actually suffering. Those are two facts and the panel was
//! only ever showing the first.
//!
//! BOTH SPELLINGS ANSWER. The prose in this project is British and the
//! published list is American, so `cond.paralysed` and
//! `cond.paralyzed` are the same condition. Writing one and reading
//! back nothing would be a silent miss, and a DM should not have to
//! know which half of the catalogue they are standing in.
//!
//! ---------------------------------------------------------------------
//! THE GLYPHS ARE PLACEHOLDERS AND SAY SO
//! ---------------------------------------------------------------------
//!
//! Dave's plan is for these to hang on character modals once there are
//! graphic elements. Until then they are one character with the real
//! answer in the tooltip, which is why `said` leads with the NAME - a
//! symbol nobody can decode is a symbol, and the hover has to carry
//! the whole meaning.

/// One condition: what it is called, and what it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Condition {
    /// The key WITHOUT the `cond.` prefix, in this project's spelling.
    pub key: &'static str,
    pub name: &'static str,
    /// A placeholder until the graphics land. One character, because a
    /// row of these sits in a line of chips.
    pub glyph: &'static str,
    /// What it does, in the fewest words that are still true. This is
    /// the tooltip and it is the only thing that carries the meaning.
    pub does: &'static str,
}

/// The fifteen. 5e has fourteen conditions and exhaustion, which is
/// six conditions wearing one name - it is in the list because a DM
/// tracking it needs somewhere to put it, and `magnitude` on the
/// effect row holds the level.
pub const ALL: [Condition; 15] = [
    Condition {
        key: "blinded", name: "Blinded", glyph: "\u{25CC}",
        does: "Cannot see and fails any check needing sight. Attacks against it have advantage; its own have disadvantage.",
    },
    Condition {
        key: "charmed", name: "Charmed", glyph: "\u{2665}",
        does: "Cannot attack the charmer or target them harmfully. The charmer has advantage on social checks against it.",
    },
    Condition {
        key: "deafened", name: "Deafened", glyph: "\u{2298}",
        does: "Cannot hear and fails any check needing hearing.",
    },
    Condition {
        key: "exhaustion", name: "Exhaustion", glyph: "\u{25BC}",
        does: "Six levels, cumulative: disadvantage on checks, then half speed, then disadvantage on attacks and saves, then half hit points, then speed 0, then death. The level is the magnitude.",
    },
    Condition {
        key: "frightened", name: "Frightened", glyph: "\u{203C}",
        does: "Disadvantage on checks and attacks while the source is in sight, and it cannot willingly move closer.",
    },
    Condition {
        key: "grappled", name: "Grappled", glyph: "\u{2282}",
        does: "Speed 0, and no bonus to speed. Ends if the grappler is incapacitated or something puts them out of reach.",
    },
    Condition {
        key: "incapacitated", name: "Incapacitated", glyph: "\u{2717}",
        does: "No actions and no reactions.",
    },
    Condition {
        key: "invisible", name: "Invisible", glyph: "\u{25CB}",
        does: "Cannot be seen without magic or a special sense; counts as heavily obscured. Attacks against it have disadvantage; its own have advantage.",
    },
    Condition {
        key: "paralysed", name: "Paralysed", glyph: "\u{2726}",
        does: "Incapacitated, cannot move or speak, and automatically fails Strength and Dexterity saves. Attacks against it have advantage, and any hit from within 5 feet is a CRIT.",
    },
    Condition {
        key: "petrified", name: "Petrified", glyph: "\u{25A3}",
        does: "Stone. Incapacitated, unaware, ten times the weight, stops ageing. Resistance to all damage, and immune to poison and disease - though one already in the system is only suspended.",
    },
    Condition {
        key: "poisoned", name: "Poisoned", glyph: "\u{2620}",
        does: "Disadvantage on attack rolls and on ability checks.",
    },
    Condition {
        key: "prone", name: "Prone", glyph: "\u{2193}",
        does: "Can only crawl until it stands, which costs half its speed. Disadvantage on its attacks. Attacks against it have advantage within 5 feet and disadvantage beyond.",
    },
    Condition {
        key: "restrained", name: "Restrained", glyph: "\u{22A0}",
        does: "Speed 0. Attacks against it have advantage, its own have disadvantage, and it has disadvantage on Dexterity saves.",
    },
    Condition {
        key: "stunned", name: "Stunned", glyph: "\u{2605}",
        does: "Incapacitated, cannot move, speaks only falteringly. Automatically fails Strength and Dexterity saves, and attacks against it have advantage.",
    },
    Condition {
        key: "unconscious", name: "Unconscious", glyph: "\u{2736}",
        does: "Incapacitated, unaware, drops what it holds and falls prone. Automatically fails Strength and Dexterity saves. Attacks have advantage, and any hit from within 5 feet is a CRIT.",
    },
];

/// The prefix that tells a condition from a spell on the same table.
pub const PREFIX: &str = "cond.";

/// The condition this effect key names, if it names one.
///
/// AMERICAN SPELLINGS ANSWER TOO, for the one word that differs.
/// Anything else is None - an unknown condition is not a condition,
/// and inventing one would put a word on a sheet that no rule
/// defines.
pub fn of_key(key: &str) -> Option<&'static Condition> {
    let bare = key.strip_prefix(PREFIX)?.trim().to_ascii_lowercase();
    let bare = match bare.as_str() {
        "paralyzed" => "paralysed",
        other => other,
    };
    ALL.iter().find(|c| c.key == bare)
}

/// The full effect key for a condition, as it is stored.
pub fn key_for(c: &Condition) -> String {
    format!("{}{}", PREFIX, c.key)
}

/* ===================== WHAT THEY DO TO A ROLL ===================== */

/// Which way a d20 leans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lean {
    Advantage,
    Disadvantage,
    Straight,
}

impl Lean {
    /// 5e's own rule: ADVANTAGE AND DISADVANTAGE CANCEL, and more than
    /// one of either changes nothing. A creature with three reasons for
    /// advantage and one for disadvantage rolls straight - the book is
    /// explicit that they do not stack and do not count up.
    pub fn and(self, other: Lean) -> Lean {
        match (self, other) {
            (a, Lean::Straight) => a,
            (Lean::Straight, b) => b,
            (a, b) if a == b => a,
            _ => Lean::Straight,
        }
    }

    /// The word `dice::d20_formula` takes.
    pub fn as_mode(self) -> &'static str {
        match self {
            Lean::Advantage => "adv",
            Lean::Disadvantage => "dis",
            Lean::Straight => "normal",
        }
    }

    pub fn of_mode(mode: &str) -> Lean {
        match mode {
            "adv" => Lean::Advantage,
            "dis" => Lean::Disadvantage,
            _ => Lean::Straight,
        }
    }
}

/// How an attack AGAINST somebody in these conditions leans.
///
/// 189. THE MODE IS NOT A PRICE, which is where 187 drew the line in
/// the wrong place. That module refused to put conditions in `grants`
/// because advantage cannot be written as a number, and that was
/// right - but it then concluded the engine should not apply advantage
/// at all, and that does not follow. `dice::d20_formula` has taken
/// "adv" since the beginning. Advantage is a WORD the dice engine
/// already speaks, not a number anybody has to invent.
///
/// Dave held a Lich's Hold Person on a cleric and the slam that
/// followed rolled `1d20+3 (normal)`.
///
/// INVISIBLE IS THE ONE THAT GOES THE OTHER WAY - you cannot see what
/// you are swinging at - and it is in the same list because a creature
/// can be both, and then they cancel.
pub fn against(keys: &[String]) -> Lean {
    let mut lean = Lean::Straight;
    for k in keys {
        let Some(c) = of_key(k) else { continue };
        lean = lean.and(match c.key {
            "blinded" | "paralysed" | "petrified" | "restrained" | "stunned"
            | "unconscious" | "prone" => Lean::Advantage,
            "invisible" => Lean::Disadvantage,
            _ => Lean::Straight,
        });
    }
    lean
}

/// How the attacks OF somebody in these conditions lean.
///
/// Not wired to anything yet - the roller's own conditions are read
/// where the roller is known, and `swing` currently reads only the
/// target's. Here because it is the other half of the same rule and
/// splitting them across two sessions is how they drift.
pub fn attacking_with(keys: &[String]) -> Lean {
    let mut lean = Lean::Straight;
    for k in keys {
        let Some(c) = of_key(k) else { continue };
        lean = lean.and(match c.key {
            "blinded" | "frightened" | "poisoned" | "prone" | "restrained" => Lean::Disadvantage,
            "invisible" => Lean::Advantage,
            _ => Lean::Straight,
        });
    }
    lean
}

/// Whether a hit on somebody in these conditions is automatically a
/// CRIT, given the attack came from within 5 feet.
///
/// 189. TWO CONDITIONS SAY IT and they say it in the same words:
/// "any attack that hits the creature is a critical hit if the
/// attacker is within 5 feet". PRONE IS NOT ONE OF THEM - it gives
/// advantage up close and nothing more, which is the kind of thing
/// that gets remembered wrong.
///
/// THE CALLER OWNS "WITHIN 5 FEET". This app has no grid, so the
/// honest stand-in is the attack's own mode: a melee swing is within
/// reach by definition and a bowshot is not.
pub fn crits_on_hit(keys: &[String]) -> bool {
    keys.iter()
        .filter_map(|k| of_key(k))
        .any(|c| matches!(c.key, "paralysed" | "unconscious"))
}

/// The condition stopping this creature acting at all, if one is.
///
/// 190. SAY, DO NOT REFUSE - which is 051's decision about turn order
/// and `spent.rs`'s about the action budget, and the right one here
/// for the same reason. Dave's cleric cast Bless while PARALYSED and
/// nothing anywhere mentioned it. A second action in a round is
/// normal at this table and so is a DM waving something through; what
/// is not acceptable is the app knowing and keeping quiet.
///
/// FIVE CONDITIONS, AND FOUR OF THEM ARE THE FIFTH. Paralysed,
/// petrified, stunned and unconscious each say "the creature is
/// incapacitated" in their own text, and incapacitated is the one
/// that says "can take no actions or reactions". They are listed
/// rather than derived because the list IS the derivation - there is
/// no field saying which conditions include which.
///
/// THE FIRST ONE FOUND, not all of them. A creature that is both
/// unconscious and paralysed is not more unable to act, and a caller
/// wanting to say why needs one reason, not a sentence.
pub fn cannot_act(keys: &[String]) -> Option<&'static Condition> {
    keys.iter()
        .filter_map(|k| of_key(k))
        .find(|c| matches!(
            c.key,
            "incapacitated" | "paralysed" | "petrified" | "stunned" | "unconscious"
        ))
}

/// What the hover says: the name first, then what it does, and the
/// level where there is one.
///
/// THE NAME LEADS because the glyph is a placeholder. A symbol nobody
/// can decode carries nothing, so the tooltip has to carry all of it.
pub fn said(c: &Condition, magnitude: Option<i64>) -> String {
    match (c.key, magnitude) {
        ("exhaustion", Some(n)) => format!("{} {} \u{2014} {}", c.name, n, c.does),
        _ => format!("{} \u{2014} {}", c.name, c.does),
    }
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn k(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| format!("cond.{}", x)).collect()
    }

    /* ---------- 189. what they do to a roll ---------- */

    #[test]
    fn a_held_target_gives_the_attacker_advantage() {
        // THE ONE DAVE FOUND. Hold Person landed and the slam that
        // followed rolled straight.
        assert_eq!(against(&k(&["paralysed"])), Lean::Advantage);
    }

    #[test]
    fn the_seven_that_give_advantage_and_the_one_that_does_not() {
        for c in ["blinded", "paralysed", "petrified", "restrained", "stunned", "unconscious", "prone"] {
            assert_eq!(against(&k(&[c])), Lean::Advantage, "{}", c);
        }
        // Being hard to see cuts the other way.
        assert_eq!(against(&k(&["invisible"])), Lean::Disadvantage);
        // And several do nothing to an attack roll at all.
        for c in ["charmed", "deafened", "exhaustion", "grappled", "incapacitated", "poisoned"] {
            assert_eq!(against(&k(&[c])), Lean::Straight, "{}", c);
        }
    }

    #[test]
    fn an_attacker_in_a_bad_way_has_disadvantage() {
        for c in ["blinded", "frightened", "poisoned", "prone", "restrained"] {
            assert_eq!(attacking_with(&k(&[c])), Lean::Disadvantage, "{}", c);
        }
        assert_eq!(attacking_with(&k(&["invisible"])), Lean::Advantage);
    }

    #[test]
    fn they_cancel_and_do_not_stack() {
        // 5e is explicit: more than one of either changes nothing, and
        // one of each cancels however many there are.
        assert_eq!(
            against(&k(&["paralysed", "restrained", "prone"])),
            Lean::Advantage,
            "three reasons is still advantage"
        );
        assert_eq!(
            against(&k(&["paralysed", "invisible"])),
            Lean::Straight,
            "one of each cancels"
        );
        assert_eq!(
            Lean::of_mode("dis").and(against(&k(&["paralysed"]))),
            Lean::Straight,
            "a DM's disadvantage cancels the condition's advantage"
        );
    }

    #[test]
    fn the_dm_still_gets_their_say_when_nothing_is_running() {
        assert_eq!(Lean::of_mode("adv").and(against(&[])), Lean::Advantage);
        assert_eq!(Lean::of_mode("dis").and(against(&[])), Lean::Disadvantage);
        assert_eq!(Lean::of_mode("normal").and(against(&[])), Lean::Straight);
    }

    #[test]
    fn the_mode_words_are_the_ones_the_dice_engine_takes() {
        // `dice::d20_formula` matches on "adv" and "dis" and treats
        // anything else as a straight roll. A third spelling here
        // would silently roll one die.
        assert_eq!(Lean::Advantage.as_mode(), "adv");
        assert_eq!(Lean::Disadvantage.as_mode(), "dis");
        assert_eq!(Lean::Straight.as_mode(), "normal");
    }

    #[test]
    fn only_helpless_turns_a_hit_into_a_crit() {
        assert!(crits_on_hit(&k(&["paralysed"])));
        assert!(crits_on_hit(&k(&["unconscious"])));
        // PRONE IS NOT ONE OF THEM. It gives advantage up close and
        // nothing more, and it is the one people remember wrong.
        for c in ["prone", "restrained", "stunned", "blinded", "petrified", "grappled"] {
            assert!(!crits_on_hit(&k(&[c])), "{} should not auto-crit", c);
        }
    }

    #[test]
    fn a_key_nobody_defined_leans_nothing() {
        // An unknown effect on somebody must not move a d20. Effects
        // and conditions share one table and most rows are spells.
        let junk = vec!["sp_bless".to_string(), "cond.dazed".to_string()];
        assert_eq!(against(&junk), Lean::Straight);
        assert_eq!(attacking_with(&junk), Lean::Straight);
        assert!(!crits_on_hit(&junk));
    }

    #[test]
    fn the_five_that_stop_a_creature_acting() {
        for c in ["incapacitated", "paralysed", "petrified", "stunned", "unconscious"] {
            assert_eq!(cannot_act(&k(&[c])).map(|x| x.key), Some(c), "{}", c);
        }
    }

    #[test]
    fn being_in_trouble_is_not_the_same_as_being_unable() {
        // Each of these is bad and none of them stops a turn. Restrained
        // is speed 0, not actionless; frightened still swings, badly.
        for c in ["blinded", "charmed", "deafened", "frightened", "grappled",
                  "invisible", "poisoned", "prone", "restrained", "exhaustion"] {
            assert!(cannot_act(&k(&[c])).is_none(), "{} should still act", c);
        }
    }

    #[test]
    fn one_reason_is_given_not_all_of_them() {
        // A creature that is both is not more unable to act, and a
        // caller wanting to say WHY needs a word, not a sentence.
        let both = k(&["paralysed", "unconscious"]);
        assert!(cannot_act(&both).is_some());
    }

    #[test]
    fn a_spell_on_somebody_does_not_stop_them_acting() {
        // Effects and conditions share one table and most rows are
        // spells. Hold Person puts BOTH on a target - the spell and
        // the condition - and only the condition stops the turn.
        assert!(cannot_act(&["sp_holdperson".to_string()]).is_none());
    }

    #[test]
    fn the_list_is_the_fifteen() {
        assert_eq!(ALL.len(), 15);
    }

    #[test]
    fn every_key_is_unique_and_lowercase() {
        // A duplicate would make `of_key` return whichever came first
        // and the other unreachable, which is the kind of thing that
        // only shows up when somebody applies the unreachable one.
        let mut seen: Vec<&str> = Vec::new();
        for c in ALL.iter() {
            assert!(!seen.contains(&c.key), "duplicate key {}", c.key);
            assert_eq!(c.key, c.key.to_ascii_lowercase(), "{} is not lowercase", c.key);
            assert!(!c.name.is_empty() && !c.does.is_empty() && !c.glyph.is_empty());
            seen.push(c.key);
        }
    }

    #[test]
    fn a_namespaced_key_resolves() {
        assert_eq!(of_key("cond.blinded").map(|c| c.name), Some("Blinded"));
        assert_eq!(of_key("cond.PRONE").map(|c| c.name), Some("Prone"));
        assert_eq!(of_key("cond. restrained ").map(|c| c.name), Some("Restrained"));
    }

    #[test]
    fn both_spellings_of_the_one_that_differs() {
        // The prose here is British and the published list is not.
        // Writing one and reading back nothing would be a silent miss.
        assert_eq!(of_key("cond.paralysed"), of_key("cond.paralyzed"));
        assert_eq!(of_key("cond.paralyzed").map(|c| c.name), Some("Paralysed"));
    }

    #[test]
    fn a_spell_key_is_not_a_condition() {
        // THE WHOLE POINT OF THE PREFIX. Effects already hold spell
        // keys, and the two share one table.
        assert!(of_key("sp_holdperson").is_none());
        assert!(of_key("blinded").is_none(), "unprefixed is not enough");
        assert!(of_key("sp_bless").is_none());
    }

    #[test]
    fn a_word_nobody_defined_is_not_a_condition() {
        // Inventing one would put a state on a sheet that no rule
        // describes - the same refusal `save_damage` makes for a word
        // it was not taught.
        for odd in ["cond.", "cond.dazed", "cond.confused", "cond.bleeding", ""] {
            assert!(of_key(odd).is_none(), "{:?} should not resolve", odd);
        }
    }

    #[test]
    fn key_for_round_trips() {
        for c in ALL.iter() {
            assert_eq!(of_key(&key_for(c)), Some(c));
        }
    }

    #[test]
    fn exhaustion_carries_its_level_and_nothing_else_does() {
        let ex = of_key("cond.exhaustion").unwrap();
        assert!(said(ex, Some(3)).starts_with("Exhaustion 3 \u{2014}"));
        // A magnitude on anything else is meaningless and is not shown
        // rather than being printed as a mystery number.
        let pr = of_key("cond.prone").unwrap();
        assert!(said(pr, Some(3)).starts_with("Prone \u{2014}"));
    }

    #[test]
    fn the_tooltip_leads_with_the_name() {
        // The glyph is a placeholder until the graphics land, so the
        // hover carries the whole meaning.
        for c in ALL.iter() {
            assert!(said(c, None).starts_with(c.name), "{} does not lead", c.key);
            assert!(said(c, None).contains(c.does));
        }
    }
}
