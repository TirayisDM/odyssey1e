//! What hurts somebody less, more, or not at all.
//!
//! 116. `species.damage_resistances` has existed since 056 and the
//! sheet printed it followed by "(DM applies)" - which is an honest
//! label for a thing that does nothing, and the Unt'garoth have been
//! resistant to fire and cold on paper for a month.
//!
//! ---------------------------------------------------------------------
//! THREE DEGREES, AND IMMUNITY IS NOT JUST MORE RESISTANCE
//! ---------------------------------------------------------------------
//!
//! Resistance halves, vulnerability doubles, immunity ignores. The
//! third is a different kind of thing rather than the end of a scale:
//! half of a very large number still kills somebody and none of it
//! never does.
//!
//! 5e'S STACKING RULE IS THAT THERE IS NO STACKING. "Multiple instances
//! of resistance or vulnerability that affect the same damage type
//! count as only one instance." Two sources of fire resistance is still
//! half - which is worth stating in code because it is the rule people
//! most often house-rule by accident.
//!
//! RESISTANCE AND VULNERABILITY TOGETHER CANCEL. Halving and then
//! doubling is the number you started with, and 5e gets there the same
//! way rather than by a special case.
//!
//! ---------------------------------------------------------------------
//! EVERY ONE SAYS WHERE IT CAME FROM
//! ---------------------------------------------------------------------
//!
//! The instruction was to lace them back to whatever activates them,
//! and it is the same reason `AbilitySource` carries a name and a grant
//! carries a `source`: a sheet saying "resistant to fire" invites the
//! question "why", and a character who stops raging needs the barbarian
//! half to go without taking the Unt'garoth half with it.
//!
//! FOUR KINDS OF SOURCE, and three of them are already one mechanism.
//! A species states its own; everything else - a spell's effect, a
//! magic item, a class feature - arrives as a GRANT, in the vocabulary
//! 100 and 114 already use. `resist.fire`, `immune.poison`,
//! `vulnerable.cold`.

use serde::{Deserialize, Serialize};

/// The thirteen damage types 5e has.
pub const TYPES: &[&str] = &[
    "acid", "bludgeoning", "cold", "fire", "force", "lightning", "necrotic",
    "piercing", "poison", "psychic", "radiant", "slashing", "thunder",
];

/// How a creature stands to one damage type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Degree {
    Resistant,
    Immune,
    Vulnerable,
}

impl Degree {
    /// The grant-target prefix that names this degree, which is how a
    /// settled choice is written back.
    pub fn as_str_target(self) -> &'static str {
        match self {
            Degree::Resistant => "resist",
            Degree::Immune => "immune",
            Degree::Vulnerable => "vulnerable",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Degree::Resistant => "resistant",
            Degree::Immune => "immune",
            Degree::Vulnerable => "vulnerable",
        }
    }


    /// Read one off a grant target's prefix.
    pub fn from_prefix(p: &str) -> Option<Degree> {
        match p {
            "resist" => Some(Degree::Resistant),
            "immune" => Some(Degree::Immune),
            "vulnerable" => Some(Degree::Vulnerable),
            _ => None,
        }
    }
}

/// One reason somebody stands as they do to one damage type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub damage_type: String,
    pub degree: Degree,
    /// What to call it on a sheet: the species' name, the spell's, the
    /// item's. The whole point of the instruction to lace these back.
    pub name: String,
}

/// Where a damage type stands once every source is counted, with the
/// reasons kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standing {
    pub damage_type: String,
    pub degree: Degree,
    /// Every source that contributed, including the ones 5e's
    /// no-stacking rule makes redundant - two reasons to resist fire is
    /// still half, and a sheet that hid the second would be hiding a
    /// fact rather than simplifying one.
    pub from: Vec<String>,
}

