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
    /// 143. TRUE WHEN IT ONLY APPLIES TO A NONMAGICAL ATTACK, which is
    /// the commonest resistance in 5e and the one 116 could not say.
    /// Half the Monster Manual is "bludgeoning, piercing and slashing
    /// from nonmagical attacks", and the qualifier IS the rule: it is
    /// what makes the party's magic sword worth carrying.
    pub nonmagical: bool,
}

/// Where a damage type stands once every source is counted, with the
/// reasons kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standing {
    pub damage_type: String,
    /// Against an ORDINARY attack - which is most of them, and is why
    /// this is the unqualified field.
    ///
    /// `None` MEANS NOTHING APPLIES, which happens when a resistance
    /// and a vulnerability cancel. The standing is still here because
    /// `vs_magic` may differ, and because the sources are worth showing
    /// either way: a DM looking at a creature that resists AND is
    /// vulnerable to slashing should see both and not an empty line.
    pub degree: Option<Degree>,
    /// 143. AND AGAINST A MAGICAL ONE. `None` means the standing does
    /// not apply at all: a werewolf resists a sword and does not resist
    /// a +1 sword, which is the whole point of the qualifier.
    ///
    /// RESOLVED SEPARATELY RATHER THAN FILTERED LATER, because a
    /// creature may have reasons of both kinds for the same damage
    /// type, and which sources survive changes the answer rather than
    /// just removing one. An Unt'garoth werewolf resists fire either
    /// way and slashing only from an ordinary blade.
    pub vs_magic: Option<Degree>,
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
/// 143. AND A TRAILING `.nonmagical` NARROWS IT.
///
/// `resist.bludgeoning|piercing|slashing.nonmagical` is half the
/// Monster Manual in one target, and until now this vocabulary could
/// only say the part before the qualifier - so 23 of the creatures 142
/// added resisted a magic sword exactly as hard as an ordinary one,
/// which is backwards.
///
/// A SUFFIX RATHER THAN A FOURTH DEGREE. "Resistant" and "resistant to
/// nonmagical" are the same degree under a condition, not two degrees -
/// `Degree` stays the three 5e has, and everything that reasons about
/// halving and doubling is untouched.
///
/// ANYTHING ELSE AFTER THE TYPES IS REFUSED, not ignored. A target
/// reading `resist.fire.nonmagicl` would otherwise sit on a sheet
/// looking exactly like a working one and apply in every case the
/// author meant to exclude - the same argument the unknown-damage-type
/// refusal above is making.
pub fn parse_target(target: &str) -> Option<(Degree, Vec<String>, bool)> {
    let (prefix, rest) = target.split_once('.')?;
    let degree = Degree::from_prefix(prefix)?;
    let (rest, nonmagical) = match rest.strip_suffix(NONMAGICAL_SUFFIX) {
        Some(head) => (head, true),
        None => (rest, false),
    };
    let kinds: Vec<String> = rest
        .split('|')
        .map(|k| k.trim().to_lowercase())
        .filter(|k| !k.is_empty())
        .collect();
    if kinds.is_empty() || !kinds.iter().all(|k| TYPES.contains(&k.as_str())) {
        return None;
    }
    Some((degree, kinds, nonmagical))
}

/// The one qualifier this vocabulary knows, spelled once. `target_for`
/// writes it and `parse_target` reads it, and nothing else should be
/// building these strings by hand.
const NONMAGICAL_SUFFIX: &str = ".nonmagical";

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
    let (_, kinds, _) = parse_target(target)?;
    (kinds.len() > 1).then_some(kinds)
}

/// Settle a grant's target against what somebody chose.
///
/// A CHOICE NOT OFFERED IS REFUSED. Casting Protection from Poison and
/// naming fire must not give fire resistance - the spell says poison,
/// and honouring a pick the grant never offered would let the picker
/// grant anything it liked.
pub fn pick(target: &str, chosen: Option<&str>) -> Result<(Degree, String, bool), String> {
    let (degree, kinds, nonmagical) = parse_target(target)
        .ok_or_else(|| format!("\"{}\" is not a resistance", target))?;
    if let [only] = &kinds[..] {
        return Ok((degree, only.clone(), nonmagical));
    }
    let want = chosen
        .map(str::trim)
        .map(str::to_lowercase)
        .filter(|c| !c.is_empty())
        .ok_or_else(|| format!("choose a damage type: {}", kinds.join(", ")))?;
    if !kinds.contains(&want) {
        return Err(format!("{} is not one of {}", want, kinds.join(", ")));
    }
    // THE QUALIFIER SURVIVES THE CHOICE. Protection from Energy does
    // not carry one, but a grant that offered a choice AND narrowed it
    // to ordinary weapons would lose half its meaning here otherwise.
    Ok((degree, want, nonmagical))
}

