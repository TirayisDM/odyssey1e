//! What a character's classes have given them, and what they still owe
//! a decision on.
//!
//! 087. DERIVED, NEVER ACCUMULATED. A character's features are not
//! granted at level-up and stored - they are what their class rows say
//! at their current levels, worked out on every read. Nothing to hand
//! out and nothing to forget; dropping a class level takes its features
//! with it without a reversal step, and a DM who corrects the catalogue
//! corrects every character at once.
//!
//! The one thing that cannot be derived is a CHOICE somebody made, and
//! that is the only thing `character_choices` stores.
//!
//! PER CLASS AND PER CLASS LEVEL, which is the whole reason this is not
//! a filter on a single number. A Fighter 4 / Bard 1 has the Fighter's
//! first four levels and the Bard's first - not the first five of
//! either, and not anything a level 5 gets. Multiclassing is where a
//! "level >= feature.level" written against the character's total goes
//! quietly wrong.
//!
//! MOST FEATURES ARE NOT A CHOICE. Second Wind, Action Surge, Sneak
//! Attack - they arrive. 5e makes you decide in four places and
//! `choose_from` names which; everything else has it NULL, which is
//! most of the 176 rows 088 seeded.

use crate::multiclass::Taken;
use serde::{Deserialize, Serialize};

/// One row of the catalogue: what a class grants, and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feature {
    pub class_key: String,
    pub level: i64,
    pub key: String,
    pub name: String,
    pub text: Option<String>,
    /// What list this makes you choose from - ability, skill,
    /// fighting_style, subclass. None means it simply happens.
    pub choose_from: Option<String>,
    /// How many to choose. An Ability Score Improvement is 2 picks of
    /// +1, which is exactly 5e's "one by 2, or two by 1".
    pub picks: i64,
}

/// A feature this character has, with whatever they chose for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Held {
    pub feature: Feature,
    /// What was chosen, in pick order. Empty for an automatic feature
    /// and for one nobody has decided yet.
    pub chosen: Vec<String>,
    /// How many picks are still outstanding. Zero for an automatic
    /// feature, which is the common case - so "what do I still owe" is
    /// `owed > 0` and reads the same for both kinds.
    pub owed: i64,
}

/// One recorded choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub class_key: String,
    pub feature_key: String,
    pub pick: i64,
    pub choice: String,
}

/// Everything these classes have granted, in reading order.
///
/// THE ORDER IS THE CLASSES' ORDER, THEN LEVEL, THEN NAME. A sheet
/// reads down a career: what the leading class gave first, earliest
/// level first. `classes` arrives lead-first from the sheet, so this
/// does not re-sort it and cannot disagree with the headline above it.
///
/// A FEATURE FOR A CLASS THEY DO NOT HOLD IS NOT THEIRS, and a feature
/// above their level in a class they do hold is not theirs yet. Both
/// fall out of the same comparison, which is why there is no second
/// filter for the second case.
pub fn held(classes: &[Taken], catalogue: &[Feature], choices: &[Choice]) -> Vec<Held> {
    let mut out: Vec<Held> = Vec::new();

    for taken in classes {
        let mut mine: Vec<&Feature> = catalogue
            .iter()
            .filter(|f| f.class_key == taken.key && f.level <= taken.level)
            .collect();
        mine.sort_by(|a, b| a.level.cmp(&b.level).then(a.name.cmp(&b.name)));

        for f in mine {
            // IN PICK ORDER, because an Ability Score Improvement is
            // two separate +1s that may land on different abilities and
            // "Strength, Constitution" reads differently from
            // "Constitution, Strength" to the person who chose.
            let mut picked: Vec<&Choice> = choices
                .iter()
                .filter(|c| c.class_key == f.class_key && c.feature_key == f.key)
                .collect();
            picked.sort_by_key(|c| c.pick);

            let chosen: Vec<String> = picked.iter().map(|c| c.choice.clone()).collect();
            // AUTOMATIC FEATURES OWE NOTHING, however many picks the
            // row happens to carry - `picks` is ignored when there is
            // nothing to choose from, which the column comment says.
            let owed = match f.choose_from {
                Some(_) => (f.picks - chosen.len() as i64).max(0),
                None => 0,
            };
            out.push(Held { feature: f.clone(), chosen, owed });
        }
    }
    out
}