/// Read a grant target into a degree and the damage types it names.
///
/// `resist.fire` is one type and settled. `resist.acid|cold|fire` names
/// THREE AND SETTLES NOTHING - Protection from Energy is "choose one of
/// acid, cold, fire, lightning, or thunder", and a vocabulary that
/// could only say one type would have forced either five separate
/// spells or a resistance to all five.
///
/// AN UNKNOWN DAMAGE TYPE IS REFUSED rather than carried, and refused
/// for the whole grant rather than quietly dropped from the list. A
/// grant saying `resist.frie` would otherwise sit on a sheet looking
/// correct and match nothing that was ever rolled; one saying
/// `resist.fire|frie` would offer a choice half of which does nothing.
pub fn parse_target(target: &str) -> Option<(Degree, Vec<String>)> {
    let (prefix, rest) = target.split_once('.')?;
    let degree = Degree::from_prefix(prefix)?;
    let kinds: Vec<String> = rest
        .split('|')
        .map(|k| k.trim().to_lowercase())
        .filter(|k| !k.is_empty())
        .collect();
    if kinds.is_empty() || !kinds.iter().all(|k| TYPES.contains(&k.as_str())) {
        return None;
    }
    Some((degree, kinds))
}

/// Whether a grant target names a resistance at all.
pub fn is_resist_target(target: &str) -> bool {
    parse_target(target).is_some()
}

/// The types a grant makes somebody choose between, where it makes them
/// choose at all.
///
/// NONE FOR THE ORDINARY CASE, which is the point: a caller that wants
/// to know whether to ask a question gets a straight answer rather than
/// a one-item list it has to recognise as "no question".
pub fn choice_offered(target: &str) -> Option<Vec<String>> {
    let (_, kinds) = parse_target(target)?;
    (kinds.len() > 1).then_some(kinds)
}

/// Settle a grant's target against what somebody chose.
///
/// A CHOICE NOT OFFERED IS REFUSED. Casting Protection from Poison and
/// naming fire must not give fire resistance - the spell says poison,
/// and honouring a pick the grant never offered would let the picker
/// grant anything it liked.
pub fn pick(target: &str, chosen: Option<&str>) -> Result<(Degree, String), String> {
    let (degree, kinds) = parse_target(target)
        .ok_or_else(|| format!("\"{}\" is not a resistance", target))?;
    if let [only] = &kinds[..] {
        return Ok((degree, only.clone()));
    }
    let want = chosen
        .map(str::trim)
        .map(str::to_lowercase)
        .filter(|c| !c.is_empty())
        .ok_or_else(|| format!("choose a damage type: {}", kinds.join(", ")))?;
    if !kinds.contains(&want) {
        return Err(format!("{} is not one of {}", want, kinds.join(", ")));
    }
    Ok((degree, want))
}

/// Every resistance a pile of grants carries, named after whatever
/// granted it.
///
/// A GRANT STILL AWAITING A CHOICE IS SKIPPED. Protection from Energy
/// is settled when it is CAST - the chosen type goes onto the effect
/// row - so a grant reaching this still offering five is reference data
/// that nobody has spent yet, and guessing one of the five here would
/// invent a resistance the caster never asked for.
pub fn from_grants(grants: &[crate::grants::Grant]) -> Vec<Source> {
    grants
        .iter()
        .filter_map(|g| {
            let (degree, kind) = pick(&g.target, None).ok()?;
            Some(Source { damage_type: kind, degree, name: g.source.clone() })
        })
        .collect()
}

/// What a species states about itself.
///
/// ALWAYS RESISTANCE. `species.damage_resistances` is the only column
/// 056 gave a people and it has no degree in it - a species that is
/// IMMUNE to something needs a grant like everything else, rather than
/// a second array beside this one that could disagree with it.
pub fn from_species(kinds: &[String], species_name: &str) -> Vec<Source> {
    kinds
        .iter()
        .map(|k| k.trim().to_lowercase())
        .filter(|k| TYPES.contains(&k.as_str()))
        .map(|k| Source {
            damage_type: k,
            degree: Degree::Resistant,
            name: species_name.to_string(),
        })
        .collect()
}