/// Write a settled degree, type and qualifier back as a grant target.
///
/// THE ONLY PLACE THIS STRING IS BUILT. `settle_choices` used to format
/// it inline, which was fine while a target was two parts and silently
/// dropped the third the moment there was one.
pub fn target_for(degree: Degree, kind: &str, nonmagical: bool) -> String {
    match nonmagical {
        true => format!("{}.{}{}", degree.as_str_target(), kind, NONMAGICAL_SUFFIX),
        false => format!("{}.{}", degree.as_str_target(), kind),
    }
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
            let (degree, kind, nonmagical) = pick(&g.target, None).ok()?;
            Some(Source { damage_type: kind, degree, name: g.source.clone(), nonmagical })
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
            // 143. A PEOPLE'S OWN RESISTANCE IS UNCONDITIONAL. The
            // Unt'garoth do not stop resisting fire because the torch
            // was enchanted, and `species.damage_resistances` is a bare
            // list of type names with nowhere to say otherwise.
            nonmagical: false,
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

        // 143. TWICE, OVER TWO SETS OF SOURCES. An ordinary attack
        // meets all of them; a magical one meets only the ones that did
        // not say "nonmagical". Resolving once and filtering afterwards
        // would get the mixed case wrong - a creature immune to fire
        // from ordinary weapons and merely resistant to it in general
        // is resistant to a flaming sword, not immune and not nothing.
        let degree = settle(&mine);
        let unqualified: Vec<&Source> =
            mine.iter().copied().filter(|s| !s.nonmagical).collect();
        let vs_magic = settle(&unqualified);

        // NOTHING EITHER WAY IS NOTHING TO SAY. Dropping it when only
        // the ORDINARY case cancels was a bug this module's own test
        // caught: a werewolf under a curse that makes slashing hurt
        // double resists an ordinary blade and does not resist a magic
        // one, so the magic one should land doubled - and the whole
        // standing was being discarded before anything could ask.
        if degree.is_none() && vs_magic.is_none() {
            continue;
        }

        out.push(Standing {
            damage_type: kind.to_string(),
            degree,
            vs_magic,
            from: mine.iter().map(|s| s.name.clone()).collect(),
        });
    }
    out
}

/// Which way a pile of reasons about one damage type comes out.
///
/// IMMUNITY FIRST. Then resistance and vulnerability, which cancel -
/// halving and doubling is where you started, and saying so here is the
/// same answer 5e reaches by applying them in order. `None` means
/// nothing applies, either because they cancelled or because there was
/// nothing to apply.
fn settle(sources: &[&Source]) -> Option<Degree> {
    if sources.is_empty() {
        return None;
    }
    let immune = sources.iter().any(|s| s.degree == Degree::Immune);
    let resistant = sources.iter().any(|s| s.degree == Degree::Resistant);
    let vulnerable = sources.iter().any(|s| s.degree == Degree::Vulnerable);
    if immune {
        Some(Degree::Immune)
    } else if resistant && vulnerable {
        None
    } else if resistant {
        Some(Degree::Resistant)
    } else {
        Some(Degree::Vulnerable)
    }
}

/// What this much damage of this type actually costs them.
///
/// HALVED MEANS ROUNDED DOWN, which is 5e everywhere it halves. One
/// point of fire against a resistant creature is nothing at all, and
/// that is the rule rather than an edge case.
pub fn against(damage: i64, standing: &[Standing], damage_type: &str, magical: bool) -> i64 {
    let d = damage.max(0);
    match applies(standing, damage_type, magical) {
        None => d,
        Some(Degree::Immune) => 0,
        Some(Degree::Resistant) => d / 2,
        Some(Degree::Vulnerable) => d * 2,
    }
}

/// Which degree actually meets this attack, if any.
///
/// 143. WHERE THE QUALIFIER IS SPENT, and the only place it is read.
/// `magical` is about the WEAPON that swung - see `attack::Attack` -
/// and a werewolf meeting a +1 longsword finds nothing here.
pub fn applies(standing: &[Standing], damage_type: &str, magical: bool) -> Option<Degree> {
    let s = standing.iter().find(|s| s.damage_type == damage_type)?;
    match magical {
        true => s.vs_magic,
        false => s.degree,
    }
}