/// What the chosen features add to ability scores, named.
///
/// 091. An Ability Score Improvement was recorded and applied to
/// nothing. The choice went into `character_choices`, the sheet listed
/// it as decided, and the score did not move - the same shape as
/// `saving_throws`, which 055 stored and 078 finally read, and
/// `price_override`, which 049 added and 070 finally applied. A feature
/// that records a decision and changes no number is half a feature.
///
/// ONE SOURCE PER FEATURE PER ABILITY. Two picks into Strength read as
/// "Ability Score Improvement +2" rather than two +1s, because that is
/// one decision; two picks split across Strength and Dexterity are two
/// sources of +1, because they are two.
///
/// READ OFF `held`, WHICH IS THE POINT. A character who drops the level
/// that granted an ASI no longer holds that feature, so the bump
/// vanishes with it and nothing has to be undone - the same property
/// that makes features derived rather than granted.
///
/// Returns (ability code, source name, value), ordered by ability so
/// a sheet gets the same answer twice.
pub fn ability_sources(held: &[Held]) -> Vec<(String, String, i64)> {
    let mut out: Vec<(String, String, i64)> = Vec::new();
    for h in held {
        if h.feature.choose_from.as_deref() != Some("ability") {
            continue;
        }
        for code in &h.chosen {
            match out
                .iter_mut()
                .find(|(a, n, _)| a == code && *n == h.feature.name)
            {
                Some(found) => found.2 += 1,
                None => out.push((code.clone(), h.feature.name.clone(), 1)),
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    out
}

/// The features still waiting on a decision.
///
/// WHAT A LEVEL-UP ACTUALLY ASKS OF SOMEBODY. Everything else about
/// levelling is automatic, so this is the short list a sheet should put
/// in front of a player rather than making them read the long one.
pub fn outstanding(held: &[Held]) -> Vec<&Held> {
    held.iter().filter(|h| h.owed > 0).collect()
}

/// Whether a choice may be recorded against this feature.
///
/// REFUSED FOR THREE DIFFERENT REASONS and they are worth telling
/// apart, because each is a different mistake: the feature does not ask
/// for anything, the character has not earned it yet, or every pick is
/// already spoken for. The last is the lock - 5e retrains at the DM's
/// say-so and the policies enforce that, so this only has to say that
/// the slot is full.
pub fn may_choose(h: &Held) -> Result<(), String> {
    if h.feature.choose_from.is_none() {
        return Err(format!("{} is not a choice", h.feature.name));
    }
    if h.owed <= 0 {
        return Err(format!(
            "{} is already decided - a DM can change it",
            h.feature.name
        ));
    }
    Ok(())
}

/* ============================ READING ============================ */

/// Every character's ability sources, for a whole roster at once.
///
/// 091. TWO REQUESTS FOR ANY NUMBER OF CHARACTERS, which is why this
/// takes a list rather than being called per person: `load_effective`
/// reads a target list or an initiative order, and one round trip per
/// combatant would be a real cost on a screen that already asks for
/// four things.
///
/// EMPTY FOR THE CLASSLESS, and that is every monster - so the usual
/// case costs nothing but the early return.
pub fn load_ability_sources(
    token: &str,
    game_id: &str,
    per_character: &[(String, Vec<crate::multiclass::Taken>)],
) -> Result<std::collections::HashMap<String, Vec<(String, String, i64)>>, String> {
    use std::collections::HashMap;

    let ids: Vec<String> = per_character
        .iter()
        .filter(|(_, cs)| !cs.is_empty())
        .map(|(id, _)| id.clone())
        .collect();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let keys: Vec<String> = per_character
        .iter()
        .flat_map(|(_, cs)| cs.iter().map(|t| t.key.clone()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // ONLY THE FEATURES THAT CAN MOVE A SCORE. Every other row in the
    // catalogue is irrelevant here, and the filter is the difference
    // between two rows per class and twenty.
    let rows = crate::supabase::rest_get(
        token,
        "class_features",
        &[
            ("select", "class_key,level,key,name,text,choose_from,picks"),
            ("class_key", &format!("in.({})", keys.join(","))),
            ("choose_from", "eq.ability"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
        ],
    )?;
    let catalogue: Vec<Feature> = rows
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(feature_from_row)
        .collect();

    let rows = crate::supabase::rest_get(
        token,
        "character_choices",
        &[
            ("select", "character_id,class_key,feature_key,pick,choice"),
            ("character_id", &format!("in.({})", ids.join(","))),
        ],
    )?;
    let mut mine: HashMap<String, Vec<Choice>> = HashMap::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let Some(cid) = r.get("character_id").and_then(|v| v.as_str()) else {
            continue;
        };
        mine.entry(cid.to_string()).or_default().push(Choice {
            class_key: as_str(r, "class_key"),
            feature_key: as_str(r, "feature_key"),
            pick: r.get("pick").and_then(|v| v.as_i64()).unwrap_or(1),
            choice: as_str(r, "choice"),
        });
    }

    let mut out = HashMap::new();
    for (id, classes) in per_character {
        let empty = Vec::new();
        let chosen = mine.get(id).unwrap_or(&empty);
        let got = ability_sources(&held(classes, &catalogue, chosen));
        if !got.is_empty() {
            out.insert(id.clone(), got);
        }
    }
    Ok(out)
}

pub fn feature_from_row(r: &serde_json::Value) -> Feature {
    Feature {
        class_key: as_str(r, "class_key"),
        level: r.get("level").and_then(|v| v.as_i64()).unwrap_or(1),
        key: as_str(r, "key"),
        name: as_str(r, "name"),
        text: r.get("text").and_then(|v| v.as_str()).map(str::to_string),
        choose_from: r.get("choose_from").and_then(|v| v.as_str()).map(str::to_string),
        picks: r.get("picks").and_then(|v| v.as_i64()).unwrap_or(1),
    }
}

fn as_str(r: &serde_json::Value, key: &str) -> String {
    r.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn f(class: &str, level: i64, key: &str, name: &str) -> Feature {
        Feature {
            class_key: class.into(),
            level,
            key: key.into(),
            name: name.into(),
            text: None,
            choose_from: None,
            picks: 1,
        }
    }

    fn choice_feature(class: &str, level: i64, key: &str, name: &str, from: &str, picks: i64) -> Feature {
        Feature { choose_from: Some(from.into()), picks, ..f(class, level, key, name) }
    }

    fn t(key: &str, level: i64) -> Taken {
        Taken { key: key.into(), level, hit_die: 10 }
    }

    fn chose(class: &str, key: &str, pick: i64, what: &str) -> Choice {
        Choice {
            class_key: class.into(),
            feature_key: key.into(),
            pick,
            choice: what.into(),
        }
    }

    fn fighter_catalogue() -> Vec<Feature> {
        vec![
            choice_feature("fighter", 1, "fighting_style", "Fighting Style", "fighting_style", 1),
            f("fighter", 1, "second_wind", "Second Wind"),
            f("fighter", 2, "action_surge", "Action Surge"),
            f("fighter", 3, "martial_archetype", "Martial Archetype"),
            choice_feature("fighter", 4, "asi_4", "Ability Score Improvement", "ability", 2),
            f("fighter", 5, "extra_attack", "Extra Attack"),
            f("bard", 1, "bardic_inspiration", "Bardic Inspiration"),
            choice_feature("bard", 3, "expertise_1", "Expertise", "skill", 2),
        ]
    }

    /* ---------- what you have ---------- */

    #[test]
    fn a_level_one_fighter_has_the_first_level_and_no_more() {
        let got = held(&[t("fighter", 1)], &fighter_catalogue(), &[]);
        let names: Vec<&str> = got.iter().map(|h| h.feature.name.as_str()).collect();
        assert_eq!(names, vec!["Fighting Style", "Second Wind"]);
    }

    #[test]
    fn levelling_brings_the_earlier_ones_with_it() {
        let got = held(&[t("fighter", 5)], &fighter_catalogue(), &[]);
        assert_eq!(got.len(), 6);
        assert_eq!(got.last().unwrap().feature.name, "Extra Attack");
    }

    #[test]
    fn a_feature_above_your_level_is_not_yours_yet() {
        let got = held(&[t("fighter", 4)], &fighter_catalogue(), &[]);
        assert!(!got.iter().any(|h| h.feature.key == "extra_attack"));
    }

    #[test]
    fn another_classs_features_are_not_yours_at_all() {
        let got = held(&[t("fighter", 20)], &fighter_catalogue(), &[]);
        assert!(!got.iter().any(|h| h.feature.class_key == "bard"));
    }

    /* ---------- the multiclass case ---------- */

    #[test]
    fn each_class_is_read_at_its_own_level() {
        // Falon: Fighter 4 / Bard 1. He has the fighter's first four
        // and the bard's first - NOT the first five of either, which is
        // what filtering on the character's total level would give.
        let got = held(
            &[t("fighter", 4), t("bard", 1)],
            &fighter_catalogue(),
            &[],
        );
        let names: Vec<&str> = got.iter().map(|h| h.feature.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "Fighting Style",
                "Second Wind",
                "Action Surge",
                "Martial Archetype",
                "Ability Score Improvement",
                "Bardic Inspiration"
            ]
        );
    }

    #[test]
    fn a_total_of_five_does_not_buy_the_fighters_fifth() {
        // The bug this guards: Fighter 4 / Bard 1 is level 5 overall
        // and must NOT have Extra Attack, which a fighter gets at
        // their own fifth.
        let got = held(&[t("fighter", 4), t("bard", 1)], &fighter_catalogue(), &[]);
        assert!(!got.iter().any(|h| h.feature.key == "extra_attack"));
    }

    #[test]
    fn the_order_follows_the_classes_as_given() {
        // Lead first, because that is the order the sheet's headline
        // reads in and this must not disagree with it.
        let got = held(&[t("bard", 3), t("fighter", 1)], &fighter_catalogue(), &[]);
        assert_eq!(got.first().unwrap().feature.class_key, "bard");
        assert_eq!(got.last().unwrap().feature.class_key, "fighter");
    }

    /* ---------- what is owed ---------- */

    #[test]
    fn an_automatic_feature_owes_nothing() {
        let got = held(&[t("fighter", 2)], &fighter_catalogue(), &[]);
        let surge = got.iter().find(|h| h.feature.key == "action_surge").unwrap();
        assert_eq!(surge.owed, 0);
        assert!(surge.chosen.is_empty());
    }

    #[test]
    fn an_undecided_choice_owes_its_picks() {
        let got = held(&[t("fighter", 4)], &fighter_catalogue(), &[]);
        let asi = got.iter().find(|h| h.feature.key == "asi_4").unwrap();
        assert_eq!(asi.owed, 2);
    }

    #[test]
    fn a_half_decided_choice_owes_the_rest() {
        let got = held(
            &[t("fighter", 4)],
            &fighter_catalogue(),
            &[chose("fighter", "asi_4", 1, "str")],
        );
        let asi = got.iter().find(|h| h.feature.key == "asi_4").unwrap();
        assert_eq!(asi.owed, 1);
        assert_eq!(asi.chosen, vec!["str"]);
    }

    #[test]
    fn a_decided_choice_owes_nothing_and_shows_both() {
        let got = held(
            &[t("fighter", 4)],
            &fighter_catalogue(),
            &[chose("fighter", "asi_4", 1, "str"), chose("fighter", "asi_4", 2, "con")],
        );
        let asi = got.iter().find(|h| h.feature.key == "asi_4").unwrap();
        assert_eq!(asi.owed, 0);
        assert_eq!(asi.chosen, vec!["str", "con"]);
    }

    #[test]
    fn picks_come_back_in_the_order_they_were_made() {
        // Not the order the rows arrived in. "Strength, Constitution"
        // reads differently from the reverse to whoever chose.
        let got = held(
            &[t("fighter", 4)],
            &fighter_catalogue(),
            &[chose("fighter", "asi_4", 2, "con"), chose("fighter", "asi_4", 1, "str")],
        );
        let asi = got.iter().find(|h| h.feature.key == "asi_4").unwrap();
        assert_eq!(asi.chosen, vec!["str", "con"]);
    }

    #[test]
    fn a_choice_against_another_classs_feature_of_the_same_key_does_not_leak() {
        // Both a fighter and a rogue have `asi_4`. The class key is
        // half of what identifies a choice, and without it a
        // Fighter/Rogue would see one decision answer both.
        let mut cat = fighter_catalogue();
        cat.push(choice_feature("rogue", 4, "asi_4", "Ability Score Improvement", "ability", 2));
        let got = held(
            &[t("fighter", 4), t("rogue", 4)],
            &cat,
            &[chose("fighter", "asi_4", 1, "str"), chose("fighter", "asi_4", 2, "con")],
        );
        let rogue = got
            .iter()
            .find(|h| h.feature.class_key == "rogue" && h.feature.key == "asi_4")
            .unwrap();
        assert_eq!(rogue.owed, 2, "the rogue still owes their own");
    }

    #[test]
    fn outstanding_is_the_short_list() {
        let got = held(&[t("fighter", 4)], &fighter_catalogue(), &[]);
        let owed = outstanding(&got);
        let names: Vec<&str> = owed.iter().map(|h| h.feature.name.as_str()).collect();
        assert_eq!(names, vec!["Fighting Style", "Ability Score Improvement"]);
    }

    /* ---------- what a choice adds to a score ---------- */

    #[test]
    fn an_ability_improvement_raises_what_was_chosen() {
        // Falon's Fighter 4: one point into Strength, one into
        // Dexterity. Recorded since 087 and applied to nothing until
        // this existed.
        let got = held(
            &[t("fighter", 4)],
            &fighter_catalogue(),
            &[chose("fighter", "asi_4", 1, "str"), chose("fighter", "asi_4", 2, "dex")],
        );
        assert_eq!(
            ability_sources(&got),
            vec![
                ("dex".to_string(), "Ability Score Improvement".to_string(), 1),
                ("str".to_string(), "Ability Score Improvement".to_string(), 1),
            ]
        );
    }

    #[test]
    fn both_picks_into_one_ability_are_one_source_of_two() {
        // One decision, so one line on the sheet - "+2" rather than
        // two "+1"s a reader has to add up.
        let got = held(
            &[t("fighter", 4)],
            &fighter_catalogue(),
            &[chose("fighter", "asi_4", 1, "str"), chose("fighter", "asi_4", 2, "str")],
        );
        assert_eq!(
            ability_sources(&got),
            vec![("str".to_string(), "Ability Score Improvement".to_string(), 2)]
        );
    }

    #[test]
    fn an_undecided_improvement_adds_nothing() {
        let got = held(&[t("fighter", 4)], &fighter_catalogue(), &[]);
        assert!(ability_sources(&got).is_empty());
    }

    #[test]
    fn a_choice_that_is_not_an_ability_adds_nothing() {
        let got = held(
            &[t("fighter", 1)],
            &fighter_catalogue(),
            &[chose("fighter", "fighting_style", 1, "defense")],
        );
        assert!(ability_sources(&got).is_empty());
    }

    #[test]
    fn dropping_the_level_takes_the_bump_with_it() {
        // The property that makes derived features worth the trouble.
        // The choice rows are still there and simply unread.
        let picks = [chose("fighter", "asi_4", 1, "str"), chose("fighter", "asi_4", 2, "dex")];
        assert_eq!(ability_sources(&held(&[t("fighter", 4)], &fighter_catalogue(), &picks)).len(), 2);
        assert!(ability_sources(&held(&[t("fighter", 3)], &fighter_catalogue(), &picks)).is_empty());
    }

    /* ---------- whether a choice may be made ---------- */

    #[test]
    fn you_cannot_choose_something_that_is_not_a_choice() {
        let got = held(&[t("fighter", 2)], &fighter_catalogue(), &[]);
        let surge = got.iter().find(|h| h.feature.key == "action_surge").unwrap();
        assert!(may_choose(surge).is_err());
    }

    #[test]
    fn you_can_choose_what_is_still_owed() {
        let got = held(&[t("fighter", 4)], &fighter_catalogue(), &[]);
        let asi = got.iter().find(|h| h.feature.key == "asi_4").unwrap();
        assert!(may_choose(asi).is_ok());
    }

    #[test]
    fn a_full_feature_says_a_dm_can_change_it() {
        let got = held(
            &[t("fighter", 1)],
            &fighter_catalogue(),
            &[chose("fighter", "fighting_style", 1, "defense")],
        );
        let style = got.iter().find(|h| h.feature.key == "fighting_style").unwrap();
        let err = may_choose(style).unwrap_err();
        assert!(err.contains("DM"), "unhelpful: {}", err);
    }

    /* ---------- edges ---------- */

    #[test]
    fn no_classes_is_no_features() {
        assert!(held(&[], &fighter_catalogue(), &[]).is_empty());
    }

    #[test]
    fn an_empty_catalogue_is_not_a_panic() {
        assert!(held(&[t("fighter", 20)], &[], &[]).is_empty());
    }

    #[test]
    fn a_stale_choice_for_a_feature_they_no_longer_have_is_simply_unread() {
        // Dropping from Fighter 4 to Fighter 2 leaves the ASI choice
        // rows behind. They are not shown and not an error - and they
        // are still there if the level comes back.
        let got = held(
            &[t("fighter", 2)],
            &fighter_catalogue(),
            &[chose("fighter", "asi_4", 1, "str")],
        );
        assert!(!got.iter().any(|h| h.feature.key == "asi_4"));
    }
}