/// Everything that is true at once, one entry per damage type.
///
/// IMMUNITY WINS, then the pair. That order is 5e's and is not
/// arbitrary: immunity is a different kind of fact, and a creature
/// immune to fire does not stop being immune because something also
/// made it vulnerable.
///
/// SORTED BY TYPE, so a sheet reads the same twice.
pub fn standing(sources: &[Source]) -> Vec<Standing> {
    let mut out: Vec<Standing> = Vec::new();

    for kind in TYPES {
        let mine: Vec<&Source> = sources.iter().filter(|s| s.damage_type == *kind).collect();
        if mine.is_empty() {
            continue;
        }
        let immune = mine.iter().any(|s| s.degree == Degree::Immune);
        let resistant = mine.iter().any(|s| s.degree == Degree::Resistant);
        let vulnerable = mine.iter().any(|s| s.degree == Degree::Vulnerable);

        // IMMUNITY FIRST. Then resistance and vulnerability, which
        // cancel - halving and doubling is where you started, and
        // saying so here is the same answer 5e reaches by applying
        // them in order.
        let degree = if immune {
            Degree::Immune
        } else if resistant && vulnerable {
            continue;
        } else if resistant {
            Degree::Resistant
        } else {
            Degree::Vulnerable
        };

        out.push(Standing {
            damage_type: kind.to_string(),
            degree,
            from: mine.iter().map(|s| s.name.clone()).collect(),
        });
    }
    out
}

/// What this much damage of this type actually costs them.
///
/// HALVED MEANS ROUNDED DOWN, which is 5e everywhere it halves. One
/// point of fire against a resistant creature is nothing at all, and
/// that is the rule rather than an edge case.
pub fn against(damage: i64, standing: &[Standing], damage_type: &str) -> i64 {
    let d = damage.max(0);
    match standing.iter().find(|s| s.damage_type == damage_type) {
        None => d,
        Some(s) => match s.degree {
            Degree::Immune => 0,
            Degree::Resistant => d / 2,
            Degree::Vulnerable => d * 2,
        },
    }
}