/// How the number changed, in words, for a log that has to explain
/// itself.
pub fn said(
    damage: i64,
    standing: &[Standing],
    damage_type: &str,
    magical: bool,
) -> Option<String> {
    let s = standing.iter().find(|x| x.damage_type == damage_type)?;
    // 143. NOTHING HAPPENED, SO NOTHING IS SAID. A werewolf that did
    // not resist the magic sword has no line to add to the log - and
    // "resistant slashing - 9 instead of 9" would be worse than silence,
    // because it reads as a rule that fired.
    let degree = applies(standing, damage_type, magical)?;
    let after = against(damage, standing, damage_type, magical);
    Some(format!(
        "{} {} ({}) - {} instead of {}",
        degree.as_str(),
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
        Source { damage_type: kind.into(), degree, name: name.into(), nonmagical: false }
    }

    /* ---------- reading a grant ---------- */

    fn one(t: &str) -> (Degree, String) {
        // The qualifier has its own tests below; these are about the
        // degree and the type.
        let (degree, kind, _) = pick(t, None).unwrap();
        (degree, kind)
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
            (Degree::Resistant, "cold".into(), false),
            "case is not a different answer"
        );
    }

    /* ------------- the nonmagical qualifier (143) ------------------ */

    fn qualified(kind: &str, degree: Degree, name: &str) -> Source {
        Source { damage_type: kind.into(), degree, name: name.into(), nonmagical: true }
    }

    #[test]
    fn the_qualifier_is_read_off_the_target() {
        let (degree, kinds, nonmagical) = parse_target("resist.slashing.nonmagical").unwrap();
        assert_eq!(degree, Degree::Resistant);
        assert_eq!(kinds, vec!["slashing".to_string()]);
        assert!(nonmagical);
    }

    #[test]
    fn half_the_monster_manual_in_one_target() {
        let (degree, kinds, nonmagical) =
            parse_target("resist.bludgeoning|piercing|slashing.nonmagical").unwrap();
        assert_eq!(degree, Degree::Resistant);
        assert_eq!(kinds.len(), 3);
        assert!(nonmagical);
    }

    #[test]
    fn an_unqualified_target_is_unchanged() {
        let (_, _, nonmagical) = parse_target("resist.fire").unwrap();
        assert!(!nonmagical);
    }

    #[test]
    fn a_misspelt_qualifier_is_refused_rather_than_ignored() {
        // The whole argument for refusing: `resist.fire.nonmagicl`
        // would read as a working target on a sheet and then apply in
        // every case its author meant to exclude.
        assert_eq!(parse_target("resist.fire.nonmagicl"), None);
        assert_eq!(parse_target("resist.fire.magical"), None);
        assert_eq!(parse_target("resist.nonmagical"), None);
    }

    #[test]
    fn a_werewolf_halves_a_sword_and_not_a_magic_sword() {
        let s = standing(&[qualified("slashing", Degree::Resistant, "Werewolf")]);
        assert_eq!(against(9, &s, "slashing", false), 4, "an ordinary blade");
        assert_eq!(against(9, &s, "slashing", true), 9, "a +1 blade lands in full");
    }

    #[test]
    fn nothing_is_said_about_a_resistance_that_did_not_fire() {
        // "resistant slashing - 9 instead of 9" reads as a rule that
        // fired. Silence is the honest answer.
        let s = standing(&[qualified("slashing", Degree::Resistant, "Werewolf")]);
        assert!(said(9, &s, "slashing", false).is_some());
        assert_eq!(said(9, &s, "slashing", true), None);
    }

    #[test]
    fn an_unconditional_resistance_still_meets_a_magic_weapon() {
        // The Unt'garoth do not stop resisting fire because the torch
        // was enchanted.
        let s = standing(&[src("fire", Degree::Resistant, "Unt'garoth")]);
        assert_eq!(against(10, &s, "fire", true), 5);
        assert_eq!(against(10, &s, "fire", false), 5);
    }

    #[test]
    fn a_creature_may_have_reasons_of_both_kinds() {
        // Immune to fire from ordinary weapons, merely resistant to it
        // in general. A flaming sword meets the resistance and not the
        // immunity - which is why the two are settled over two sets of
        // sources rather than resolved once and filtered.
        let s = standing(&[
            qualified("fire", Degree::Immune, "Hide"),
            src("fire", Degree::Resistant, "Unt'garoth"),
        ]);
        assert_eq!(against(10, &s, "fire", false), 0, "immune to an ordinary flame");
        assert_eq!(against(10, &s, "fire", true), 5, "resistant to an enchanted one");
    }

    #[test]
    fn both_sources_are_named_whichever_one_applied() {
        // A sheet saying "resistant to fire" invites the question "why",
        // and the answer does not change because this particular sword
        // was enchanted.
        let s = standing(&[
            qualified("slashing", Degree::Resistant, "Werewolf"),
            src("slashing", Degree::Resistant, "Barkskin"),
        ]);
        assert_eq!(s[0].from, vec!["Werewolf".to_string(), "Barkskin".to_string()]);
        assert_eq!(s[0].vs_magic, Some(Degree::Resistant), "Barkskin still applies");
    }

    #[test]
    fn a_qualified_resistance_and_an_ordinary_vulnerability_still_cancel() {
        let s = standing(&[
            qualified("slashing", Degree::Resistant, "Werewolf"),
            src("slashing", Degree::Vulnerable, "Curse"),
        ]);
        assert_eq!(against(10, &s, "slashing", false), 10, "half then double");
        // Against a magic sword only the vulnerability is left.
        assert_eq!(against(10, &s, "slashing", true), 20);
    }

    #[test]
    fn the_target_round_trips_through_a_choice() {
        let (degree, kind, nonmagical) =
            pick("resist.acid|cold|fire.nonmagical", Some("cold")).unwrap();
        assert!(nonmagical, "a choice must not lose the qualifier");
        assert_eq!(target_for(degree, &kind, nonmagical), "resist.cold.nonmagical");
        assert_eq!(parse_target(&target_for(degree, &kind, nonmagical)).unwrap().2, true);
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
            (Degree::Resistant, "poison".into(), false)
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
        assert_eq!(got[0].degree, Some(Degree::Resistant));
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
        assert_eq!(got[0].degree, Some(Degree::Resistant));
        assert_eq!(got[0].from.len(), 2);
    }

    #[test]
    fn immunity_beats_resistance() {
        let got = standing(&[
            src("poison", Degree::Resistant, "Dwarf"),
            src("poison", Degree::Immune, "Purity of Body"),
        ]);
        assert_eq!(got[0].degree, Some(Degree::Immune));
    }

    #[test]
    fn immunity_also_beats_vulnerability() {
        // Immunity is a different kind of fact, not the end of a scale.
        let got = standing(&[
            src("fire", Degree::Vulnerable, "a curse"),
            src("fire", Degree::Immune, "Holy Aura"),
        ]);
        assert_eq!(got[0].degree, Some(Degree::Immune));
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
        assert_eq!(against(10, &s, "fire", false), 5);
        assert_eq!(against(7, &s, "fire", false), 3, "rounded down, which is 5e");
    }

    #[test]
    fn one_point_against_resistance_is_nothing() {
        // The rule rather than an edge case.
        let s = standing(&[src("fire", Degree::Resistant, "Unt'garoth")]);
        assert_eq!(against(1, &s, "fire", false), 0);
    }

    #[test]
    fn immunity_is_none_of_it() {
        let s = standing(&[src("poison", Degree::Immune, "Purity of Body")]);
        assert_eq!(against(40, &s, "poison", false), 0);
    }

    #[test]
    fn vulnerability_doubles() {
        let s = standing(&[src("cold", Degree::Vulnerable, "a curse")]);
        assert_eq!(against(7, &s, "cold", false), 14);
    }

    #[test]
    fn a_type_nobody_has_an_opinion_about_passes_through() {
        let s = standing(&[src("fire", Degree::Resistant, "Unt'garoth")]);
        assert_eq!(against(10, &s, "slashing", false), 10);
    }

    #[test]
    fn negative_damage_is_not_healing_by_the_back_door() {
        let s = standing(&[src("fire", Degree::Vulnerable, "x")]);
        assert_eq!(against(-5, &s, "fire", false), 0);
    }

    /* ---------- explaining itself ---------- */

    #[test]
    fn the_line_says_why_and_by_how_much() {
        let s = standing(&[src("fire", Degree::Resistant, "Unt'garoth")]);
        assert_eq!(
            said(10, &s, "fire", false).as_deref(),
            Some("resistant fire (Unt'garoth) - 5 instead of 10")
        );
    }

    #[test]
    fn nothing_to_say_about_a_type_nobody_resists() {
        let s = standing(&[src("fire", Degree::Resistant, "Unt'garoth")]);
        assert_eq!(said(10, &s, "cold", false), None);
    }

    #[test]
    fn every_damage_type_five_e_has_is_here() {
        assert_eq!(TYPES.len(), 13);
        for t in ["acid", "force", "necrotic", "psychic", "radiant", "thunder"] {
            assert!(TYPES.contains(&t), "{} is missing", t);
        }
    }
}