/// How the number changed, in words, for a log that has to explain
/// itself.
pub fn said(damage: i64, standing: &[Standing], damage_type: &str) -> Option<String> {
    let s = standing.iter().find(|x| x.damage_type == damage_type)?;
    let after = against(damage, standing, damage_type);
    Some(format!(
        "{} {} ({}) - {} instead of {}",
        s.degree.as_str(),
        damage_type,
        s.from.join(", "),
        after,
        damage.max(0)
    ))
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn src(kind: &str, degree: Degree, name: &str) -> Source {
        Source { damage_type: kind.into(), degree, name: name.into() }
    }

    /* ---------- reading a grant ---------- */

    fn one(t: &str) -> (Degree, String) {
        pick(t, None).unwrap()
    }

    #[test]
    fn a_grant_target_names_a_degree_and_a_type() {
        assert_eq!(one("resist.fire"), (Degree::Resistant, "fire".into()));
        assert_eq!(one("immune.poison"), (Degree::Immune, "poison".into()));
        assert_eq!(one("vulnerable.cold"), (Degree::Vulnerable, "cold".into()));
    }

    #[test]
    fn an_unknown_damage_type_is_refused() {
        // A typo would otherwise sit on a sheet looking correct and
        // match nothing that was ever rolled.
        assert_eq!(parse_target("resist.frie"), None);
        assert_eq!(parse_target("resist."), None);
    }

    #[test]
    fn something_that_is_not_a_resistance_is_not_one() {
        assert_eq!(parse_target("attack"), None);
        assert_eq!(parse_target("skill.ath"), None);
        assert!(!is_resist_target("ac"));
        assert!(is_resist_target("resist.acid"));
    }

    /* ---------- a grant that asks a question ---------- */

    #[test]
    fn several_types_is_a_choice_and_one_type_is_not() {
        // Protection from Energy against Protection from Poison.
        assert_eq!(
            choice_offered("resist.acid|cold|fire|lightning|thunder"),
            Some(vec!["acid".into(), "cold".into(), "fire".into(),
                      "lightning".into(), "thunder".into()])
        );
        assert_eq!(choice_offered("resist.poison"), None);
    }

    #[test]
    fn a_choice_nobody_made_is_not_guessed_at() {
        // Guessing one of five would invent a resistance the caster
        // never asked for.
        let e = pick("resist.acid|cold|fire", None).unwrap_err();
        assert!(e.contains("choose"), "{}", e);
        assert!(e.contains("acid, cold, fire"), "{}", e);
    }

    #[test]
    fn the_chosen_one_settles_it() {
        assert_eq!(
            pick("resist.acid|cold|fire", Some("Cold")).unwrap(),
            (Degree::Resistant, "cold".into()),
            "case is not a different answer"
        );
    }

    #[test]
    fn a_pick_outside_what_was_offered_is_refused() {
        let e = pick("resist.acid|cold|fire", Some("necrotic")).unwrap_err();
        assert!(e.contains("not one of"), "{}", e);
    }

    #[test]
    fn a_pick_against_a_spell_that_offered_no_choice_is_ignored() {
        // Protection from Poison says poison. Naming fire does not get
        // you fire - a picker that could override the grant could grant
        // anything it liked.
        assert_eq!(
            pick("resist.poison", Some("fire")).unwrap(),
            (Degree::Resistant, "poison".into())
        );
    }

    /* ---------- where they come from ---------- */

    #[test]
    fn a_species_states_resistance_and_nothing_stronger() {
        let got = from_species(&["cold".into(), "Fire".into()], "Unt'garoth");
        assert_eq!(got.len(), 2);
        assert!(got.iter().all(|s| s.degree == Degree::Resistant));
        assert!(got.iter().all(|s| s.name == "Unt'garoth"));
        assert_eq!(got[1].damage_type, "fire", "normalised");
    }

    #[test]
    fn a_species_word_that_is_not_a_damage_type_is_dropped() {
        // 056 let a DM type this array freely and "disease" is in
        // several published statblocks. It is not damage.
        assert!(from_species(&["disease".into()], "Somebody").is_empty());
    }

    #[test]
    fn a_grant_carries_its_own_name_through() {
        let gs = vec![crate::grants::Grant {
            target: "resist.poison".into(),
            mode: crate::grants::Mode::Add,
            value: 0,
            dice: None,
            needs_attunement: false,
            source: "Protection from Poison".into(),
        }];
        let got = from_grants(&gs);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Protection from Poison");
        assert_eq!(got[0].damage_type, "poison");
    }

    #[test]
    fn a_grant_still_waiting_on_a_choice_grants_nothing_yet() {
        // Protection from Energy is settled when it is cast; one
        // reaching here unspent is reference data.
        let gs = vec![crate::grants::Grant {
            target: "resist.acid|cold|fire|lightning|thunder".into(),
            mode: crate::grants::Mode::Add,
            value: 0,
            dice: None,
            needs_attunement: false,
            source: "Protection from Energy".into(),
        }];
        assert!(from_grants(&gs).is_empty());
    }

    #[test]
    fn a_grant_that_is_not_a_resistance_is_passed_over() {
        let gs = vec![crate::grants::Grant {
            target: "ac".into(),
            mode: crate::grants::Mode::Add,
            value: 1,
            dice: None,
            needs_attunement: false,
            source: "a cloak".into(),
        }];
        assert!(from_grants(&gs).is_empty());
    }

    /* ---------- what is true at once ---------- */

    #[test]
    fn one_source_is_one_standing() {
        let got = standing(&[src("fire", Degree::Resistant, "Unt'garoth")]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].degree, Degree::Resistant);
        assert_eq!(got[0].from, vec!["Unt'garoth"]);
    }

    #[test]
    fn two_reasons_to_resist_is_still_resistance_and_both_are_kept() {
        // 5e: "multiple instances... count as only one instance". The
        // rule people house-rule by accident, and a sheet hiding the
        // second reason would be hiding a fact rather than simplifying.
        let got = standing(&[
            src("fire", Degree::Resistant, "Unt'garoth"),
            src("fire", Degree::Resistant, "Ring of Fire Resistance"),
        ]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].degree, Degree::Resistant);
        assert_eq!(got[0].from.len(), 2);
    }

    #[test]
    fn immunity_beats_resistance() {
        let got = standing(&[
            src("poison", Degree::Resistant, "Dwarf"),
            src("poison", Degree::Immune, "Purity of Body"),
        ]);
        assert_eq!(got[0].degree, Degree::Immune);
    }

    #[test]
    fn immunity_also_beats_vulnerability() {
        // Immunity is a different kind of fact, not the end of a scale.
        let got = standing(&[
            src("fire", Degree::Vulnerable, "a curse"),
            src("fire", Degree::Immune, "Holy Aura"),
        ]);
        assert_eq!(got[0].degree, Degree::Immune);
    }

    #[test]
    fn resistance_and_vulnerability_cancel_to_nothing_at_all() {
        // Halving and doubling is where you started, so there is no
        // standing worth printing.
        let got = standing(&[
            src("cold", Degree::Resistant, "Unt'garoth"),
            src("cold", Degree::Vulnerable, "a curse"),
        ]);
        assert!(got.is_empty());
    }

    #[test]
    fn different_types_do_not_interact() {
        let got = standing(&[
            src("fire", Degree::Resistant, "Unt'garoth"),
            src("cold", Degree::Vulnerable, "a curse"),
        ]);
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn the_order_is_the_damage_types_own() {
        // So a sheet reads the same twice.
        let got = standing(&[
            src("thunder", Degree::Resistant, "x"),
            src("acid", Degree::Resistant, "y"),
        ]);
        assert_eq!(got[0].damage_type, "acid");
        assert_eq!(got[1].damage_type, "thunder");
    }

    #[test]
    fn nothing_is_nothing() {
        assert!(standing(&[]).is_empty());
    }

    /* ---------- what it costs them ---------- */

    #[test]
    fn resistance_halves_and_rounds_down() {
        let s = standing(&[src("fire", Degree::Resistant, "Unt'garoth")]);
        assert_eq!(against(10, &s, "fire"), 5);
        assert_eq!(against(7, &s, "fire"), 3, "rounded down, which is 5e");
    }

    #[test]
    fn one_point_against_resistance_is_nothing() {
        // The rule rather than an edge case.
        let s = standing(&[src("fire", Degree::Resistant, "Unt'garoth")]);
        assert_eq!(against(1, &s, "fire"), 0);
    }

    #[test]
    fn immunity_is_none_of_it() {
        let s = standing(&[src("poison", Degree::Immune, "Purity of Body")]);
        assert_eq!(against(40, &s, "poison"), 0);
    }

    #[test]
    fn vulnerability_doubles() {
        let s = standing(&[src("cold", Degree::Vulnerable, "a curse")]);
        assert_eq!(against(7, &s, "cold"), 14);
    }

    #[test]
    fn a_type_nobody_has_an_opinion_about_passes_through() {
        let s = standing(&[src("fire", Degree::Resistant, "Unt'garoth")]);
        assert_eq!(against(10, &s, "slashing"), 10);
    }

    #[test]
    fn negative_damage_is_not_healing_by_the_back_door() {
        let s = standing(&[src("fire", Degree::Vulnerable, "x")]);
        assert_eq!(against(-5, &s, "fire"), 0);
    }

    /* ---------- explaining itself ---------- */

    #[test]
    fn the_line_says_why_and_by_how_much() {
        let s = standing(&[src("fire", Degree::Resistant, "Unt'garoth")]);
        assert_eq!(
            said(10, &s, "fire").as_deref(),
            Some("resistant fire (Unt'garoth) - 5 instead of 10")
        );
    }

    #[test]
    fn nothing_to_say_about_a_type_nobody_resists() {
        let s = standing(&[src("fire", Degree::Resistant, "Unt'garoth")]);
        assert_eq!(said(10, &s, "cold"), None);
    }

    #[test]
    fn every_damage_type_five_e_has_is_here() {
        assert_eq!(TYPES.len(), 13);
        for t in ["acid", "force", "necrotic", "psychic", "radiant", "thunder"] {
            assert!(TYPES.contains(&t), "{} is missing", t);
        }
    }
}
